use actix_files::{Files, NamedFile};
use actix_web::dev::{ServiceRequest, ServiceResponse, fn_service};
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};

const FRONTEND_DIR: &str = "../frontend/build";
const INDEX_FILE: &str = "../frontend/build/index.html";
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
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4400);

    std::fs::metadata(INDEX_FILE).expect("frontend not built");

    HttpServer::new(|| {
        App::new().service(health).service(map).service(
            Files::new("/", FRONTEND_DIR)
                .index_file("index.html")
                .default_handler(fn_service(fallback)),
        )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
