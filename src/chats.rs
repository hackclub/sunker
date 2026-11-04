use crate::auth;
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde_json::{Value, json};
use sqlx::PgPool;

const ACCESS: &str = "CASE WHEN \"user\".is_banned THEN chat.user_id = \"user\".id
                      ELSE chat.id = ANY(\"user\".allowed_chats) END";

async fn allowed(req: &HttpRequest, pool: &PgPool) -> sqlx::Result<Option<Vec<Value>>> {
    let Some(user) = auth::user_id(req, pool).await? else {
        return Ok(None);
    };
    let chats: Vec<(i32, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT chat.id, chat.name FROM chat
         JOIN \"user\" ON {ACCESS}
         WHERE \"user\".id = $1
         ORDER BY chat.id"
    )))
    .bind(user)
    .fetch_all(pool)
    .await?;
    Ok(Some(
        chats
            .into_iter()
            .map(|(id, name)| json!({ "id": id, "name": name }))
            .collect(),
    ))
}

#[get("/api/chats")]
pub async fn list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    match allowed(&req, &pool).await {
        Ok(Some(chats)) => HttpResponse::Ok().json(chats),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => {
            eprintln!("listing chats failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn member(req: &HttpRequest, pool: &PgPool, chat: i32) -> Result<i32, HttpResponse> {
    let failed = |e: sqlx::Error| {
        eprintln!("checking chat access failed: {e}");
        HttpResponse::InternalServerError().finish()
    };
    let Some(user) = auth::user_id(req, pool).await.map_err(failed)? else {
        return Err(HttpResponse::Unauthorized().finish());
    };
    let allowed: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT EXISTS (SELECT 1 FROM chat JOIN \"user\" ON {ACCESS} WHERE chat.id = $1 AND \"user\".id = $2)"
    )))
    .bind(chat)
    .bind(user)
    .fetch_one(pool)
    .await
    .map_err(failed)?;
    if allowed { Ok(user) } else { Err(HttpResponse::Forbidden().finish()) }
}

#[get("/api/chats/{id:\\d+}/messages")]
pub async fn messages(
    req: HttpRequest,
    id: web::Path<i32>,
    query: web::Query<Vec<(String, String)>>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let user = match member(&req, &pool, *id).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    let around = query
        .iter()
        .find(|(name, _)| name == "around")
        .and_then(|(_, value)| value.parse::<i64>().ok());
    let rows: Result<Vec<Value>, _> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
             'id', m.id,
             'author', jsonb_build_object('handle', c.handle),
             'contents', m.contents,
             'created_at', m.created_at,
             'flagged', EXISTS (SELECT 1 FROM message_flag f WHERE f.message_id = m.id AND f.user_id = $2),
             'reply_to', (SELECT jsonb_build_object(
                              'id', r.id,
                              'contents', CASE WHEN r.deleted_at IS NULL THEN r.contents END,
                              'handle', rc.handle,
                              'deleted', r.deleted_at IS NOT NULL)
                          FROM message r LEFT JOIN character rc ON rc.user_id = r.author_id WHERE r.id = m.reply_to)
         )
         FROM (
             SELECT * FROM message
             WHERE chat_id = $1 AND deleted_at IS NULL
               AND ($3::bigint IS NULL OR id >= coalesce(
                   (SELECT min(id) FROM (SELECT id FROM message WHERE chat_id = $1 AND deleted_at IS NULL AND id <= $3 ORDER BY id DESC LIMIT 25) older), $3))
             ORDER BY CASE WHEN $3::bigint IS NULL THEN created_at END DESC, created_at
             LIMIT CASE WHEN $3::bigint IS NULL THEN 100 ELSE 1000 END
         ) m
         LEFT JOIN character c ON c.user_id = m.author_id
         ORDER BY m.created_at",
    )
    .bind(*id)
    .bind(user)
    .bind(around)
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            eprintln!("listing messages failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn send(pool: &PgPool, author: i32, chat: i32, body: &Value) -> HttpResponse {
    let contents = body["contents"].as_str().unwrap_or_default().trim();
    if contents.is_empty() || contents.chars().count() > 2000 {
        return HttpResponse::BadRequest().body("Messages must be 1 to 2000 characters");
    }
    let id: Result<Option<i64>, _> = sqlx::query_scalar(
        "INSERT INTO message (author_id, chat_id, contents, reply_to)
         SELECT $1, $2, $3, $4
         WHERE $4::bigint IS NULL OR EXISTS (SELECT 1 FROM message WHERE id = $4 AND chat_id = $2)
         RETURNING id",
    )
    .bind(author)
    .bind(chat)
    .bind(contents)
    .bind(body["reply_to"].as_i64())
    .fetch_optional(pool)
    .await;
    match id {
        Ok(Some(id)) => HttpResponse::Ok().json(json!({ "id": id })),
        Ok(None) => HttpResponse::BadRequest().body("That reply is to a message in another chat"),
        Err(e) => {
            eprintln!("sending message failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/api/chats/{id:\\d+}/messages")]
pub async fn create(
    req: HttpRequest,
    id: web::Path<i32>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let user = match member(&req, &pool, *id).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    let recent: Result<bool, _> = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM message WHERE author_id = $1 AND chat_id = $2 AND created_at > now() - interval '3 seconds')",
    )
    .bind(user)
    .bind(*id)
    .fetch_one(pool.get_ref())
    .await;
    match recent {
        Ok(false) => send(&pool, user, *id, &body).await,
        Ok(true) => HttpResponse::TooManyRequests().body("Slow down, wait 3 seconds between messages"),
        Err(e) => {
            eprintln!("checking slowmode failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/api/messages/{id:\\d+}/flag")]
pub async fn flag(
    req: HttpRequest,
    id: web::Path<i64>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let Some(flagged) = body["flagged"].as_bool() else {
        return HttpResponse::BadRequest().body("missing flagged");
    };
    let outcome = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(None);
        };
        let applied = if flagged {
            "INSERT INTO message_flag (message_id, user_id) SELECT id, $2 FROM target ON CONFLICT DO NOTHING"
        } else {
            "DELETE FROM message_flag WHERE user_id = $2 AND message_id IN (SELECT id FROM target)"
        };
        let found: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "WITH target AS (
                 SELECT m.id FROM message m
                 JOIN chat ON chat.id = m.chat_id
                 JOIN \"user\" ON {ACCESS}
                 WHERE m.id = $1 AND \"user\".id = $2 AND NOT \"user\".is_banned AND m.deleted_at IS NULL
             ), applied AS ({applied})
             SELECT EXISTS (SELECT 1 FROM target)"
        )))
        .bind(*id)
        .bind(user)
        .fetch_one(pool.get_ref())
        .await?;
        Ok::<_, sqlx::Error>(Some(found))
    }
    .await;
    match outcome {
        Ok(Some(true)) => HttpResponse::Ok().json(json!({ "flagged": flagged })),
        Ok(Some(false)) => HttpResponse::NotFound().finish(),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => {
            eprintln!("flagging message failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
