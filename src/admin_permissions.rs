use crate::admin::{self, Admin, PERMISSIONS, Permission};
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;

const PAGE: i64 = 50;

const PERSON: &str = "jsonb_build_object('id', u.id, 'name', nullif(concat_ws(' ', u.first_name, u.last_name), ''), 'email', u.email, 'hca_id', u.hca_id)";

fn failed(what: &str, e: sqlx::Error) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

fn pattern(search: &str) -> String {
    format!(
        "%{}%",
        search
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

fn columns() -> String {
    PERMISSIONS
        .iter()
        .map(|name| format!("p.{name}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn names(permissions: &[bool]) -> String {
    let granted: Vec<&str> = PERMISSIONS
        .iter()
        .zip(permissions)
        .filter(|(_, value)| **value)
        .map(|(name, _)| *name)
        .collect();
    if granted.is_empty() { "nothing".into() } else { granted.join(", ") }
}

async fn writer(req: &HttpRequest, pool: &PgPool) -> Result<Admin, HttpResponse> {
    let admin = admin::require(req, pool, Permission::Administration).await?;
    if admin.superadmin {
        Ok(admin)
    } else {
        Err(HttpResponse::Forbidden().body("Only superadmins can change permissions"))
    }
}

async fn log(
    tx: &mut Transaction<'_, Postgres>,
    admin: i32,
    action: &str,
    user: i32,
    summary: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO admin_action (admin_id, action, user_id, summary) VALUES ($1, $2, $3, $4)")
        .bind(admin)
        .bind(action)
        .bind(user)
        .bind(summary)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[get("/api/admin/permissions")]
pub async fn list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Administration).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let superadmins = admin::superadmins();
    let result: sqlx::Result<(Value, Value)> = async {
        let found: Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_object_agg(u.hca_id, {PERSON}), '{{}}') FROM \"user\" u WHERE u.hca_id = ANY($1)"
        )))
        .bind(&superadmins)
        .fetch_one(pool.get_ref())
        .await?;
        let users: Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_agg(jsonb_build_object(
                 'user', {PERSON},
                 'permissions', to_jsonb(ARRAY[{}]),
                 'created_at', p.created_at
             ) ORDER BY p.created_at), '[]')
             FROM permission p JOIN \"user\" u ON u.id = p.user_id",
            columns()
        )))
        .fetch_one(pool.get_ref())
        .await?;
        Ok((found, users))
    }
    .await;
    let (found, users) = match result {
        Ok(rows) => rows,
        Err(e) => return failed("listing permissions", e),
    };
    let users: Vec<Value> = users
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            let granted: Vec<bool> = row["permissions"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|value| value.as_bool().unwrap_or(false))
                .collect();
            json!({
                "user": row["user"],
                "permissions": admin::permissions_json(&granted),
                "created_at": row["created_at"],
            })
        })
        .collect();
    let superadmins: Vec<Value> = superadmins
        .iter()
        .map(|hca_id| json!({ "hca_id": hca_id, "user": found.get(hca_id) }))
        .collect();
    HttpResponse::Ok().json(json!({
        "permissions": PERMISSIONS,
        "can_write": admin.superadmin,
        "superadmins": superadmins,
        "users": users,
    }))
}

#[get("/api/admin/permissions/search")]
pub async fn candidates(
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if let Err(response) = writer(&req, &pool).await {
        return response;
    }
    let search = query.get("q").map(|q| q.trim()).unwrap_or_default();
    if search.is_empty() {
        return HttpResponse::Ok().json(json!([]));
    }
    let rows: sqlx::Result<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT coalesce(jsonb_agg({PERSON} ORDER BY u.id), '[]') FROM (
             SELECT * FROM \"user\" u
             WHERE u.deleted_at IS NULL
               AND NOT EXISTS (SELECT 1 FROM permission p WHERE p.user_id = u.id)
               AND u.hca_id <> ALL($3)
               AND (u.id::text = $1 OR u.hca_id ILIKE $2 OR u.email ILIKE $2 OR u.slack_id ILIKE $2
                    OR concat_ws(' ', u.first_name, u.last_name) ILIKE $2)
             ORDER BY u.id
             LIMIT 10
         ) u"
    )))
    .bind(search)
    .bind(pattern(search))
    .bind(admin::superadmins())
    .fetch_one(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => failed("searching users", e),
    }
}

#[post("/api/admin/permissions/{id:\\d+}")]
pub async fn add(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match writer(&req, &pool).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let outcome: sqlx::Result<Option<&str>> = async {
        let mut tx = pool.begin().await?;
        let hca_id: Option<String> =
            sqlx::query_scalar("SELECT hca_id FROM \"user\" WHERE id = $1 AND deleted_at IS NULL")
                .bind(*id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(hca_id) = hca_id else {
            return Ok(Some("User not found"));
        };
        if admin::superadmins().contains(&hca_id) {
            return Ok(Some("Superadmins already have every permission"));
        }
        let inserted = sqlx::query("INSERT INTO permission (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(*id)
            .execute(&mut *tx)
            .await?;
        if inserted.rows_affected() == 0 {
            return Ok(Some("User already has elevated access"));
        }
        log(&mut tx, admin.id, "permission_add", *id, "Added to elevated access").await?;
        tx.commit().await?;
        Ok(None)
    }
    .await;
    match outcome {
        Ok(None) => HttpResponse::NoContent().finish(),
        Ok(Some(message)) => HttpResponse::BadRequest().body(message),
        Err(e) => failed("adding permissions", e),
    }
}

#[put("/api/admin/permissions/{id:\\d+}")]
pub async fn update(
    req: HttpRequest,
    id: web::Path<i32>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match writer(&req, &pool).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let outcome: sqlx::Result<Result<(), HttpResponse>> = async {
        let mut tx = pool.begin().await?;
        let old: Option<Vec<bool>> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT ARRAY[{}] FROM permission p WHERE p.user_id = $1 FOR UPDATE",
            columns()
        )))
        .bind(*id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(old) = old else {
            return Ok(Err(HttpResponse::NotFound().finish()));
        };
        let new: Vec<bool> = PERMISSIONS
            .iter()
            .zip(&old)
            .map(|(name, old)| body["permissions"][name].as_bool().unwrap_or(*old))
            .collect();
        let changed = |from: bool| {
            let names: Vec<&str> = PERMISSIONS
                .iter()
                .zip(old.iter().zip(&new))
                .filter(|(_, (old, new))| **old == from && **new != from)
                .map(|(name, _)| *name)
                .collect();
            names.join(", ")
        };
        let (granted, revoked) = (changed(false), changed(true));
        let summary = match (granted.is_empty(), revoked.is_empty()) {
            (true, true) => return Ok(Err(HttpResponse::BadRequest().body("Nothing to change"))),
            (false, true) => format!("Granted {granted}"),
            (true, false) => format!("Revoked {revoked}"),
            (false, false) => format!("Granted {granted}; revoked {revoked}"),
        };
        let set: Vec<String> = PERMISSIONS
            .iter()
            .enumerate()
            .map(|(i, name)| format!("{name} = $2[{}]", i + 1))
            .collect();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE permission SET {} WHERE user_id = $1",
            set.join(", ")
        )))
        .bind(*id)
        .bind(&new)
        .execute(&mut *tx)
        .await?;
        log(&mut tx, admin.id, "permission_update", *id, &summary).await?;
        tx.commit().await?;
        Ok(Ok(()))
    }
    .await;
    match outcome {
        Ok(Ok(())) => HttpResponse::NoContent().finish(),
        Ok(Err(response)) => response,
        Err(e) => failed("updating permissions", e),
    }
}

#[delete("/api/admin/permissions/{id:\\d+}")]
pub async fn remove(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match writer(&req, &pool).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let outcome: sqlx::Result<bool> = async {
        let mut tx = pool.begin().await?;
        let old: Option<Vec<bool>> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "DELETE FROM permission p WHERE p.user_id = $1 RETURNING ARRAY[{}]",
            columns()
        )))
        .bind(*id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(old) = old else {
            return Ok(false);
        };
        let summary = format!("Removed from elevated access (had {})", names(&old));
        log(&mut tx, admin.id, "permission_remove", *id, &summary).await?;
        tx.commit().await?;
        Ok(true)
    }
    .await;
    match outcome {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().finish(),
        Err(e) => failed("removing permissions", e),
    }
}

#[get("/api/admin/audit")]
pub async fn audit(
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Administration).await {
        return response;
    }
    let param = |key: &str| query.get(key).map(|value| value.trim()).filter(|value| !value.is_empty());
    let time = |key: &str| {
        param(key)
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
    };
    let number = |key: &str| param(key).and_then(|value| value.parse::<i32>().ok());
    let search = param("q").unwrap_or_default();
    let target = param("user").unwrap_or_default();
    let rows: sqlx::Result<Vec<Value>> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
             'id', aa.id,
             'action', aa.action,
             'summary', aa.summary,
             'reason', aa.reason,
             'created_at', aa.created_at,
             'admin', jsonb_build_object('id', a.id, 'name', nullif(concat_ws(' ', a.first_name, a.last_name), '')),
             'user', CASE WHEN t.id IS NOT NULL THEN
                 jsonb_build_object('id', t.id, 'name', nullif(concat_ws(' ', t.first_name, t.last_name), '')) END
         )
         FROM admin_action aa
         JOIN \"user\" a ON a.id = aa.admin_id
         LEFT JOIN \"user\" t ON t.id = aa.user_id
         WHERE ($1 = '' OR aa.summary ILIKE $2 OR aa.reason ILIKE $2 OR aa.action ILIKE $2
                OR concat_ws(' ', a.first_name, a.last_name) ILIKE $2
                OR concat_ws(' ', t.first_name, t.last_name) ILIKE $2)
           AND ($3::text IS NULL OR aa.action = $3)
           AND ($4::int IS NULL OR aa.admin_id = $4)
           AND ($5 = '' OR t.id::text = $5 OR t.hca_id ILIKE $6 OR t.email ILIKE $6 OR t.slack_id ILIKE $6
                OR concat_ws(' ', t.first_name, t.last_name) ILIKE $6)
           AND ($7::timestamptz IS NULL OR aa.created_at >= $7)
           AND ($8::timestamptz IS NULL OR aa.created_at < $8)
           AND ($9::int IS NULL OR aa.id < $9)
         ORDER BY aa.id DESC
         LIMIT $10",
    )
    .bind(search)
    .bind(pattern(search))
    .bind(param("action"))
    .bind(number("admin"))
    .bind(target)
    .bind(pattern(target))
    .bind(time("from"))
    .bind(time("to"))
    .bind(number("before"))
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "actions": rows, "more": rows.len() as i64 == PAGE })),
        Err(e) => failed("listing admin actions", e),
    }
}

#[get("/api/admin/audit/filters")]
pub async fn audit_filters(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Administration).await {
        return response;
    }
    let filters: sqlx::Result<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
             'actions', (SELECT coalesce(jsonb_agg(DISTINCT action ORDER BY action), '[]') FROM admin_action),
             'admins', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                            'id', u.id, 'name', nullif(concat_ws(' ', u.first_name, u.last_name), ''))
                        ORDER BY u.first_name, u.id), '[]')
                        FROM \"user\" u WHERE u.id IN (SELECT admin_id FROM admin_action))
         )",
    )
    .fetch_one(pool.get_ref())
    .await;
    match filters {
        Ok(filters) => HttpResponse::Ok().json(filters),
        Err(e) => failed("loading audit filters", e),
    }
}
