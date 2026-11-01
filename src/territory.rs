use actix_web::{HttpResponse, get, web};
use serde_json::Value;
use sqlx::PgPool;

#[get("/api/territory")]
pub async fn list(pool: web::Data<PgPool>) -> HttpResponse {
    let rows: Result<Vec<Value>, _> = sqlx::query_scalar(
        "SELECT jsonb_build_object('row', hex_row, 'col', hex_col, 'occupied', is_occupied, 'enemies', enemies)
         FROM territory",
    )
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            eprintln!("listing territory failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
