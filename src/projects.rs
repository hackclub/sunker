use crate::auth;
use actix_web::web::{Bytes, BytesMut};
use actix_web::{HttpRequest, HttpResponse, delete, get, patch, post, put, web};
use futures_util::StreamExt;
use hc_helper::hackatime::data::{get_hackatime_project_summaries_with_token, get_hackatime_user_with_token};
use hc_helper::hackatime::login::{self, CallbackArgs, Scopes};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{LazyLock, Mutex};

const CALLBACK: &str = "/hackatime/callback";
const SCREENSHOT_LIMIT: usize = 5 * 1024 * 1024;
const SCREENSHOT_STORE_LIMIT: usize = 256 * 1024 * 1024;
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];

const PROJECT: &str = "jsonb_build_object(
    'id', p.id, 'contract', c.name, 'ring', c.ring, 'name', p.name, 'description', p.description,
    'repo_url', p.repo_url, 'screenshot_url', p.screenshot_url, 'hackatime_projects', p.hackatime_projects
)";

static SCREENSHOTS: LazyLock<Mutex<HashMap<i32, (&'static str, Bytes)>>> = LazyLock::new(Default::default);

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

async fn owned(pool: &PgPool, user: i32, id: i32) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM project WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL)")
        .bind(id)
        .bind(user)
        .fetch_one(pool)
        .await
}

#[get("/api/projects")]
pub async fn list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<Option<Value>> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(None);
        };
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_agg({PROJECT} ORDER BY p.created_at DESC), '[]')
             FROM project p JOIN contract c ON c.slug = p.contract_slug
             WHERE p.user_id = $1 AND p.deleted_at IS NULL"
        )))
        .bind(user)
        .fetch_one(pool.get_ref())
        .await
        .map(Some)
    }
    .await;
    match result {
        Ok(Some(projects)) => HttpResponse::Ok().json(json!({ "projects": projects })),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("listing projects", e),
    }
}

#[post("/api/projects")]
pub async fn create(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let contract = body["contract"].as_str().unwrap_or_default();
    let result: sqlx::Result<Option<Option<i32>>> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(None);
        };
        sqlx::query_scalar(
            "INSERT INTO project (user_id, contract_slug, name)
             SELECT $1, slug, '' FROM contract WHERE slug = $2 AND deleted_at IS NULL
             RETURNING id",
        )
        .bind(user)
        .bind(contract)
        .fetch_optional(pool.get_ref())
        .await
        .map(Some)
    }
    .await;
    match result {
        Ok(Some(Some(id))) => HttpResponse::Ok().json(json!({ "id": id })),
        Ok(Some(None)) => HttpResponse::NotFound().body("That contract doesn't exist anymore"),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("creating a project", e),
    }
}

fn field(body: &Value, key: &str, limit: usize, label: &str) -> Result<Option<String>, String> {
    match body.get(key) {
        None => Ok(None),
        Some(value) => {
            let value = value.as_str().ok_or(format!("{label} must be text"))?.trim().to_string();
            if value.chars().count() > limit {
                return Err(format!("{label} must be under {limit} characters"));
            }
            Ok(Some(value))
        }
    }
}

fn sniff(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, ..] => Some("image/png"),
        [0xff, 0xd8, 0xff, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        _ => None,
    }
}

async fn hackatime_token(req: &HttpRequest, pool: &PgPool, user: i32) -> sqlx::Result<Option<String>> {
    let Some(token) = login::get_hackatime_token_with_handling(req) else {
        return Ok(None);
    };
    let htme_id: Option<i64> = sqlx::query_scalar("SELECT htme_id FROM \"user\" WHERE id = $1")
        .bind(user)
        .fetch_one(pool)
        .await?;
    let Some(htme_id) = htme_id else {
        return Ok(None);
    };
    Ok(match get_hackatime_user_with_token(&token).await {
        Ok(account) if account.user_id as i64 == htme_id => Some(token),
        _ => None,
    })
}

async fn hackatime_names(req: &HttpRequest, pool: &PgPool, user: i32) -> sqlx::Result<Option<Vec<String>>> {
    let Some(token) = hackatime_token(req, pool, user).await? else {
        return Ok(None);
    };
    Ok(get_hackatime_project_summaries_with_token(&token)
        .await
        .ok()
        .map(|projects| projects.into_iter().map(|project| project.name).collect()))
}

#[patch("/api/projects/{id:\\d+}")]
pub async fn save(req: HttpRequest, id: web::Path<i32>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let parsed = (|| {
        let name = field(&body, "name", 100, "The name")?;
        let description = field(&body, "description", 2000, "The description")?;
        let repo_url = field(&body, "repo_url", 500, "The code URL")?;
        if let Some(url) = repo_url.as_deref().filter(|url| !url.is_empty())
            && !url.starts_with("https://")
            && !url.starts_with("http://")
        {
            return Err("The code URL needs to start with https://".to_string());
        }
        let hackatime = match body.get("hackatime_projects") {
            None => None,
            Some(value) => Some(
                value
                    .as_array()
                    .ok_or("Hackatime projects must be a list")?
                    .iter()
                    .map(|name| name.as_str().map(str::to_string).ok_or("Hackatime projects must be names"))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        };
        Ok::<_, String>((name, description, repo_url, hackatime))
    })();
    let (name, description, repo_url, hackatime) = match parsed {
        Ok(parsed) => parsed,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let result: sqlx::Result<HttpResponse> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(HttpResponse::Unauthorized().finish());
        };
        if !owned(&pool, user, *id).await? {
            return Ok(HttpResponse::NotFound().finish());
        }
        if let Some(hackatime) = hackatime.as_ref().filter(|names| !names.is_empty()) {
            let Some(known) = hackatime_names(&req, &pool, user).await? else {
                return Ok(HttpResponse::BadRequest().body("Link Hackatime first"));
            };
            if let Some(unknown) = hackatime.iter().find(|name| !known.contains(name)) {
                return Ok(HttpResponse::BadRequest().body(format!("{unknown} isn't one of your Hackatime projects")));
            }
            let clash: Option<String> = sqlx::query_scalar(
                "SELECT h FROM project, unnest(hackatime_projects) h
                 WHERE user_id = $1 AND id <> $2 AND deleted_at IS NULL AND h = ANY($3) LIMIT 1",
            )
            .bind(user)
            .bind(*id)
            .bind(hackatime)
            .fetch_optional(pool.get_ref())
            .await?;
            if let Some(clash) = clash {
                return Ok(HttpResponse::Conflict().body(format!("{clash} is already attached to another project")));
            }
        }
        sqlx::query(
            "UPDATE project SET
                 name = coalesce($2, name),
                 description = coalesce($3, description),
                 repo_url = CASE WHEN $4 THEN NULLIF($5, '') ELSE repo_url END,
                 hackatime_projects = coalesce($6, hackatime_projects),
                 updated_at = now()
             WHERE id = $1 AND user_id = $7 AND deleted_at IS NULL",
        )
        .bind(*id)
        .bind(name)
        .bind(description)
        .bind(repo_url.is_some())
        .bind(repo_url)
        .bind(hackatime)
        .bind(user)
        .execute(pool.get_ref())
        .await?;
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("saving a project", e))
}

#[delete("/api/projects/{id:\\d+}")]
pub async fn remove(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<HttpResponse> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(HttpResponse::Unauthorized().finish());
        };
        let done = sqlx::query("UPDATE project SET deleted_at = now() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL")
            .bind(*id)
            .bind(user)
            .execute(pool.get_ref())
            .await?;
        if done.rows_affected() == 0 {
            return Ok(HttpResponse::NotFound().finish());
        }
        SCREENSHOTS.lock().unwrap_or_else(|e| e.into_inner()).remove(&*id);
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("deleting a project", e))
}

#[put("/api/projects/{id:\\d+}/screenshot")]
pub async fn upload(req: HttpRequest, id: web::Path<i32>, mut payload: web::Payload, pool: web::Data<PgPool>) -> HttpResponse {
    let declared = req.headers().get("Content-Type").and_then(|kind| kind.to_str().ok()).unwrap_or_default();
    if !IMAGE_TYPES.contains(&declared) {
        return HttpResponse::UnsupportedMediaType().body("Screenshots must be PNG, JPEG, WebP or GIF");
    }
    let result: sqlx::Result<HttpResponse> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(HttpResponse::Unauthorized().finish());
        };
        if !owned(&pool, user, *id).await? {
            return Ok(HttpResponse::NotFound().finish());
        }
        let mut bytes = BytesMut::new();
        while let Some(chunk) = payload.next().await {
            let Ok(chunk) = chunk else {
                return Ok(HttpResponse::BadRequest().finish());
            };
            if bytes.len() + chunk.len() > SCREENSHOT_LIMIT {
                return Ok(HttpResponse::PayloadTooLarge().body("Screenshots must be under 5 MB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let Some(kind) = sniff(&bytes) else {
            return Ok(HttpResponse::UnsupportedMediaType().body("Screenshots must be PNG, JPEG, WebP or GIF"));
        };
        {
            let mut screenshots = SCREENSHOTS.lock().unwrap_or_else(|e| e.into_inner());
            let stored: usize = screenshots.iter().filter(|(key, _)| **key != *id).map(|(_, (_, bytes))| bytes.len()).sum();
            if stored + bytes.len() > SCREENSHOT_STORE_LIMIT {
                return Ok(HttpResponse::InsufficientStorage().body("Screenshot storage is full, try again later"));
            }
            screenshots.insert(*id, (kind, bytes.freeze()));
        }
        let url = format!("/api/projects/{}/screenshot?v={}", *id, chrono::Utc::now().timestamp_millis());
        sqlx::query("UPDATE project SET screenshot_url = $2, updated_at = now() WHERE id = $1")
            .bind(*id)
            .bind(&url)
            .execute(pool.get_ref())
            .await?;
        Ok(HttpResponse::Ok().json(json!({ "screenshot_url": url })))
    }
    .await;
    result.unwrap_or_else(|e| failed("uploading a screenshot", e))
}

#[get("/api/projects/{id:\\d+}/screenshot")]
pub async fn screenshot(id: web::Path<i32>) -> HttpResponse {
    let stored = SCREENSHOTS.lock().unwrap_or_else(|e| e.into_inner()).get(&*id).cloned();
    match stored {
        Some((kind, bytes)) => HttpResponse::Ok()
            .content_type(kind)
            .insert_header(("X-Content-Type-Options", "nosniff"))
            .body(bytes),
        None => HttpResponse::NotFound().finish(),
    }
}

#[get("/api/hackatime/projects")]
pub async fn hackatime_list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<HttpResponse> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(HttpResponse::Unauthorized().finish());
        };
        let Some(token) = hackatime_token(&req, &pool, user).await? else {
            return Ok(HttpResponse::Ok().json(json!({ "linked": false })));
        };
        Ok(match get_hackatime_project_summaries_with_token(&token).await {
            Ok(projects) => HttpResponse::Ok().json(json!({
                "linked": true,
                "projects": projects
                    .into_iter()
                    .map(|project| json!({ "name": project.name, "seconds": project.total_duration_seconds }))
                    .collect::<Vec<_>>(),
            })),
            Err(e) => {
                eprintln!("loading Hackatime projects failed: {e}");
                HttpResponse::Ok().json(json!({ "linked": false }))
            }
        })
    }
    .await;
    result.unwrap_or_else(|e| failed("loading Hackatime projects", e))
}

#[get("/hackatime/login")]
pub async fn link(req: HttpRequest) -> HttpResponse {
    let scopes = Scopes {
        profile: true,
        read: true,
        admin: false,
    };
    login::handle_login(&req, scopes, auth::site_url(CALLBACK), true).await
}

#[get("/hackatime/callback")]
pub async fn linked(req: HttpRequest, query: web::Query<CallbackArgs>, pool: web::Data<PgPool>) -> HttpResponse {
    let token = Rc::new(Cell::new(None));
    let sink = token.clone();
    let response = login::handle_callback(
        &req,
        query,
        auth::site_url(CALLBACK),
        "/hq?left=projects".to_string(),
        Some(async move |value| sink.set(Some(value))),
        true,
    )
    .await;
    let Some(token) = token.take() else {
        return response;
    };
    let result: anyhow::Result<Option<bool>> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(None);
        };
        let account = get_hackatime_user_with_token(&token).await?;
        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM \"user\" WHERE htme_id = $1 AND id <> $2)")
            .bind(account.user_id as i64)
            .bind(user)
            .fetch_one(pool.get_ref())
            .await?;
        if taken {
            return Ok(Some(false));
        }
        sqlx::query("UPDATE \"user\" SET htme_id = $2, htme_trust_level = $3, htme_linked_at = now() WHERE id = $1")
            .bind(user)
            .bind(account.user_id as i64)
            .bind(&account.trust_factor.trust_level)
            .execute(pool.get_ref())
            .await?;
        Ok(Some(true))
    }
    .await;
    match result {
        Ok(Some(false)) => HttpResponse::Conflict().body("That Hackatime account is already linked to another Sunker account"),
        Ok(Some(true)) => response,
        Ok(None) => HttpResponse::Unauthorized().body("Sign in to Sunker before linking Hackatime"),
        Err(e) => {
            eprintln!("linking Hackatime failed: {e:#}");
            HttpResponse::InternalServerError().body("Couldn't link Hackatime, please try again")
        }
    }
}
