use actix_web::{HttpResponse, get, web};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

pub type Hex = (i32, i32);

pub fn placed() -> anyhow::Result<HashSet<Hex>> {
    let map: Value = serde_json::from_slice(&std::fs::read(crate::MAP_FILE)?)?;
    Ok(map["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(r, row)| {
            row.as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .filter(|(_, tile)| !tile.is_null())
                .map(move |(c, _)| (r as i32, c as i32))
        })
        .collect())
}

pub fn neighbours((row, col): Hex) -> [Hex; 6] {
    let odd = row & 1;
    [
        (row - 1, col + odd),
        (row, col + 1),
        (row + 1, col + odd),
        (row + 1, col + odd - 1),
        (row, col - 1),
        (row - 1, col + odd - 1),
    ]
}

pub fn rings(placed: &HashSet<Hex>, occupied: &HashSet<Hex>) -> HashMap<Hex, u32> {
    let mut rings = HashMap::new();
    let mut queue: Vec<Hex> = occupied
        .iter()
        .copied()
        .filter(|hex| neighbours(*hex).iter().any(|other| placed.contains(other) && !occupied.contains(other)))
        .collect();
    for hex in &queue {
        rings.insert(*hex, 0);
    }
    let mut i = 0;
    while i < queue.len() {
        let hex = queue[i];
        for other in neighbours(hex) {
            if occupied.contains(&other) && !rings.contains_key(&other) {
                rings.insert(other, rings[&hex] + 1);
                queue.push(other);
            }
        }
        i += 1;
    }
    rings
}

#[get("/api/territory")]
pub async fn list(pool: web::Data<PgPool>) -> HttpResponse {
    let rows: Result<Vec<(i32, i32, Value)>, _> =
        sqlx::query_as("SELECT hex_row, hex_col, enemies FROM territory WHERE is_occupied")
            .fetch_all(pool.get_ref())
            .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("listing territory failed: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let placed = match placed() {
        Ok(placed) => placed,
        Err(e) => {
            eprintln!("reading the map failed: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let occupied: HashSet<Hex> = rows.iter().map(|(r, c, _)| (*r, *c)).collect();
    let rings = rings(&placed, &occupied);
    let tiles: Vec<Value> = rows
        .into_iter()
        .map(|(row, col, enemies)| {
            let hidden = rings.get(&(row, col)).is_some_and(|ring| *ring > 1);
            json!({ "row": row, "col": col, "occupied": true, "enemies": if hidden { json!({}) } else { enemies } })
        })
        .collect();
    HttpResponse::Ok().json(tiles)
}
