use crate::admin::{self, Permission};
use crate::{mines, prize_images};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

pub async fn snapshot(tx: &mut Transaction<'_, Postgres>, reason: &str, admin: Option<i32>) -> sqlx::Result<()> {
    sqlx::query(
        "WITH state AS (
             SELECT (SELECT coalesce(jsonb_agg(to_jsonb(z) - 'updated_at' ORDER BY z.id), '[]') FROM prize z) AS prizes,
                    (SELECT coalesce(jsonb_agg(to_jsonb(t) - 'updated_at' ORDER BY t.slug), '[]') FROM contract t) AS contracts
         )
         INSERT INTO mines_snapshot (reason, admin_id, prizes, contracts)
         SELECT $1, $2, prizes, contracts FROM state
         WHERE NOT EXISTS (
             SELECT 1 FROM (SELECT prizes, contracts FROM mines_snapshot ORDER BY id DESC LIMIT 1) last
             WHERE last.prizes = state.prizes AND last.contracts = state.contracts
         )",
    )
    .bind(reason)
    .bind(admin)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[get("/api/admin/mines/history")]
pub async fn list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Mines).await {
        return response;
    }
    let history: sqlx::Result<Value> = sqlx::query_scalar(
        "SELECT coalesce(jsonb_agg(jsonb_build_object(
             'id', s.id, 'reason', s.reason, 'created_at', s.created_at,
             'admin', nullif(concat_ws(' ', u.first_name, u.last_name), ''),
             'prizes', (SELECT count(*) FROM jsonb_array_elements(s.prizes) e WHERE e->>'deleted_at' IS NULL),
             'contracts', (SELECT count(*) FROM jsonb_array_elements(s.contracts) e WHERE e->>'deleted_at' IS NULL)
         ) ORDER BY s.id DESC), '[]')
         FROM mines_snapshot s LEFT JOIN \"user\" u ON u.id = s.admin_id",
    )
    .fetch_one(pool.get_ref())
    .await;
    match history {
        Ok(history) => HttpResponse::Ok().json(history),
        Err(e) => {
            eprintln!("listing the mines history failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/api/admin/mines/history/{id:\\d+}/restore")]
pub async fn restore(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let result: sqlx::Result<HttpResponse> = async {
        let mut tx = pool.begin().await?;
        let Some((prizes, contracts)): Option<(Value, Value)> =
            sqlx::query_as("SELECT prizes, contracts FROM mines_snapshot WHERE id = $1").bind(*id).fetch_optional(&mut *tx).await?
        else {
            return Ok(HttpResponse::NotFound().finish());
        };
        snapshot(&mut tx, &format!("Before restoring snapshot #{id}"), Some(admin.id)).await?;
        sqlx::query("UPDATE economy SET sheet_sync = false").execute(&mut *tx).await?;
        sqlx::query(
            "UPDATE prize p SET ring = s.ring, name = s.name, tagline = s.tagline, description = s.description,
                 base_cost_cents = s.base_cost_cents, additional_cost_cents = s.additional_cost_cents,
                 fulfillment_links = s.fulfillment_links, image_url = s.image_url, deleted_at = s.deleted_at
             FROM jsonb_populate_recordset(NULL::prize, $1) s WHERE p.id = s.id",
        )
        .bind(&prizes)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO prize (id, ring, name, tagline, description, base_cost_cents, additional_cost_cents, fulfillment_links, image_url, created_at, deleted_at)
             SELECT id, ring, name, tagline, description, base_cost_cents, additional_cost_cents, fulfillment_links, image_url, created_at, deleted_at
             FROM jsonb_populate_recordset(NULL::prize, $1) s WHERE NOT EXISTS (SELECT 1 FROM prize WHERE prize.id = s.id)",
        )
        .bind(&prizes)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE prize SET deleted_at = now()
             WHERE deleted_at IS NULL AND id NOT IN (SELECT (e->>'id')::int FROM jsonb_array_elements($1) e)",
        )
        .bind(&prizes)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE contract c SET ring = s.ring, name = s.name, description = s.description, image_url = s.image_url,
                 video_url = s.video_url, criteria = s.criteria, resources = s.resources, deleted_at = s.deleted_at
             FROM jsonb_populate_recordset(NULL::contract, $1) s WHERE c.slug = s.slug",
        )
        .bind(&contracts)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO contract (slug, ring, name, description, image_url, video_url, criteria, resources, created_at, deleted_at)
             SELECT slug, ring, name, description, image_url, video_url, criteria, resources, created_at, deleted_at
             FROM jsonb_populate_recordset(NULL::contract, $1) s WHERE NOT EXISTS (SELECT 1 FROM contract WHERE contract.slug = s.slug)",
        )
        .bind(&contracts)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE contract SET deleted_at = now()
             WHERE deleted_at IS NULL AND slug NOT IN (SELECT e->>'slug' FROM jsonb_array_elements($1) e)",
        )
        .bind(&contracts)
        .execute(&mut *tx)
        .await?;
        mines::log(&mut tx, admin.id, "mines_restore", format!("Restored the mines to snapshot #{id} and turned off sheet sync")).await?;
        tx.commit().await?;
        prize_images::refresh(&pool);
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| {
        eprintln!("restoring a mines snapshot failed: {e}");
        HttpResponse::InternalServerError().finish()
    })
}
