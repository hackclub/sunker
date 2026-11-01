use crate::admin::{self, Permission};
use crate::territory::{self, Hex};
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::{BTreeMap, HashMap, HashSet};

const ENEMIES: [&str; 3] = ["chunker", "crawler", "hopper"];
const KINDS: [&str; 3] = ["move", "spawn", "retreat"];
const SLOTS: [&str; 2] = ["warning", "followup"];

const WAVE: &str = "jsonb_build_object(
    'id', w.id,
    'name', w.name,
    'notes', w.notes,
    'status', w.status,
    'resolve_at', w.resolve_at,
    'resolved_at', w.resolved_at,
    'created_at', w.created_at,
    'updated_at', w.updated_at,
    'creator', (SELECT nullif(concat_ws(' ', u.first_name, u.last_name), '') FROM \"user\" u WHERE u.id = w.created_by),
    'orders', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                   'id', o.id,
                   'kind', o.kind,
                   'force', o.force,
                   'path', o.path,
                   'taken', o.taken,
                   'delivered', o.delivered,
                   'steps', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                                 'row', s.hex_row, 'col', s.hex_col, 'arrived', s.arrived, 'lost', s.lost, 'halted', s.halted
                             ) ORDER BY s.step), '[]') FROM enemy_step s WHERE s.order_id = o.id)
               ) ORDER BY o.position), '[]') FROM enemy_order o WHERE o.wave_id = w.id),
    'announcements', (SELECT coalesce(jsonb_object_agg(a.slot, jsonb_build_object(
                          'title', a.title, 'body', a.body, 'publish_at', a.publish_at, 'on_resolve', a.on_resolve
                      )), '{}') FROM announcement a WHERE a.wave_id = w.id)
)";

type Force = BTreeMap<String, i32>;

struct Order {
    kind: String,
    force: Force,
    path: Vec<Hex>,
}

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

fn force(value: &Value) -> Option<Force> {
    let mut force = Force::new();
    for (kind, count) in value.as_object()? {
        let count = count.as_i64()?;
        if !ENEMIES.contains(&kind.as_str()) || !(0..=100_000).contains(&count) {
            return None;
        }
        if count > 0 {
            force.insert(kind.clone(), count as i32);
        }
    }
    Some(force)
}

fn hexes(value: &Value) -> Option<Vec<Hex>> {
    value
        .as_array()?
        .iter()
        .map(|hex| Some((hex[0].as_i64()? as i32, hex[1].as_i64()? as i32)))
        .collect()
}

fn describe(force: &Force) -> String {
    if force.is_empty() {
        return "nothing".into();
    }
    force
        .iter()
        .map(|(kind, count)| format!("{count}x {kind}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn take(from: &mut Force, want: &Force) -> Force {
    let mut taken = Force::new();
    for (kind, count) in want {
        let have = from.get(kind).copied().unwrap_or(0);
        let n = (*count).min(have);
        if n == 0 {
            continue;
        }
        taken.insert(kind.clone(), n);
        if have == n {
            from.remove(kind);
        } else {
            from.insert(kind.clone(), have - n);
        }
    }
    taken
}

fn add(to: &mut Force, more: &Force) {
    for (kind, count) in more {
        *to.entry(kind.clone()).or_default() += count;
    }
}

fn defend(_hex: Hex, _force: &Force) -> (Force, bool) {
    (Force::new(), false)
}

fn orders(value: &Value, placed: &HashSet<Hex>) -> Result<Vec<Order>, String> {
    let list = value.as_array().ok_or("Orders must be a list")?;
    list.iter()
        .enumerate()
        .map(|(i, order)| {
            let n = i + 1;
            let kind = order["kind"]
                .as_str()
                .filter(|kind| KINDS.contains(kind))
                .ok_or(format!("Order {n} has an unknown kind"))?;
            let force = force(&order["force"])
                .filter(|force| !force.is_empty())
                .ok_or(format!("Order {n} needs at least one enemy"))?;
            let path = hexes(&order["path"]).ok_or(format!("Order {n} has a broken path"))?;
            if kind == "move" && path.len() < 2 {
                return Err(format!("Order {n} needs somewhere to move to"));
            }
            if kind != "move" && path.len() != 1 {
                return Err(format!("Order {n} must target exactly one tile"));
            }
            if let Some((row, col)) = path.iter().find(|hex| !placed.contains(hex)) {
                return Err(format!("Order {n} leaves the map at {row}, {col}"));
            }
            if path.windows(2).any(|pair| !territory::neighbours(pair[0]).contains(&pair[1])) {
                return Err(format!("Order {n} skips a tile"));
            }
            Ok(Order {
                kind: kind.into(),
                force,
                path,
            })
        })
        .collect()
}

fn time(value: &Value) -> Result<Option<DateTime<Utc>>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => DateTime::parse_from_rfc3339(text)
            .map(|time| Some(time.with_timezone(&Utc)))
            .map_err(|_| "That time isn't valid".into()),
        _ => Err("That time isn't valid".into()),
    }
}

async fn log(
    tx: &mut Transaction<'_, Postgres>,
    admin: i32,
    action: &str,
    summary: String,
    reason: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO admin_action (admin_id, action, summary, reason) VALUES ($1, $2, $3, $4)")
        .bind(admin)
        .bind(action)
        .bind(summary)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn lock(tx: &mut Transaction<'_, Postgres>, id: i32) -> sqlx::Result<Option<(String, String)>> {
    sqlx::query_as("SELECT name, status FROM enemy_wave WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
}

async fn resolve(tx: &mut Transaction<'_, Postgres>, id: i32) -> sqlx::Result<()> {
    let rows: Vec<(i32, String, Value, Value)> =
        sqlx::query_as("SELECT id, kind, force, path FROM enemy_order WHERE wave_id = $1 ORDER BY position")
            .bind(id)
            .fetch_all(&mut **tx)
            .await?;
    let orders: Vec<(i32, String, Force, Vec<Hex>)> = rows
        .into_iter()
        .map(|(order, kind, want, path)| (order, kind, force(&want).unwrap_or_default(), hexes(&path).unwrap_or_default()))
        .filter(|(_, _, _, path)| !path.is_empty())
        .collect();
    let touched: HashSet<Hex> = orders.iter().flat_map(|(_, _, _, path)| path.iter().copied()).collect();
    let (rows, cols): (Vec<i32>, Vec<i32>) = touched.iter().copied().unzip();
    let current: Vec<(i32, i32, Value)> = sqlx::query_as(
        "SELECT hex_row, hex_col, enemies FROM territory
         WHERE (hex_row, hex_col) IN (SELECT * FROM unnest($1::int[], $2::int[])) FOR UPDATE",
    )
    .bind(&rows)
    .bind(&cols)
    .fetch_all(&mut **tx)
    .await?;
    let existing: HashSet<Hex> = current.iter().map(|(row, col, _)| (*row, *col)).collect();
    let mut state: HashMap<Hex, Force> = touched.iter().map(|hex| (*hex, Force::new())).collect();
    for (row, col, enemies) in current {
        state.insert((row, col), force(&enemies).unwrap_or_default());
    }
    let snapshot: Vec<Value> = state
        .iter()
        .map(|((row, col), enemies)| json!({ "row": row, "col": col, "enemies": enemies }))
        .collect();

    for (order, kind, want, path) in &orders {
        let source = path[0];
        let mut steps = vec![];
        let (taken, delivered) = match kind.as_str() {
            "spawn" => {
                add(state.entry(source).or_default(), want);
                steps.push((source, want.clone(), Force::new(), false));
                (want.clone(), want.clone())
            }
            "retreat" => {
                let taken = take(state.entry(source).or_default(), want);
                steps.push((source, taken.clone(), Force::new(), false));
                (taken, Force::new())
            }
            _ => {
                let taken = take(state.entry(source).or_default(), want);
                let mut carried = taken.clone();
                let mut end = source;
                steps.push((source, taken.clone(), Force::new(), false));
                for hex in &path[1..] {
                    if carried.is_empty() {
                        break;
                    }
                    let arrived = carried.clone();
                    let (losses, halted) = defend(*hex, &carried);
                    let lost = take(&mut carried, &losses);
                    steps.push((*hex, arrived, lost, halted));
                    end = *hex;
                    if halted {
                        break;
                    }
                }
                add(state.entry(end).or_default(), &carried);
                (taken, carried)
            }
        };
        sqlx::query("UPDATE enemy_order SET taken = $2, delivered = $3 WHERE id = $1")
            .bind(order)
            .bind(json!(taken))
            .bind(json!(delivered))
            .execute(&mut **tx)
            .await?;
        sqlx::query("DELETE FROM enemy_step WHERE order_id = $1")
            .bind(order)
            .execute(&mut **tx)
            .await?;
        for (step, ((row, col), arrived, lost, halted)) in steps.into_iter().enumerate() {
            sqlx::query(
                "INSERT INTO enemy_step (order_id, step, hex_row, hex_col, arrived, lost, halted)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(order)
            .bind(step as i32)
            .bind(row)
            .bind(col)
            .bind(json!(arrived))
            .bind(json!(lost))
            .bind(halted)
            .execute(&mut **tx)
            .await?;
        }
    }

    for ((row, col), enemies) in &state {
        if enemies.is_empty() && !existing.contains(&(*row, *col)) {
            continue;
        }
        sqlx::query(
            "INSERT INTO territory (hex_row, hex_col, enemies) VALUES ($1, $2, $3)
             ON CONFLICT (hex_row, hex_col) DO UPDATE SET enemies = EXCLUDED.enemies",
        )
        .bind(row)
        .bind(col)
        .bind(json!(enemies))
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query("UPDATE enemy_wave SET status = 'resolved', resolved_at = now(), snapshot = $2 WHERE id = $1")
        .bind(id)
        .bind(Value::Array(snapshot))
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "UPDATE announcement SET publish_at = now()
         WHERE wave_id = $1 AND slot = 'followup' AND on_resolve AND publish_at IS NULL",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn resolve_due(pool: &PgPool) {
    loop {
        let result: sqlx::Result<bool> = async {
            let mut tx = pool.begin().await?;
            let id: Option<i32> = sqlx::query_scalar(
                "SELECT id FROM enemy_wave WHERE status = 'scheduled' AND resolve_at <= now()
                 ORDER BY resolve_at, id LIMIT 1 FOR UPDATE SKIP LOCKED",
            )
            .fetch_optional(&mut *tx)
            .await?;
            let Some(id) = id else { return Ok(false) };
            resolve(&mut tx, id).await?;
            tx.commit().await?;
            Ok(true)
        }
        .await;
        match result {
            Ok(true) => continue,
            Ok(false) => break,
            Err(e) => {
                eprintln!("resolving due waves failed: {e}");
                break;
            }
        }
    }
}

#[get("/api/admin/joel")]
pub async fn overview(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Joel).await {
        return response;
    }
    let state: sqlx::Result<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT jsonb_build_object(
             'territory', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                               'row', hex_row, 'col', hex_col, 'enemies', enemies, 'occupied', is_occupied,
                               'last_occupied_at', last_occupied_at, 'last_freed_at', last_freed_at
                           )), '[]') FROM territory),
             'waves', (SELECT coalesce(jsonb_agg({WAVE} ORDER BY w.created_at DESC), '[]') FROM enemy_wave w),
             'latest', (SELECT id FROM enemy_wave WHERE status = 'resolved' ORDER BY resolved_at DESC LIMIT 1),
             'now', now()
         )"
    )))
    .fetch_one(pool.get_ref())
    .await;
    match state {
        Ok(state) => HttpResponse::Ok().json(state),
        Err(e) => failed("loading joel view", e),
    }
}

#[post("/api/admin/joel/waves")]
pub async fn create(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Joel).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let name = body["name"].as_str().unwrap_or_default().trim();
    if name.is_empty() || name.chars().count() > 100 {
        return HttpResponse::BadRequest().body("Give the wave a name under 100 characters");
    }
    let id: sqlx::Result<i32> =
        sqlx::query_scalar("INSERT INTO enemy_wave (name, notes, created_by) VALUES ($1, $2, $3) RETURNING id")
            .bind(name)
            .bind(body["notes"].as_str().unwrap_or_default().trim())
            .bind(admin.id)
            .fetch_one(pool.get_ref())
            .await;
    match id {
        Ok(id) => HttpResponse::Ok().json(json!({ "id": id })),
        Err(e) => failed("creating a wave", e),
    }
}

#[put("/api/admin/joel/waves/{id:\\d+}")]
pub async fn save(
    req: HttpRequest,
    id: web::Path<i32>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Joel).await {
        return response;
    }
    let id = *id;
    let name = body["name"].as_str().map(str::trim);
    if name.is_some_and(|name| name.is_empty() || name.chars().count() > 100) {
        return HttpResponse::BadRequest().body("Give the wave a name under 100 characters");
    }
    let orders = if body["orders"].is_null() {
        None
    } else {
        let placed = match territory::placed() {
            Ok(placed) => placed,
            Err(e) => return failed("reading the map", e),
        };
        match orders(&body["orders"], &placed) {
            Ok(orders) => Some(orders),
            Err(message) => return HttpResponse::BadRequest().body(message),
        }
    };
    let result: sqlx::Result<HttpResponse> = async {
        let mut tx = pool.begin().await?;
        let Some((_, status)) = lock(&mut tx, id).await? else {
            return Ok(HttpResponse::NotFound().finish());
        };
        sqlx::query("UPDATE enemy_wave SET name = coalesce($2, name), notes = coalesce($3, notes) WHERE id = $1")
            .bind(id)
            .bind(name)
            .bind(body["notes"].as_str().map(str::trim))
            .execute(&mut *tx)
            .await?;
        if let Some(orders) = orders {
            if status != "draft" {
                return Ok(HttpResponse::Conflict().body("Only drafts can change their orders"));
            }
            sqlx::query("DELETE FROM enemy_order WHERE wave_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            for (position, order) in orders.iter().enumerate() {
                sqlx::query("INSERT INTO enemy_order (wave_id, position, kind, force, path) VALUES ($1, $2, $3, $4, $5)")
                    .bind(id)
                    .bind(position as i32)
                    .bind(&order.kind)
                    .bind(json!(order.force))
                    .bind(json!(order.path.iter().map(|(row, col)| [row, col]).collect::<Vec<_>>()))
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("saving a wave", e))
}

#[delete("/api/admin/joel/waves/{id:\\d+}")]
pub async fn remove(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Joel).await {
        return response;
    }
    let deleted = sqlx::query("DELETE FROM enemy_wave WHERE id = $1 AND status IN ('draft', 'cancelled')")
        .bind(*id)
        .execute(pool.get_ref())
        .await;
    match deleted {
        Ok(done) if done.rows_affected() == 0 => {
            HttpResponse::Conflict().body("Only drafts and cancelled waves can be deleted")
        }
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => failed("deleting a wave", e),
    }
}

#[post("/api/admin/joel/waves/{id:\\d+}/{action}")]
pub async fn act(
    req: HttpRequest,
    path: web::Path<(i32, String)>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Joel).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let (id, action) = path.into_inner();
    let result: sqlx::Result<HttpResponse> = async {
        let mut tx = pool.begin().await?;
        let Some((name, status)) = lock(&mut tx, id).await? else {
            return Ok(HttpResponse::NotFound().finish());
        };
        let conflict = |message: &str| Ok(HttpResponse::Conflict().body(message.to_string()));
        let orders: i64 = sqlx::query_scalar("SELECT count(*) FROM enemy_order WHERE wave_id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        match action.as_str() {
            "schedule" => {
                if status != "draft" {
                    return conflict("Only drafts can be scheduled");
                }
                if orders == 0 {
                    return conflict("Add at least one order first");
                }
                let at = match time(&body["resolve_at"]) {
                    Ok(Some(at)) => at,
                    Ok(None) => return Ok(HttpResponse::BadRequest().body("Pick when the wave resolves")),
                    Err(message) => return Ok(HttpResponse::BadRequest().body(message)),
                };
                sqlx::query("UPDATE enemy_wave SET status = 'scheduled', resolve_at = $2 WHERE id = $1")
                    .bind(id)
                    .bind(at)
                    .execute(&mut *tx)
                    .await?;
                log(&mut tx, admin.id, "joel_schedule", format!("Scheduled wave \"{name}\" for {at}"), None).await?;
            }
            "unschedule" => {
                if status != "scheduled" {
                    return conflict("This wave isn't scheduled");
                }
                sqlx::query("UPDATE enemy_wave SET status = 'draft' WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                log(&mut tx, admin.id, "joel_unschedule", format!("Unscheduled wave \"{name}\""), None).await?;
            }
            "resolve" => {
                if status != "draft" && status != "scheduled" {
                    return conflict("This wave has already finished");
                }
                if orders == 0 {
                    return conflict("Add at least one order first");
                }
                resolve(&mut tx, id).await?;
                log(&mut tx, admin.id, "joel_resolve", format!("Resolved wave \"{name}\""), None).await?;
            }
            "cancel" => {
                if status != "draft" && status != "scheduled" {
                    return conflict("This wave has already finished");
                }
                sqlx::query("UPDATE enemy_wave SET status = 'cancelled' WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                log(&mut tx, admin.id, "joel_cancel", format!("Cancelled wave \"{name}\""), None).await?;
            }
            "reopen" => {
                if status != "cancelled" {
                    return conflict("Only cancelled waves can be reopened");
                }
                sqlx::query("UPDATE enemy_wave SET status = 'draft' WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
            "rollback" => {
                let latest: Option<i32> = sqlx::query_scalar(
                    "SELECT id FROM enemy_wave WHERE status = 'resolved' ORDER BY resolved_at DESC LIMIT 1",
                )
                .fetch_optional(&mut *tx)
                .await?;
                if status != "resolved" || latest != Some(id) {
                    return conflict("Only the most recently resolved wave can be rolled back");
                }
                let snapshot: Value = sqlx::query_scalar("SELECT coalesce(snapshot, '[]') FROM enemy_wave WHERE id = $1")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                for tile in snapshot.as_array().into_iter().flatten() {
                    sqlx::query(
                        "INSERT INTO territory (hex_row, hex_col, enemies) VALUES ($1, $2, $3)
                         ON CONFLICT (hex_row, hex_col) DO UPDATE SET enemies = EXCLUDED.enemies",
                    )
                    .bind(tile["row"].as_i64().unwrap_or_default() as i32)
                    .bind(tile["col"].as_i64().unwrap_or_default() as i32)
                    .bind(&tile["enemies"])
                    .execute(&mut *tx)
                    .await?;
                }
                sqlx::query("UPDATE enemy_wave SET status = 'rolled_back' WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE announcement SET publish_at = NULL WHERE wave_id = $1 AND slot = 'followup'")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                log(&mut tx, admin.id, "joel_rollback", format!("Rolled back wave \"{name}\""), None).await?;
            }
            _ => return Ok(HttpResponse::NotFound().finish()),
        }
        tx.commit().await?;
        Ok(HttpResponse::NoContent().finish())
    }
    .await;
    result.unwrap_or_else(|e| failed("updating a wave", e))
}

#[put("/api/admin/joel/waves/{id:\\d+}/announcements/{slot}")]
pub async fn announce(
    req: HttpRequest,
    path: web::Path<(i32, String)>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Joel).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let (id, slot) = path.into_inner();
    if !SLOTS.contains(&slot.as_str()) {
        return HttpResponse::NotFound().finish();
    }
    let publish_at = match time(&body["publish_at"]) {
        Ok(at) => at,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let on_resolve = slot == "followup" && body["on_resolve"].as_bool().unwrap_or(false);
    let saved = sqlx::query(
        "INSERT INTO announcement (wave_id, slot, title, body, publish_at, on_resolve, created_by)
         SELECT id, $2, $3, $4, $5, $6, $7 FROM enemy_wave WHERE id = $1
         ON CONFLICT (wave_id, slot) DO UPDATE SET
             title = EXCLUDED.title, body = EXCLUDED.body, publish_at = EXCLUDED.publish_at, on_resolve = EXCLUDED.on_resolve",
    )
    .bind(id)
    .bind(&slot)
    .bind(body["title"].as_str().unwrap_or_default().trim())
    .bind(body["body"].as_str().unwrap_or_default().trim())
    .bind(if on_resolve { None } else { publish_at })
    .bind(on_resolve)
    .bind(admin.id)
    .execute(pool.get_ref())
    .await;
    match saved {
        Ok(done) if done.rows_affected() == 0 => HttpResponse::NotFound().finish(),
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => failed("saving an announcement", e),
    }
}

#[delete("/api/admin/joel/waves/{id:\\d+}/announcements/{slot}")]
pub async fn unannounce(req: HttpRequest, path: web::Path<(i32, String)>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, Permission::Joel).await {
        return response;
    }
    let (id, slot) = path.into_inner();
    let deleted = sqlx::query("DELETE FROM announcement WHERE wave_id = $1 AND slot = $2")
        .bind(id)
        .bind(slot)
        .execute(pool.get_ref())
        .await;
    match deleted {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => failed("deleting an announcement", e),
    }
}

#[put("/api/admin/joel/tiles/{row:\\d+}/{col:\\d+}")]
pub async fn edit(
    req: HttpRequest,
    path: web::Path<(i32, i32)>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, Permission::Joel).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let hex = path.into_inner();
    let Some(enemies) = force(&body["enemies"]) else {
        return HttpResponse::BadRequest().body("Those enemy counts aren't valid");
    };
    match territory::placed() {
        Ok(placed) if !placed.contains(&hex) => return HttpResponse::BadRequest().body("That tile isn't on the map"),
        Ok(_) => {}
        Err(e) => return failed("reading the map", e),
    }
    let reason = body["reason"].as_str().map(str::trim).filter(|reason| !reason.is_empty());
    let result: sqlx::Result<()> = async {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "INSERT INTO territory (hex_row, hex_col, enemies) VALUES ($1, $2, $3)
             ON CONFLICT (hex_row, hex_col) DO UPDATE SET enemies = EXCLUDED.enemies",
        )
        .bind(hex.0)
        .bind(hex.1)
        .bind(json!(enemies))
        .execute(&mut *tx)
        .await?;
        let summary = format!("Set tile {}, {} to {}", hex.0, hex.1, describe(&enemies));
        log(&mut tx, admin.id, "joel_edit", summary, reason).await?;
        tx.commit().await
    }
    .await;
    match result {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => failed("editing a tile", e),
    }
}

#[get("/api/announcements")]
pub async fn announcements(pool: web::Data<PgPool>) -> HttpResponse {
    let rows: sqlx::Result<Vec<Value>> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', id, 'title', title, 'body', body, 'published_at', publish_at)
         FROM announcement WHERE publish_at <= now() ORDER BY publish_at DESC LIMIT 50",
    )
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => failed("listing announcements", e),
    }
}
