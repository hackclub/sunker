use crate::http;
use actix_web::{HttpResponse, get, rt::time::sleep, web};
use futures_util::{StreamExt, stream};
use image::{DynamicImage, GrayImage, Rgba, RgbaImage, imageops::FilterType};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, LazyLock, Mutex, OnceLock, PoisonError, RwLock},
    time::Duration,
};
use tract_onnx::prelude::*;

const MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/silueta.onnx"));
const SIDE: usize = 320;
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];
const CUTOFF: f32 = 160.0;
const RAMP: f32 = 4.0;
const MAX_DOWNLOAD: usize = 20_000_000;
const MAX_OUTPUT: usize = 100_000;
const LIGHT: u8 = 0xb0;
const NEUTRAL: u8 = 16;
const MATCH: u8 = 24;
const KEEP: u8 = 64;
const GROW: usize = SIDE / 32;
const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif", "image/svg+xml"];
const MAX_ATTEMPTS: u32 = 4;
const MAX_WAIT: u64 = 10;

#[derive(Clone)]
struct Stored {
    url: String,
    content_type: String,
    bytes: web::Bytes,
    done: bool,
}

type Model = Arc<TypedRunnableModel>;

static IMAGES: LazyLock<RwLock<HashMap<i32, Stored>>> = LazyLock::new(Default::default);
static PENDING: LazyLock<Mutex<HashMap<i32, String>>> = LazyLock::new(Default::default);
static LOADED: OnceLock<Option<Model>> = OnceLock::new();

pub fn refresh(pool: &PgPool) {
    let pool = pool.clone();
    actix_web::rt::spawn(async move {
        let wanted: Vec<(i32, String)> =
            match sqlx::query_as("SELECT id, image_url FROM prize WHERE deleted_at IS NULL AND image_url IS NOT NULL")
                .fetch_all(&pool)
                .await
            {
                Ok(wanted) => wanted,
                Err(e) => {
                    eprintln!("listing prize images failed: {e}");
                    return;
                }
            };
        let work: Vec<(i32, String)> = {
            let mut images = IMAGES.write().unwrap_or_else(PoisonError::into_inner);
            let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
            let urls: HashMap<i32, &String> = wanted.iter().map(|(id, url)| (*id, url)).collect();
            images.retain(|id, stored| urls.get(id) == Some(&&stored.url));
            let work: Vec<(i32, String)> = wanted
                .iter()
                .filter(|(id, url)| !images.contains_key(id) && pending.get(id) != Some(url))
                .cloned()
                .collect();
            pending.extend(work.iter().cloned());
            work
        };
        let workers = std::thread::available_parallelism().map_or(1, |cores| cores.get());
        stream::iter(work).for_each_concurrent(workers, |(id, url)| load(id, url)).await;
    });
}

fn wanted(id: i32, url: &str) -> bool {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner).get(&id).is_some_and(|pending| pending == url)
}

fn store(id: i32, stored: Stored) {
    if wanted(id, &stored.url) {
        IMAGES.write().unwrap_or_else(PoisonError::into_inner).insert(id, stored);
    }
}

async fn load(id: i32, url: String) {
    match download(&url).await {
        Ok((content_type, bytes)) => {
            store(id, Stored { url: url.clone(), content_type: content_type.clone(), bytes: bytes.clone(), done: false });
            let source = bytes.clone();
            let stored = match web::block(move || cut_out(&source)).await {
                Ok(Ok(Some((content_type, bytes)))) => Stored { url: url.clone(), content_type, bytes, done: true },
                Ok(Ok(None)) => Stored { url: url.clone(), content_type, bytes, done: true },
                Ok(Err(e)) => {
                    eprintln!("removing the background of prize {id}'s image failed: {e:#}");
                    Stored { url: url.clone(), content_type, bytes, done: true }
                }
                Err(e) => {
                    eprintln!("background removal for prize {id} stopped: {e:#}");
                    return;
                }
            };
            store(id, stored);
        }
        Err(e) => eprintln!("loading prize {id}'s image failed: {e:#}"),
    }
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    if pending.get(&id) == Some(&url) {
        pending.remove(&id);
    }
}

fn model() -> Option<&'static Model> {
    LOADED
        .get_or_init(|| {
            tract_onnx::onnx()
                .model_for_read(&mut Cursor::new(MODEL))
                .and_then(|model| model.with_input_fact(0, f32::fact([1, 3, SIDE, SIDE]).into()))
                .and_then(|model| model.into_optimized())
                .and_then(|model| model.into_runnable())
                .map_err(|e| eprintln!("loading the background removal model failed: {e:#}"))
                .ok()
        })
        .as_ref()
}

async fn download(url: &str) -> anyhow::Result<(String, web::Bytes)> {
    let mut attempt = 1;
    let mut response = loop {
        let response = http::CLIENT.get(url).send().await?;
        if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS || attempt == MAX_ATTEMPTS {
            break response.error_for_status()?;
        }
        let wait = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
            .max(2u64.pow(attempt))
            .min(MAX_WAIT);
        sleep(Duration::from_secs(wait)).await;
        attempt += 1;
    };
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !IMAGE_TYPES.contains(&content_type.as_str()) {
        anyhow::bail!("unsupported content type {content_type:?}");
    }
    if response.content_length().is_some_and(|length| length > MAX_DOWNLOAD as u64) {
        anyhow::bail!("image is too large");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > MAX_DOWNLOAD {
            anyhow::bail!("image is too large");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok((content_type, bytes.into()))
}

fn distance(pixel: &Rgba<u8>, colour: [u8; 3]) -> u8 {
    pixel.0[..3].iter().zip(colour).map(|(&channel, target)| channel.abs_diff(target)).max().unwrap_or(0)
}

fn backdrop(image: &RgbaImage) -> Option<[u8; 3]> {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let edge: Vec<&Rgba<u8>> = (0..width)
        .flat_map(|x| [(x, 0), (x, height - 1)])
        .chain((1..height.saturating_sub(1)).flat_map(|y| [(0, y), (width - 1, y)]))
        .map(|(x, y)| image.get_pixel(x, y))
        .collect();
    let opaque: Vec<&&Rgba<u8>> = edge.iter().filter(|pixel| pixel.0[3] == 255).collect();
    if opaque.len() * 2 <= edge.len() {
        return None;
    }
    let colour: [u8; 3] = std::array::from_fn(|c| {
        let mut values: Vec<u8> = opaque.iter().map(|pixel| pixel.0[c]).collect();
        values.sort_unstable();
        values[values.len() / 2]
    });
    let (low, high) = (*colour.iter().min()?, *colour.iter().max()?);
    if low < LIGHT || high - low > NEUTRAL {
        return None;
    }
    (opaque.iter().filter(|pixel| distance(pixel, colour) <= MATCH).count() * 2 > edge.len()).then_some(colour)
}

fn cut_out(bytes: &[u8]) -> anyhow::Result<Option<(String, web::Bytes)>> {
    if image::guess_format(bytes).is_err() {
        return Ok(None);
    }
    let image = image::load_from_memory(bytes)?;
    let mut rgba = image.to_rgba8();
    let Some(backdrop) = backdrop(&rgba) else {
        return Ok(None);
    };
    let Some(model) = model() else {
        return Ok(None);
    };
    let (mask, grown) = matte(model, &image)?;
    for ((pixel, alpha), near) in rgba.pixels_mut().zip(mask.pixels()).zip(grown.pixels()) {
        let keep = (f32::from(distance(pixel, backdrop).saturating_sub(MATCH)) * 255.0 / f32::from(KEEP - MATCH)).min(255.0) as u8;
        pixel.0[3] = alpha.0[0].max(keep.min(near.0[0]));
    }
    Ok(Some(("image/webp".into(), compress(rgba).into())))
}

fn grow(mask: &GrayImage) -> GrayImage {
    let side = SIDE as u32;
    let reach = |c: u32| c.saturating_sub(GROW as u32)..=(c + GROW as u32).min(side - 1);
    let wide = GrayImage::from_fn(side, side, |x, y| image::Luma([reach(x).map(|x| mask.get_pixel(x, y).0[0]).max().unwrap_or(0)]));
    GrayImage::from_fn(side, side, |x, y| image::Luma([reach(y).map(|y| wide.get_pixel(x, y).0[0]).max().unwrap_or(0)]))
}

fn sharpen(mask: &GrayImage, width: u32, height: u32) -> GrayImage {
    let mut mask = image::imageops::resize(mask, width, height, FilterType::Lanczos3);
    for pixel in mask.pixels_mut() {
        let ramp = (f32::from(pixel.0[0]) - CUTOFF) * 255.0 / (255.0 - CUTOFF) * RAMP;
        pixel.0[0] = ramp.clamp(0.0, 255.0) as u8;
    }
    mask
}

fn matte(model: &Model, image: &DynamicImage) -> anyhow::Result<(GrayImage, GrayImage)> {
    let small = image::imageops::resize(&image.to_rgb8(), SIDE as u32, SIDE as u32, FilterType::Lanczos3);
    let input = tract_ndarray::Array4::from_shape_fn((1, 3, SIDE, SIDE), |(_, c, y, x)| {
        (f32::from(small.get_pixel(x as u32, y as u32).0[c]) / 255.0 - MEAN[c]) / STD[c]
    });
    let outputs = model.run(tvec!(Tensor::from(input).into()))?;
    let logits = outputs[0].to_plain_array_view::<f32>()?;
    let mask = GrayImage::from_fn(SIDE as u32, SIDE as u32, |x, y| {
        let value = logits[[0, 0, y as usize, x as usize]];
        image::Luma([(255.0_f32 / (1.0 + (-value).exp())).round() as u8])
    });
    Ok((sharpen(&mask, image.width(), image.height()), sharpen(&grow(&mask), image.width(), image.height())))
}

fn compress(image: RgbaImage) -> Vec<u8> {
    let mut image = DynamicImage::ImageRgba8(image);
    loop {
        let rgba = image.to_rgba8();
        for quality in [80.0, 65.0, 50.0] {
            let webp = webp::Encoder::from_rgba(&rgba, image.width(), image.height()).encode(quality);
            if webp.len() <= MAX_OUTPUT {
                return webp.to_vec();
            }
        }
        image = image.resize((image.width() * 3 / 4).max(1), (image.height() * 3 / 4).max(1), FilterType::Lanczos3);
    }
}

#[get("/api/prizes/{id:\\d+}/image")]
pub async fn serve(id: web::Path<i32>) -> HttpResponse {
    let images = IMAGES.read().unwrap_or_else(PoisonError::into_inner);
    match images.get(&id.into_inner()) {
        Some(stored) => HttpResponse::Ok()
            .content_type(stored.content_type.as_str())
            .insert_header(("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'; sandbox"))
            .insert_header(("Cache-Control", if stored.done { "public, max-age=86400" } else { "no-store" }))
            .body(stored.bytes.clone()),
        None => HttpResponse::NotFound().insert_header(("Cache-Control", "no-store")).finish(),
    }
}
