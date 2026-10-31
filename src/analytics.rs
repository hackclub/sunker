use crate::{admin, auth};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use futures_util::future::{try_join, try_join_all, try_join3, try_join4};
use serde_json::{Map, Value, json};
use sqlx::{AssertSqlSafe, PgPool};

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

pub async fn record_login(req: &HttpRequest, pool: &PgPool) -> sqlx::Result<()> {
    let Some(user) = auth::user_id(req, pool).await? else {
        return Ok(());
    };
    sqlx::query(
        "INSERT INTO analytics (user_id, kind)
         SELECT $1, 'login'
         WHERE NOT EXISTS (
             SELECT 1 FROM analytics
             WHERE user_id = $1 AND kind = 'login' AND created_at >= date_trunc('day', now(), 'UTC')
         )",
    )
    .bind(user)
    .execute(pool)
    .await?;
    Ok(())
}

#[post("/api/visit")]
pub async fn visit(body: web::Json<Value>, pool: web::Data<PgPool>) -> HttpResponse {
    let visitor = body["visitor"].as_str().unwrap_or_default().trim();
    if visitor.is_empty() || visitor.len() > 64 {
        return HttpResponse::BadRequest().finish();
    }
    let result = sqlx::query(
        "INSERT INTO analytics (visitor_id, kind)
         SELECT $1, 'visit'
         WHERE NOT EXISTS (
             SELECT 1 FROM analytics
             WHERE visitor_id = $1 AND kind = 'visit' AND created_at >= date_trunc('day', now(), 'UTC')
         )",
    )
    .bind(visitor)
    .execute(pool.get_ref())
    .await;
    match result {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("recording visit failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

const DAY: &str = "YYYY-MM-DD";
const HOUR: &str = "YYYY-MM-DD\"T\"HH24";
const DAILY_ONLY: [&str; 6] = [
    "repeat_logins",
    "dau",
    "retention_1",
    "retention_1_base",
    "retention_7",
    "retention_7_base",
];

fn bucket(column: &str, format: &str) -> String {
    format!("to_char({column} AT TIME ZONE 'UTC', '{format}')")
}

fn recent(sql: &str, columns: &str) -> String {
    format!(
        "SELECT * FROM ({sql}) AS bucketed ({columns}) WHERE key >= {}",
        bucket("(now() - interval '47 hours')", HOUR)
    )
}

fn resources() -> String {
    RESOURCES
        .map(|name| format!("('{name}', {name})"))
        .join(", ")
}

async fn series(pool: &PgPool, sql: &str) -> sqlx::Result<Value> {
    let rows: Vec<(String, f64)> = sqlx::query_as(AssertSqlSafe(sql.to_string())).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(day, value)| (day, json!(value)))
        .collect::<Map<_, _>>()
        .into())
}

async fn breakdown(pool: &PgPool, sql: &str) -> sqlx::Result<Value> {
    let rows: Vec<(String, String, f64)> = sqlx::query_as(AssertSqlSafe(sql.to_string())).fetch_all(pool).await?;
    let mut categories = Map::new();
    for (day, category, value) in rows {
        if let Some(days) = categories
            .entry(category)
            .or_insert_with(|| json!({}))
            .as_object_mut()
        {
            days.insert(day, json!(value));
        }
    }
    Ok(categories.into())
}

fn queries(format: &str) -> Vec<(&'static str, String)> {
    let day = |column: &str| bucket(column, format);
    let count = |column: &str, from: &str| {
        format!(
            "SELECT {}, count(*)::float8 FROM {from} GROUP BY 1",
            day(column)
        )
    };
    let distinct = |column: &str, who: &str, from: &str| {
        format!(
            "SELECT {}, count(DISTINCT {who})::float8 FROM {from} GROUP BY 1",
            day(column)
        )
    };
    let first = |column: &str, who: &str, from: &str| {
        format!(
            "SELECT {}, count(*)::float8 FROM (SELECT {who}, min({column}) AS at FROM {from} GROUP BY {who}) firsts GROUP BY 1",
            day("at")
        )
    };
    let retention = |offset: i32| {
        let signup = "(u.created_at AT TIME ZONE 'UTC')::date";
        let base = format!("FROM \"user\" u WHERE {signup} + {offset} <= (now() AT TIME ZONE 'UTC')::date");
        (
            format!(
                "SELECT {}, count(*) FILTER (WHERE EXISTS (
                     SELECT 1 FROM analytics a
                     WHERE a.user_id = u.id AND a.kind = 'login'
                       AND (a.created_at AT TIME ZONE 'UTC')::date >= {signup} + {offset}
                 ))::float8 {base} GROUP BY 1",
                day("u.created_at")
            ),
            format!("SELECT {}, count(*)::float8 {base} GROUP BY 1", day("u.created_at")),
        )
    };
    let (retention_1, retention_1_base) = retention(1);
    let (retention_7, retention_7_base) = retention(7);
    let ships = "ship WHERE deleted_at IS NULL";
    let orders = "\"order\" WHERE deleted_at IS NULL";
    vec![
        ("signups", count("created_at", "\"user\"")),
        ("referrals", count("created_at", "\"user\" WHERE referred_by IS NOT NULL")),
        ("logins", distinct("created_at", "user_id", "analytics WHERE kind = 'login'")),
        (
            "repeat_logins",
            "SELECT day, count(*) FILTER (WHERE day > first)::float8 FROM (
                 SELECT day, min(day) OVER (PARTITION BY user_id) AS first FROM (
                     SELECT DISTINCT user_id, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day
                     FROM analytics WHERE kind = 'login'
                 ) days
             ) logins GROUP BY day"
                .to_string(),
        ),
        ("dau", distinct("created_at", "user_id", "analytics WHERE kind = 'active'")),
        ("duc", distinct("created_at", "author_id", "message")),
        ("messages", count("created_at", "message")),
        ("projects", count("created_at", "project WHERE deleted_at IS NULL")),
        ("shipped", count("created_at", ships)),
        ("approved", count("decided_at", &format!("{ships} AND status = 'approved'"))),
        ("rejected", count("decided_at", &format!("{ships} AND status = 'rejected'"))),
        (
            "hours_approved",
            format!(
                "SELECT {}, (sum(approved_seconds) / 3600.0)::float8 FROM {ships} AND status = 'approved' GROUP BY 1",
                day("decided_at")
            ),
        ),
        ("orders", count("created_at", orders)),
        ("orders_fulfilled", count("fulfilled_at", &format!("{orders} AND fulfilled_at IS NOT NULL"))),
        ("feedback_promoters", count("created_at", "feedback WHERE deleted_at IS NULL AND score >= 9")),
        ("feedback_passives", count("created_at", "feedback WHERE deleted_at IS NULL AND score BETWEEN 7 AND 8")),
        ("feedback_detractors", count("created_at", "feedback WHERE deleted_at IS NULL AND score <= 6")),
        ("feedback_count", count("created_at", "feedback WHERE deleted_at IS NULL")),
        ("retention_1", retention_1),
        ("retention_1_base", retention_1_base),
        ("retention_7", retention_7),
        ("retention_7_base", retention_7_base),
        ("funnel_visit", first("created_at", "visitor_id", "analytics WHERE kind = 'visit'")),
        ("funnel_onboard", count("onboarded_at", "\"user\" WHERE onboarded_at IS NOT NULL")),
        ("funnel_hackatime", count("htme_linked_at", "\"user\" WHERE htme_linked_at IS NOT NULL")),
        ("funnel_project", first("created_at", "user_id", "project WHERE deleted_at IS NULL")),
        (
            "funnel_time",
            first(
                "created_at",
                "user_id",
                "project WHERE deleted_at IS NULL AND time_logged > 0",
            ),
        ),
        (
            "funnel_ship",
            first(
                "ship.created_at",
                "project.user_id",
                "ship JOIN project ON project.id = ship.project_id WHERE ship.deleted_at IS NULL",
            ),
        ),
        ("funnel_prize", first("created_at", "user_id", orders)),
    ]
}

fn breakdowns(format: &str) -> [(&'static str, String); 4] {
    let day = |column: &str| bucket(column, format);
    let ledger = |filter: &str, sign: &str| {
        format!(
            "SELECT {}, kind, sum({sign}amount)::float8
             FROM resource_ledger, LATERAL (VALUES {}) AS granted (kind, amount)
             WHERE {filter} GROUP BY 1, 2",
            day("created_at"),
            resources()
        )
    };
    [
        (
            "ships_by_ring",
            format!(
                "SELECT {}, 'Ring ' || ring, count(*)::float8 FROM ship WHERE deleted_at IS NULL GROUP BY 1, 2",
                day("created_at")
            ),
        ),
        ("resources", ledger("amount > 0", "")),
        ("spent", ledger("amount < 0", "-")),
        (
            "countries",
            format!(
                "SELECT {}, coalesce(nullif(upper(trim(addr_country)), ''), 'unknown'), count(*)::float8
                 FROM \"user\" WHERE deleted_at IS NULL GROUP BY 1, 2",
                day("created_at")
            ),
        ),
    ]
}

async fn buckets(pool: &PgPool, format: &str) -> sqlx::Result<Value> {
    let hourly = format == HOUR;
    let (series_values, breakdown_values) = try_join(
        try_join_all(
            queries(format)
                .into_iter()
                .filter(|(key, _)| !hourly || !DAILY_ONLY.contains(key))
                .map(|(key, sql)| async move {
                    let sql = if hourly { recent(&sql, "key, value") } else { sql };
                    Ok::<_, sqlx::Error>((key.to_string(), series(pool, &sql).await?))
                }),
        ),
        try_join_all(breakdowns(format).into_iter().map(|(key, sql)| async move {
            let sql = if hourly { recent(&sql, "key, category, value") } else { sql };
            Ok::<_, sqlx::Error>((key.to_string(), breakdown(pool, &sql).await?))
        })),
    )
    .await?;
    Ok(json!({
        "series": series_values.into_iter().collect::<Map<_, _>>(),
        "breakdowns": breakdown_values.into_iter().collect::<Map<_, _>>(),
    }))
}

async fn summary(pool: &PgPool) -> sqlx::Result<Value> {
    let held = format!(
        "SELECT kind, sum(amount)::float8
         FROM resource_ledger, LATERAL (VALUES {}) AS held (kind, amount) GROUP BY kind",
        resources()
    );
    let chat = format!(
        "SELECT {}, count(*)::float8 FROM message GROUP BY 1",
        bucket("created_at", HOUR)
    );
    let (mut daily, hourly, (guilds, held, hours_building), (chat, today, hour)) = try_join4(
        buckets(pool, DAY),
        buckets(pool, HOUR),
        try_join3(
            sqlx::query_as::<_, (String, f64)>(
                "SELECT coalesce(guild.name, 'No guild'), count(*)::float8
                 FROM \"user\" LEFT JOIN guild ON guild.id = \"user\".guild_id
                 WHERE \"user\".deleted_at IS NULL
                 GROUP BY guild.id, guild.name ORDER BY guild.id NULLS LAST",
            )
            .fetch_all(pool),
            sqlx::query_as::<_, (String, f64)>(AssertSqlSafe(held)).fetch_all(pool),
            sqlx::query_scalar::<_, f64>(
                "SELECT (coalesce(sum(time_logged), 0) / 3600.0)::float8 FROM project WHERE deleted_at IS NULL",
            )
            .fetch_one(pool),
        ),
        try_join3(
            series(pool, &chat),
            sqlx::query_scalar::<_, String>(AssertSqlSafe(format!("SELECT {}", bucket("now()", DAY)))).fetch_one(pool),
            sqlx::query_scalar::<_, String>(AssertSqlSafe(format!("SELECT {}", bucket("now()", HOUR)))).fetch_one(pool),
        ),
    )
    .await?;
    Ok(json!({
        "today": today,
        "hour": hour,
        "series": daily["series"].take(),
        "breakdowns": daily["breakdowns"].take(),
        "hourly": hourly,
        "activity": { "chat": chat, "coding": {} },
        "guilds": guilds.into_iter().map(|(name, value)| json!({ "name": name, "value": value })).collect::<Vec<_>>(),
        "held": held.into_iter().map(|(name, value)| (name, json!(value))).collect::<Map<_, _>>(),
        "hours_building": hours_building,
    }))
}

#[get("/api/admin/analytics")]
pub async fn dashboard(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, admin::Permission::Analytics).await {
        return response;
    }
    match summary(&pool).await {
        Ok(data) => HttpResponse::Ok().json(data),
        Err(e) => {
            eprintln!("loading analytics failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
