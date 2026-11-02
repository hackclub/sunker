use crate::auth;
use crate::territory::{self, Hex};
use actix_web::{HttpRequest, HttpResponse, get, patch, post, web};
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

const OPERATIONS_FILE: &str = "operations.json";
const RESOURCES: [&str; 7] = ["chiprock", "granite", "potash", "bauxite", "shale", "kalkite", "cuprite"];
const CHIPROCK: usize = 0;
const POTASH: usize = 2;
const SHALE: usize = 4;
const CUPRITE: usize = 6;
const MAX_CUPRITE: i64 = 10;
const SEA_BASE: i64 = 6;
const COLUMNS: &str = "id, name, status, hex_row, hex_col, route, kind, cuprite, cost, path";

type Cost = [i64; 7];
type Columns = (i32, String, String, Option<i32>, Option<i32>, Option<String>, Option<String>, i32, Option<Value>, Option<Value>);

enum Failure {
    Unauthorized,
    NotFound,
    Bad(&'static str),
    Internal(String),
}

impl From<sqlx::Error> for Failure {
    fn from(e: sqlx::Error) -> Self {
        Failure::Internal(e.to_string())
    }
}

fn respond(what: &str, result: Result<Value, Failure>) -> HttpResponse {
    match result {
        Ok(value) => HttpResponse::Ok().json(value),
        Err(Failure::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(Failure::NotFound) => HttpResponse::NotFound().body("That operation doesn't exist"),
        Err(Failure::Bad(message)) => HttpResponse::BadRequest().body(message),
        Err(Failure::Internal(e)) => {
            eprintln!("{what} failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn signed_in(req: &HttpRequest, pool: &PgPool) -> Result<i32, Failure> {
    auth::user_id(req, pool).await?.ok_or(Failure::Unauthorized)
}

struct Tile {
    base: String,
    variants: Vec<String>,
}

impl Tile {
    fn has(&self, variant: &str) -> bool {
        self.variants.iter().any(|other| other == variant)
    }

    fn road(&self) -> bool {
        self.has("street") || self.has("highway")
    }

    fn water(&self) -> bool {
        self.base == "ocean" && !self.road()
    }
}

struct Spot<'a> {
    tile: &'a Tile,
    enemies: Option<&'a Value>,
}

fn adjust(slug: &str, _spot: &Spot, cost: Cost) -> Option<Cost> {
    match slug {
        _ => Some(cost),
    }
}

async fn apply(tx: &mut Transaction<'_, Postgres>, effect: &str, (row, col): Hex) -> sqlx::Result<()> {
    match effect {
        "liberate" => {
            sqlx::query("UPDATE territory SET is_occupied = false, enemies = '{}' WHERE hex_row = $1 AND hex_col = $2")
                .bind(row)
                .bind(col)
                .execute(&mut **tx)
                .await?;
        }
        _ => {}
    }
    Ok(())
}

struct Route {
    cost: Cost,
    path: Vec<Hex>,
}

struct World {
    tiles: HashMap<Hex, Tile>,
    occupied: HashMap<Hex, Value>,
    rings: HashMap<Hex, u32>,
    origins: Vec<Hex>,
}

async fn world(pool: &PgPool) -> Result<World, Failure> {
    let map: Value = std::fs::read(crate::MAP_FILE)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
        .map_err(Failure::Internal)?;
    let mut tiles = HashMap::new();
    for (r, row) in map["rows"].as_array().into_iter().flatten().enumerate() {
        for (c, tile) in row.as_array().into_iter().flatten().enumerate() {
            if tile.is_null() {
                continue;
            }
            let variants = tile["variants"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(String::from)).collect();
            tiles.insert((r as i32, c as i32), Tile { base: tile["base"].as_str().unwrap_or("plain").to_string(), variants });
        }
    }
    let rows: Vec<(i32, i32, Value)> = sqlx::query_as("SELECT hex_row, hex_col, enemies FROM territory WHERE is_occupied")
        .fetch_all(pool)
        .await?;
    let occupied: HashMap<Hex, Value> = rows.into_iter().map(|(row, col, enemies)| ((row, col), enemies)).collect();
    let rings = territory::rings(&tiles.keys().copied().collect(), &occupied.keys().copied().collect::<HashSet<_>>());
    let origins = tiles.iter().filter(|(_, tile)| tile.has("origin")).map(|(hex, _)| *hex).collect();
    Ok(World { tiles, occupied, rings, origins })
}

impl World {
    fn selectable(&self, hex: Hex) -> bool {
        self.tiles.get(&hex).is_some_and(|tile| !tile.has("mine")) && (!self.occupied.contains_key(&hex) || self.rings.get(&hex) == Some(&0))
    }

    fn route(&self, to: Hex, sea: bool) -> Option<Route> {
        let mut best: HashMap<Hex, (i64, i64, i64)> = self.origins.iter().map(|hex| (*hex, (0, 0, 0))).collect();
        let mut previous = HashMap::new();
        let mut queue: BinaryHeap<_> = self.origins.iter().map(|hex| Reverse(((0i64, 0i64, 0i64), *hex))).collect();
        while let Some(Reverse((key, hex))) = queue.pop() {
            if hex == to {
                let mut path = vec![to];
                while let Some(back) = previous.get(path.last().unwrap()) {
                    path.push(*back);
                }
                path.reverse();
                let (chiprock, shale, tiles) = key;
                let mut cost = [0; 7];
                cost[CHIPROCK] = chiprock;
                cost[SHALE] = if shale > 0 { shale + SEA_BASE } else { 0 };
                cost[POTASH] = tiles / 2;
                return Some(Route { cost, path });
            }
            if best.get(&hex).is_some_and(|known| *known < key) {
                continue;
            }
            for next in territory::neighbours(hex) {
                let Some(tile) = self.tiles.get(&next) else {
                    continue;
                };
                if tile.has("mine") || (tile.water() && !sea) || (next != to && self.occupied.contains_key(&next)) {
                    continue;
                }
                let (chiprock, shale) = if tile.water() { (0, 1) } else if tile.road() { (2, 0) } else { (4, 0) };
                let candidate = (key.0 + chiprock, key.1 + shale, key.2 + 1);
                if best.get(&next).is_some_and(|known| *known <= candidate) {
                    continue;
                }
                best.insert(next, candidate);
                previous.insert(next, hex);
                queue.push(Reverse((candidate, next)));
            }
        }
        None
    }

    fn plan(&self, catalog: &Map<String, Value>, hex: Hex) -> Plan {
        let land = self.route(hex, false);
        let sea = self
            .route(hex, true)
            .filter(|sea| sea.cost[SHALE] > 0 && land.as_ref().is_none_or(|land| sea.cost[CHIPROCK] < land.cost[CHIPROCK]));
        let options = self.tiles.get(&hex).map_or_else(Vec::new, |tile| {
            let spot = Spot { tile, enemies: self.occupied.get(&hex) };
            catalog
                .iter()
                .filter(|(_, entry)| allowed(&entry["requirements"], &spot))
                .filter_map(|(slug, entry)| adjust(slug, &spot, parse(&entry["cost"])).map(|cost| (slug.clone(), entry.clone(), cost)))
                .collect()
        });
        Plan { land, sea, options }
    }
}

struct Plan {
    land: Option<Route>,
    sea: Option<Route>,
    options: Vec<(String, Value, Cost)>,
}

impl Plan {
    fn route(&self, wanted: Option<&str>) -> Option<(&'static str, &Route)> {
        match (wanted, &self.sea) {
            (Some("sea"), Some(sea)) => Some(("sea", sea)),
            _ => self.land.as_ref().map(|land| ("land", land)).or(self.sea.as_ref().map(|sea| ("sea", sea))),
        }
    }

    fn option(&self, kind: Option<&str>) -> Option<&(String, Value, Cost)> {
        self.options.iter().find(|(slug, _, _)| Some(slug.as_str()) == kind)
    }
}

fn catalog() -> Result<Map<String, Value>, Failure> {
    std::fs::read(OPERATIONS_FILE)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string()))
        .and_then(|value| value.as_object().cloned().ok_or("operations.json must be an object".to_string()))
        .map_err(Failure::Internal)
}

fn allowed(requirements: &Value, spot: &Spot) -> bool {
    let listed = |key: &str| requirements[key].as_array().map(|entries| entries.iter().filter_map(Value::as_str).collect::<Vec<_>>());
    let any = |key: &str, test: &dyn Fn(&str) -> bool| listed(key).is_none_or(|entries| entries.into_iter().any(test));
    let none = |key: &str, test: &dyn Fn(&str) -> bool| listed(key).is_none_or(|entries| !entries.into_iter().any(test));
    let base = |base: &str| base == spot.tile.base;
    let variant = |variant: &str| spot.tile.has(variant);
    let enemy = |enemy: &str| spot.enemies.is_some_and(|enemies| enemies[enemy].as_i64().unwrap_or(0) > 0);
    any("bases", &base)
        && none("unsupported_bases", &base)
        && any("variants", &variant)
        && none("unsupported_variants", &variant)
        && any("enemies", &enemy)
        && none("unsupported_enemies", &enemy)
        && requirements["occupied"].as_bool().is_none_or(|occupied| occupied == spot.enemies.is_some())
}

fn parse(value: &Value) -> Cost {
    RESOURCES.map(|resource| value[resource].as_i64().unwrap_or(0).max(0))
}

fn named(cost: Cost) -> Value {
    Value::Object(RESOURCES.iter().zip(cost).map(|(name, amount)| (name.to_string(), json!(amount))).collect())
}

fn breakdown(operation: Cost, transport: Cost, cuprite: i64) -> Value {
    let (kept, whole) = (3i64.pow(cuprite as u32), 4i64.pow(cuprite as u32));
    let mut total: Cost = std::array::from_fn(|i| ((operation[i] + transport[i]) * kept + whole - 1) / whole);
    total[CUPRITE] += cuprite;
    json!({
        "operation": named(operation),
        "transport": named(transport),
        "cuprite": cuprite,
        "multiplier": kept as f64 / whole as f64,
        "total": named(total),
    })
}

fn hexes(path: &[Hex]) -> Value {
    json!(path.iter().map(|(row, col)| [row, col]).collect::<Vec<_>>())
}

struct Row {
    id: i32,
    name: String,
    status: String,
    hex: Option<Hex>,
    route: Option<String>,
    kind: Option<String>,
    cuprite: i32,
    cost: Option<Value>,
    path: Option<Value>,
}

async fn fetch<'e>(executor: impl sqlx::PgExecutor<'e>, user: i32, id: i32, lock: bool) -> Result<Row, Failure> {
    let row: Option<Columns> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM operation WHERE id = $1 AND user_id = $2{}",
        if lock { " FOR UPDATE" } else { "" }
    )))
    .bind(id)
    .bind(user)
    .fetch_optional(executor)
    .await?;
    let (id, name, status, row, col, route, kind, cuprite, cost, path) = row.ok_or(Failure::NotFound)?;
    Ok(Row { id, name, status, hex: row.zip(col), route, kind, cuprite, cost, path })
}

async fn sums<'e>(executor: impl sqlx::PgExecutor<'e>, column: &str, id: i32) -> sqlx::Result<Cost> {
    let totals: (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {} FROM resource_ledger WHERE {column} = $1",
        RESOURCES.map(|resource| format!("coalesce(sum({resource}), 0)::bigint")).join(", ")
    )))
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok([totals.0, totals.1, totals.2, totals.3, totals.4, totals.5, totals.6])
}

async fn allocated<'e>(executor: impl sqlx::PgExecutor<'e>, id: i32) -> sqlx::Result<Cost> {
    Ok(sums(executor, "operation_id", id).await?.map(|amount| -amount))
}

async fn ledger(tx: &mut Transaction<'_, Postgres>, user: i32, id: i32, reason: &str, amounts: Cost) -> sqlx::Result<()> {
    let mut query = sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO resource_ledger (user_id, operation_id, reason, {}) VALUES ($1, $2, $3, {})",
        RESOURCES.join(", "),
        (4..4 + RESOURCES.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ")
    )))
    .bind(user)
    .bind(id)
    .bind(reason);
    for amount in amounts {
        query = query.bind(amount as i32);
    }
    query.execute(&mut **tx).await?;
    Ok(())
}

async fn view(pool: &PgPool, user: i32, id: i32, built: Option<World>) -> Result<Value, Failure> {
    let row = fetch(pool, user, id, false).await?;
    let catalog = catalog()?;
    let mut view = json!({
        "id": row.id,
        "name": row.name,
        "status": row.status,
        "tile": row.hex.map(|(r, c)| [r, c]),
        "route": row.route,
        "kind": row.kind,
        "cuprite": row.cuprite,
        "max_cuprite": MAX_CUPRITE,
        "operation": row.kind.as_ref().and_then(|kind| catalog.get(kind)).map(|entry| json!({ "name": entry["name"], "description": entry["description"] })),
        "allocated": named(allocated(pool, id).await?),
        "balance": named(sums(pool, "user_id", user).await?),
        "breakdown": row.cost,
        "path": row.path,
    });
    if row.status != "draft" {
        return Ok(view);
    }
    let world = match built {
        Some(world) => world,
        None => world(pool).await?,
    };
    view["selectable"] = json!(world.tiles.keys().filter(|hex| world.selectable(**hex)).map(|(r, c)| [r, c]).collect::<Vec<_>>());
    let Some(hex) = row.hex else {
        return Ok(view);
    };
    let plan = world.plan(&catalog, hex);
    let route = |route: &Option<Route>| route.as_ref().map(|route| json!({ "cost": named(route.cost), "tiles": route.path.len() - 1 }));
    view["routes"] = json!({ "land": route(&plan.land), "sea": route(&plan.sea) });
    view["options"] = json!(
        plan.options
            .iter()
            .map(|(slug, entry, cost)| json!({ "slug": slug, "name": entry["name"], "description": entry["description"], "cost": named(*cost) }))
            .collect::<Vec<_>>()
    );
    if let Some((_, chosen)) = plan.route(row.route.as_deref()) {
        view["path"] = hexes(&chosen.path);
        if let Some((_, _, cost)) = plan.option(row.kind.as_deref()) {
            view["breakdown"] = breakdown(*cost, chosen.cost, row.cuprite as i64);
        }
    }
    Ok(view)
}

fn name(value: &Value) -> Result<String, Failure> {
    let name = value.as_str().ok_or(Failure::Bad("Name must be text"))?.trim().to_string();
    match name.chars().count() {
        0 => Err(Failure::Bad("Give the operation a name")),
        1..=100 => Ok(name),
        _ => Err(Failure::Bad("Name must be under 100 characters")),
    }
}

#[get("/api/operations")]
pub async fn list(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        Ok(sqlx::query_scalar(
            "SELECT coalesce(jsonb_agg(jsonb_build_object('id', id, 'name', name, 'status', status) ORDER BY created_at DESC), '[]')
             FROM operation WHERE user_id = $1",
        )
        .bind(user)
        .fetch_one(pool.get_ref())
        .await?)
    }
    .await;
    respond("listing operations", result)
}

#[post("/api/operations")]
pub async fn create(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        let name = name(&body["name"])?;
        let id: i32 = sqlx::query_scalar("INSERT INTO operation (user_id, name) VALUES ($1, $2) RETURNING id")
            .bind(user)
            .bind(name)
            .fetch_one(pool.get_ref())
            .await?;
        Ok(json!({ "id": id }))
    }
    .await;
    respond("creating an operation", result)
}

#[get("/api/operations/{id:\\d+}")]
pub async fn show(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        view(&pool, user, *id, None).await
    }
    .await;
    respond("showing an operation", result)
}

#[patch("/api/operations/{id:\\d+}")]
pub async fn save(req: HttpRequest, id: web::Path<i32>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        let mut row = fetch(pool.get_ref(), user, *id, false).await?;
        if row.status != "draft" {
            return Err(Failure::Bad("Move the operation back to draft to change it"));
        }
        let world = world(&pool).await?;
        if let Some(value) = body.get("name") {
            row.name = name(value)?;
        }
        if let Some(tile) = body.get("tile") {
            row.hex = match (tile[0].as_i64(), tile[1].as_i64()) {
                _ if tile.is_null() => None,
                (Some(r), Some(c)) if world.selectable((r as i32, c as i32)) => Some((r as i32, c as i32)),
                _ => return Err(Failure::Bad("That tile can't be picked")),
            };
        }
        if let Some(route) = body.get("route") {
            row.route = route.as_str().map(String::from);
        }
        if let Some(kind) = body.get("kind") {
            row.kind = kind.as_str().map(String::from);
        }
        if let Some(cuprite) = body.get("cuprite") {
            row.cuprite = cuprite
                .as_i64()
                .filter(|cuprite| (0..=MAX_CUPRITE).contains(cuprite))
                .ok_or(Failure::Bad("That much cuprite can't be requested"))? as i32;
        }
        match row.hex {
            None => (row.route, row.kind) = (None, None),
            Some(hex) => {
                let plan = world.plan(&catalog()?, hex);
                row.route = plan.route(row.route.as_deref()).map(|(route, _)| route.to_string());
                if plan.option(row.kind.as_deref()).is_none() {
                    if body.get("kind").is_some_and(|kind| !kind.is_null()) {
                        return Err(Failure::Bad("That operation can't happen on this tile"));
                    }
                    row.kind = None;
                }
            }
        }
        sqlx::query(
            "UPDATE operation SET name = $3, hex_row = $4, hex_col = $5, route = $6, kind = $7, cuprite = $8
             WHERE id = $1 AND user_id = $2 AND status = 'draft'",
        )
        .bind(row.id)
        .bind(user)
        .bind(&row.name)
        .bind(row.hex.map(|(r, _)| r))
        .bind(row.hex.map(|(_, c)| c))
        .bind(&row.route)
        .bind(&row.kind)
        .bind(row.cuprite)
        .execute(pool.get_ref())
        .await?;
        view(&pool, user, row.id, Some(world)).await
    }
    .await;
    respond("saving an operation", result)
}

#[post("/api/operations/{id:\\d+}/confirm")]
pub async fn confirm(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        let world = world(&pool).await?;
        let catalog = catalog()?;
        let mut tx = pool.begin().await?;
        let row = fetch(&mut *tx, user, *id, true).await?;
        if row.status != "draft" {
            return Err(Failure::Bad("Only drafts can be confirmed"));
        }
        let hex = row.hex.filter(|hex| world.selectable(*hex)).ok_or(Failure::Bad("Pick a tile first"))?;
        let plan = world.plan(&catalog, hex);
        let (route, chosen) = plan.route(row.route.as_deref()).ok_or(Failure::Bad("There's no route to that tile"))?;
        let (_, _, cost) = plan.option(row.kind.as_deref()).ok_or(Failure::Bad("Pick an operation first"))?;
        sqlx::query("UPDATE operation SET status = 'gathering', route = $2, cost = $3, path = $4 WHERE id = $1")
            .bind(row.id)
            .bind(route)
            .bind(breakdown(*cost, chosen.cost, row.cuprite as i64))
            .bind(hexes(&chosen.path))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        view(&pool, user, row.id, None).await
    }
    .await;
    respond("confirming an operation", result)
}

#[post("/api/operations/{id:\\d+}/draft")]
pub async fn draft(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        let mut tx = pool.begin().await?;
        let row = fetch(&mut *tx, user, *id, true).await?;
        if row.status != "gathering" {
            return Err(Failure::Bad("Only gathering operations can go back to draft"));
        }
        let allocated = allocated(&mut *tx, row.id).await?;
        if allocated.iter().any(|amount| *amount != 0) {
            ledger(&mut tx, user, row.id, "Operation refund", allocated).await?;
        }
        sqlx::query("UPDATE operation SET status = 'draft', cost = NULL, path = NULL WHERE id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        view(&pool, user, row.id, None).await
    }
    .await;
    respond("drafting an operation", result)
}

#[post("/api/operations/{id:\\d+}/allocate")]
pub async fn allocate(req: HttpRequest, id: web::Path<i32>, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        let mut tx = pool.begin().await?;
        let row = fetch(&mut *tx, user, *id, true).await?;
        if row.status != "gathering" {
            return Err(Failure::Bad("Confirm the operation before allocating"));
        }
        sqlx::query("SELECT 1 FROM \"user\" WHERE id = $1 FOR UPDATE")
            .bind(user)
            .execute(&mut *tx)
            .await?;
        let total = parse(&row.cost.unwrap_or_default()["total"]);
        let allocated = allocated(&mut *tx, row.id).await?;
        let balance = sums(&mut *tx, "user_id", user).await?;
        let wanted = parse(&body);
        let amounts: Cost = std::array::from_fn(|i| wanted[i].min(total[i] - allocated[i]).min(balance[i]).max(0));
        if amounts.iter().all(|amount| *amount == 0) {
            return Err(Failure::Bad("Nothing to allocate"));
        }
        ledger(&mut tx, user, row.id, "Operation allocation", amounts.map(|amount| -amount)).await?;
        tx.commit().await?;
        view(&pool, user, row.id, None).await
    }
    .await;
    respond("allocating to an operation", result)
}

#[post("/api/operations/{id:\\d+}/launch")]
pub async fn launch(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    let result = async {
        let user = signed_in(&req, &pool).await?;
        let world = world(&pool).await?;
        let catalog = catalog()?;
        let mut tx = pool.begin().await?;
        let row = fetch(&mut *tx, user, *id, true).await?;
        if row.status != "gathering" {
            return Err(Failure::Bad("Only gathering operations can launch"));
        }
        let still = row.hex.filter(|hex| world.selectable(*hex)).is_some_and(|hex| world.plan(&catalog, hex).option(row.kind.as_deref()).is_some());
        if !still {
            return Err(Failure::Bad("The target tile has changed, move the operation back to draft to replan it"));
        }
        let total = parse(&row.cost.unwrap_or_default()["total"]);
        let allocated = allocated(&mut *tx, row.id).await?;
        if allocated.iter().zip(total).any(|(have, need)| *have < need) {
            return Err(Failure::Bad("Not enough resources allocated yet"));
        }
        let effect = row.kind.as_ref().and_then(|kind| catalog.get(kind)).and_then(|entry| entry["effect"].as_str()).unwrap_or_default();
        if let Some(hex) = row.hex {
            apply(&mut tx, effect, hex).await?;
        }
        sqlx::query("UPDATE operation SET status = 'completed', launched_at = now() WHERE id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        view(&pool, user, row.id, None).await
    }
    .await;
    respond("launching an operation", result)
}
