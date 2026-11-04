use crate::analytics;
use actix_web::cookie::Cookie;
use actix_web::{HttpRequest, HttpResponse, get, web};
use chrono::NaiveDate;
use hc_helper::auth::data::{forget_auth_token, get_auth_data_with_token};
use hc_helper::auth::login::{self, CallbackArgs, Scopes, TOKEN_COOKIE};
use hc_helper::hackatime;
use sqlx::PgPool;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

const CALLBACK: &str = "/callback";
const REFERRAL_COOKIE: &str = "sunker_ref";

pub fn site_url(path: &str) -> String {
    let base_url = hc_helper::keys::base_url();
    let host = match base_url.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => host,
        _ => &base_url,
    };
    let scheme = match host {
        "localhost" | "127.0.0.1" | "[::1]" => "http",
        _ => "https",
    };
    format!("{scheme}://{base_url}{path}")
}

pub async fn user_id(req: &HttpRequest, pool: &PgPool) -> sqlx::Result<Option<i32>> {
    let Some(token) = login::get_auth_token_with_handling(req) else {
        return Ok(None);
    };
    let Ok(data) = get_auth_data_with_token(&token).await else {
        return Ok(None);
    };
    sqlx::query_scalar(
        "WITH found AS (SELECT id, last_seen_at FROM \"user\" WHERE hca_id = $1 AND deleted_at IS NULL),
              seen AS (
                  UPDATE \"user\" SET last_seen_at = now() FROM found
                  WHERE \"user\".id = found.id AND (found.last_seen_at IS NULL OR found.last_seen_at < now() - interval '30 seconds')
              )
         SELECT id FROM found",
    )
    .bind(&data.id)
    .fetch_optional(pool)
    .await
}

async fn save_user(pool: &PgPool, token: &str, referral: Option<&str>) -> anyhow::Result<()> {
    let data = get_auth_data_with_token(token).await?;
    let address = data
        .primary_address()
        .or_else(|| data.addresses.first().cloned())
        .unwrap_or_default();
    let birthday = NaiveDate::parse_from_str(data.birthday.trim(), "%Y-%m-%d").ok();
    sqlx::query(
        "INSERT INTO \"user\" (
             hca_id, email, first_name, last_name, verification_status, ysws_eligible,
             birthday, slack_id, phone_number,
             addr_first_name, addr_last_name, addr_line_1, addr_line_2, addr_city,
             addr_state, addr_postcode, addr_country, addr_phone_number,
             allowed_chats, referred_by
         ) VALUES (
             $1, NULLIF($2, ''), NULLIF($3, ''), NULLIF($4, ''), NULLIF($5, ''), $6,
             $7, NULLIF($8, ''), NULLIF($9, ''),
             NULLIF($10, ''), NULLIF($11, ''), NULLIF($12, ''), NULLIF($13, ''), NULLIF($14, ''),
             NULLIF($15, ''), NULLIF($16, ''), NULLIF($17, ''), NULLIF($18, ''),
             ARRAY(SELECT id FROM chat WHERE slug = 'all'),
             (SELECT id FROM \"user\" WHERE referral_code = $19 AND deleted_at IS NULL)
         )
         ON CONFLICT (hca_id) DO UPDATE SET
             email = EXCLUDED.email,
             first_name = EXCLUDED.first_name,
             last_name = EXCLUDED.last_name,
             verification_status = EXCLUDED.verification_status,
             ysws_eligible = EXCLUDED.ysws_eligible,
             birthday = EXCLUDED.birthday,
             slack_id = EXCLUDED.slack_id,
             phone_number = EXCLUDED.phone_number,
             addr_first_name = EXCLUDED.addr_first_name,
             addr_last_name = EXCLUDED.addr_last_name,
             addr_line_1 = EXCLUDED.addr_line_1,
             addr_line_2 = EXCLUDED.addr_line_2,
             addr_city = EXCLUDED.addr_city,
             addr_state = EXCLUDED.addr_state,
             addr_postcode = EXCLUDED.addr_postcode,
             addr_country = EXCLUDED.addr_country,
             addr_phone_number = EXCLUDED.addr_phone_number",
    )
    .bind(&data.id)
    .bind(&data.primary_email)
    .bind(&data.first_name)
    .bind(&data.last_name)
    .bind(&data.verification_status)
    .bind(data.ysws_eligible)
    .bind(birthday)
    .bind(&data.slack_id)
    .bind(&data.phone_number)
    .bind(&address.first_name)
    .bind(&address.last_name)
    .bind(&address.line_1)
    .bind(&address.line_2)
    .bind(&address.city)
    .bind(&address.state)
    .bind(&address.postal_code)
    .bind(&address.country)
    .bind(&address.phone_number)
    .bind(referral)
    .execute(pool)
    .await?;
    Ok(())
}

#[get("/login")]
pub async fn start(req: HttpRequest, query: web::Query<HashMap<String, String>>) -> HttpResponse {
    let scopes = Scopes {
        openid: false,
        profile: false,
        email: false,
        name: false,
        slack_id: true,
        verification_status: true,
        basic_info: true,
        addresses: true,
        phone_number: true,
    };
    let mut response = login::handle_login(&req, scopes, site_url(CALLBACK)).await;
    if let Some(code) = query.into_inner().remove("ref").filter(|code| !code.is_empty() && code.len() <= 32) {
        let cookie = Cookie::build(REFERRAL_COOKIE, code)
            .path("/")
            .http_only(true)
            .max_age(actix_web::cookie::time::Duration::hours(1))
            .finish();
        let _ = response.add_cookie(&cookie);
    }
    response
}

#[get("/callback")]
pub async fn callback(
    req: HttpRequest,
    query: web::Query<CallbackArgs>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let token = Rc::new(Cell::new(None));
    let sink = token.clone();
    let response = login::handle_callback(
        &req,
        query,
        site_url(CALLBACK),
        "/hq".to_string(),
        Some(async move |value| sink.set(Some(value))),
        true,
    )
    .await;
    let Some(token) = token.take() else {
        return response;
    };
    let referral = req.cookie(REFERRAL_COOKIE);
    match save_user(&pool, &token, referral.as_ref().map(Cookie::value)).await {
        Ok(()) => {
            let mut response = response;
            if referral.is_some() {
                let _ = response.add_removal_cookie(&Cookie::build(REFERRAL_COOKIE, "").path("/").finish());
            }
            response
        }
        Err(e) => {
            eprintln!("saving user failed: {e:#}");
            HttpResponse::InternalServerError().body("Couldn't sign you in, please try again")
        }
    }
}

#[get("/api/session")]
pub async fn session(req: HttpRequest, pool: web::Data<PgPool>) -> HttpResponse {
    if login::get_auth_token_with_handling(&req).is_none() {
        return HttpResponse::Unauthorized().finish();
    }
    if let Err(e) = analytics::record_login(&req, &pool).await {
        eprintln!("recording login failed: {e}");
    }
    HttpResponse::NoContent().finish()
}

#[get("/logout")]
pub async fn logout(req: HttpRequest) -> HttpResponse {
    if let Some(token) = login::get_auth_token_with_handling(&req) {
        forget_auth_token(&token);
    }
    let mut response = HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish();
    let _ = response.add_removal_cookie(&Cookie::new(TOKEN_COOKIE, ""));
    let _ = response.add_removal_cookie(&Cookie::new(hackatime::login::TOKEN_COOKIE, ""));
    response
}
