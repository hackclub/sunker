use crate::admin::{self, Admin, Permission};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde_json::{Value, json};
use sqlx::PgPool;

const PAGE: i64 = 100;

const AUTHOR: &str = "jsonb_build_object(
    'id', u.id,
    'handle', c.handle,
    'name', nullif(concat_ws(' ', u.first_name, u.last_name), ''),
    'banned', u.is_banned
)";

fn failed(what: &str, e: sqlx::Error) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

#[get("/api/admin/moderation/chats")]
pub async fn chats(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Moderation).await {
        return response;
    }
    let rows: Result<Vec<Value>, _> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'id', ch.id,
             'name', ch.name,
             'kind', CASE WHEN ch.user_id IS NOT NULL THEN 'private' WHEN ch.guild_id IS NOT NULL THEN 'guild' ELSE 'all' END,
             'user', CASE WHEN ch.user_id IS NOT NULL THEN {AUTHOR} END
         )
         FROM chat ch
         LEFT JOIN \"user\" u ON u.id = ch.user_id
         LEFT JOIN character c ON c.user_id = u.id
         WHERE ch.user_id IS NULL OR u.is_banned
         ORDER BY ch.user_id IS NOT NULL, ch.guild_id IS NOT NULL, ch.id"
    )))
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => failed("listing moderation chats", e),
    }
}

#[get("/api/admin/moderation/chats/{id:\\d+}/messages")]
pub async fn messages(
    req: HttpRequest,
    id: web::Path<i32>,
    query: web::Query<Vec<(String, String)>>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Moderation).await {
        return response;
    }
    let param = |key: &str| -> Option<i64> {
        query
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, value)| value.parse().ok())
    };
    let rows: Result<Vec<Value>, _> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'id', m.id,
             'author', {AUTHOR},
             'contents', m.contents,
             'created_at', m.created_at,
             'deleted_at', m.deleted_at,
             'flags', (SELECT count(*) FROM message_flag f WHERE f.message_id = m.id AND f.dismissed_at IS NULL),
             'reply_to', (SELECT jsonb_build_object('id', r.id, 'contents', r.contents, 'handle', rc.handle, 'deleted', r.deleted_at IS NOT NULL)
                          FROM message r LEFT JOIN character rc ON rc.user_id = r.author_id WHERE r.id = m.reply_to)
         )
         FROM (
             SELECT * FROM message
             WHERE chat_id = $1
               AND ($2::bigint IS NULL OR id < $2)
               AND ($3::bigint IS NULL OR id >= coalesce(
                   (SELECT min(id) FROM (SELECT id FROM message WHERE chat_id = $1 AND id <= $3 ORDER BY id DESC LIMIT 25) older), $3))
             ORDER BY CASE WHEN $3::bigint IS NULL THEN id END DESC, id
             LIMIT CASE WHEN $3::bigint IS NULL THEN $4 ELSE 1000 END
         ) m
         JOIN \"user\" u ON u.id = m.author_id
         LEFT JOIN character c ON c.user_id = u.id
         ORDER BY m.id"
    )))
    .bind(*id)
    .bind(param("before"))
    .bind(param("from"))
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => failed("listing moderation messages", e),
    }
}

#[get("/api/admin/moderation/flags")]
pub async fn flags(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Moderation).await {
        return response;
    }
    let rows: Result<Vec<Value>, _> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'id', m.id,
             'chat', jsonb_build_object('id', ch.id, 'name', ch.name),
             'author', {AUTHOR},
             'contents', m.contents,
             'created_at', m.created_at,
             'flags', f.count,
             'flagged_at', f.latest
         )
         FROM (
             SELECT message_id, count(*) AS count, max(created_at) AS latest FROM message_flag
             WHERE dismissed_at IS NULL GROUP BY message_id
         ) f
         JOIN message m ON m.id = f.message_id AND m.deleted_at IS NULL
         JOIN chat ch ON ch.id = m.chat_id
         JOIN \"user\" u ON u.id = m.author_id
         LEFT JOIN character c ON c.user_id = u.id
         ORDER BY f.latest DESC"
    )))
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => failed("listing flags", e),
    }
}

async fn record(
    pool: &PgPool,
    admin: &Admin,
    action: &str,
    sql: &str,
    target: i64,
    summary: String,
) -> HttpResponse {
    let outcome = async {
        let mut tx = pool.begin().await?;
        let user: Option<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_string()))
            .bind(target)
            .bind(admin.id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(user) = user else {
            return Ok(None);
        };
        sqlx::query(
            "INSERT INTO admin_action (admin_id, action, user_id, summary) VALUES ($1, $2, $3, $4)",
        )
        .bind(admin.id)
        .bind(action)
        .bind(user)
        .bind(&summary)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(Some(summary))
    }
    .await;
    match outcome {
        Ok(Some(summary)) => HttpResponse::Ok().json(json!({ "summary": summary })),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => failed(&format!("running {action}"), e),
    }
}

#[post("/api/admin/moderation/messages/{id:\\d+}/dismiss")]
pub async fn dismiss(req: HttpRequest, id: web::Path<i64>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Moderation).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let sql = "WITH dismissed AS (
                   UPDATE message_flag SET dismissed_at = now(), dismissed_by = $2
                   WHERE message_id = $1 AND dismissed_at IS NULL RETURNING message_id
               )
               SELECT m.author_id FROM message m WHERE m.id = $1 AND EXISTS (SELECT 1 FROM dismissed)";
    record(&pool, &admin, "flag_dismiss", sql, *id, format!("Dismissed flags on message #{id}")).await
}

#[post("/api/admin/moderation/messages/{id:\\d+}/delete")]
pub async fn delete(req: HttpRequest, id: web::Path<i64>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Moderation).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let sql = "UPDATE message SET deleted_at = now(), deleted_by = $2
               WHERE id = $1 AND deleted_at IS NULL RETURNING author_id";
    record(&pool, &admin, "message_delete", sql, *id, format!("Deleted message #{id}")).await
}

#[post("/api/admin/moderation/users/{id:\\d+}/ban")]
pub async fn ban(req: HttpRequest, id: web::Path<i64>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Moderation).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    if !admin.can(Permission::Ban) {
        return HttpResponse::Forbidden().body("You don't have permission to ban");
    }
    let sql = "UPDATE \"user\" SET is_banned = true WHERE id = $1::int AND id <> $2 RETURNING id";
    record(&pool, &admin, "ban", sql, *id, "Banned user".to_string()).await
}

#[post("/api/admin/moderation/chats/{id:\\d+}/messages")]
pub async fn send(
    req: HttpRequest,
    id: web::Path<i32>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Moderation).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let private: Result<bool, _> =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM chat WHERE id = $1 AND user_id IS NOT NULL)")
            .bind(*id)
            .fetch_one(pool.get_ref())
            .await;
    match private {
        Ok(true) => crate::chats::send(&pool, admin.id, *id, &body).await,
        Ok(false) => HttpResponse::NotFound().finish(),
        Err(e) => failed("checking moderation chat", e),
    }
}
