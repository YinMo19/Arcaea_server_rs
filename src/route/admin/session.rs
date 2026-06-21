//! Web admin authentication: cookie/session management, login/logout and the
//! `require_*` guards used by every admin route handler.

use chrono::Utc;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::{get, post, State};
use sha2::{Digest, Sha256};
use std::env;
use std::sync::RwLock;

use crate::error::ArcError;
use crate::route::common::{success_return, success_return_no_value, EmptyResponse, RouteResult};
use crate::service::UserService;
use crate::DbPool;

use super::helpers::resolve_admin_user;
use super::models::{AdminLoginRequest, AdminSessionResponse, AdminUserSummary, WebLoginUserRow, WebSession};
use super::{AdminConfig, ADMIN_CONFIG, ADMIN_COOKIE, ADMIN_ROLE};

fn admin_credentials() -> (String, String) {
    let configured = ADMIN_CONFIG
        .get_or_init(|| RwLock::new(AdminConfig::default()))
        .read()
        .ok()
        .map(|guard| guard.clone())
        .unwrap_or_default();

    let username = env::var("ADMIN_USERNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(configured.username);
    let password = env::var("ADMIN_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or(configured.password);

    (username, password)
}

fn web_session_secret() -> String {
    env::var("WEB_SESSION_SECRET")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            let (username, password) = admin_credentials();
            format!("{username}:{password}")
        })
}

fn web_session_signature(user_id: i32, role: i8, password_hash: &str) -> String {
    let secret = web_session_secret();
    let joined = format!("{user_id}:{role}:{password_hash}:{secret}");
    format!("{:x}", Sha256::digest(joined.as_bytes()))
}

fn parse_web_session_cookie(value: &str) -> Option<(i32, i8, &str)> {
    let mut parts = value.splitn(3, ':');
    let user_id = parts.next()?.parse::<i32>().ok()?;
    let role = parts.next()?.parse::<i8>().ok()?;
    let signature = parts.next()?;
    Some((user_id, role, signature))
}

fn set_admin_cookie(cookies: &CookieJar<'_>, user_id: i32, role: i8, password_hash: &str) {
    let value = format!(
        "{user_id}:{role}:{}",
        web_session_signature(user_id, role, password_hash)
    );
    let mut cookie = Cookie::new(ADMIN_COOKIE, value);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/web");
    cookies.add(cookie);
}

fn clear_admin_cookie(cookies: &CookieJar<'_>) {
    let mut cookie = Cookie::from(ADMIN_COOKIE);
    cookie.set_path("/web");
    cookies.remove(cookie);
}

fn web_app_title() -> String {
    env::var("TITLE")
        .or_else(|_| env::var("title"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Arcaea Server".to_string())
}

fn web_login_background() -> Option<String> {
    env::var("LOGIN_BACKGROUND_URL")
        .or_else(|_| env::var("login_background_url"))
        .or_else(|_| env::var("LOGIN_BACKGROUND"))
        .or_else(|_| env::var("login_background"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn web_login_position() -> String {
    env::var("LOGIN_CARD_POSITION")
        .or_else(|_| env::var("login_card_position"))
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| matches!(value.as_str(), "left" | "center" | "right"))
        .unwrap_or_else(|| "center".to_string())
}

fn web_login_card_opacity() -> f64 {
    env::var("LOGIN_CARD_OPACITY")
        .or_else(|_| env::var("login_card_opacity"))
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

fn web_surface_opacity() -> f64 {
    env::var("WEB_SURFACE_OPACITY")
        .or_else(|_| env::var("web_surface_opacity"))
        .or_else(|_| env::var("MAIN_SURFACE_OPACITY"))
        .or_else(|_| env::var("main_surface_opacity"))
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) async fn current_web_session(
    cookies: &CookieJar<'_>,
    pool: &DbPool,
) -> Result<Option<WebSession>, ArcError> {
    let Some(cookie) = cookies.get(ADMIN_COOKIE) else {
        return Ok(None);
    };
    let Some((cookie_user_id, cookie_role, signature)) = parse_web_session_cookie(cookie.value())
    else {
        return Ok(None);
    };

    let Some(user) = load_web_login_user_by_id(pool, cookie_user_id).await? else {
        return Ok(None);
    };

    let user_role = user.web_role();
    let password_hash = user.password.as_deref().unwrap_or_default();
    if password_hash.is_empty() || cookie_role != user_role {
        return Ok(None);
    }
    let expected = web_session_signature(user.user_id, user_role, password_hash);
    if signature != expected {
        return Ok(None);
    }

    Ok(Some(WebSession {
        user: AdminUserSummary {
            user_id: user.user_id,
            name: user.name.unwrap_or_default(),
            user_code: user.user_code.unwrap_or_default(),
        },
        role: user_role,
    }))
}

pub(super) async fn require_web_session(
    cookies: &CookieJar<'_>,
    pool: &DbPool,
) -> Result<WebSession, ArcError> {
    current_web_session(cookies, pool)
        .await?
        .ok_or_else(web_unauthorized)
}

fn web_unauthorized() -> ArcError {
    ArcError::no_access("Login required", 401)
}

pub(super) async fn require_admin_api(
    cookies: &CookieJar<'_>,
    pool: &DbPool,
) -> Result<WebSession, ArcError> {
    let session = require_web_session(cookies, pool).await?;
    if session.role == ADMIN_ROLE {
        Ok(session)
    } else {
        Err(ArcError::no_access("Admin role required", 403))
    }
}

/// Resolve the target user for a score-image / user-score request: admins can
/// target anyone, regular users are pinned to their own account.
pub(super) async fn resolve_score_image_user(
    session: &WebSession,
    user_id: Option<i32>,
    name: Option<&str>,
    user_code: Option<&str>,
    pool: &DbPool,
) -> Result<AdminUserSummary, ArcError> {
    if session.role == ADMIN_ROLE && (user_id.is_some() || name.is_some() || user_code.is_some()) {
        resolve_admin_user(user_id, name, user_code, pool).await
    } else {
        Ok(session.user.clone())
    }
}

async fn load_web_login_user(
    pool: &DbPool,
    username: &str,
) -> Result<Option<WebLoginUserRow>, ArcError> {
    sqlx::query_as!(
        WebLoginUserRow,
        r#"
        SELECT
            u.user_id,
            u.name,
            u.user_code,
            u.password,
            u.ban_flag,
            CAST(
                CASE
                    WHEN EXISTS (
                        SELECT 1
                        FROM user_role ur
                        WHERE ur.user_id = u.user_id
                          AND ur.role_id IN ('admin', 'system')
                    )
                    THEN 1
                    ELSE 0
                END AS SIGNED
            ) AS `role!: i64`
        FROM user u
        WHERE u.name = ?
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ArcError::input(format!("登录查询失败: {err}")))
}

async fn load_web_login_user_by_id(
    pool: &DbPool,
    user_id: i32,
) -> Result<Option<WebLoginUserRow>, ArcError> {
    sqlx::query_as!(
        WebLoginUserRow,
        r#"
        SELECT
            u.user_id,
            u.name,
            u.user_code,
            u.password,
            u.ban_flag,
            CAST(
                CASE
                    WHEN EXISTS (
                        SELECT 1
                        FROM user_role ur
                        WHERE ur.user_id = u.user_id
                          AND ur.role_id IN ('admin', 'system')
                    )
                    THEN 1
                    ELSE 0
                END AS SIGNED
            ) AS `role!: i64`
        FROM user u
        WHERE u.user_id = ?
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询登录状态失败: {err}")))
}

/// First-time bootstrap: when no admin role exists yet, allow the configured
/// admin credentials to create the initial admin user.
async fn bootstrap_config_admin_user(
    pool: &DbPool,
    username: &str,
    password_hash: &str,
) -> Result<WebLoginUserRow, ArcError> {
    let admin_count = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT user_id) as `count!: i64` FROM user_role WHERE role_id IN ('admin', 'system')",
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询管理员用户失败: {err}")))?;
    if admin_count > 0 {
        return Err(ArcError::no_access("Incorrect username or password", 401));
    }

    let now = Utc::now().timestamp_millis();
    sqlx::query!(
        r#"
        INSERT INTO user (
            name, password, join_date, user_code, rating_ptt,
            character_id, is_skill_sealed, is_char_uncapped, is_char_uncapped_override,
            is_hide_rating, favorite_character, max_stamina_notification_enabled,
            current_map, ticket, prog_boost, email
        ) VALUES (?, ?, ?, '123456789', 0, 0, 0, 0, 0, 0, -1, 0, '', 0, 0, 'admin@admin.com')
        "#,
        username,
        password_hash,
        now
    )
    .execute(pool)
    .await
    .map_err(|err| ArcError::input(format!("创建管理员用户失败: {err}")))?;

    let user_id = sqlx::query_scalar!(
        "SELECT user_id as `user_id!: i32` FROM user WHERE name = ?",
        username
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询管理员用户失败: {err}")))?;

    sqlx::query!(
        "INSERT IGNORE INTO user_role (user_id, role_id) VALUES (?, 'admin')",
        user_id
    )
    .execute(pool)
    .await
    .map_err(|err| ArcError::input(format!("授予管理员角色失败: {err}")))?;

    load_web_login_user(pool, username)
        .await?
        .ok_or_else(|| ArcError::no_data("管理员用户不存在", -2))
}

fn web_session_response(user: &WebLoginUserRow) -> AdminSessionResponse {
    AdminSessionResponse {
        logged_in: true,
        role: user.web_role(),
        app_title: web_app_title(),
        login_background: web_login_background(),
        login_position: web_login_position(),
        login_card_opacity: web_login_card_opacity(),
        web_surface_opacity: web_surface_opacity(),
        user: Some(AdminUserSummary {
            user_id: user.user_id,
            name: user.name.clone().unwrap_or_default(),
            user_code: user.user_code.clone().unwrap_or_default(),
        }),
    }
}

#[get("/api/session")]
pub(super) async fn admin_api_session(
    cookies: &CookieJar<'_>,
    pool: &State<DbPool>,
) -> RouteResult<AdminSessionResponse> {
    let session = current_web_session(cookies, pool.inner()).await?;
    Ok(success_return(AdminSessionResponse {
        logged_in: session.is_some(),
        role: session.as_ref().map(|session| session.role).unwrap_or(0),
        app_title: web_app_title(),
        login_background: web_login_background(),
        login_position: web_login_position(),
        login_card_opacity: web_login_card_opacity(),
        web_surface_opacity: web_surface_opacity(),
        user: session.map(|session| session.user),
    }))
}

#[post("/api/login", format = "json", data = "<payload>")]
pub(super) async fn admin_api_login(
    payload: rocket::serde::json::Json<AdminLoginRequest>,
    cookies: &CookieJar<'_>,
    pool: &State<DbPool>,
) -> RouteResult<AdminSessionResponse> {
    let username = payload.username.trim();
    if username.is_empty() {
        return Err(ArcError::no_access("Incorrect username or password", 401));
    }

    let user = match load_web_login_user(pool.inner(), username).await? {
        Some(user) => user,
        None => {
            let (admin_username, admin_password) = admin_credentials();
            if username == admin_username && payload.password == admin_password {
                let password_hash = UserService::hash_password(&payload.password);
                bootstrap_config_admin_user(pool.inner(), username, &password_hash).await?
            } else {
                return Err(ArcError::no_access("Incorrect username or password", 401));
            }
        }
    };

    if super::helpers::is_ban_flag_active(user.ban_flag.as_deref()) {
        return Err(ArcError::no_access("Account is banned", 403));
    }

    let password_hash = user.password.as_deref().unwrap_or_default();
    if password_hash.is_empty() || password_hash != UserService::hash_password(&payload.password) {
        return Err(ArcError::no_access("Incorrect username or password", 401));
    }

    set_admin_cookie(cookies, user.user_id, user.web_role(), password_hash);
    Ok(success_return(web_session_response(&user)))
}

#[post("/api/logout")]
pub(super) fn admin_api_logout(cookies: &CookieJar<'_>) -> RouteResult<EmptyResponse> {
    clear_admin_cookie(cookies);
    Ok(success_return_no_value())
}
