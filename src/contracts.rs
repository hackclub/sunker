use crate::admin::{self, Permission};
use crate::{economy, history, prize_images};
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Postgres, Transaction};

const RESOURCES: [&str; 9] = [
    "chiprock",
    "granite",
    "potash",
    "bauxite",
    "shale",
    "kalkite",
    "cuprite",
    "ring_upgrade",
    "scrip",
];

const CONTRACT: &str = "jsonb_build_object(
    'slug', t.slug, 'ring', t.ring, 'name', t.name, 'description', t.description, 'image_url', t.image_url,
    'video_url', t.video_url, 'criteria', t.criteria, 'resources', t.resources
)";

const PRIZE: &str = "jsonb_build_object(
    'id', z.id, 'ring', z.ring, 'name', z.name, 'tagline', z.tagline, 'description', z.description,
    'image', '/api/prizes/' || z.id || '/image?v=' || left(md5(z.image_url), 8)
)";

const INTERNAL: &str = "jsonb_build_object(
    'base_cost_cents', z.base_cost_cents, 'additional_cost_cents', z.additional_cost_cents, 'fulfillment_links', z.fulfillment_links,
    'image_url', z.image_url
)";

struct Contract {
    ring: i16,
    name: String,
    description: String,
    image_url: Option<String>,
    video_url: Option<String>,
    criteria: Vec<String>,
    resources: Value,
}

struct Prize {
    ring: i16,
    name: String,
    tagline: String,
    description: String,
    image_url: Option<String>,
    base_cost_cents: i32,
    additional_cost_cents: i32,
    fulfillment_links: Vec<String>,
}

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

fn text(body: &Value, key: &str) -> String {
    body[key].as_str().unwrap_or_default().trim().to_string()
}

fn name(body: &Value) -> Result<String, String> {
    let name = text(body, "name");
    if name.is_empty() || name.chars().count() > 100 {
        return Err("Give it a name under 100 characters".into());
    }
    Ok(name)
}

fn ring(body: &Value) -> Result<i16, String> {
    body["ring"]
        .as_i64()
        .filter(|ring| (0..=5).contains(ring))
        .map(|ring| ring as i16)
        .ok_or_else(|| "Pick a ring from 0 to 5".into())
}

fn url(value: &str) -> Result<String, String> {
    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(value.to_string())
    } else {
        Err(format!("\"{value}\" is not an http(s) link"))
    }
}

fn link(body: &Value, key: &str) -> Result<Option<String>, String> {
    let value = text(body, key);
    if value.is_empty() { Ok(None) } else { url(&value).map(Some) }
}

fn lines(body: &Value, key: &str) -> Vec<String> {
    body[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|line| line.as_str().map(str::trim))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn cents(body: &Value, key: &str) -> Result<i32, String> {
    body[key]
        .as_i64()
        .filter(|cents| (0..=i32::MAX as i64).contains(cents))
        .map(|cents| cents as i32)
        .ok_or_else(|| "Costs must be zero or more".into())
}

fn contract(body: &Value) -> Result<Contract, String> {
    let mut resources = Map::new();
    for (key, amount) in body["resources"].as_object().into_iter().flatten() {
        if !RESOURCES.contains(&key.as_str()) {
            return Err(format!("Unknown resource {key}"));
        }
        let amount = amount.as_i64().filter(|amount| *amount >= 0).ok_or("Resource amounts must be zero or more")?;
        if amount > 0 {
            resources.insert(key.clone(), json!(amount));
        }
    }
    Ok(Contract {
        ring: ring(body)?,
        name: name(body)?,
        description: text(body, "description"),
        image_url: link(body, "image_url")?,
        video_url: link(body, "video_url")?,
        criteria: lines(body, "criteria"),
        resources: Value::Object(resources),
    })
}

fn prize(body: &Value) -> Result<Prize, String> {
    Ok(Prize {
        ring: ring(body)?,
        name: name(body)?,
        tagline: text(body, "tagline"),
        description: text(body, "description"),
        image_url: link(body, "image_url")?,
        base_cost_cents: cents(body, "base_cost_cents")?,
        additional_cost_cents: cents(body, "additional_cost_cents")?,
        fulfillment_links: lines(body, "fulfillment_links").iter().map(|line| url(line)).collect::<Result<_, _>>()?,
    })
}

pub fn slug(name: &str) -> String {
    let slug = name
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() { "contract".into() } else { slug.chars().take(60).collect() }
}

pub async fn fresh_slug(tx: &mut Transaction<'_, Postgres>, name: &str) -> sqlx::Result<String> {
    let base = slug(name);
    let mut slug = base.clone();
    let mut n = 2;
    while sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM contract WHERE slug = $1)")
        .bind(&slug)
        .fetch_one(&mut **tx)
        .await?
    {
        slug = format!("{base}-{n}");
        n += 1;
    }
    Ok(slug)
}

async fn locked(pool: &PgPool, what: &str) -> Option<HttpResponse> {
    match economy::synced(pool).await {
        Ok(false) => None,
        Ok(true) => Some(HttpResponse::Conflict().body(format!("{what} come from the economy sheet, turn off sheet sync to add or remove them"))),
        Err(e) => Some(failed("checking sheet sync", e)),
    }
}

pub async fn log(tx: &mut Transaction<'_, Postgres>, admin: i32, action: &str, summary: String) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO admin_action (admin_id, action, summary) VALUES ($1, $2, $3)")
        .bind(admin)
        .bind(action)
        .bind(summary)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[get("/api/contracts")]
pub async fn list(pool: web::Data<PgPool>) -> HttpResponse {
    let contracts: sqlx::Result<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'contracts', (SELECT coalesce(jsonb_agg({CONTRACT} ORDER BY t.ring DESC, t.name), '[]') FROM contract t WHERE t.deleted_at IS NULL),
             'prizes', (SELECT coalesce(jsonb_agg({PRIZE} ORDER BY z.ring DESC, z.name), '[]') FROM prize z WHERE z.deleted_at IS NULL),
             'rings', (SELECT coalesce(jsonb_agg(jsonb_build_object('ring', r.ring, 'min_hours', r.min_hours) ORDER BY r.ring DESC), '[]') FROM ring r)
         )"
    )))
    .fetch_one(pool.get_ref())
    .await;
    match contracts {
        Ok(contracts) => HttpResponse::Ok().json(contracts),
        Err(e) => failed("listing the contracts", e),
    }
}

#[get("/api/admin/mines")]
pub async fn overview(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Mines).await {
        return response;
    }
    let mines: sqlx::Result<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'contracts', (SELECT coalesce(jsonb_agg({CONTRACT} ORDER BY t.ring DESC, t.name), '[]') FROM contract t WHERE t.deleted_at IS NULL),
             'prizes', (SELECT coalesce(jsonb_agg({PRIZE} || {INTERNAL} ORDER BY z.ring DESC, z.name), '[]') FROM prize z WHERE z.deleted_at IS NULL)
         )"
    )))
    .fetch_one(pool.get_ref())
    .await;
    match mines {
        Ok(mines) => HttpResponse::Ok().json(mines),
        Err(e) => failed("loading the mines admin", e),
    }
}

#[post("/api/admin/mines/contracts")]
pub async fn create_contract(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    if let Some(response) = locked(&pool, "Contracts").await {
        return response;
    }
    let contract = match contract(&body) {
        Ok(contract) => contract,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let result: sqlx::Result<String> = async {
        let mut tx = pool.begin().await?;
        history::snapshot(&mut tx, &format!("Before creating contract {}", contract.name), Some(admin.id)).await?;
        let slug = fresh_slug(&mut tx, &contract.name).await?;
        sqlx::query(
            "INSERT INTO contract (slug, ring, name, description, image_url, video_url, criteria, resources)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&slug)
        .bind(contract.ring)
        .bind(&contract.name)
        .bind(&contract.description)
        .bind(&contract.image_url)
        .bind(&contract.video_url)
        .bind(&contract.criteria)
        .bind(&contract.resources)
        .execute(&mut *tx)
        .await?;
        log(&mut tx, admin.id, "contract_create", format!("Created contract {} (ring {})", contract.name, contract.ring)).await?;
        tx.commit().await?;
        Ok(slug)
    }
    .await;
    match result {
        Ok(slug) => HttpResponse::Ok().json(json!({ "slug": slug })),
        Err(e) => failed("creating a contract", e),
    }
}

#[put("/api/admin/mines/contracts/{slug}")]
pub async fn save_contract(
    req: HttpRequest,
    slug: web::Path<String>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let contract = match contract(&body) {
        Ok(contract) => contract,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let result: sqlx::Result<HttpResponse> = async {
        let synced = economy::synced(&pool).await?;
        let mut tx = pool.begin().await?;
        history::snapshot(&mut tx, &format!("Before saving contract {}", contract.name), Some(admin.id)).await?;
        let done = sqlx::query(
            "UPDATE contract SET ring = CASE WHEN $9 THEN ring ELSE $2 END, name = CASE WHEN $9 THEN name ELSE $3 END,
                 description = CASE WHEN $9 THEN description ELSE $4 END, image_url = $5, video_url = $6,
                 criteria = CASE WHEN $9 THEN criteria ELSE $7 END, resources = $8
             WHERE slug = $1 AND deleted_at IS NULL",
        )
        .bind(slug.as_str())
        .bind(contract.ring)
        .bind(&contract.name)
        .bind(&contract.description)
        .bind(&contract.image_url)
        .bind(&contract.video_url)
        .bind(&contract.criteria)
        .bind(&contract.resources)
        .bind(synced)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Ok(HttpResponse::NotFound().finish());
        }
        log(&mut tx, admin.id, "contract_update", format!("Updated contract {}", contract.name)).await?;
        tx.commit().await?;
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("saving a contract", e))
}

#[delete("/api/admin/mines/contracts/{slug}")]
pub async fn remove_contract(req: HttpRequest, slug: web::Path<String>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    if let Some(response) = locked(&pool, "Contracts").await {
        return response;
    }
    let result: sqlx::Result<HttpResponse> = async {
        let mut tx = pool.begin().await?;
        history::snapshot(&mut tx, &format!("Before deleting contract {slug}"), Some(admin.id)).await?;
        let name: Option<String> = sqlx::query_scalar(
            "UPDATE contract SET deleted_at = now() WHERE slug = $1 AND deleted_at IS NULL RETURNING name",
        )
        .bind(slug.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(name) = name else {
            return Ok(HttpResponse::NotFound().finish());
        };
        log(&mut tx, admin.id, "contract_delete", format!("Deleted contract {name}")).await?;
        tx.commit().await?;
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("deleting a contract", e))
}

#[post("/api/admin/mines/prizes")]
pub async fn create_prize(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    if let Some(response) = locked(&pool, "Prizes").await {
        return response;
    }
    let prize = match prize(&body) {
        Ok(prize) => prize,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let result: sqlx::Result<i32> = async {
        let mut tx = pool.begin().await?;
        history::snapshot(&mut tx, &format!("Before creating prize {}", prize.name), Some(admin.id)).await?;
        let id = sqlx::query_scalar(
            "INSERT INTO prize (ring, name, tagline, description, base_cost_cents, additional_cost_cents, fulfillment_links, image_url)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
        )
        .bind(prize.ring)
        .bind(&prize.name)
        .bind(&prize.tagline)
        .bind(&prize.description)
        .bind(prize.base_cost_cents)
        .bind(prize.additional_cost_cents)
        .bind(&prize.fulfillment_links)
        .bind(&prize.image_url)
        .fetch_one(&mut *tx)
        .await?;
        log(&mut tx, admin.id, "prize_create", format!("Created prize {} (ring {})", prize.name, prize.ring)).await?;
        tx.commit().await?;
        prize_images::refresh(&pool);
        Ok(id)
    }
    .await;
    match result {
        Ok(id) => HttpResponse::Ok().json(json!({ "id": id })),
        Err(e) => failed("creating a prize", e),
    }
}

#[put("/api/admin/mines/prizes/{id:\\d+}")]
pub async fn save_prize(
    req: HttpRequest,
    id: web::Path<i32>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let prize = match prize(&body) {
        Ok(prize) => prize,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let result: sqlx::Result<HttpResponse> = async {
        let synced = economy::synced(&pool).await?;
        let mut tx = pool.begin().await?;
        history::snapshot(&mut tx, &format!("Before saving prize {}", prize.name), Some(admin.id)).await?;
        let done = sqlx::query(
            "UPDATE prize SET ring = CASE WHEN $9 THEN ring ELSE $2 END, name = CASE WHEN $9 THEN name ELSE $3 END,
                 tagline = $4, description = $5, base_cost_cents = CASE WHEN $9 THEN base_cost_cents ELSE $6 END,
                 additional_cost_cents = $7, fulfillment_links = $8, image_url = $10
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(*id)
        .bind(prize.ring)
        .bind(&prize.name)
        .bind(&prize.tagline)
        .bind(&prize.description)
        .bind(prize.base_cost_cents)
        .bind(prize.additional_cost_cents)
        .bind(&prize.fulfillment_links)
        .bind(synced)
        .bind(&prize.image_url)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() == 0 {
            return Ok(HttpResponse::NotFound().finish());
        }
        log(&mut tx, admin.id, "prize_update", format!("Updated prize {}", prize.name)).await?;
        tx.commit().await?;
        prize_images::refresh(&pool);
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("saving a prize", e))
}

#[delete("/api/admin/mines/prizes/{id:\\d+}")]
pub async fn remove_prize(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    if let Some(response) = locked(&pool, "Prizes").await {
        return response;
    }
    let result: sqlx::Result<HttpResponse> = async {
        let mut tx = pool.begin().await?;
        history::snapshot(&mut tx, &format!("Before deleting prize #{id}"), Some(admin.id)).await?;
        let name: Option<String> =
            sqlx::query_scalar("UPDATE prize SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL RETURNING name")
                .bind(*id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(name) = name else {
            return Ok(HttpResponse::NotFound().finish());
        };
        log(&mut tx, admin.id, "prize_delete", format!("Deleted prize {name}")).await?;
        tx.commit().await?;
        prize_images::refresh(&pool);
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("deleting a prize", e))
}
