use crate::{admin, auth};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde_json::{Value, json};
use sqlx::PgPool;

pub const STATUSES: [&str; 4] = ["pending", "rejected", "ordered", "fulfilled"];
const PAGE: i64 = 200;

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

#[get("/api/orders")]
pub async fn list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<Option<Value>> = async {
        let Some(user) = auth::user_id(&req, &pool).await? else {
            return Ok(None);
        };
        sqlx::query_scalar(
            "SELECT coalesce(jsonb_agg(jsonb_build_object(
                 'id', o.id, 'status', o.status, 'created_at', o.created_at, 'prize', z.name
             ) ORDER BY o.created_at DESC), '[]')
             FROM \"order\" o JOIN prize z ON z.id = o.prize_id
             WHERE o.user_id = $1 AND o.deleted_at IS NULL",
        )
        .bind(user)
        .fetch_one(pool.get_ref())
        .await
        .map(Some)
    }
    .await;
    match result {
        Ok(Some(orders)) => HttpResponse::Ok().json(json!({ "orders": orders })),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("listing orders", e),
    }
}

#[get("/api/admin/orders")]
pub async fn overview(req: HttpRequest, query: web::Query<Vec<(String, String)>>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, admin::Permission::Resources).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let status = query
        .iter()
        .find(|(key, _)| key == "status")
        .map(|(_, value)| value.as_str())
        .filter(|value| STATUSES.contains(value));
    let result: sqlx::Result<Value> = sqlx::query_scalar(
        "SELECT coalesce(jsonb_agg(row ORDER BY row.created_at DESC), '[]') FROM (
             SELECT o.id, o.user_id, o.status, o.tracking_number, o.created_at, o.updated_at, o.fulfilled_at,
                    z.name AS prize, z.ring AS prize_ring,
                    nullif(concat_ws(' ', u.first_name, u.last_name), '') AS user_name
             FROM \"order\" o JOIN prize z ON z.id = o.prize_id JOIN \"user\" u ON u.id = o.user_id
             WHERE o.deleted_at IS NULL AND ($1::text IS NULL OR o.status = $1)
             ORDER BY o.created_at DESC LIMIT $2
         ) row",
    )
    .bind(status)
    .bind(PAGE)
    .fetch_one(pool.get_ref())
    .await;
    match result {
        Ok(orders) => HttpResponse::Ok().json(json!({ "orders": orders, "users": admin.can(admin::Permission::Users) })),
        Err(e) => failed("listing admin orders", e),
    }
}

#[post("/api/admin/orders/{id:\\d+}")]
pub async fn update(req: HttpRequest, id: web::Path<i32>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, admin::Permission::Resources).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let owner: sqlx::Result<Option<i32>> = sqlx::query_scalar("SELECT user_id FROM \"order\" WHERE id = $1")
        .bind(*id)
        .fetch_optional(pool.get_ref())
        .await;
    let user = match owner {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("Order not found"),
        Err(e) => return failed("finding an order", e),
    };
    let mut body = body.into_inner();
    body["kind"] = "order_status".into();
    body["order_id"] = (*id).into();
    crate::admin_users::run(&pool, user, admin.id, &body).await
}
