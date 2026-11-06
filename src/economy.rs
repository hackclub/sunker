use crate::admin::{self, Permission};
use crate::{contracts, history, prize_images};
use actix_web::{HttpRequest, HttpResponse, get, post, put, web};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::LazyLock;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

const ECONOMY_TAB: u64 = 0;
const PROJECTS_TAB: u64 = 722531100;

const STATE: &str = "SELECT jsonb_build_object(
    'sheet_sync', e.sheet_sync, 'dollars_per_hour', e.dollars_per_hour, 'synced_at', e.synced_at, 'sync_error', e.sync_error,
    'rings', (SELECT coalesce(jsonb_agg(jsonb_build_object('ring', r.ring, 'description', r.description, 'min_hours', r.min_hours) ORDER BY r.ring DESC), '[]') FROM ring r)
) FROM economy e";

#[derive(Default)]
struct Sheet {
    rate: Option<f64>,
    rings: Vec<(i16, String, f64)>,
    prizes: Vec<(i16, String, i32)>,
    contracts: Vec<(i16, String, String, Vec<String>)>,
}

fn csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            (true, '"') => quoted = false,
            (true, c) => cell.push(c),
            (false, '"') => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut cell)),
            (false, '\n') => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            (false, '\r') => {}
            (false, c) => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

fn cell(row: &[String], column: usize) -> &str {
    row.get(column).map_or("", |cell| cell.trim())
}

fn number(text: &str) -> Option<f64> {
    text.trim().trim_start_matches('$').replace(',', "").trim().parse().ok().filter(|n: &f64| n.is_finite())
}

fn ring(label: &str) -> Option<i16> {
    label.trim().strip_prefix("Ring ")?.trim().parse().ok().filter(|ring| (0..=5).contains(ring))
}

fn hours(line: &str) -> bool {
    let line = line.to_lowercase();
    ["hours", "hour", "hrs", "hr"].iter().any(|unit| line.strip_suffix(unit).is_some_and(|n| number(n).is_some()))
}

fn named(name: &str, what: &str) -> Result<String, String> {
    if name.chars().count() > 100 {
        return Err(format!("The {what} name \"{name}\" is over 100 characters"));
    }
    Ok(name.to_string())
}

fn economy(rows: &[Vec<String>], sheet: &mut Sheet) -> Result<(), String> {
    let header = rows.first().ok_or("The Economy tab is empty")?;
    let column = |name: &str| {
        header
            .iter()
            .position(|cell| cell.trim().eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("The Economy tab has no \"{name}\" column"))
    };
    let (number_column, description_column, hours_column) = (column("Ring Number")?, column("Quick Description")?, column("Min Hours")?);
    for row in &rows[1..] {
        let Some(n) = cell(row, number_column).parse::<i16>().ok().filter(|n| (0..=5).contains(n)) else {
            continue;
        };
        let hours = number(cell(row, hours_column)).ok_or_else(|| format!("Ring {n} min hours isn't a number"))?;
        sheet.rings.push((n, cell(row, description_column).to_string(), hours));
    }
    for (c, label) in header.iter().enumerate() {
        let Some(ring) = ring(label) else {
            continue;
        };
        for (r, row) in rows.iter().enumerate().skip(1) {
            let name = cell(row, c);
            if name.is_empty() || name.eq_ignore_ascii_case("Prize Name") {
                continue;
            }
            let price = number(cell(row, c + 1))
                .filter(|price| *price >= 0.0)
                .ok_or_else(|| format!("Economy row {}: {name} has no price", r + 1))?;
            if sheet.prizes.iter().any(|(other, existing, _)| *other == ring && existing.eq_ignore_ascii_case(name)) {
                return Err(format!("{name} is in ring {ring} twice"));
            }
            sheet.prizes.push((ring, named(name, "prize")?, (price * 100.0).round() as i32));
        }
    }
    if let Ok(c) = column("Config") {
        for row in &rows[1..] {
            if cell(row, c).eq_ignore_ascii_case("Dollar per hour rate") {
                sheet.rate = Some(number(cell(row, c + 1)).ok_or("The dollar per hour rate isn't a number")?);
            }
        }
    }
    Ok(())
}

fn projects(rows: &[Vec<String>], sheet: &mut Sheet) -> Result<(), String> {
    for (r, row) in rows.iter().enumerate() {
        if !cell(row, 1).eq_ignore_ascii_case("Name") {
            continue;
        }
        let ring = ring(cell(row, 0)).ok_or_else(|| format!("Projects row {}: no \"Ring N\" label next to Name", r + 1))?;
        let block = rows[r + 1..].iter().take_while(|next| !cell(next, 1).eq_ignore_ascii_case("Name"));
        let find = |label: &str| block.clone().find(|next| cell(next, 1).eq_ignore_ascii_case(label));
        let (description, requirements) = (find("Quick Description"), find("Requirements"));
        for c in 2..row.len() {
            let name = cell(row, c);
            if name.is_empty() {
                continue;
            }
            if sheet.contracts.iter().any(|(other, existing, ..)| *other == ring && existing.eq_ignore_ascii_case(name)) {
                return Err(format!("{name} is in ring {ring} twice"));
            }
            let criteria = requirements
                .map_or("", |row| cell(row, c))
                .lines()
                .map(|line| line.trim().trim_start_matches(['-', '*', '•']).trim())
                .filter(|line| !line.is_empty() && !hours(line))
                .map(str::to_string)
                .collect();
            sheet.contracts.push((ring, named(name, "contract")?, description.map_or("", |row| cell(row, c)).to_string(), criteria));
        }
    }
    Ok(())
}

async fn tab(id: &str, gid: u64) -> Result<Vec<Vec<String>>, String> {
    let response = CLIENT
        .get(format!("https://docs.google.com/spreadsheets/d/{id}/export?format=csv&gid={gid}"))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Google Sheets: {e}"))?;
    let csv_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .is_some_and(|kind| kind.as_bytes().starts_with(b"text/csv"));
    if !response.status().is_success() || !csv_type {
        return Err(format!(
            "Google answered {} instead of a CSV, check the sheet is shared as anyone with the link can view",
            response.status()
        ));
    }
    let text = response.text().await.map_err(|e| format!("Couldn't read the sheet: {e}"))?;
    Ok(csv(&text))
}

async fn read() -> Result<Sheet, String> {
    let id = std::env::var("ECONOMY_SHEET").map_err(|_| "ECONOMY_SHEET isn't set on the server".to_string())?;
    let mut sheet = Sheet::default();
    economy(&tab(&id, ECONOMY_TAB).await?, &mut sheet)?;
    projects(&tab(&id, PROJECTS_TAB).await?, &mut sheet)?;
    Ok(sheet)
}

async fn apply(pool: &PgPool, sheet: &Sheet, admin: Option<i32>) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    history::snapshot(&mut tx, "Before sheet sync", admin).await?;
    sqlx::query("UPDATE economy SET dollars_per_hour = $1, synced_at = now(), sync_error = NULL")
        .bind(sheet.rate)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM ring").execute(&mut *tx).await?;
    for (ring, description, hours) in &sheet.rings {
        sqlx::query("INSERT INTO ring (ring, description, min_hours) VALUES ($1, $2, $3)")
            .bind(ring)
            .bind(description)
            .bind(hours)
            .execute(&mut *tx)
            .await?;
    }
    let mut kept = Vec::new();
    for (ring, name, cents) in &sheet.prizes {
        let id: i32 = sqlx::query_scalar(
            "WITH found AS (SELECT id FROM prize WHERE ring = $1 AND lower(name) = lower($2) ORDER BY deleted_at IS NOT NULL, id DESC LIMIT 1),
                  updated AS (UPDATE prize SET name = $2, base_cost_cents = $3, deleted_at = NULL FROM found WHERE prize.id = found.id RETURNING prize.id),
                  inserted AS (INSERT INTO prize (ring, name, base_cost_cents) SELECT $1, $2, $3 WHERE NOT EXISTS (SELECT 1 FROM found) RETURNING id)
             SELECT id FROM updated UNION ALL SELECT id FROM inserted",
        )
        .bind(ring)
        .bind(name)
        .bind(cents)
        .fetch_one(&mut *tx)
        .await?;
        kept.push(id);
    }
    sqlx::query("UPDATE prize SET deleted_at = now() WHERE deleted_at IS NULL AND NOT (id = ANY($1))")
        .bind(&kept)
        .execute(&mut *tx)
        .await?;
    let mut slugs = Vec::new();
    for (ring, name, description, criteria) in &sheet.contracts {
        let found: Option<String> = sqlx::query_scalar(
            "SELECT slug FROM contract WHERE ring = $1 AND lower(name) = lower($2) ORDER BY deleted_at IS NOT NULL LIMIT 1",
        )
        .bind(ring)
        .bind(name)
        .fetch_optional(&mut *tx)
        .await?;
        let slug = match found {
            Some(slug) => {
                sqlx::query("UPDATE contract SET name = $2, description = $3, criteria = $4, deleted_at = NULL WHERE slug = $1")
                    .bind(&slug)
                    .bind(name)
                    .bind(description)
                    .bind(criteria)
                    .execute(&mut *tx)
                    .await?;
                slug
            }
            None => {
                let slug = contracts::fresh_slug(&mut tx, name).await?;
                sqlx::query("INSERT INTO contract (slug, ring, name, description, criteria) VALUES ($1, $2, $3, $4, $5)")
                    .bind(&slug)
                    .bind(ring)
                    .bind(name)
                    .bind(description)
                    .bind(criteria)
                    .execute(&mut *tx)
                    .await?;
                slug
            }
        };
        slugs.push(slug);
    }
    sqlx::query("UPDATE contract SET deleted_at = now() WHERE deleted_at IS NULL AND NOT (slug = ANY($1))")
        .bind(&slugs)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    prize_images::refresh(pool);
    Ok(())
}

async fn sync(pool: &PgPool, admin: Option<i32>) -> Result<String, String> {
    let outcome = match read().await {
        Ok(sheet) => apply(pool, &sheet, admin)
            .await
            .map(|()| format!("{} prizes, {} contracts and {} rings", sheet.prizes.len(), sheet.contracts.len(), sheet.rings.len()))
            .map_err(|e| {
                eprintln!("saving the economy sheet failed: {e}");
                "Couldn't save the sheet to the database".to_string()
            }),
        Err(message) => Err(message),
    };
    if let Err(message) = &outcome {
        if let Err(e) = sqlx::query("UPDATE economy SET sync_error = $1").bind(message).execute(pool).await {
            eprintln!("recording the economy sync error failed: {e}");
        }
    }
    outcome
}

pub async fn synced(pool: &PgPool) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT sheet_sync FROM economy").fetch_one(pool).await
}

pub async fn startup(pool: &PgPool) {
    match synced(pool).await {
        Ok(true) => match sync(pool, None).await {
            Ok(summary) => println!("economy sheet synced: {summary}"),
            Err(message) => eprintln!("economy sheet sync failed: {message}"),
        },
        Ok(false) => {}
        Err(e) => eprintln!("checking economy sheet sync failed: {e}"),
    }
}

async fn state(pool: &PgPool) -> HttpResponse {
    match sqlx::query_scalar::<_, Value>(STATE).fetch_one(pool).await {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(e) => {
            eprintln!("loading the economy failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn record(pool: &PgPool, admin: i32, action: &str, summary: String) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    contracts::log(&mut tx, admin, action, summary).await?;
    tx.commit().await
}

#[get("/api/admin/economy")]
pub async fn overview(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Mines).await {
        return response;
    }
    state(&pool).await
}

#[put("/api/admin/economy")]
pub async fn toggle(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let Some(on) = body["sheet_sync"].as_bool() else {
        return HttpResponse::BadRequest().body("missing sheet_sync");
    };
    let result = async {
        sqlx::query("UPDATE economy SET sheet_sync = $1").bind(on).execute(pool.get_ref()).await?;
        let summary = if on { "Turned on economy sheet sync" } else { "Turned off economy sheet sync" };
        record(&pool, admin.id, "economy_sync_toggle", summary.into()).await
    }
    .await;
    if let Err(e) = result {
        eprintln!("toggling economy sheet sync failed: {e}");
        return HttpResponse::InternalServerError().finish();
    }
    if on {
        if let Ok(summary) = sync(&pool, Some(admin.id)).await {
            if let Err(e) = record(&pool, admin.id, "economy_sync", format!("Synced {summary} from the economy sheet")).await {
                eprintln!("logging the economy sync failed: {e}");
            }
        }
    }
    state(&pool).await
}

#[post("/api/admin/economy/refresh")]
pub async fn refresh(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Mines).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    match synced(&pool).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Conflict().body("Turn on sheet sync first"),
        Err(e) => {
            eprintln!("checking economy sheet sync failed: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }
    if let Ok(summary) = sync(&pool, Some(admin.id)).await {
        if let Err(e) = record(&pool, admin.id, "economy_sync", format!("Synced {summary} from the economy sheet")).await {
            eprintln!("logging the economy sync failed: {e}");
        }
    }
    state(&pool).await
}
