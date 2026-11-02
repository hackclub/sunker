use actix_web::{HttpRequest, HttpResponse, get, web};
use hc_helper::auth::data::get_auth_data_with_token;
use hc_helper::auth::login;
use serde_json::{Map, Value, json};
use sqlx::PgPool;

pub const PERMISSIONS: [&str; 11] = [
    "analytics",
    "users",
    "ban",
    "resources",
    "chats",
    "moderation",
    "administration",
    "review",
    "joel",
    "mines",
    "video",
];

#[derive(Clone, Copy)]
pub enum Permission {
    Analytics,
    Users,
    Ban,
    Resources,
    Chats,
    Moderation,
    Administration,
    Joel = 8,
    Mines,
    Video,
}

pub struct Admin {
    pub id: i32,
    pub superadmin: bool,
    pub permissions: [bool; PERMISSIONS.len()],
}

impl Admin {
    pub fn can(&self, permission: Permission) -> bool {
        self.permissions[permission as usize]
    }

    pub fn json(&self) -> Value {
        json!({ "superadmin": self.superadmin, "permissions": permissions_json(&self.permissions) })
    }
}

pub fn permissions_json(permissions: &[bool]) -> Value {
    let map: Map<String, Value> = PERMISSIONS
        .iter()
        .zip(permissions)
        .map(|(name, value)| (name.to_string(), Value::Bool(*value)))
        .collect();
    Value::Object(map)
}

pub fn superadmins() -> Vec<String> {
    std::env::var("SUPERADMINS")
        .unwrap_or_default()
        .split(',')
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

async fn current(req: &HttpRequest, pool: &PgPool) -> Result<Admin, HttpResponse> {
    let Some(token) = login::get_auth_token_with_handling(req) else {
        return Err(HttpResponse::Unauthorized().finish());
    };
    let Ok(data) = get_auth_data_with_token(&token).await else {
        return Err(HttpResponse::Unauthorized().finish());
    };
    let columns: Vec<String> = PERMISSIONS
        .iter()
        .map(|name| format!("coalesce(p.{name}, false)"))
        .collect();
    let row: Option<(i32, Vec<bool>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT u.id, ARRAY[{}] FROM \"user\" u LEFT JOIN permission p ON p.user_id = u.id
         WHERE u.hca_id = $1 AND u.deleted_at IS NULL",
        columns.join(", ")
    )))
    .bind(&data.id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("loading admin permissions failed: {e}");
        HttpResponse::InternalServerError().finish()
    })?;
    let Some((id, granted)) = row else {
        return Err(HttpResponse::Forbidden().finish());
    };
    let superadmin = superadmins().contains(&data.id);
    let mut permissions = [superadmin; PERMISSIONS.len()];
    for (permission, value) in permissions.iter_mut().zip(granted) {
        *permission |= value;
    }
    Ok(Admin {
        id,
        superadmin,
        permissions,
    })
}

pub async fn require(
    req: &HttpRequest,
    pool: &PgPool,
    permission: Permission,
) -> Result<Admin, HttpResponse> {
    let admin = current(req, pool).await?;
    if admin.can(permission) {
        Ok(admin)
    } else {
        Err(HttpResponse::Forbidden().finish())
    }
}

#[get("/api/admin")]
pub async fn check(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    match current(&req, &pool).await {
        Ok(admin) if admin.permissions.contains(&true) => HttpResponse::Ok().json(admin.json()),
        Ok(_) => HttpResponse::Forbidden().finish(),
        Err(response) => response,
    }
}
