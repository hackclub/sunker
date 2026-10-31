mod admin;
mod admin_users;
mod analytics;
mod auth;
mod chats;
mod database;

use actix_files::{Files, NamedFile};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, fn_service};
use actix_web::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderValue};
use actix_web::{App, HttpResponse, HttpServer, Responder, get, middleware, web};

const FRONTEND_DIR: &str = "frontend/build";
const INDEX_FILE: &str = "frontend/build/index.html";
const MAP_FILE: &str = "map.json";

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

    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = web::Data::new(database::init(&url).await.map_err(std::io::Error::other)?);

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
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
            .service((auth::start, auth::callback, auth::session, auth::logout))
            .service(chats::list)
            .service(admin::check)
            .service((analytics::visit, analytics::dashboard))
            .service((admin_users::list, admin_users::show, admin_users::private, admin_users::action))
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
