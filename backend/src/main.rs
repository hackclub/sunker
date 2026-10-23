use actix_files::{Files, NamedFile};
use actix_web::{get, web, App, HttpServer, Responder};

const FRONTEND_DIR: &str = "../frontend/build";

#[get("/api/health")]
async fn health() -> impl Responder {
    web::Json(serde_json::json!({ "status": "ok" }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4400);

    HttpServer::new(|| {
        App::new().service(health).service(
            Files::new("/", FRONTEND_DIR)
                .index_file("index.html")
                .default_handler(
                    NamedFile::open(format!("{FRONTEND_DIR}/index.html"))
                        .expect("frontend not built"),
                ),
        )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
