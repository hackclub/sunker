use actix_files::{Files, NamedFile};
use actix_web::cookie::Cookie;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, fn_service};
use actix_web::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderValue};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, middleware, web};
use hc_helper::auth::data::forget_auth_token;
use hc_helper::auth::login::{self, CallbackArgs, Scopes, TOKEN_COOKIE};
use std::future::Ready;

const FRONTEND_DIR: &str = "../frontend/build";
const INDEX_FILE: &str = "../frontend/build/index.html";
const MAP_FILE: &str = "map.json";
const CALLBACK: &str = "/callback";

fn site_url(path: &str) -> String {
    let base_url = hc_helper::keys::base_url();
    let host = match base_url.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => host,
        _ => &base_url,
    };
    let scheme = match host {
        "localhost" | "127.0.0.1" | "[::1]" => "http",
        _ => "https",
    };
    format!("{scheme}://{base_url}{path}")
}

#[get("/login")]
async fn login_start(req: HttpRequest) -> HttpResponse {
    let scopes = Scopes {
        openid: false,
        profile: false,
        email: false,
        name: false,
        slack_id: true,
        verification_status: true,
        basic_info: true,
        addresses: true,
        phone_number: true,
    };
    login::handle_login(&req, scopes, site_url(CALLBACK)).await
}

#[get("/callback")]
async fn login_callback(req: HttpRequest, query: web::Query<CallbackArgs>) -> HttpResponse {
    login::handle_callback(
        &req,
        query,
        site_url(CALLBACK),
        "/hq".to_string(),
        None::<fn(String) -> Ready<()>>,
        true,
    )
    .await
}

#[get("/api/session")]
async fn session(req: HttpRequest) -> HttpResponse {
    match login::get_auth_token_with_handling(&req) {
        Some(_) => HttpResponse::NoContent().finish(),
        None => HttpResponse::Unauthorized().finish(),
    }
}

#[get("/logout")]
async fn logout(req: HttpRequest) -> HttpResponse {
    if let Some(token) = login::get_auth_token_with_handling(&req) {
        forget_auth_token(&token);
    }
    let mut response = HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish();
    let _ = response.add_removal_cookie(&Cookie::new(TOKEN_COOKIE, ""));
    response
}

#[get("/api/health")]
async fn health() -> impl Responder {
    web::Json(serde_json::json!({ "status": "ok" }))
}

#[get("/api/map")]
async fn map() -> std::io::Result<NamedFile> {
    NamedFile::open(MAP_FILE)
}

async fn fallback(req: ServiceRequest) -> actix_web::Result<ServiceResponse> {
    let (req, _) = req.into_parts();
    let path = req.path();
    let response = if path.starts_with("/api/") {
        HttpResponse::NotFound().json(serde_json::json!({ "error": "not found" }))
    } else if path.starts_with("/_app/")
        || path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.contains('.'))
    {
        HttpResponse::NotFound().finish()
    } else {
        NamedFile::open(INDEX_FILE)?.into_response(&req)
    };
    Ok(ServiceResponse::new(req, response))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4400);

    std::fs::metadata(INDEX_FILE).expect("frontend not built");

    HttpServer::new(|| {
        App::new()
            .wrap_fn(|req, srv| {
                let immutable = req.path().starts_with("/_app/immutable/");
                let response = srv.call(req);
                async move {
                    let mut response = response.await?;
                    let html = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .is_some_and(|kind| kind.as_bytes().starts_with(b"text/html"));
                    let policy = if immutable && response.status().is_success() {
                        Some("public, max-age=31536000, immutable")
                    } else if html {
                        Some("no-cache")
                    } else {
                        None
                    };
                    if let Some(policy) = policy {
                        response
                            .headers_mut()
                            .insert(CACHE_CONTROL, HeaderValue::from_static(policy));
                    }
                    Ok(response)
                }
            })
            .wrap(middleware::Compress::default())
            .service(health)
            .service(map)
            .service((login_start, login_callback, session, logout))
            .service(
                Files::new("/", FRONTEND_DIR)
                    .index_file("index.html")
                    .default_handler(fn_service(fallback)),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
