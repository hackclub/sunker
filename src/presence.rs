use crate::auth;
use actix_web::{HttpRequest, HttpResponse, get, web};
use serde_json::Value;
use sqlx::PgPool;

#[get("/api/online")]
pub async fn online(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<Option<Vec<Value>>> = async {
        if auth::user_id(&req, &pool).await?.is_none() {
            return Ok(None);
        }
        let rows = sqlx::query_scalar(
            "SELECT jsonb_build_object(
                 'id', u.id,
                 'character', jsonb_build_object('handle', c.handle, 'attributes', c.attributes),
                 'guild', (SELECT jsonb_build_object('slug', g.slug, 'name', g.name) FROM guild g WHERE g.id = u.guild_id)
             )
             FROM \"user\" u
             JOIN character c ON c.user_id = u.id
             WHERE u.deleted_at IS NULL AND NOT u.is_banned AND u.last_seen_at > now() - interval '5 minutes'
             ORDER BY u.id
             LIMIT 100",
        )
        .fetch_all(pool.get_ref())
        .await?;
        Ok(Some(rows))
    }
    .await;
    match result {
        Ok(Some(rows)) => HttpResponse::Ok().json(rows),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => {
            eprintln!("listing online users failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
