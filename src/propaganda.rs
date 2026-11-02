use crate::admin::{self, Permission};
use crate::auth;
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use reqwest::{StatusCode, Url};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const API: &str = "https://unified-socials-db.hackclub.com/api/v1/tracked-posts";
const HOSTS: [&str; 6] = ["youtube.com", "youtu.be", "instagram.com", "tiktok.com", "facebook.com", "fb.watch"];
const HOURLY_LIMIT: u32 = 100;
const UPGRADE_HOURS: i64 = 100;
const FILM: &str = "Film";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
static SUBMITTED: LazyLock<Mutex<(Instant, u32)>> = LazyLock::new(|| Mutex::new((Instant::now(), 0)));

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

fn token() -> String {
    std::env::var("SOCIALS_TOKEN").unwrap_or_default()
}

fn supported(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        ["http", "https"].contains(&url.scheme())
            && url
                .host_str()
                .is_some_and(|host| HOSTS.iter().any(|known| host == *known || host.ends_with(&format!(".{known}"))))
    })
}

fn reserve() -> bool {
    let mut submitted = SUBMITTED.lock().unwrap();
    if submitted.0.elapsed() >= Duration::from_secs(3600) {
        *submitted = (Instant::now(), 0);
    }
    submitted.1 += 1;
    submitted.1 <= HOURLY_LIMIT
}

async fn attribute(_tracked_post_id: &str) {
    // The socials API can't set attribution yet, so this only reads who it should go to
    let _author = std::env::var("SOCIALS_ATTRIBUTION").unwrap_or_default();
}

async fn refresh(pool: &PgPool, id: i32, tracked_post_id: &str) -> anyhow::Result<()> {
    let tracked: Value = CLIENT
        .get(format!("{API}/{tracked_post_id}"))
        .bearer_auth(token())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let metric = &tracked["metrics"]["weighted_watch_hours"];
    let hours = metric.as_f64().or_else(|| metric["value"].as_f64());
    sqlx::query(
        "UPDATE video SET title = coalesce($2, title), watch_seconds = coalesce($3, watch_seconds), tracked = $4, polled_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(tracked["post"]["title"].as_str())
    .bind(hours.map(|hours| (hours * 3600.0).round() as i64))
    .bind(&tracked)
    .execute(pool)
    .await?;
    Ok(())
}

async fn grant(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(
        "WITH due AS (
             SELECT v.user_id, floor(sum(v.watch_seconds) / ($1 * 3600.0))::int
                 - coalesce((SELECT sum(l.ring_upgrade) FROM resource_ledger l WHERE l.user_id = v.user_id AND l.reason = $2), 0)::int AS owed
             FROM video v WHERE v.status = 'approved' GROUP BY v.user_id
         ), owed AS (
             SELECT * FROM due WHERE owed > 0
         ), granted AS (
             UPDATE resources r SET ring_upgrade = ring_upgrade + owed.owed FROM owed WHERE r.user_id = owed.user_id
         )
         INSERT INTO resource_ledger (user_id, reason, ring_upgrade) SELECT user_id, $2, owed FROM owed",
    )
    .bind(UPGRADE_HOURS)
    .bind(FILM)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn poll(pool: &PgPool) {
    let due: Vec<(i32, String)> = match sqlx::query_as(
        "SELECT id, tracked_post_id FROM video
         WHERE tracked_post_id IS NOT NULL AND status <> 'rejected' AND (polled_at IS NULL OR polled_at < now() - interval '1 day')",
    )
    .fetch_all(pool)
    .await
    {
        Ok(due) => due,
        Err(e) => return eprintln!("listing videos to poll failed: {e}"),
    };
    for (id, tracked_post_id) in due {
        if let Err(e) = refresh(pool, id, &tracked_post_id).await {
            eprintln!("polling video #{id} failed: {e:#}");
        }
    }
    if let Err(e) = grant(pool).await {
        eprintln!("granting film ring upgrades failed: {e}");
    }
}

#[get("/api/propaganda")]
pub async fn overview(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<Option<(Value, String)>> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(None);
        };
        sqlx::query_as(
            "SELECT jsonb_build_object(
                 'videos', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                     'id', v.id, 'url', v.url, 'title', v.title, 'hours', v.watch_seconds / 3600.0, 'status', v.status, 'polled_at', v.polled_at
                 ) ORDER BY v.created_at DESC), '[]') FROM video v WHERE v.user_id = u.id),
                 'hours', (SELECT coalesce(sum(v.watch_seconds), 0) / 3600.0 FROM video v WHERE v.user_id = u.id AND v.status = 'approved'),
                 'upgrades', (SELECT coalesce(sum(l.ring_upgrade), 0) FROM resource_ledger l WHERE l.user_id = u.id AND l.reason = $2)
             ), u.referral_code
             FROM \"user\" u WHERE u.id = $1",
        )
        .bind(user)
        .bind(FILM)
        .fetch_optional(pool.get_ref())
        .await
    }
    .await;
    match result {
        Ok(Some((mut view, code))) => {
            view["link"] = json!(auth::site_url(&format!("/?ref={code}")));
            HttpResponse::Ok().json(view)
        }
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("loading propaganda", e),
    }
}

#[post("/api/propaganda/videos")]
pub async fn submit(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let url = body["url"].as_str().unwrap_or_default().trim();
    let user = match auth::user_id(&req, &pool).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::Unauthorized().finish(),
        Err(e) => return failed("submitting a video", e),
    };
    if !supported(url) {
        return HttpResponse::BadRequest().body("That isn't a YouTube, Instagram, TikTok or Facebook link");
    }
    match sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM video WHERE url = $1)")
        .bind(url)
        .fetch_one(pool.get_ref())
        .await
    {
        Ok(true) => return HttpResponse::Conflict().body("That video has already been submitted"),
        Ok(false) => {}
        Err(e) => return failed("submitting a video", e),
    }
    if !reserve() {
        return HttpResponse::TooManyRequests().body("Too many videos were submitted this hour, try again later");
    }
    let response = match CLIENT.post(API).bearer_auth(token()).json(&json!({ "url": url })).send().await {
        Ok(response) => response,
        Err(e) => return failed("tracking a video", e),
    };
    if response.status() == StatusCode::BAD_REQUEST {
        return HttpResponse::BadRequest().body("That link isn't a post that can be tracked");
    }
    let tracked: Value = match response.error_for_status() {
        Ok(response) => match response.json().await {
            Ok(tracked) => tracked,
            Err(e) => return failed("tracking a video", e),
        },
        Err(e) => return failed("tracking a video", e),
    };
    let Some(tracked_post_id) = tracked["tracked_post_id"]
        .as_str()
        .map(str::to_string)
        .or_else(|| tracked["tracked_post_id"].as_i64().map(|id| id.to_string()))
    else {
        return failed("tracking a video", "no tracked_post_id in the response");
    };
    attribute(&tracked_post_id).await;
    let id: i32 = match sqlx::query_scalar(
        "INSERT INTO video (user_id, url, tracked_post_id) VALUES ($1, $2, $3) ON CONFLICT (url) DO NOTHING RETURNING id",
    )
    .bind(user)
    .bind(url)
    .bind(&tracked_post_id)
    .fetch_optional(pool.get_ref())
    .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return HttpResponse::Conflict().body("That video has already been submitted"),
        Err(e) => return failed("saving a video", e),
    };
    if let Err(e) = refresh(&pool, id, &tracked_post_id).await {
        eprintln!("polling video #{id} failed: {e:#}");
    }
    HttpResponse::NoContent().finish()
}

#[get("/api/admin/videos")]
pub async fn queue(req: HttpRequest, query: web::Query<HashMap<String, String>>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Video).await {
        return response;
    }
    let status = query.get("status").map(String::as_str).unwrap_or("pending");
    let person = "jsonb_build_object('id', x.id, 'name', nullif(concat_ws(' ', x.first_name, x.last_name), ''), 'slack_id', x.slack_id)";
    let videos: sqlx::Result<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'videos', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                 'id', v.id, 'url', v.url, 'title', v.title, 'hours', v.watch_seconds / 3600.0, 'status', v.status,
                 'tracked', v.tracked, 'polled_at', v.polled_at, 'created_at', v.created_at, 'reviewed_at', v.reviewed_at,
                 'user', (SELECT {person} FROM \"user\" x WHERE x.id = v.user_id),
                 'reviewer', (SELECT {person} FROM \"user\" x WHERE x.id = v.reviewed_by)
             ) ORDER BY v.created_at), '[]') FROM video v WHERE v.status = $1),
             'counts', (SELECT coalesce(jsonb_object_agg(status, total), '{{}}') FROM (SELECT status, count(*) AS total FROM video GROUP BY status) c)
         )"
    )))
    .bind(status)
    .fetch_one(pool.get_ref())
    .await;
    match videos {
        Ok(videos) => HttpResponse::Ok().json(videos),
        Err(e) => failed("listing videos to verify", e),
    }
}

#[post("/api/admin/videos/{id:\\d+}/refresh")]
pub async fn repoll(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Video).await {
        return response;
    }
    let id = id.into_inner();
    let tracked_post_id: Option<String> = match sqlx::query_scalar("SELECT tracked_post_id FROM video WHERE id = $1")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(found) => found.flatten(),
        Err(e) => return failed("refreshing a video", e),
    };
    let Some(tracked_post_id) = tracked_post_id else {
        return HttpResponse::NotFound().finish();
    };
    match refresh(&pool, id, &tracked_post_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("polling video #{id} failed: {e:#}");
            HttpResponse::BadGateway().body("The socials API couldn't be reached")
        }
    }
}

#[post("/api/admin/videos/{id:\\d+}")]
pub async fn verify(req: HttpRequest, id: web::Path<i32>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Video).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let status = body["status"].as_str().unwrap_or_default();
    if !["approved", "rejected"].contains(&status) {
        return HttpResponse::BadRequest().body("Status must be approved or rejected");
    }
    let result: sqlx::Result<Option<i32>> = async {
        let mut tx = pool.begin().await?;
        let row: Option<(i32, String)> = sqlx::query_as(
            "UPDATE video SET status = $2, reviewed_by = $3, reviewed_at = now()
             WHERE id = $1 AND status <> 'approved' AND status <> $2 RETURNING user_id, url",
        )
        .bind(*id)
        .bind(status)
        .bind(admin.id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((user, url)) = row else {
            return Ok(None);
        };
        sqlx::query("INSERT INTO admin_action (admin_id, action, user_id, summary) VALUES ($1, $2, $3, $4)")
            .bind(admin.id)
            .bind(if status == "approved" { "video_approve" } else { "video_reject" })
            .bind(user)
            .bind(format!("{} video {url}", if status == "approved" { "Approved" } else { "Rejected" }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(user))
    }
    .await;
    match result {
        Ok(Some(_)) => {
            if status == "approved" {
                if let Err(e) = grant(&pool).await {
                    eprintln!("granting film ring upgrades failed: {e}");
                }
            }
            HttpResponse::NoContent().finish()
        }
        Ok(None) => HttpResponse::Conflict().body("That video was already verified"),
        Err(e) => failed("verifying a video", e),
    }
}
