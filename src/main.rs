mod admin;
mod admin_permissions;
mod admin_users;
mod analytics;
mod auth;
mod chats;
mod database;
mod joel;
mod mines;
mod moderation;
mod operations;
mod projects;
mod propaganda;
mod styling;
mod territory;

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
    styling::seed(&pool).await.map_err(std::io::Error::other)?;

    let ticker = pool.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            joel::resolve_due(&ticker).await;
        }
    });

    let poller = pool.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            propaganda::poll(&poller).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(web::PayloadConfig::new(16 << 20))
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
            .service(territory::list)
            .service(mines::list)
            .service((
                mines::overview,
                mines::create_contract,
                mines::save_contract,
                mines::remove_contract,
                mines::create_prize,
                mines::save_prize,
                mines::remove_prize,
            ))
            .service(joel::announcements)
            .service((
                joel::overview,
                joel::create,
                joel::save,
                joel::remove,
                joel::act,
                joel::announce,
                joel::unannounce,
                joel::edit,
            ))
            .service((
                projects::list,
                projects::create,
                projects::save,
                projects::remove,
                projects::upload,
                projects::screenshot,
                projects::hackatime_list,
                projects::link,
                projects::linked,
            ))
            .service((
                operations::list,
                operations::create,
                operations::show,
                operations::save,
                operations::confirm,
                operations::draft,
                operations::allocate,
                operations::launch,
            ))
            .service((auth::start, auth::callback, auth::session, auth::logout))
            .service((propaganda::overview, propaganda::submit))
            .service((propaganda::queue, propaganda::repoll, propaganda::verify))
            .service((styling::manifest, styling::image, styling::events))
            .service((
                styling::overview,
                styling::version,
                styling::upload,
                styling::activate,
                styling::remove_version,
                styling::declare,
                styling::save_slot,
                styling::remove_slot,
                styling::set_colour,
                styling::import,
                styling::progress,
            ))
            .service((chats::list, chats::messages, chats::create, chats::flag))
            .service((
                moderation::chats,
                moderation::messages,
                moderation::send,
                moderation::flags,
                moderation::dismiss,
                moderation::delete,
                moderation::ban,
            ))
            .service(admin::check)
            .service((analytics::visit, analytics::dashboard))
            .service((admin_users::list, admin_users::show, admin_users::private, admin_users::action))
            .service((
                admin_permissions::list,
                admin_permissions::candidates,
                admin_permissions::add,
                admin_permissions::update,
                admin_permissions::remove,
                admin_permissions::audit,
                admin_permissions::audit_filters,
            ))
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
