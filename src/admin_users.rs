use crate::admin;
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};

const PAGE: i64 = 50;
const RALLY_SCRIP: i32 = 10;

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

const SLACK_PREFERENCES: [&str; 5] = [
    "enabled",
    "announcements",
    "events",
    "ship_updates",
    "inactivity_reminders",
];

const PRIVATE: &str = "ARRAY['birthday', 'phone_number', 'addr_first_name', 'addr_last_name', 'addr_line_1',
    'addr_line_2', 'addr_city', 'addr_state', 'addr_postcode', 'addr_country', 'addr_phone_number']";

fn failed(what: &str, e: sqlx::Error) -> HttpResponse {
    eprintln!("{what} failed: {e}");
    HttpResponse::InternalServerError().finish()
}

#[get("/api/admin/users")]
pub async fn list(
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, admin::Permission::Users).await {
        return response;
    }
    let param = |key: &str| {
        query
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.trim().to_string())
    };
    let search = param("q").unwrap_or_default();
    let pattern = format!(
        "%{}%",
        search
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let before: Option<i32> = param("before").and_then(|value| value.parse().ok());
    let rows: Result<Vec<Value>, _> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
             'id', u.id,
             'name', nullif(concat_ws(' ', u.first_name, u.last_name), ''),
             'email', u.email,
             'slack_id', u.slack_id,
             'handle', c.handle,
             'guild', g.name,
             'scrip', r.scrip,
             'projects', (SELECT count(*) FROM project p WHERE p.user_id = u.id AND p.deleted_at IS NULL),
             'ships', (SELECT count(*) FROM ship s JOIN project p ON p.id = s.project_id
                       WHERE p.user_id = u.id AND s.deleted_at IS NULL),
             'last_seen', (SELECT max(created_at) FROM analytics a WHERE a.user_id = u.id),
             'is_banned', u.is_banned,
             'deleted_at', u.deleted_at,
             'created_at', u.created_at
         )
         FROM \"user\" u
         LEFT JOIN character c ON c.user_id = u.id
         LEFT JOIN guild g ON g.id = u.guild_id
         LEFT JOIN resources r ON r.user_id = u.id
         WHERE ($1 = '' OR u.id::text = $1 OR u.hca_id ILIKE $2 OR u.htme_id::text = $1
                OR u.email ILIKE $2 OR concat_ws(' ', u.first_name, u.last_name) ILIKE $2
                OR u.slack_id ILIKE $2 OR u.referral_code ILIKE $2 OR c.handle ILIKE $2)
           AND ($3::int IS NULL OR u.id < $3)
         ORDER BY u.id DESC
         LIMIT $4",
    )
    .bind(&search)
    .bind(&pattern)
    .bind(before)
    .bind(PAGE)
    .fetch_all(pool.get_ref())
    .await;
    match rows {
        Ok(rows) => HttpResponse::Ok().json(json!({ "users": rows, "more": rows.len() as i64 == PAGE })),
        Err(e) => failed("listing users", e),
    }
}

async fn detail(pool: &PgPool, id: i32) -> sqlx::Result<Option<Value>> {
    let person = "jsonb_build_object('id', x.id, 'name', nullif(concat_ws(' ', x.first_name, x.last_name), ''))";
    let sql = format!(
        "SELECT jsonb_build_object(
             'user', to_jsonb(u) - {PRIVATE},
             'guild', (SELECT to_jsonb(g) FROM guild g WHERE g.id = u.guild_id),
             'referrer', (SELECT {person} FROM \"user\" x WHERE x.id = u.referred_by),
             'referrals', (SELECT coalesce(jsonb_agg({person} ORDER BY x.id), '[]') FROM \"user\" x WHERE x.referred_by = u.id),
             'character', (SELECT to_jsonb(c) FROM character c WHERE c.user_id = u.id),
             'resources', (SELECT to_jsonb(r) - 'user_id' FROM resources r WHERE r.user_id = u.id),
             'ledger', (SELECT coalesce(jsonb_agg(to_jsonb(l) - 'user_id' || jsonb_build_object('admin', (SELECT {person} FROM \"user\" x WHERE x.id = l.admin_id)) ORDER BY l.created_at DESC), '[]')
                        FROM resource_ledger l WHERE l.user_id = u.id),
             'ledger_by', (SELECT coalesce(jsonb_agg(to_jsonb(l) - 'admin_id' || jsonb_build_object('user', (SELECT {person} FROM \"user\" x WHERE x.id = l.user_id)) ORDER BY l.created_at DESC), '[]')
                           FROM resource_ledger l WHERE l.admin_id = u.id),
             'projects', (SELECT coalesce(jsonb_agg(to_jsonb(p) - 'user_id' || jsonb_build_object('contract_name', e.name, 'ring', e.ring) ORDER BY p.created_at DESC), '[]')
                          FROM project p JOIN contract e ON e.slug = p.contract_slug WHERE p.user_id = u.id),
             'ships', (SELECT coalesce(jsonb_agg(to_jsonb(s) || jsonb_build_object('project_name', p.name, 'reviewer', (SELECT {person} FROM \"user\" x WHERE x.id = s.reviewer_id)) ORDER BY s.created_at DESC), '[]')
                       FROM ship s JOIN project p ON p.id = s.project_id WHERE p.user_id = u.id),
             'reviewed_ships', (SELECT coalesce(jsonb_agg(to_jsonb(s) || jsonb_build_object('project_name', p.name, 'owner', (SELECT {person} FROM \"user\" x WHERE x.id = p.user_id)) ORDER BY s.decided_at DESC NULLS LAST), '[]')
                                FROM ship s JOIN project p ON p.id = s.project_id WHERE s.reviewer_id = u.id),
             'orders', (SELECT coalesce(jsonb_agg(to_jsonb(o) - 'user_id' - {PRIVATE} || jsonb_build_object('prize_name', z.name, 'prize_ring', z.ring, 'base_cost_cents', z.base_cost_cents) ORDER BY o.created_at DESC), '[]')
                        FROM \"order\" o JOIN prize z ON z.id = o.prize_id WHERE o.user_id = u.id),
             'chats', (SELECT coalesce(jsonb_agg(jsonb_build_object('id', ch.id, 'name', ch.name, 'allowed', ch.id = ANY(u.allowed_chats)) ORDER BY ch.id), '[]') FROM chat ch WHERE ch.user_id IS NULL),
             'guilds', (SELECT coalesce(jsonb_agg(jsonb_build_object('id', g.id, 'name', g.name) ORDER BY g.id), '[]') FROM guild g),
             'message_count', (SELECT count(*) FROM message m WHERE m.author_id = u.id),
             'messages', (SELECT coalesce(jsonb_agg(recent ORDER BY recent.created_at DESC), '[]') FROM (
                              SELECT m.id, m.contents, m.created_at, m.deleted_at, ch.name AS chat,
                                     (SELECT {person} FROM \"user\" x WHERE x.id = m.deleted_by) AS deleted_by
                              FROM message m JOIN chat ch ON ch.id = m.chat_id
                              WHERE m.author_id = u.id ORDER BY m.created_at DESC LIMIT 100) recent),
             'analytics_counts', (SELECT coalesce(jsonb_object_agg(kind, total), '{{}}') FROM (
                                      SELECT kind, count(*) AS total FROM analytics a WHERE a.user_id = u.id GROUP BY kind) counts),
             'analytics', (SELECT coalesce(jsonb_agg(recent ORDER BY recent.created_at DESC), '[]') FROM (
                               SELECT a.id, a.kind, a.created_at FROM analytics a
                               WHERE a.user_id = u.id ORDER BY a.created_at DESC LIMIT 50) recent),
             'admin_actions', (SELECT coalesce(jsonb_agg(to_jsonb(aa) - 'user_id' || jsonb_build_object('admin', (SELECT {person} FROM \"user\" x WHERE x.id = aa.admin_id)) ORDER BY aa.created_at DESC), '[]')
                               FROM admin_action aa WHERE aa.user_id = u.id),
             'admin_actions_by', (SELECT coalesce(jsonb_agg(to_jsonb(aa) - 'admin_id' || jsonb_build_object('user', (SELECT {person} FROM \"user\" x WHERE x.id = aa.user_id)) ORDER BY aa.created_at DESC), '[]')
                                  FROM admin_action aa WHERE aa.admin_id = u.id),
             'slack_preference', (SELECT to_jsonb(sp) - 'user_id' FROM slack_preference sp WHERE sp.user_id = u.id),
             'slack_dms', (SELECT coalesce(jsonb_agg(to_jsonb(d) - 'user_id' ORDER BY d.created_at DESC), '[]') FROM slack_dm d WHERE d.user_id = u.id),
             'feedback', (SELECT coalesce(jsonb_agg(to_jsonb(f) - 'user_id' ORDER BY f.created_at DESC), '[]') FROM feedback f WHERE f.user_id = u.id),
             'sessions', (SELECT coalesce(jsonb_agg(jsonb_build_object('id', s.id, 'created_at', s.created_at, 'expires_at', s.expires_at) ORDER BY s.created_at DESC), '[]')
                          FROM session s WHERE s.user_id = u.id),
             'auth_tokens', (SELECT coalesce(jsonb_agg(jsonb_build_object('provider', t.provider, 'updated_at', t.updated_at) ORDER BY t.provider), '[]')
                             FROM auth_token t WHERE t.user_id = u.id)
         )
         FROM \"user\" u WHERE u.id = $1"
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[get("/api/admin/users/{id:\\d+}")]
pub async fn show(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, admin::Permission::Users).await {
        return response;
    }
    match detail(&pool, *id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => failed("loading user", e),
    }
}

#[get("/api/admin/users/{id:\\d+}/private")]
pub async fn private(req: HttpRequest, id: web::Path<i32>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Err(response) = admin::require(&req, &pool, admin::Permission::Users).await {
        return response;
    }
    let sql = format!(
        "SELECT jsonb_build_object(
             'user', (SELECT jsonb_object_agg(key, value) FROM jsonb_each(to_jsonb(u)) WHERE key = ANY({PRIVATE})),
             'orders', (SELECT coalesce(jsonb_object_agg(o.id, (SELECT jsonb_object_agg(key, value) FROM jsonb_each(to_jsonb(o)) WHERE key = ANY({PRIVATE}))), '{{}}')
                        FROM \"order\" o WHERE o.user_id = u.id)
         )
         FROM \"user\" u WHERE u.id = $1"
    );
    let data: Result<Option<Value>, _> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(*id)
        .fetch_optional(pool.get_ref())
        .await;
    match data {
        Ok(Some(data)) => HttpResponse::Ok().json(data),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => failed("loading private user fields", e),
    }
}

enum Failure {
    Bad(String),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for Failure {
    fn from(e: sqlx::Error) -> Self {
        let code = e
            .as_database_error()
            .and_then(|db| db.code())
            .map(|code| code.to_string());
        match code.as_deref() {
            Some("23514") => Failure::Bad("That would break a check, e.g. a negative balance".into()),
            Some("23505") => Failure::Bad("That value is already taken".into()),
            Some("23503") => Failure::Bad("That refers to something that doesn't exist".into()),
            _ => Failure::Database(e),
        }
    }
}

fn bad(message: &str) -> Failure {
    Failure::Bad(message.into())
}

fn affected(result: sqlx::postgres::PgQueryResult, what: &str) -> Result<(), Failure> {
    if result.rows_affected() == 0 {
        Err(bad(&format!("{what} not found for this user")))
    } else {
        Ok(())
    }
}

fn number(body: &Value, key: &str) -> Result<i32, Failure> {
    body[key]
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| bad(&format!("missing {key}")))
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    user: i32,
    admin: i32,
    body: &Value,
) -> Result<(String, String), Failure> {
    let kind = body["kind"].as_str().unwrap_or_default();
    let summary = match kind {
        "ban" | "unban" => {
            let banned = kind == "ban";
            if banned && user == admin {
                return Err(bad("You can't ban yourself"));
            }
            let result = sqlx::query("UPDATE \"user\" SET is_banned = $2 WHERE id = $1")
                .bind(user)
                .bind(banned)
                .execute(&mut **tx)
                .await?;
            affected(result, "User")?;
            if banned { "Banned user" } else { "Unbanned user" }.to_string()
        }
        "delete" | "restore" => {
            let result = sqlx::query(
                "UPDATE \"user\" SET deleted_at = CASE WHEN $2 THEN now() END WHERE id = $1",
            )
            .bind(user)
            .bind(kind == "delete")
            .execute(&mut **tx)
            .await?;
            affected(result, "User")?;
            if kind == "delete" { "Soft deleted user" } else { "Restored user" }.to_string()
        }
        "guild" => {
            let guild = body["guild_id"].as_i64().map(|id| id as i32);
            let result = sqlx::query("UPDATE \"user\" SET guild_id = $2 WHERE id = $1")
                .bind(user)
                .bind(guild)
                .execute(&mut **tx)
                .await?;
            affected(result, "User")?;
            let name: Option<String> = sqlx::query_scalar("SELECT name FROM guild WHERE id = $1")
                .bind(guild)
                .fetch_optional(&mut **tx)
                .await?;
            format!("Set guild to {}", name.unwrap_or_else(|| "none".into()))
        }
        "chats" => {
            let chats: Vec<i32> = body["chat_ids"]
                .as_array()
                .ok_or_else(|| bad("missing chat_ids"))?
                .iter()
                .filter_map(|id| id.as_i64().map(|id| id as i32))
                .collect();
            let result = sqlx::query(
                "UPDATE \"user\" SET allowed_chats = ARRAY(SELECT id FROM chat WHERE id = ANY($2) AND user_id IS NULL ORDER BY id)
                 WHERE id = $1",
            )
            .bind(user)
            .bind(&chats)
            .execute(&mut **tx)
            .await?;
            affected(result, "User")?;
            let names: Vec<String> =
                sqlx::query_scalar("SELECT name FROM chat WHERE id = ANY($1) AND user_id IS NULL ORDER BY id")
                    .bind(&chats)
                    .fetch_all(&mut **tx)
                    .await?;
            format!("Set allowed chats to [{}]", names.join(", "))
        }
        "handle" => {
            let handle = body["handle"].as_str().unwrap_or_default().trim();
            if handle.is_empty() {
                return Err(bad("handle can't be empty"));
            }
            let old: Option<String> = sqlx::query_scalar(
                "UPDATE character c SET handle = $2 FROM character old
                 WHERE c.user_id = $1 AND old.user_id = c.user_id RETURNING old.handle",
            )
            .bind(user)
            .bind(handle)
            .fetch_optional(&mut **tx)
            .await?;
            let old = old.ok_or_else(|| bad("Character not found for this user"))?;
            format!("Renamed character from {old} to {handle}")
        }
        "resources" => {
            let deltas: Vec<i32> = RESOURCES
                .iter()
                .map(|name| body["deltas"][name].as_i64().unwrap_or(0) as i32)
                .collect();
            if deltas.iter().all(|delta| *delta == 0) {
                return Err(bad("nothing to change"));
            }
            let reason = body["reason"].as_str().unwrap_or_default().trim();
            if reason.is_empty() {
                return Err(bad("a reason is needed for the ledger"));
            }
            let sets = RESOURCES
                .iter()
                .enumerate()
                .map(|(i, name)| format!("{name} = {name} + ${}", i + 2))
                .collect::<Vec<_>>()
                .join(", ");
            let mut update = sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE resources SET {sets} WHERE user_id = $1"
            )))
            .bind(user);
            for delta in &deltas {
                update = update.bind(*delta);
            }
            affected(update.execute(&mut **tx).await?, "Resources")?;
            let mut insert = sqlx::query(sqlx::AssertSqlSafe(format!(
                "INSERT INTO resource_ledger (user_id, admin_id, reason, {}) VALUES ($1, $2, $3, {})",
                RESOURCES.join(", "),
                (4..4 + RESOURCES.len())
                    .map(|i| format!("${i}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
            .bind(user)
            .bind(admin)
            .bind(reason);
            for delta in &deltas {
                insert = insert.bind(*delta);
            }
            insert.execute(&mut **tx).await?;
            let changes = RESOURCES
                .iter()
                .zip(&deltas)
                .filter(|(_, delta)| **delta != 0)
                .map(|(name, delta)| format!("{name} {delta:+}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Adjusted resources: {changes}")
        }
        "project_delete" | "project_restore" => {
            let project = number(body, "project_id")?;
            let result = sqlx::query(
                "UPDATE project SET deleted_at = CASE WHEN $3 THEN now() END WHERE id = $2 AND user_id = $1",
            )
            .bind(user)
            .bind(project)
            .bind(kind == "project_delete")
            .execute(&mut **tx)
            .await?;
            affected(result, "Project")?;
            let verb = if kind == "project_delete" { "Soft deleted" } else { "Restored" };
            format!("{verb} project #{project}")
        }
        "ship_decide" => {
            let ship = number(body, "ship_id")?;
            let status = body["status"].as_str().unwrap_or_default();
            if !["pending", "approved", "rejected"].contains(&status) {
                return Err(bad("status must be pending, approved or rejected"));
            }
            let awarded = body["awarded_ring"].as_i64().map(|ring| ring as i16);
            let approved = body["approved_seconds"].as_i64();
            let rejection = body["rejection_reason"]
                .as_str()
                .map(str::trim)
                .filter(|reason| !reason.is_empty());
            let result = sqlx::query(
                "UPDATE ship SET
                     status = $3,
                     awarded_ring = CASE WHEN $3 = 'approved' THEN coalesce($4, ring) END,
                     approved_seconds = CASE WHEN $3 = 'approved' THEN coalesce($5, tracked_seconds) END,
                     rejection_reason = CASE WHEN $3 = 'rejected' THEN $6 END,
                     reviewer_id = CASE WHEN $3 = 'pending' THEN NULL ELSE $7 END,
                     decided_at = CASE WHEN $3 = 'pending' THEN NULL ELSE now() END
                 WHERE id = $2 AND project_id IN (SELECT id FROM project WHERE user_id = $1)",
            )
            .bind(user)
            .bind(ship)
            .bind(status)
            .bind(awarded)
            .bind(approved)
            .bind(rejection)
            .bind(admin)
            .execute(&mut **tx)
            .await?;
            affected(result, "Ship")?;
            if status == "approved" {
                sqlx::query(
                    "WITH due AS (
                         SELECT u.referred_by AS id, 'Rallied user #' || u.id AS reason FROM \"user\" u
                         WHERE u.id = $1 AND u.referred_by IS NOT NULL
                           AND NOT EXISTS (SELECT 1 FROM resource_ledger l WHERE l.user_id = u.referred_by AND l.reason = 'Rallied user #' || u.id)
                     ), granted AS (
                         UPDATE resources r SET scrip = scrip + $2 FROM due WHERE r.user_id = due.id
                     )
                     INSERT INTO resource_ledger (user_id, reason, scrip) SELECT id, reason, $2 FROM due",
                )
                .bind(user)
                .bind(RALLY_SCRIP)
                .execute(&mut **tx)
                .await?;
            }
            format!("Set ship #{ship} to {status}")
        }
        "ship_delete" | "ship_restore" => {
            let ship = number(body, "ship_id")?;
            let result = sqlx::query(
                "UPDATE ship SET deleted_at = CASE WHEN $3 THEN now() END
                 WHERE id = $2 AND project_id IN (SELECT id FROM project WHERE user_id = $1)",
            )
            .bind(user)
            .bind(ship)
            .bind(kind == "ship_delete")
            .execute(&mut **tx)
            .await?;
            affected(result, "Ship")?;
            let verb = if kind == "ship_delete" { "Soft deleted" } else { "Restored" };
            format!("{verb} ship #{ship}")
        }
        "order_status" => {
            let order = number(body, "order_id")?;
            let status = body["status"].as_str().unwrap_or_default();
            if !crate::orders::STATUSES.contains(&status) {
                return Err(bad("status must be pending, rejected, ordered or fulfilled"));
            }
            let tracking = body["tracking_number"]
                .as_str()
                .map(str::trim)
                .filter(|number| !number.is_empty());
            let result = sqlx::query(
                "UPDATE \"order\" SET
                     status = $3,
                     fulfilled_at = CASE WHEN $3 = 'fulfilled' THEN coalesce(fulfilled_at, now()) END,
                     tracking_number = coalesce($4, tracking_number)
                 WHERE id = $2 AND user_id = $1",
            )
            .bind(user)
            .bind(order)
            .bind(status)
            .bind(tracking)
            .execute(&mut **tx)
            .await?;
            affected(result, "Order")?;
            format!("Set order #{order} to {status}")
        }
        "message_delete" | "message_restore" => {
            let message = body["message_id"].as_i64().ok_or_else(|| bad("missing message_id"))?;
            let delete = kind == "message_delete";
            let result = sqlx::query(
                "UPDATE message SET
                     deleted_at = CASE WHEN $3 THEN now() END,
                     deleted_by = CASE WHEN $3 THEN $4 END
                 WHERE id = $2 AND author_id = $1",
            )
            .bind(user)
            .bind(message)
            .bind(delete)
            .bind(admin)
            .execute(&mut **tx)
            .await?;
            affected(result, "Message")?;
            let verb = if delete { "Deleted" } else { "Restored" };
            format!("{verb} message #{message}")
        }
        "slack_preference" => {
            let field = body["field"].as_str().unwrap_or_default();
            let Some(field) = SLACK_PREFERENCES.iter().find(|name| **name == field) else {
                return Err(bad("unknown Slack preference"));
            };
            let value = body["value"].as_bool().ok_or_else(|| bad("missing value"))?;
            let result = sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE slack_preference SET {field} = $2 WHERE user_id = $1"
            )))
            .bind(user)
            .bind(value)
            .execute(&mut **tx)
            .await?;
            affected(result, "Slack preference")?;
            format!("Set Slack preference {field} to {value}")
        }
        _ => return Err(bad("unknown action")),
    };
    Ok((kind.to_string(), summary))
}

#[post("/api/admin/users/{id:\\d+}/action")]
pub async fn action(
    req: HttpRequest,
    id: web::Path<i32>,
    body: web::Json<Value>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let admin = match admin::require(&req, &pool, admin::Permission::Users).await {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let needed = match body["kind"].as_str().unwrap_or_default() {
        "ban" | "unban" | "delete" | "restore" => Some(admin::Permission::Ban),
        "resources" | "order_status" => Some(admin::Permission::Resources),
        "guild" | "chats" | "slack_preference" => Some(admin::Permission::Chats),
        "handle" | "project_delete" | "project_restore" | "ship_decide" | "ship_delete" | "ship_restore"
        | "message_delete" | "message_restore" => Some(admin::Permission::Moderation),
        _ => None,
    };
    if needed.is_some_and(|permission| !admin.can(permission)) {
        return HttpResponse::Forbidden().body("You don't have permission for that action");
    }
    run(&pool, *id, admin.id, &body).await
}

pub async fn run(pool: &PgPool, user: i32, admin: i32, body: &Value) -> HttpResponse {
    let reason = body["reason"]
        .as_str()
        .map(str::trim)
        .filter(|reason| !reason.is_empty());
    let outcome = async {
        let mut tx = pool.begin().await?;
        let (kind, summary) = apply(&mut tx, user, admin, body).await?;
        sqlx::query(
            "INSERT INTO admin_action (admin_id, action, user_id, summary, reason) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(admin)
        .bind(&kind)
        .bind(user)
        .bind(&summary)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok::<_, Failure>(summary)
    }
    .await;
    match outcome {
        Ok(summary) => HttpResponse::Ok().json(json!({ "summary": summary })),
        Err(Failure::Bad(message)) => HttpResponse::BadRequest().body(message),
        Err(Failure::Database(e)) => failed("running user action", e),
    }
}
