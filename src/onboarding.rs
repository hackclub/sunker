use crate::admin;
use actix_web::{HttpRequest, HttpResponse, get, post, put, web};
use hc_helper::auth::data::get_auth_data_with_token;
use hc_helper::auth::login;
use serde_json::{Map, Value, json};
use sqlx::PgPool;

pub const STEPS: [&str; 4] = ["passport", "guild", "hq", "done"];
const NAME_LIMIT: usize = 32;
const AGE_LIMIT: usize = 20;
const OCCUPATION_LIMIT: usize = 64;
const BACKSTORY_LIMIT: usize = 500;
const FEATURE_LIMIT: usize = 60;
const GUILD_LOCK: i64 = 0x5375_6e6b;

fn failed(what: &str, e: impl std::fmt::Display) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

async fn account(req: &HttpRequest, pool: &PgPool) -> sqlx::Result<Option<(i32, String)>> {
    let Some(token) = login::get_auth_token_with_handling(req) else {
        return Ok(None);
    };
    let Ok(data) = get_auth_data_with_token(&token).await else {
        return Ok(None);
    };
    let id: Option<i32> = sqlx::query_scalar("SELECT id FROM \"user\" WHERE hca_id = $1 AND deleted_at IS NULL")
        .bind(&data.id)
        .fetch_optional(pool)
        .await?;
    Ok(id.map(|id| (id, data.id)))
}

#[get("/api/me")]
pub async fn me(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<Option<Value>> = async {
        let Some((user, hca)) = account(&req, &pool).await? else {
            return Ok(None);
        };
        let mut me: Value = sqlx::query_scalar(
            "SELECT jsonb_build_object(
                 'stage', u.onboarding_step,
                 'banned', u.is_banned,
                 'character', (SELECT jsonb_build_object('handle', c.handle, 'attributes', c.attributes) FROM character c WHERE c.user_id = u.id),
                 'guild', (SELECT jsonb_build_object('slug', g.slug, 'name', g.name) FROM guild g WHERE g.id = u.guild_id),
                 'resources', (SELECT jsonb_build_object(
                     'chiprock', coalesce(sum(l.chiprock), 0), 'granite', coalesce(sum(l.granite), 0),
                     'potash', coalesce(sum(l.potash), 0), 'bauxite', coalesce(sum(l.bauxite), 0),
                     'shale', coalesce(sum(l.shale), 0), 'kalkite', coalesce(sum(l.kalkite), 0),
                     'cuprite', coalesce(sum(l.cuprite), 0), 'ring_upgrade', coalesce(sum(l.ring_upgrade), 0),
                     'scrip', coalesce(sum(l.scrip), 0)
                 ) FROM resource_ledger l WHERE l.user_id = u.id)
             ) FROM \"user\" u WHERE u.id = $1",
        )
        .bind(user)
        .fetch_one(pool.get_ref())
        .await?;
        me["superadmin"] = admin::superadmins().contains(&hca).into();
        Ok(Some(me))
    }
    .await;
    match result {
        Ok(Some(me)) => HttpResponse::Ok().json(me),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("loading the current user", e),
    }
}

fn text(body: &Value, key: &str, limit: usize, label: &str) -> Result<String, String> {
    let value = body[key].as_str().unwrap_or_default().trim().to_string();
    if value.is_empty() {
        return Err(format!("{label} can't be empty"));
    }
    if value.chars().count() > limit {
        return Err(format!("{label} must be under {limit} characters"));
    }
    Ok(value)
}

fn features(body: &Value) -> Result<Vec<Value>, String> {
    let list = body["features"].as_array().ok_or("Add at least one feature to your headshot")?;
    if list.is_empty() {
        return Err("Add at least one feature to your headshot".into());
    }
    if list.len() > FEATURE_LIMIT {
        return Err(format!("A headshot can have at most {FEATURE_LIMIT} features"));
    }
    list.iter()
        .map(|feature| {
            let shape = feature["shape"].as_str().filter(|shape| !shape.is_empty() && shape.len() <= 32).ok_or("Unknown feature")?;
            let colour = feature["colour"]
                .as_str()
                .filter(|colour| colour.len() == 7 && colour.starts_with('#') && colour[1..].bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or("Unknown feature colour")?;
            let number = |key: &str| feature[key].as_f64().filter(|n| n.is_finite()).ok_or("Feature is missing its position");
            let mut clean = Map::new();
            clean.insert("shape".into(), shape.into());
            clean.insert("colour".into(), colour.into());
            clean.insert("x".into(), number("x")?.clamp(0.0, 1.0).into());
            clean.insert("y".into(), number("y")?.clamp(0.0, 1.0).into());
            clean.insert("rotation".into(), (number("rotation")? % 360.0).into());
            Ok(Value::Object(clean))
        })
        .collect()
}

#[put("/api/character")]
pub async fn save_character(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let parsed = (|| {
        Ok::<_, String>((
            text(&body, "handle", NAME_LIMIT, "Name")?,
            json!({
                "age": text(&body, "age", AGE_LIMIT, "Age")?,
                "occupation": text(&body, "occupation", OCCUPATION_LIMIT, "Occupation")?,
                "backstory": text(&body, "backstory", BACKSTORY_LIMIT, "Backstory")?,
                "features": features(&body)?,
                "hue": body["hue"].as_f64().filter(|hue| hue.is_finite()).map(|hue| hue.rem_euclid(360.0).round()),
            }),
        ))
    })();
    let (handle, attributes) = match parsed {
        Ok(parsed) => parsed,
        Err(message) => return HttpResponse::BadRequest().body(message),
    };
    let result: sqlx::Result<Option<bool>> = async {
        let Some((user, _)) = account(&req, &pool).await? else {
            return Ok(None);
        };
        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM character WHERE lower(handle) = lower($1) AND user_id <> $2)")
            .bind(&handle)
            .bind(user)
            .fetch_one(pool.get_ref())
            .await?;
        if taken {
            return Ok(Some(false));
        }
        sqlx::query(
            "INSERT INTO character (user_id, handle, attributes) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET handle = EXCLUDED.handle, attributes = EXCLUDED.attributes",
        )
        .bind(user)
        .bind(&handle)
        .bind(&attributes)
        .execute(pool.get_ref())
        .await?;
        Ok(Some(true))
    }
    .await;
    match result {
        Ok(Some(true)) => HttpResponse::Ok().json(json!({ "handle": handle, "attributes": attributes })),
        Ok(Some(false)) => HttpResponse::Conflict().body("That name is taken"),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => HttpResponse::Conflict().body("That name is taken"),
        Err(e) => failed("saving a character", e),
    }
}

async fn assign_guild(pool: &PgPool, user: i32) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(GUILD_LOCK).execute(&mut *tx).await?;
    sqlx::query(
        "WITH chosen AS (
             SELECT coalesce(
                 (SELECT r.guild_id FROM \"user\" r WHERE r.id = u.referred_by AND r.deleted_at IS NULL),
                 (SELECT g.id FROM guild g
                  LEFT JOIN \"user\" m ON m.guild_id = g.id AND m.deleted_at IS NULL AND NOT m.is_banned
                  GROUP BY g.id ORDER BY count(m.id), random() LIMIT 1)
             ) AS id
             FROM \"user\" u WHERE u.id = $1 AND u.guild_id IS NULL
         )
         UPDATE \"user\" u SET
             guild_id = chosen.id,
             allowed_chats = ARRAY(SELECT DISTINCT unnest(u.allowed_chats || ARRAY(SELECT id FROM chat WHERE guild_id = chosen.id)) ORDER BY 1)
         FROM chosen WHERE u.id = $1",
    )
    .bind(user)
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

#[post("/api/onboarding")]
pub async fn advance(req: HttpRequest, body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let Some(next) = STEPS.iter().position(|step| Some(*step) == body["step"].as_str()) else {
        return HttpResponse::BadRequest().body("Unknown onboarding step");
    };
    let result: sqlx::Result<Option<Result<(), &str>>> = async {
        let Some((user, _)) = account(&req, &pool).await? else {
            return Ok(None);
        };
        let (current, filled): (Option<String>, bool) = sqlx::query_as(
            "SELECT onboarding_step, EXISTS (SELECT 1 FROM character WHERE user_id = $1) FROM \"user\" WHERE id = $1",
        )
        .bind(user)
        .fetch_one(pool.get_ref())
        .await?;
        let at = current.and_then(|step| STEPS.iter().position(|known| *known == step));
        if at.is_some_and(|at| next <= at) {
            return Ok(Some(Ok(())));
        }
        if next > at.map_or(0, |at| at + 1) {
            return Ok(Some(Err("Finish the earlier onboarding steps first")));
        }
        if STEPS[next] == "guild" {
            if !filled {
                return Ok(Some(Err("Fill out your passport first")));
            }
            assign_guild(&pool, user).await?;
        }
        sqlx::query(
            "UPDATE \"user\" SET onboarding_step = $2, onboarded_at = CASE WHEN $2 = 'done' THEN now() END WHERE id = $1",
        )
        .bind(user)
        .bind(STEPS[next])
        .execute(pool.get_ref())
        .await?;
        Ok(Some(Ok(())))
    }
    .await;
    match result {
        Ok(Some(Ok(()))) => HttpResponse::NoContent().finish(),
        Ok(Some(Err(message))) => HttpResponse::Conflict().body(message),
        Ok(None) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("advancing onboarding", e),
    }
}

#[post("/api/onboarding/reset")]
pub async fn reset(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    let result: sqlx::Result<bool> = async {
        let Some((user, _)) = account(&req, &pool).await? else {
            return Ok(false);
        };
        sqlx::query("UPDATE \"user\" SET onboarding_step = NULL, onboarded_at = NULL WHERE id = $1")
            .bind(user)
            .execute(pool.get_ref())
            .await?;
        Ok(true)
    }
    .await;
    match result {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::Unauthorized().finish(),
        Err(e) => failed("resetting onboarding", e),
    }
}
