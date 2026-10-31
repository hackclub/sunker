use crate::auth;
use actix_web::{HttpRequest, HttpResponse, get, web};
use serde_json::{Value, json};
use sqlx::PgPool;

const ACCESS: &str = "chat.id = ANY(\"user\".allowed_chats)";

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
