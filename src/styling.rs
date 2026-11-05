use crate::admin::{self, Permission};
use actix_web::http::header::{CACHE_CONTROL, ContentEncoding};
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use futures_util::StreamExt;
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

include!(concat!(env!("OUT_DIR"), "/originals.rs"));

const MAX_SIDE: i32 = 8192;
const PERSON: &str = "jsonb_build_object('id', x.id, 'name', nullif(concat_ws(' ', x.first_name, x.last_name), ''))";

struct Active {
    id: i32,
    content_type: String,
    data: web::Bytes,
}

#[derive(Default)]
struct Cache {
    images: HashMap<String, Active>,
    manifest: Value,
}

static CACHE: LazyLock<RwLock<Cache>> = LazyLock::new(Default::default);
static REVISION: AtomicU64 = AtomicU64::new(0);
static RELOADING: futures_util::lock::Mutex<()> = futures_util::lock::Mutex::new(());

type Outcome = sqlx::Result<Result<(), HttpResponse>>;

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

fn bytes<const N: usize>(data: &[u8], at: usize) -> Option<[u8; N]> {
    data.get(at..at + N)?.try_into().ok()
}

fn measure(data: &[u8]) -> Option<(&'static str, i32, i32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(("image/png", u32::from_be_bytes(bytes(data, 16)?) as i32, u32::from_be_bytes(bytes(data, 20)?) as i32));
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some(("image/gif", u16::from_le_bytes(bytes(data, 6)?) as i32, u16::from_le_bytes(bytes(data, 8)?) as i32));
    }
    if !data.starts_with(b"RIFF") || data.get(8..12) != Some(&b"WEBP"[..]) {
        return None;
    }
    let (width, height) = match data.get(12..16)? {
        b"VP8 " => (
            (u16::from_le_bytes(bytes(data, 26)?) & 0x3fff) as i32,
            (u16::from_le_bytes(bytes(data, 28)?) & 0x3fff) as i32,
        ),
        b"VP8L" => {
            let bits = u32::from_le_bytes(bytes(data, 21)?);
            ((bits & 0x3fff) as i32 + 1, ((bits >> 14) & 0x3fff) as i32 + 1)
        }
        b"VP8X" => {
            let [w0, w1, w2] = bytes(data, 24)?;
            let [h0, h1, h2] = bytes(data, 27)?;
            (i32::from_le_bytes([w0, w1, w2, 0]) + 1, i32::from_le_bytes([h0, h1, h2, 0]) + 1)
        }
        _ => return None,
    };
    Some(("image/webp", width, height))
}

fn title(slug: &str) -> String {
    let mut name = String::new();
    for (i, c) in slug.chars().enumerate() {
        if c == '-' {
            name.push(' ');
        } else if i == 0 {
            name.extend(c.to_uppercase());
        } else {
            if c.is_ascii_digit() && name.chars().last().is_some_and(|last| last.is_alphabetic()) {
                name.push(' ');
            }
            name.push(c);
        }
    }
    name
}

fn colour_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 64 && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn hex(value: &str) -> bool {
    matches!(value.len(), 7 | 9) && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn size(body: &Value, key: &str) -> Result<Option<i32>, HttpResponse> {
    match &body[key] {
        Value::Null => Ok(None),
        value => match value.as_i64() {
            Some(side) if (1..=MAX_SIDE as i64).contains(&side) => Ok(Some(side as i32)),
            _ => Err(HttpResponse::BadRequest().body(format!("The suggested {key} must be between 1 and {MAX_SIDE}"))),
        },
    }
}

fn slot_fields(body: &Value) -> Result<(&str, &str, Option<i32>, Option<i32>), HttpResponse> {
    let name = body["name"].as_str().unwrap_or_default().trim();
    let brief = body["brief"].as_str().unwrap_or_default().trim();
    if name.is_empty() || name.chars().count() > 60 || brief.chars().count() > 2000 {
        return Err(HttpResponse::BadRequest().body("Names are up to 60 characters and briefs up to 2000"));
    }
    Ok((name, brief, size(body, "width")?, size(body, "height")?))
}

fn colour_value(value: &Value) -> Result<Option<String>, HttpResponse> {
    match value {
        Value::Null => Ok(None),
        Value::String(value) if hex(value) => Ok(Some(value.to_lowercase())),
        _ => Err(HttpResponse::BadRequest().body("Colours look like #rrggbb or #rrggbbaa")),
    }
}

async fn apply_colour(tx: &mut Transaction<'_, Postgres>, admin: i32, key: &str, value: &Option<String>) -> sqlx::Result<()> {
    sqlx::query("UPDATE colour SET active = false WHERE key = $1 AND active").bind(key).execute(&mut **tx).await?;
    if let Some(value) = value {
        sqlx::query("INSERT INTO colour (key, value, active, set_by) VALUES ($1, $2, true, $3)")
            .bind(key)
            .bind(value)
            .bind(admin)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn log(tx: &mut Transaction<'_, Postgres>, admin: i32, action: &str, summary: String) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO admin_action (admin_id, action, summary) VALUES ($1, $2, $3)")
        .bind(admin)
        .bind(action)
        .bind(summary)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn reload(pool: &PgPool) -> sqlx::Result<()> {
    let _turn = RELOADING.lock().await;
    let images: Vec<(String, i32, String, i32, i32, bool)> = sqlx::query_as(
        "SELECT i.slug, i.id, i.content_type, i.width, i.height, s.smooth FROM image i JOIN image_slot s ON s.slug = i.slug WHERE i.active",
    )
    .fetch_all(pool)
    .await?;
    let colours: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM colour WHERE active").fetch_all(pool).await?;
    let mut kept: HashMap<i32, web::Bytes> = {
        let cache = CACHE.read().unwrap();
        cache.images.values().map(|active| (active.id, active.data.clone())).collect()
    };
    let missing: Vec<i32> = images.iter().map(|(_, id, ..)| *id).filter(|id| !kept.contains_key(id)).collect();
    if !missing.is_empty() {
        let fetched: Vec<(i32, Vec<u8>)> = sqlx::query_as("SELECT id, data FROM image WHERE id = ANY($1)").bind(&missing).fetch_all(pool).await?;
        kept.extend(fetched.into_iter().map(|(id, data)| (id, data.into())));
    }
    let listing = json!({
        "images": images
            .iter()
            .map(|(slug, id, _, width, height, smooth)| (slug.clone(), json!({ "id": id, "width": width, "height": height, "smooth": smooth })))
            .collect::<Map<_, _>>(),
        "colours": colours.into_iter().map(|(key, value)| (key, Value::String(value))).collect::<Map<_, _>>(),
    });
    let images = images
        .into_iter()
        .filter_map(|(slug, id, content_type, ..)| Some((slug, Active { id, content_type, data: kept.remove(&id)? })))
        .collect();
    *CACHE.write().unwrap() = Cache { images, manifest: listing };
    REVISION.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

pub async fn seed(pool: &PgPool) -> sqlx::Result<()> {
    for (slug, _, data) in ORIGINALS {
        let Some((kind, width, height)) = measure(data) else {
            eprintln!("couldn't read the size of the {slug} image");
            continue;
        };
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO image_slot (slug, name, built_in) VALUES ($1, $2, true) ON CONFLICT (slug) DO UPDATE SET built_in = true")
            .bind(slug)
            .bind(title(slug))
            .execute(&mut *tx)
            .await?;
        let current: bool = sqlx::query_scalar(
            "SELECT coalesce((SELECT data = $2 FROM image WHERE slug = $1 AND original ORDER BY created_at DESC, id DESC LIMIT 1), false)",
        )
        .bind(slug)
        .bind(*data)
        .fetch_one(&mut *tx)
        .await?;
        if !current {
            let replace: bool = sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM image WHERE slug = $1 AND active AND NOT original)")
                .bind(slug)
                .fetch_one(&mut *tx)
                .await?;
            if replace {
                sqlx::query("UPDATE image SET active = false WHERE slug = $1 AND active").bind(slug).execute(&mut *tx).await?;
            }
            let restored = sqlx::query(
                "UPDATE image SET created_at = now(), active = $3 WHERE id = (SELECT id FROM image WHERE slug = $1 AND original AND data = $2 ORDER BY id DESC LIMIT 1)",
            )
            .bind(slug)
            .bind(*data)
            .bind(replace)
            .execute(&mut *tx)
            .await?;
            if restored.rows_affected() == 0 {
                sqlx::query("INSERT INTO image (slug, data, content_type, width, height, original, active) VALUES ($1, $2, $3, $4, $5, true, $6)")
                    .bind(slug)
                    .bind(*data)
                    .bind(kind)
                    .bind(width)
                    .bind(height)
                    .bind(replace)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
    }
    let slugs: Vec<&str> = ORIGINALS.iter().map(|(slug, ..)| *slug).collect();
    sqlx::query("UPDATE image_slot SET built_in = false WHERE built_in AND NOT slug = ANY($1)").bind(&slugs).execute(pool).await?;
    reload(pool).await
}

async fn done(pool: &PgPool, what: &str, outcome: Outcome) -> HttpResponse {
    match outcome {
        Ok(Ok(())) => match reload(pool).await {
            Ok(()) => HttpResponse::NoContent().finish(),
            Err(e) => failed("reloading styles", e),
        },
        Ok(Err(response)) => response,
        Err(e) => failed(what, e),
    }
}

#[get("/api/styling")]
pub async fn manifest() -> HttpResponse {
    HttpResponse::Ok().insert_header((CACHE_CONTROL, "no-cache")).json(&CACHE.read().unwrap().manifest)
}

#[get("/api/styling/images/{slug}")]
pub async fn image(slug: web::Path<String>, query: web::Query<HashMap<String, String>>) -> HttpResponse {
    let cache = CACHE.read().unwrap();
    let Some(active) = cache.images.get(slug.as_str()) else {
        return HttpResponse::NotFound().finish();
    };
    let pinned = query.get("v").is_some_and(|v| *v == active.id.to_string());
    HttpResponse::Ok()
        .content_type(active.content_type.as_str())
        .insert_header((CACHE_CONTROL, if pinned { "public, max-age=31536000, immutable" } else { "no-cache" }))
        .body(active.data.clone())
}

#[get("/api/styling/events")]
pub async fn events() -> HttpResponse {
    let ticks = actix_web::rt::time::interval(Duration::from_secs(1));
    let changes = futures_util::stream::unfold((REVISION.load(Ordering::Relaxed), ticks), |(mut seen, mut ticks)| async move {
        let mut idle = 0;
        loop {
            ticks.tick().await;
            let now = REVISION.load(Ordering::Relaxed);
            if now != seen {
                seen = now;
                return Some((web::Bytes::from(format!("data: {now}\n\n")), (seen, ticks)));
            }
            idle += 1;
            if idle == 15 {
                return Some((web::Bytes::from_static(b": keepalive\n\n"), (seen, ticks)));
            }
        }
    });
    let stream = futures_util::stream::once(async { web::Bytes::from_static(b": connected\n\n") })
        .chain(changes)
        .map(Ok::<_, actix_web::Error>);
    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header((CACHE_CONTROL, "no-cache"))
        .insert_header(ContentEncoding::Identity)
        .streaming(stream)
}

#[get("/api/admin/styling")]
pub async fn overview(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Styling).await {
        return response;
    }
    let styling: sqlx::Result<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'slots', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                 'slug', s.slug, 'name', s.name, 'brief', s.brief, 'width', s.width, 'height', s.height, 'smooth', s.smooth,
                 'built_in', s.built_in, 'created_at', s.created_at,
                 'versions', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                     'id', i.id, 'width', i.width, 'height', i.height, 'content_type', i.content_type, 'bytes', octet_length(i.data),
                     'original', i.original, 'active', i.active, 'created_at', i.created_at,
                     'uploaded_by', (SELECT {PERSON} FROM \"user\" x WHERE x.id = i.uploaded_by)
                 ) ORDER BY i.created_at DESC, i.id DESC), '[]') FROM image i WHERE i.slug = s.slug)
             ) ORDER BY s.slug), '[]') FROM image_slot s),
             'colours', (SELECT coalesce(jsonb_object_agg(h.key, h.history), '{{}}') FROM (
                 SELECT c.key, jsonb_agg(jsonb_build_object(
                     'id', c.id, 'value', c.value, 'active', c.active, 'created_at', c.created_at,
                     'set_by', (SELECT {PERSON} FROM \"user\" x WHERE x.id = c.set_by)
                 ) ORDER BY c.created_at DESC, c.id DESC) AS history
                 FROM colour c GROUP BY c.key
             ) h),
             'progress', (SELECT coalesce(jsonb_object_agg(p.key, jsonb_build_object(
                 'at', p.completed_at, 'by', (SELECT {PERSON} FROM \"user\" x WHERE x.id = p.completed_by)
             )), '{{}}') FROM style_progress p WHERE p.kind = 'image')
         )"
    )))
    .fetch_one(pool.get_ref())
    .await;
    match styling {
        Ok(styling) => HttpResponse::Ok().json(styling),
        Err(e) => failed("loading the styling admin", e),
    }
}

#[get("/api/admin/styling/versions/{id:\\d+}")]
pub async fn version(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Styling).await {
        return response;
    }
    let found: sqlx::Result<Option<(String, Vec<u8>)>> = sqlx::query_as("SELECT content_type, data FROM image WHERE id = $1")
        .bind(*id)
        .fetch_optional(pool.get_ref())
        .await;
    match found {
        Ok(Some((content_type, data))) => HttpResponse::Ok()
            .content_type(content_type)
            .insert_header((CACHE_CONTROL, "private, max-age=31536000, immutable"))
            .body(data),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => failed("loading an image version", e),
    }
}

#[post("/api/admin/styling/images/{slug}")]
pub async fn upload(req: HttpRequest, slug: web::Path<String>, body: web::Bytes, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let Some((kind, width, height)) = measure(&body) else {
        return HttpResponse::BadRequest().body("That isn't a PNG, WebP or GIF");
    };
    if !(1..=MAX_SIDE).contains(&width) || !(1..=MAX_SIDE).contains(&height) {
        return HttpResponse::BadRequest().body(format!("Images must be between 1 and {MAX_SIDE} pixels on each side"));
    }
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM image_slot WHERE slug = $1)")
            .bind(slug.as_str())
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            return Ok(Err(HttpResponse::NotFound().body("That image doesn't exist")));
        }
        sqlx::query("UPDATE image SET active = false WHERE slug = $1 AND active").bind(slug.as_str()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO image (slug, data, content_type, width, height, active, uploaded_by) VALUES ($1, $2, $3, $4, $5, true, $6)")
            .bind(slug.as_str())
            .bind(&body[..])
            .bind(kind)
            .bind(width)
            .bind(height)
            .bind(admin.id)
            .execute(&mut *tx)
            .await?;
        log(&mut tx, admin.id, "styling_upload", format!("Uploaded a new {slug} image ({width}x{height})")).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "uploading an image", outcome).await
}

#[post("/api/admin/styling/versions/{id:\\d+}/activate")]
pub async fn activate(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let Some(slug): Option<String> = sqlx::query_scalar("SELECT slug FROM image WHERE id = $1").bind(*id).fetch_optional(&mut *tx).await? else {
            return Ok(Err(HttpResponse::NotFound().finish()));
        };
        sqlx::query("UPDATE image SET active = false WHERE slug = $1 AND active").bind(&slug).execute(&mut *tx).await?;
        sqlx::query("UPDATE image SET active = true WHERE id = $1").bind(*id).execute(&mut *tx).await?;
        log(&mut tx, admin.id, "styling_revert", format!("Switched {slug} to image version #{id}")).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "switching an image version", outcome).await
}

#[delete("/api/admin/styling/versions/{id:\\d+}")]
pub async fn remove_version(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let Some(slug): Option<String> = sqlx::query_scalar("DELETE FROM image WHERE id = $1 AND NOT original AND NOT active RETURNING slug")
            .bind(*id)
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Ok(Err(HttpResponse::Conflict().body("The original and the version in use can't be deleted")));
        };
        log(&mut tx, admin.id, "styling_delete", format!("Deleted {slug} image version #{id}")).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "deleting an image version", outcome).await
}

#[post("/api/admin/styling/slots")]
pub async fn declare(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let name = body["name"].as_str().unwrap_or_default().trim();
    if name.is_empty() || name.chars().count() > 60 || !name.chars().any(|c| c.is_ascii_alphanumeric()) {
        return HttpResponse::BadRequest().body("Give the image a name of up to 60 characters");
    }
    let count = body["count"].as_i64().unwrap_or(1);
    if !(1..=50).contains(&count) {
        return HttpResponse::BadRequest().body("Declare between 1 and 50 images at a time");
    }
    let brief = body["brief"].as_str().unwrap_or_default().trim();
    if brief.chars().count() > 2000 {
        return HttpResponse::BadRequest().body("Keep the brief under 2000 characters");
    }
    let (width, height) = match (size(&body, "width"), size(&body, "height")) {
        (Ok(width), Ok(height)) => (width, height),
        (Err(response), _) | (_, Err(response)) => return response,
    };
    let base = crate::mines::slug(name);
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let joiner = if base.ends_with(|c: char| c.is_ascii_digit()) { "-" } else { "" };
        let taken: Vec<String> = sqlx::query_scalar("SELECT slug FROM image_slot WHERE slug = $1 OR slug ~ ('^' || $1 || $2 || '[0-9]+$')")
            .bind(&base)
            .bind(joiner)
            .fetch_all(&mut *tx)
            .await?;
        let highest = taken
            .iter()
            .map(|slug| slug.get(base.len() + joiner.len()..).unwrap_or_default().parse().unwrap_or(1))
            .max();
        let slots: Vec<(String, String)> = match highest {
            None if count == 1 => vec![(base.clone(), name.to_string())],
            _ => {
                let start = highest.unwrap_or(0) + 1;
                (start..start + count).map(|n| (format!("{base}{joiner}{n}"), format!("{name} {n}"))).collect()
            }
        };
        for (slug, label) in &slots {
            let created = sqlx::query("INSERT INTO image_slot (slug, name, brief, width, height, declared_by) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (slug) DO NOTHING")
                .bind(slug)
                .bind(label)
                .bind(brief)
                .bind(width)
                .bind(height)
                .bind(admin.id)
                .execute(&mut *tx)
                .await?;
            if created.rows_affected() == 0 {
                return Ok(Err(HttpResponse::Conflict().body(format!("There's already an image called {slug}"))));
            }
        }
        log(&mut tx, admin.id, "styling_declare", format!("Declared {count} {name} image{}", if count == 1 { "" } else { "s" })).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "declaring images", outcome).await
}

#[put("/api/admin/styling/slots/{slug}")]
pub async fn save_slot(req: HttpRequest, slug: web::Path<String>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let (name, brief, width, height) = match slot_fields(&body) {
        Ok(fields) => fields,
        Err(response) => return response,
    };
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let saved = sqlx::query("UPDATE image_slot SET name = $2, brief = $3, width = $4, height = $5 WHERE slug = $1")
            .bind(slug.as_str())
            .bind(name)
            .bind(brief)
            .bind(width)
            .bind(height)
            .execute(&mut *tx)
            .await?;
        if saved.rows_affected() == 0 {
            return Ok(Err(HttpResponse::NotFound().finish()));
        }
        log(&mut tx, admin.id, "styling_slot", format!("Edited the {slug} image brief")).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "saving an image brief", outcome).await
}

#[put("/api/admin/styling/slots/{slug}/filter")]
pub async fn set_filter(req: HttpRequest, slug: web::Path<String>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let Some(smooth) = body["smooth"].as_bool() else {
        return HttpResponse::BadRequest().body("smooth must be true or false");
    };
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let saved = sqlx::query("UPDATE image_slot SET smooth = $2 WHERE slug = $1").bind(slug.as_str()).bind(smooth).execute(&mut *tx).await?;
        if saved.rows_affected() == 0 {
            return Ok(Err(HttpResponse::NotFound().finish()));
        }
        let filtering = if smooth { "interpolated" } else { "nearest" };
        log(&mut tx, admin.id, "styling_filter", format!("Set the {slug} image to {filtering} filtering")).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "setting image filtering", outcome).await
}

#[delete("/api/admin/styling/slots/{slug}")]
pub async fn remove_slot(req: HttpRequest, slug: web::Path<String>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        let removed = sqlx::query("DELETE FROM image_slot WHERE slug = $1 AND NOT built_in").bind(slug.as_str()).execute(&mut *tx).await?;
        if removed.rows_affected() == 0 {
            return Ok(Err(HttpResponse::Conflict().body("Built-in images can't be removed")));
        }
        sqlx::query("DELETE FROM style_progress WHERE kind = 'image' AND key = $1").bind(slug.as_str()).execute(&mut *tx).await?;
        log(&mut tx, admin.id, "styling_remove", format!("Removed the declared {slug} image and its uploads")).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "removing an image", outcome).await
}

#[put("/api/admin/styling/colours/{key}")]
pub async fn set_colour(req: HttpRequest, key: web::Path<String>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    if !colour_key(&key) {
        return HttpResponse::BadRequest().body("That isn't a colour");
    }
    let value = match colour_value(&body["value"]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        apply_colour(&mut tx, admin.id, &key, &value).await?;
        let summary = match &value {
            Some(value) => format!("Set the {key} colour to {value}"),
            None => format!("Reset the {key} colour to its default"),
        };
        log(&mut tx, admin.id, "styling_colour", summary).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "setting a colour", outcome).await
}

#[post("/api/admin/styling/import")]
pub async fn import(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let mut colours = Vec::new();
    for (key, value) in body["colours"].as_object().into_iter().flatten() {
        if !colour_key(key) {
            return HttpResponse::BadRequest().body(format!("{key} isn't a colour"));
        }
        match colour_value(value) {
            Ok(value) => colours.push((key.as_str(), value)),
            Err(response) => return response,
        }
    }
    let mut slots = Vec::new();
    for slot in body["slots"].as_array().into_iter().flatten() {
        let slug = slot["slug"].as_str().unwrap_or_default();
        if !colour_key(slug) {
            return HttpResponse::BadRequest().body(format!("{slug} isn't an image name"));
        }
        match slot_fields(slot) {
            Ok(fields) => slots.push((slug, fields, slot["smooth"].as_bool())),
            Err(response) => return response,
        }
    }
    let outcome: Outcome = async {
        let mut tx = pool.begin().await?;
        for (key, value) in &colours {
            apply_colour(&mut tx, admin.id, key, value).await?;
        }
        for (slug, (name, brief, width, height), smooth) in &slots {
            sqlx::query(
                "INSERT INTO image_slot (slug, name, brief, width, height, declared_by, smooth) VALUES ($1, $2, $3, $4, $5, $6, coalesce($7, true))
                 ON CONFLICT (slug) DO UPDATE SET name = $2, brief = $3, width = $4, height = $5, smooth = coalesce($7, image_slot.smooth)",
            )
            .bind(slug)
            .bind(name)
            .bind(brief)
            .bind(width)
            .bind(height)
            .bind(admin.id)
            .bind(smooth)
            .execute(&mut *tx)
            .await?;
        }
        log(&mut tx, admin.id, "styling_import", format!("Loaded {} colours and {} image briefs from a file", colours.len(), slots.len())).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    done(&pool, "loading styling from a file", outcome).await
}

#[put("/api/admin/styling/progress")]
pub async fn progress(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Styling).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let key = body["key"].as_str().unwrap_or_default();
    if key.is_empty() || key.len() > 64 {
        return HttpResponse::BadRequest().finish();
    }
    let completed = body["completed"].as_bool().unwrap_or(false);
    let result = if completed {
        sqlx::query("INSERT INTO style_progress (kind, key, completed_by) SELECT 'image', slug, $2 FROM image_slot WHERE slug = $1 ON CONFLICT DO NOTHING")
            .bind(key)
            .bind(admin.id)
            .execute(pool.get_ref())
            .await
    } else {
        sqlx::query("DELETE FROM style_progress WHERE kind = 'image' AND key = $1").bind(key).execute(pool.get_ref()).await
    };
    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => failed("marking styling progress", e),
    }
}
