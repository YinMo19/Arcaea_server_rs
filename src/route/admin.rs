use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use rand::Rng;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::serde::json::Json;
use rocket::{delete, get, patch, post, routes, Route, State};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::collections::HashMap;
use std::env;
use std::sync::{OnceLock, RwLock};

use crate::config::CONFIG;
use crate::error::ArcError;
use crate::route::common::{success_return, success_return_no_value, EmptyResponse, RouteResult};
use crate::service::OperationManager;
use crate::DbPool;

const ADMIN_COOKIE: &str = "arcaea_admin_session";

#[derive(Debug, Clone)]
pub struct AdminConfig {
    pub username: String,
    pub password: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            username: CONFIG.username.clone(),
            password: CONFIG.password.clone(),
        }
    }
}

static ADMIN_CONFIG: OnceLock<RwLock<AdminConfig>> = OnceLock::new();

pub fn set_admin_config(config: AdminConfig) {
    let lock = ADMIN_CONFIG.get_or_init(|| RwLock::new(AdminConfig::default()));
    if let Ok(mut guard) = lock.write() {
        *guard = config;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentOpView {
    name: String,
    operator: String,
    time: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserListView {
    user_id: i32,
    name: String,
    user_code: String,
    rating_ptt: i32,
    ticket: i32,
    last_play: String,
    banned: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongRowView {
    song_id: String,
    name_en: String,
    rating_pst: String,
    rating_prs: String,
    rating_ftr: String,
    rating_byd: String,
    rating_etr: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRowView {
    item_id: String,
    item_type: String,
    is_available: i32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseRowView {
    purchase_name: String,
    price: String,
    orig_price: String,
    discount_from: String,
    discount_to: String,
    discount_reason: String,
    item_summary: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseItemRowView {
    purchase_name: String,
    item_id: String,
    item_type: String,
    amount: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserSummary {
    user_id: i32,
    name: String,
    user_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminScoreRowView {
    user_id: i32,
    name: Option<String>,
    song_id: String,
    difficulty: i32,
    score: i32,
    shiny_perfect_count: i32,
    perfect_count: i32,
    near_count: i32,
    miss_count: i32,
    clear_type: i32,
    best_clear_type: i32,
    rating: f64,
    time_played: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserScoresResponse {
    user: AdminUserSummary,
    scores: Vec<AdminScoreRowView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminChartTopResponse {
    song_id: String,
    name_en: String,
    difficulty: i32,
    scores: Vec<AdminScoreRowView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminActionResponse {
    message: String,
    affected_rows: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRedeemUsersResponse {
    code: String,
    users: Vec<AdminUserSummary>,
}

#[derive(Debug, Deserialize)]
pub struct AdminSongPayload {
    sid: String,
    name_en: String,
    rating_pst: String,
    rating_prs: String,
    rating_ftr: String,
    rating_byd: String,
    rating_etr: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminSongDeletePayload {
    sid: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminItemPayload {
    item_id: String,
    item_type: String,
    is_available: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AdminItemDeletePayload {
    item_id: String,
    item_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminPurchasePayload {
    purchase_name: String,
    price: Option<String>,
    orig_price: Option<String>,
    discount_from: Option<String>,
    discount_to: Option<String>,
    discount_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminPurchaseDeletePayload {
    purchase_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminPurchaseItemPayload {
    purchase_name: String,
    item_id: String,
    item_type: String,
    amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminPurchaseItemDeletePayload {
    purchase_name: String,
    item_id: String,
    item_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminUserSelectorPayload {
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUserTicketPayload {
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    ticket: i32,
    all_users: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUserPasswordPayload {
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminUserPurchasePayload {
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    method: String,
    all_users: Option<bool>,
    item_types: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct AdminScoreDeletePayload {
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    song_id: Option<String>,
    difficulty: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AdminPresentPayload {
    present_id: String,
    expire_ts: Option<String>,
    description: Option<String>,
    item_id: String,
    item_type: String,
    amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminPresentDeletePayload {
    present_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminPresentDeliverPayload {
    present_id: String,
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    all_users: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminRedeemPayload {
    code: Option<String>,
    random_amount: Option<i32>,
    redeem_type: i32,
    item_id: String,
    item_type: String,
    amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminRedeemDeletePayload {
    code: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminUserScoreQuery {
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminSessionResponse {
    logged_in: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminDashboardApiResponse {
    online_users: i64,
    online_growth: f64,
    score_submits: i64,
    score_error_rate: f64,
    present_count: i64,
    alert_count: i64,
    recent_ops: Vec<RecentOpView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPageResponse<T> {
    rows: Vec<T>,
    total: i64,
    page: i64,
    page_size: i64,
}

#[derive(FromRow)]
struct RecentLoginRow {
    name: Option<String>,
    login_time: Option<i64>,
}

#[derive(FromRow)]
struct UserListDbRow {
    user_id: i32,
    name: Option<String>,
    user_code: Option<String>,
    rating_ptt: Option<i32>,
    ticket: Option<i32>,
    time_played: Option<i64>,
    password: Option<String>,
}

#[derive(FromRow)]
struct ChartDbRow {
    song_id: String,
    name: Option<String>,
    rating_pst: Option<i32>,
    rating_prs: Option<i32>,
    rating_ftr: Option<i32>,
    rating_byn: Option<i32>,
    rating_etr: Option<i32>,
}

#[derive(FromRow)]
struct ItemDbRow {
    item_id: String,
    r#type: String,
    is_available: Option<i8>,
}

#[derive(FromRow)]
struct PurchaseDbRow {
    purchase_name: String,
    price: Option<i32>,
    orig_price: Option<i32>,
    discount_from: Option<i64>,
    discount_to: Option<i64>,
    discount_reason: Option<String>,
}

#[derive(Clone, FromRow)]
struct PurchaseItemDbRow {
    purchase_name: String,
    item_id: String,
    r#type: String,
    amount: Option<i32>,
}

struct AdminUserDbSummary {
    user_id: i32,
    name: Option<String>,
    user_code: Option<String>,
}

fn expected_admin_cookie_value() -> String {
    let (username, password) = admin_credentials();
    let inner = format!("{:x}", Sha256::digest(password.as_bytes()));
    let joined = format!("{}{}", username, inner);
    format!("{:x}", Sha256::digest(joined.as_bytes()))
}

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

fn is_admin_logged_in(cookies: &CookieJar<'_>) -> bool {
    let expected = expected_admin_cookie_value();
    cookies
        .get(ADMIN_COOKIE)
        .map(|cookie| cookie.value() == expected)
        .unwrap_or(false)
}

fn set_admin_cookie(cookies: &CookieJar<'_>) {
    let mut cookie = Cookie::new(ADMIN_COOKIE, expected_admin_cookie_value());
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

fn format_timestamp(ts: Option<i64>) -> String {
    let Some(ts) = ts else {
        return "-".to_string();
    };

    let sec = if ts > 2_000_000_000_000 {
        ts / 1000
    } else {
        ts
    };

    Local
        .timestamp_opt(sec, 0)
        .single()
        .map(|x| x.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_rating_input_tenths(value: Option<i32>) -> String {
    match value {
        Some(v) if v >= 0 => format!("{:.1}", v as f64 / 10.0),
        _ => "-1".to_string(),
    }
}

fn normalize_chart_text(raw: &str, field: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{field} 不能为空"));
    }

    let truncated: String = value.chars().take(200).collect();
    Ok(truncated)
}

fn normalize_optional_text(raw: Option<&str>, max_len: usize) -> String {
    let text = raw.unwrap_or("").trim();
    text.chars().take(max_len).collect()
}

fn parse_optional_i32_input(raw: Option<&str>, field: &str) -> Result<Option<i32>, String> {
    let Some(value) = raw.map(str::trim) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i32>()
        .map(Some)
        .map_err(|_| format!("{field} 必须是整数"))
}

fn parse_positive_i32_input(raw: Option<&str>, field: &str) -> Result<i32, String> {
    let value = raw
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .ok_or_else(|| format!("{field} 不能为空"))?;
    let parsed = value
        .parse::<i32>()
        .map_err(|_| format!("{field} 必须是整数"))?;
    if parsed <= 0 {
        return Err(format!("{field} 必须大于 0"));
    }
    Ok(parsed)
}

fn parse_discount_datetime_input(raw: Option<&str>, field: &str) -> Result<i64, String> {
    let value = raw.map(str::trim).unwrap_or("");
    if value.is_empty() {
        return Ok(-1);
    }
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| format!("{field} 时间格式错误"))?;
    let local_dt = Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| format!("{field} 时间非法"))?;
    Ok(local_dt.timestamp_millis())
}

fn format_discount_datetime_input(value: Option<i64>) -> String {
    let Some(ts) = value else {
        return String::new();
    };
    if ts <= 0 {
        return String::new();
    }
    Local
        .timestamp_millis_opt(ts)
        .single()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M").to_string())
        .unwrap_or_default()
}

fn parse_rating_input_tenths(raw: &str, field: &str) -> Result<i32, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{field} 不能为空"));
    }

    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{field} 必须是数字"))?;
    if !parsed.is_finite() {
        return Err(format!("{field} 非法"));
    }
    if parsed < 0.0 {
        return Ok(-1);
    }

    let tenths = (parsed * 10.0) as i32;
    Ok(tenths)
}

fn chart_db_row_to_song_view(row: ChartDbRow) -> SongRowView {
    SongRowView {
        song_id: row.song_id,
        name_en: row.name.unwrap_or_default(),
        rating_pst: format_rating_input_tenths(row.rating_pst),
        rating_prs: format_rating_input_tenths(row.rating_prs),
        rating_ftr: format_rating_input_tenths(row.rating_ftr),
        rating_byd: format_rating_input_tenths(row.rating_byn),
        rating_etr: format_rating_input_tenths(row.rating_etr),
    }
}

fn normalize_item_available(value: Option<i32>) -> i32 {
    if value.unwrap_or(0) != 0 {
        1
    } else {
        0
    }
}

fn item_db_row_to_item_view(row: ItemDbRow) -> ItemRowView {
    ItemRowView {
        item_id: row.item_id,
        item_type: row.r#type,
        is_available: normalize_item_available(row.is_available.map(i32::from)),
    }
}

fn purchase_db_row_to_purchase_view(row: PurchaseDbRow, item_summary: String) -> PurchaseRowView {
    PurchaseRowView {
        purchase_name: row.purchase_name,
        price: row.price.map(|value| value.to_string()).unwrap_or_default(),
        orig_price: row
            .orig_price
            .map(|value| value.to_string())
            .unwrap_or_default(),
        discount_from: format_discount_datetime_input(row.discount_from),
        discount_to: format_discount_datetime_input(row.discount_to),
        discount_reason: row.discount_reason.unwrap_or_default(),
        item_summary,
    }
}

fn purchase_item_db_row_to_view(row: PurchaseItemDbRow) -> PurchaseItemRowView {
    PurchaseItemRowView {
        purchase_name: row.purchase_name,
        item_id: row.item_id,
        item_type: row.r#type,
        amount: row.amount.unwrap_or(1).to_string(),
    }
}

fn clean_query_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn normalize_page(page: Option<i64>, page_size: Option<i64>) -> (i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(25).clamp(10, 100);
    (page, page_size)
}

fn clamp_page(page: i64, page_size: i64, total: i64) -> (i64, i64) {
    let page_count = ((total.max(1) + page_size - 1) / page_size).max(1);
    let page = page.clamp(1, page_count);
    (page, (page - 1) * page_size)
}

fn page_response<T>(rows: Vec<T>, total: i64, page: i64, page_size: i64) -> AdminPageResponse<T> {
    AdminPageResponse {
        rows,
        total,
        page,
        page_size,
    }
}

fn filter_sql(filters: &[&str]) -> String {
    if filters.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", filters.join(" AND "))
    }
}

fn bind_strings_scalar<'q, O>(
    mut query: sqlx::query::QueryScalar<'q, sqlx::MySql, O, sqlx::mysql::MySqlArguments>,
    values: &'q [String],
) -> sqlx::query::QueryScalar<'q, sqlx::MySql, O, sqlx::mysql::MySqlArguments> {
    for value in values {
        query = query.bind(value);
    }
    query
}

fn bind_strings<'q>(
    mut query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    values: &'q [String],
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    for value in values {
        query = query.bind(value);
    }
    query
}

fn bind_strings_as<'q, T>(
    mut query: sqlx::query::QueryAs<'q, sqlx::MySql, T, sqlx::mysql::MySqlArguments>,
    values: &'q [String],
) -> sqlx::query::QueryAs<'q, sqlx::MySql, T, sqlx::mysql::MySqlArguments> {
    for value in values {
        query = query.bind(value);
    }
    query
}

fn admin_unauthorized() -> ArcError {
    ArcError::no_access("Admin login required", 401)
}

fn require_admin_api(cookies: &CookieJar<'_>) -> Result<(), ArcError> {
    is_admin_logged_in(cookies)
        .then_some(())
        .ok_or_else(admin_unauthorized)
}

fn hash_user_password(password: &str) -> String {
    format!("{:x}", Sha256::digest(password.as_bytes()))
}

fn clean_optional_payload_text(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

async fn resolve_admin_user(
    user_id: Option<i32>,
    name: Option<&str>,
    user_code: Option<&str>,
    pool: &DbPool,
) -> Result<AdminUserSummary, ArcError> {
    let row = if let Some(user_id) = user_id {
        sqlx::query_as!(
            AdminUserDbSummary,
            "SELECT user_id, name, user_code FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(pool)
        .await
    } else if let Some(user_code) = user_code {
        sqlx::query_as!(
            AdminUserDbSummary,
            "SELECT user_id, name, user_code FROM user WHERE user_code = ?",
            user_code
        )
        .fetch_optional(pool)
        .await
    } else if let Some(name) = name {
        sqlx::query_as!(
            AdminUserDbSummary,
            "SELECT user_id, name, user_code FROM user WHERE name = ?",
            name
        )
        .fetch_optional(pool)
        .await
    } else {
        return Err(ArcError::input("需要提供 user_id、name 或 user_code"));
    }
    .map_err(|err| ArcError::input(format!("查询用户失败: {err}")))?;

    row.map(|row| AdminUserSummary {
        user_id: row.user_id,
        name: row.name.unwrap_or_default(),
        user_code: row.user_code.unwrap_or_default(),
    })
    .ok_or_else(|| ArcError::no_data("玩家不存在", -2))
}

async fn resolve_admin_user_from_selector(
    selector: &AdminUserSelectorPayload,
    pool: &DbPool,
) -> Result<AdminUserSummary, ArcError> {
    resolve_admin_user(
        selector.user_id,
        clean_optional_payload_text(&selector.name),
        clean_optional_payload_text(&selector.user_code),
        pool,
    )
    .await
}

async fn update_admin_user_ticket(
    payload: &AdminUserTicketPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let affected_rows = if payload.all_users.unwrap_or(false) {
        sqlx::query!("UPDATE user SET ticket = ?", payload.ticket)
            .execute(pool)
            .await
    } else {
        let user = resolve_admin_user(
            payload.user_id,
            clean_optional_payload_text(&payload.name),
            clean_optional_payload_text(&payload.user_code),
            pool,
        )
        .await?;
        sqlx::query!(
            "UPDATE user SET ticket = ? WHERE user_id = ?",
            payload.ticket,
            user.user_id
        )
        .execute(pool)
        .await
    }
    .map_err(|err| ArcError::input(format!("更新记忆源点失败: {err}")))?
    .rows_affected();

    Ok(AdminActionResponse {
        message: "记忆源点已更新".to_string(),
        affected_rows,
    })
}

async fn update_admin_user_password(
    payload: &AdminUserPasswordPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let password = payload.password.trim();
    if password.len() < 8 || password.len() > 32 {
        return Err(ArcError::input("密码长度必须在 8-32 之间"));
    }

    let user = resolve_admin_user(
        payload.user_id,
        clean_optional_payload_text(&payload.name),
        clean_optional_payload_text(&payload.user_code),
        pool,
    )
    .await?;
    let password_hash = hash_user_password(password);
    let affected_rows = sqlx::query!(
        "UPDATE user SET password = ? WHERE user_id = ?",
        password_hash,
        user.user_id
    )
    .execute(pool)
    .await
    .map_err(|err| ArcError::input(format!("重置密码失败: {err}")))?
    .rows_affected();

    Ok(AdminActionResponse {
        message: "密码已重置".to_string(),
        affected_rows,
    })
}

async fn ban_admin_user(
    payload: &AdminUserSelectorPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let user = resolve_admin_user_from_selector(payload, pool).await?;
    let affected_rows = sqlx::query!(
        "UPDATE user SET password = '' WHERE user_id = ?",
        user.user_id
    )
    .execute(pool)
    .await
    .map_err(|err| ArcError::input(format!("封禁用户失败: {err}")))?
    .rows_affected();

    Ok(AdminActionResponse {
        message: "用户已封禁".to_string(),
        affected_rows,
    })
}

fn normalize_admin_item_types(item_types: &Option<Vec<String>>) -> Vec<String> {
    const ALLOWED: &[&str] = &[
        "single",
        "pack",
        "world_song",
        "world_unlock",
        "course_banner",
        "online_banner",
    ];
    let mut values = item_types
        .as_ref()
        .map(|items| {
            items
                .iter()
                .map(|item| item.trim())
                .filter(|item| ALLOWED.contains(item))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["single".to_string(), "pack".to_string()]);
    values.sort();
    values.dedup();
    if values.is_empty() {
        values.push("single".to_string());
        values.push("pack".to_string());
    }
    values
}

async fn update_admin_user_purchase(
    payload: &AdminUserPurchasePayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let item_types = normalize_admin_item_types(&payload.item_types);
    let method = payload.method.trim();
    if !matches!(method, "unlock" | "lock") {
        return Err(ArcError::input("method 必须是 unlock 或 lock"));
    }

    let users = if payload.all_users.unwrap_or(false) {
        sqlx::query_scalar!("SELECT user_id FROM user")
            .fetch_all(pool)
            .await
            .map_err(|err| ArcError::input(format!("查询用户失败: {err}")))?
    } else {
        let user = resolve_admin_user(
            payload.user_id,
            clean_optional_payload_text(&payload.name),
            clean_optional_payload_text(&payload.user_code),
            pool,
        )
        .await?;
        vec![user.user_id]
    };

    if users.is_empty() {
        return Ok(AdminActionResponse {
            message: "没有匹配用户".to_string(),
            affected_rows: 0,
        });
    }

    let placeholders = vec!["?"; item_types.len()].join(", ");
    let affected_rows = if method == "lock" {
        let user_placeholders = vec!["?"; users.len()].join(", ");
        let sql = format!(
            "DELETE FROM user_item WHERE user_id IN ({user_placeholders}) AND type IN ({placeholders})"
        );
        let mut query = sqlx::query(&sql);
        for user_id in &users {
            query = query.bind(user_id);
        }
        for item_type in &item_types {
            query = query.bind(item_type);
        }
        query
            .execute(pool)
            .await
            .map_err(|err| ArcError::input(format!("锁定购买失败: {err}")))?
            .rows_affected()
    } else {
        let item_sql = format!("SELECT item_id, type FROM item WHERE type IN ({placeholders})");
        let mut item_query = sqlx::query_as::<_, (String, String)>(&item_sql);
        for item_type in &item_types {
            item_query = item_query.bind(item_type);
        }
        let items = item_query
            .fetch_all(pool)
            .await
            .map_err(|err| ArcError::input(format!("查询物品失败: {err}")))?;
        let mut affected_rows = 0;
        for user_id in &users {
            for (item_id, item_type) in &items {
                affected_rows += sqlx::query!(
                    "INSERT INTO user_item (user_id, item_id, type, amount)
                     VALUES (?, ?, ?, 1)
                     ON DUPLICATE KEY UPDATE amount = 1",
                    user_id,
                    item_id,
                    item_type
                )
                .execute(pool)
                .await
                .map_err(|err| ArcError::input(format!("解锁购买失败: {err}")))?
                .rows_affected();
            }
        }
        affected_rows
    };

    Ok(AdminActionResponse {
        message: if method == "unlock" {
            "购买内容已解锁".to_string()
        } else {
            "购买内容已锁定".to_string()
        },
        affected_rows,
    })
}

async fn delete_admin_scores(
    payload: &AdminScoreDeletePayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let song_id = clean_optional_payload_text(&payload.song_id).map(str::to_string);
    let difficulty = payload.difficulty.filter(|value| (0..=4).contains(value));
    let user = if payload.user_id.is_some()
        || clean_optional_payload_text(&payload.name).is_some()
        || clean_optional_payload_text(&payload.user_code).is_some()
    {
        Some(
            resolve_admin_user(
                payload.user_id,
                clean_optional_payload_text(&payload.name),
                clean_optional_payload_text(&payload.user_code),
                pool,
            )
            .await?,
        )
    } else {
        None
    };

    if song_id.is_none() && difficulty.is_none() && user.is_none() {
        return Err(ArcError::input(
            "至少提供 song_id、difficulty 或玩家条件之一",
        ));
    }

    let mut filters = Vec::new();
    let mut binds = Vec::new();
    if let Some(song_id) = &song_id {
        filters.push("song_id = ?");
        binds.push(song_id.clone());
    }
    if difficulty.is_some() {
        filters.push("difficulty = ?");
    }
    if user.is_some() {
        filters.push("user_id = ?");
    }

    let sql = format!("DELETE FROM best_score{}", filter_sql(&filters));
    let mut query = bind_strings(sqlx::query(&sql), &binds);
    if let Some(difficulty) = difficulty {
        query = query.bind(difficulty);
    }
    if let Some(user) = &user {
        query = query.bind(user.user_id);
    }
    let affected_rows = query
        .execute(pool)
        .await
        .map_err(|err| ArcError::input(format!("删除成绩失败: {err}")))?
        .rows_affected();

    if let Some(user) = user {
        if song_id.is_none() && difficulty.is_none() {
            sqlx::query!(
                "UPDATE user
                 SET rating_ptt = 0, song_id = '', difficulty = 0, score = 0,
                     shiny_perfect_count = 0, perfect_count = 0, near_count = 0,
                     miss_count = 0, health = 0, time_played = 0, rating = 0,
                     world_rank_score = 0
                 WHERE user_id = ?",
                user.user_id
            )
            .execute(pool)
            .await
            .map_err(|err| ArcError::input(format!("重置用户成绩摘要失败: {err}")))?;
            sqlx::query!("DELETE FROM recent30 WHERE user_id = ?", user.user_id)
                .execute(pool)
                .await
                .map_err(|err| ArcError::input(format!("删除 recent30 失败: {err}")))?;
        }
    }

    Ok(AdminActionResponse {
        message: "成绩已删除".to_string(),
        affected_rows,
    })
}

fn normalize_admin_required_text(
    raw: &str,
    field: &str,
    max_len: usize,
) -> Result<String, ArcError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(ArcError::input(format!("{field} 不能为空")));
    }
    Ok(value.chars().take(max_len).collect())
}

fn parse_admin_amount(raw: Option<&str>, field: &str) -> Result<i32, ArcError> {
    let value = raw.map(str::trim).filter(|value| !value.is_empty());
    let amount = if let Some(value) = value {
        value
            .parse::<i32>()
            .map_err(|_| ArcError::input(format!("{field} 必须是整数")))?
    } else {
        1
    };
    if amount <= 0 {
        return Err(ArcError::input(format!("{field} 必须大于 0")));
    }
    Ok(amount)
}

fn parse_admin_expire_ts(raw: Option<&str>) -> Result<i64, ArcError> {
    let value = raw.map(str::trim).unwrap_or("");
    if value.is_empty() {
        return Err(ArcError::input("expire_ts 不能为空"));
    }
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| ArcError::input("expire_ts 时间格式错误"))?;
    let local_dt = Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| ArcError::input("expire_ts 时间非法"))?;
    Ok(local_dt.timestamp_millis())
}

fn random_redeem_code() -> String {
    const CHARS: &[u8] = b"AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz0123456789";
    let mut rng = rand::thread_rng();
    (0..10)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

async fn require_admin_item_exists(
    item_id: &str,
    item_type: &str,
    pool: &DbPool,
) -> Result<(), ArcError> {
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM item WHERE item_id = ? AND type = ?",
        item_id,
        item_type
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询物品失败: {err}")))?;
    if exists <= 0 {
        return Err(ArcError::no_data("物品不存在", -2));
    }
    Ok(())
}

async fn create_admin_present(
    payload: &AdminPresentPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let present_id = normalize_admin_required_text(&payload.present_id, "present_id", 200)?;
    let description = normalize_optional_text(payload.description.as_deref(), 200);
    let item_id = normalize_admin_required_text(&payload.item_id, "item_id", 200)?;
    let item_type = normalize_admin_required_text(&payload.item_type, "type", 200)?;
    let amount = parse_admin_amount(payload.amount.as_deref(), "amount")?;
    let expire_ts = parse_admin_expire_ts(payload.expire_ts.as_deref())?;
    require_admin_item_exists(&item_id, &item_type, pool).await?;

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM present WHERE present_id = ?",
        present_id
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询奖励失败: {err}")))?;
    if exists > 0 {
        return Err(ArcError::input("奖励已存在"));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| ArcError::input(format!("事务创建失败: {err}")))?;
    sqlx::query!(
        "INSERT INTO present (present_id, expire_ts, description) VALUES (?, ?, ?)",
        &present_id,
        expire_ts,
        description
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| ArcError::input(format!("新增奖励失败: {err}")))?;
    let affected_rows = sqlx::query!(
        "INSERT INTO present_item (present_id, item_id, type, amount) VALUES (?, ?, ?, ?)",
        &present_id,
        item_id,
        item_type,
        amount
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| ArcError::input(format!("新增奖励物品失败: {err}")))?
    .rows_affected();
    tx.commit()
        .await
        .map_err(|err| ArcError::input(format!("新增奖励失败: {err}")))?;

    Ok(AdminActionResponse {
        message: "奖励已新增".to_string(),
        affected_rows,
    })
}

async fn delete_admin_present(
    payload: &AdminPresentDeletePayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let present_id = normalize_admin_required_text(&payload.present_id, "present_id", 200)?;
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM present WHERE present_id = ?",
        &present_id
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询奖励失败: {err}")))?;
    if exists <= 0 {
        return Err(ArcError::no_data("奖励不存在", -2));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| ArcError::input(format!("事务创建失败: {err}")))?;
    sqlx::query!("DELETE FROM user_present WHERE present_id = ?", &present_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("删除用户奖励失败: {err}")))?;
    sqlx::query!("DELETE FROM present_item WHERE present_id = ?", &present_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("删除奖励物品失败: {err}")))?;
    let affected_rows = sqlx::query!("DELETE FROM present WHERE present_id = ?", &present_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("删除奖励失败: {err}")))?
        .rows_affected();
    tx.commit()
        .await
        .map_err(|err| ArcError::input(format!("删除奖励失败: {err}")))?;

    Ok(AdminActionResponse {
        message: "奖励已删除".to_string(),
        affected_rows,
    })
}

async fn deliver_admin_present(
    payload: &AdminPresentDeliverPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let present_id = normalize_admin_required_text(&payload.present_id, "present_id", 200)?;
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM present WHERE present_id = ?",
        &present_id
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询奖励失败: {err}")))?;
    if exists <= 0 {
        return Err(ArcError::no_data("奖励不存在", -2));
    }

    let affected_rows = if payload.all_users.unwrap_or(false) {
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| ArcError::input(format!("事务创建失败: {err}")))?;
        sqlx::query!("DELETE FROM user_present WHERE present_id = ?", &present_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| ArcError::input(format!("清理奖励分发失败: {err}")))?;
        let done = sqlx::query!(
            "INSERT INTO user_present (user_id, present_id)
             SELECT user_id, ? FROM user",
            &present_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("分发奖励失败: {err}")))?;
        tx.commit()
            .await
            .map_err(|err| ArcError::input(format!("分发奖励失败: {err}")))?;
        done.rows_affected()
    } else {
        let user = resolve_admin_user(
            payload.user_id,
            clean_optional_payload_text(&payload.name),
            clean_optional_payload_text(&payload.user_code),
            pool,
        )
        .await?;
        sqlx::query!(
            "INSERT IGNORE INTO user_present (user_id, present_id) VALUES (?, ?)",
            user.user_id,
            &present_id
        )
        .execute(pool)
        .await
        .map_err(|err| ArcError::input(format!("分发奖励失败: {err}")))?
        .rows_affected()
    };

    Ok(AdminActionResponse {
        message: "奖励已分发".to_string(),
        affected_rows,
    })
}

async fn create_admin_redeem(
    payload: &AdminRedeemPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let item_id = normalize_admin_required_text(&payload.item_id, "item_id", 200)?;
    let item_type = normalize_admin_required_text(&payload.item_type, "type", 200)?;
    let amount = parse_admin_amount(payload.amount.as_deref(), "amount")?;
    require_admin_item_exists(&item_id, &item_type, pool).await?;

    let code = clean_optional_payload_text(&payload.code).map(str::to_string);
    let random_amount = payload.random_amount.unwrap_or(0);
    if code.is_some() && random_amount > 0 {
        return Err(ArcError::input("只能使用一种添加方式"));
    }
    if code.is_none() && random_amount <= 0 {
        return Err(ArcError::input("需要提供 code 或 random_amount"));
    }
    if !(0..=1).contains(&payload.redeem_type) {
        return Err(ArcError::input("redeem_type 必须是 0 或 1"));
    }

    let mut codes = Vec::new();
    if let Some(code) = code {
        if code.len() < 10 || code.len() > 20 {
            return Err(ArcError::input("兑换码长度必须在 10-20 之间"));
        }
        let exists = sqlx::query_scalar!(
            "SELECT COUNT(*) as `count!: i64` FROM redeem WHERE code = ?",
            code
        )
        .fetch_one(pool)
        .await
        .map_err(|err| ArcError::input(format!("查询兑换码失败: {err}")))?;
        if exists > 0 {
            return Err(ArcError::input("兑换码已存在"));
        }
        codes.push(code);
    } else {
        if random_amount > 1000 {
            return Err(ArcError::input("random_amount 必须在 1-1000 之间"));
        }
        while codes.len() < random_amount as usize {
            let code = random_redeem_code();
            let exists = sqlx::query_scalar!(
                "SELECT COUNT(*) as `count!: i64` FROM redeem WHERE code = ?",
                code
            )
            .fetch_one(pool)
            .await
            .map_err(|err| ArcError::input(format!("查询兑换码失败: {err}")))?;
            if exists == 0 && !codes.contains(&code) {
                codes.push(code);
            }
        }
    }

    let mut affected_rows = 0;
    let mut tx = pool
        .begin()
        .await
        .map_err(|err| ArcError::input(format!("事务创建失败: {err}")))?;
    for code in &codes {
        affected_rows += sqlx::query!(
            "INSERT INTO redeem (code, type) VALUES (?, ?)",
            code,
            payload.redeem_type
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("新增兑换码失败: {err}")))?
        .rows_affected();
        sqlx::query!(
            "INSERT INTO redeem_item (code, item_id, type, amount) VALUES (?, ?, ?, ?)",
            code,
            &item_id,
            &item_type,
            amount
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("新增兑换码物品失败: {err}")))?;
    }
    tx.commit()
        .await
        .map_err(|err| ArcError::input(format!("新增兑换码失败: {err}")))?;

    Ok(AdminActionResponse {
        message: if codes.len() == 1 {
            format!("兑换码已新增: {}", codes[0])
        } else {
            format!("兑换码已新增: {} 个", codes.len())
        },
        affected_rows,
    })
}

async fn delete_admin_redeem(
    payload: &AdminRedeemDeletePayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let code = normalize_admin_required_text(&payload.code, "code", 200)?;
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM redeem WHERE code = ?",
        &code
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询兑换码失败: {err}")))?;
    if exists <= 0 {
        return Err(ArcError::no_data("兑换码不存在", -2));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| ArcError::input(format!("事务创建失败: {err}")))?;
    sqlx::query!("DELETE FROM user_redeem WHERE code = ?", &code)
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("删除用户兑换记录失败: {err}")))?;
    sqlx::query!("DELETE FROM redeem_item WHERE code = ?", &code)
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("删除兑换码物品失败: {err}")))?;
    let affected_rows = sqlx::query!("DELETE FROM redeem WHERE code = ?", &code)
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("删除兑换码失败: {err}")))?
        .rows_affected();
    tx.commit()
        .await
        .map_err(|err| ArcError::input(format!("删除兑换码失败: {err}")))?;

    Ok(AdminActionResponse {
        message: "兑换码已删除".to_string(),
        affected_rows,
    })
}

async fn load_admin_redeem_users(
    code: Option<&str>,
    pool: &DbPool,
) -> Result<AdminRedeemUsersResponse, ArcError> {
    let code = code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ArcError::input("需要提供 code"))?;
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM redeem WHERE code = ?",
        code
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询兑换码失败: {err}")))?;
    if exists <= 0 {
        return Err(ArcError::no_data("兑换码不存在", -2));
    }

    let users = sqlx::query_as!(
        AdminUserDbSummary,
        "SELECT u.user_id, u.name, u.user_code
         FROM user u
         JOIN user_redeem ur ON ur.user_id = u.user_id
         WHERE ur.code = ?
         ORDER BY u.user_id ASC",
        code
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询兑换用户失败: {err}")))?
    .into_iter()
    .map(|row| AdminUserSummary {
        user_id: row.user_id,
        name: row.name.unwrap_or_default(),
        user_code: row.user_code.unwrap_or_default(),
    })
    .collect();

    Ok(AdminRedeemUsersResponse {
        code: code.to_string(),
        users,
    })
}

async fn load_admin_user_scores(
    query: &AdminUserScoreQuery,
    pool: &DbPool,
) -> Result<AdminUserScoresResponse, ArcError> {
    let user = resolve_admin_user(
        query.user_id,
        clean_optional_payload_text(&query.name),
        clean_optional_payload_text(&query.user_code),
        pool,
    )
    .await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let rows = sqlx::query!(
        "SELECT bs.user_id, u.name, bs.song_id, bs.difficulty, bs.score,
                bs.shiny_perfect_count, bs.perfect_count, bs.near_count, bs.miss_count,
                bs.clear_type, bs.best_clear_type, bs.rating, bs.time_played
         FROM best_score bs
         JOIN user u ON u.user_id = bs.user_id
         WHERE bs.user_id = ?
         ORDER BY bs.rating DESC, bs.score DESC
         LIMIT ?",
        user.user_id,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询玩家成绩失败: {err}")))?
    .into_iter()
    .map(|row| AdminScoreRowView {
        user_id: row.user_id,
        name: row.name,
        song_id: row.song_id,
        difficulty: row.difficulty,
        score: row.score.unwrap_or_default(),
        shiny_perfect_count: row.shiny_perfect_count.unwrap_or_default(),
        perfect_count: row.perfect_count.unwrap_or_default(),
        near_count: row.near_count.unwrap_or_default(),
        miss_count: row.miss_count.unwrap_or_default(),
        clear_type: row.clear_type.unwrap_or_default(),
        best_clear_type: row.best_clear_type.unwrap_or_default(),
        rating: row.rating.unwrap_or(0.0),
        time_played: format_timestamp(row.time_played),
    })
    .collect();

    Ok(AdminUserScoresResponse { user, scores: rows })
}

async fn load_admin_chart_top(
    sid: Option<&str>,
    difficulty: i32,
    limit: Option<i64>,
    pool: &DbPool,
) -> Result<AdminChartTopResponse, ArcError> {
    let sid = sid
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ArcError::input("需要提供 song_id"))?;
    let difficulty = difficulty.clamp(0, 4);
    let like = format!("%{sid}%");
    let chart = sqlx::query!(
        "SELECT song_id, name FROM chart WHERE song_id LIKE ? OR name LIKE ? LIMIT 1",
        like,
        like
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询歌曲失败: {err}")))?
    .ok_or_else(|| ArcError::no_data("歌曲不存在", -2))?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let rows = sqlx::query!(
        "SELECT bs.user_id, u.name, bs.song_id, bs.difficulty, bs.score,
                bs.shiny_perfect_count, bs.perfect_count, bs.near_count, bs.miss_count,
                bs.clear_type, bs.best_clear_type, bs.rating, bs.time_played
         FROM best_score bs
         JOIN user u ON u.user_id = bs.user_id
         WHERE bs.song_id = ? AND bs.difficulty = ?
         ORDER BY bs.score DESC, bs.time_played ASC
         LIMIT ?",
        chart.song_id,
        difficulty,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询排行榜失败: {err}")))?
    .into_iter()
    .map(|row| AdminScoreRowView {
        user_id: row.user_id,
        name: row.name,
        song_id: row.song_id,
        difficulty: row.difficulty,
        score: row.score.unwrap_or_default(),
        shiny_perfect_count: row.shiny_perfect_count.unwrap_or_default(),
        perfect_count: row.perfect_count.unwrap_or_default(),
        near_count: row.near_count.unwrap_or_default(),
        miss_count: row.miss_count.unwrap_or_default(),
        clear_type: row.clear_type.unwrap_or_default(),
        best_clear_type: row.best_clear_type.unwrap_or_default(),
        rating: row.rating.unwrap_or(0.0),
        time_played: format_timestamp(row.time_played),
    })
    .collect();

    Ok(AdminChartTopResponse {
        song_id: chart.song_id.clone(),
        name_en: chart.name.unwrap_or_default(),
        difficulty,
        scores: rows,
    })
}

async fn load_dashboard_api(pool: &DbPool) -> AdminDashboardApiResponse {
    let now_ms = Utc::now().timestamp_millis();
    let one_day_ms = 86_400_000i64;

    let online_users = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT user_id) FROM login WHERE login_time >= ?",
        now_ms - one_day_ms
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let yesterday_online = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT user_id) FROM login WHERE login_time >= ? AND login_time < ?",
        now_ms - one_day_ms * 2,
        now_ms - one_day_ms
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let online_growth = if yesterday_online <= 0 {
        0.0
    } else {
        ((online_users - yesterday_online) as f64 / yesterday_online as f64 * 1000.0).round() / 10.0
    };

    let score_submits = sqlx::query_scalar!("SELECT COUNT(*) FROM best_score")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let present_count = sqlx::query_scalar!("SELECT COUNT(*) FROM user_present")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let alert_count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM user WHERE COALESCE(password, '') = ''")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let recent_login_rows = sqlx::query_as!(
        RecentLoginRow,
        "SELECT u.name as name, l.login_time as login_time
         FROM login l
         JOIN user u ON u.user_id = l.user_id
         ORDER BY l.login_time DESC
         LIMIT 8",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let recent_ops = if recent_login_rows.is_empty() {
        vec![RecentOpView {
            name: "service_boot".to_string(),
            operator: "system".to_string(),
            time: format_timestamp(Some(now_ms)),
            status: "ok".to_string(),
        }]
    } else {
        recent_login_rows
            .into_iter()
            .map(|row| RecentOpView {
                name: "user_login".to_string(),
                operator: row.name.unwrap_or_else(|| "unknown".to_string()),
                time: format_timestamp(row.login_time),
                status: "ok".to_string(),
            })
            .collect()
    };

    AdminDashboardApiResponse {
        online_users,
        online_growth,
        score_submits,
        score_error_rate: 0.0,
        present_count,
        alert_count,
        recent_ops,
    }
}

async fn load_admin_users(
    q: Option<&str>,
    status: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> AdminPageResponse<UserListView> {
    let keyword = clean_query_value(q);
    let status = status
        .map(str::trim)
        .filter(|value| matches!(*value, "normal" | "banned"));

    let (filters, binds) = user_filters(keyword.as_deref(), status);
    let where_sql = filter_sql(&filters);
    let count_sql = format!("SELECT COUNT(*) FROM user{where_sql}");
    let total = bind_strings_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &binds)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
         FROM user{where_sql}
         ORDER BY rating_ptt DESC, user_id ASC
         LIMIT ? OFFSET ?"
    );
    let rows = bind_strings_as(sqlx::query_as::<_, UserListDbRow>(&row_sql), &binds)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| UserListView {
            user_id: row.user_id,
            name: row.name.unwrap_or_default(),
            user_code: row.user_code.unwrap_or_default(),
            rating_ptt: row.rating_ptt.unwrap_or(0),
            ticket: row.ticket.unwrap_or(0),
            last_play: format_timestamp(row.time_played),
            banned: row.password.unwrap_or_default().is_empty(),
        })
        .collect();

    page_response(rows, total, page, page_size)
}

fn user_filters(keyword: Option<&str>, status: Option<&str>) -> (Vec<&'static str>, Vec<String>) {
    let mut filters = Vec::new();
    let mut binds = Vec::new();
    if let Some(kw) = keyword {
        let like = format!("%{kw}%");
        filters.push("(name LIKE ? OR user_code LIKE ?)");
        binds.push(like.clone());
        binds.push(like);
    }

    match status {
        Some("banned") => {
            filters.push("COALESCE(password, '') = ''");
        }
        Some("normal") => {
            filters.push("COALESCE(password, '') <> ''");
        }
        _ => {}
    }

    (filters, binds)
}

async fn load_admin_songs(
    q: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> AdminPageResponse<SongRowView> {
    let query = clean_query_value(q).unwrap_or_default();
    let (where_sql, binds) = if query.is_empty() {
        (String::new(), Vec::new())
    } else {
        let like = format!("%{query}%");
        (
            " WHERE song_id LIKE ? OR name LIKE ?".to_string(),
            vec![like.clone(), like],
        )
    };

    let count_sql = format!("SELECT COUNT(*) FROM chart{where_sql}");
    let total = bind_strings_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &binds)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT song_id, name, rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr
         FROM chart{where_sql}
         ORDER BY song_id ASC
         LIMIT ? OFFSET ?"
    );
    let rows = bind_strings_as(sqlx::query_as::<_, ChartDbRow>(&row_sql), &binds)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(chart_db_row_to_song_view)
        .collect();

    page_response(rows, total, page, page_size)
}

async fn load_admin_items(
    q: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> AdminPageResponse<ItemRowView> {
    let query = clean_query_value(q).unwrap_or_default();
    let (where_sql, binds) = if query.is_empty() {
        (String::new(), Vec::new())
    } else {
        let like = format!("%{query}%");
        (
            " WHERE item_id LIKE ? OR type LIKE ?".to_string(),
            vec![like.clone(), like],
        )
    };

    let count_sql = format!("SELECT COUNT(*) FROM item{where_sql}");
    let total = bind_strings_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &binds)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT item_id, type, is_available
         FROM item{where_sql}
         ORDER BY type, item_id
         LIMIT ? OFFSET ?"
    );
    let rows = bind_strings_as(sqlx::query_as::<_, ItemDbRow>(&row_sql), &binds)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(item_db_row_to_item_view)
        .collect();

    page_response(rows, total, page, page_size)
}

async fn load_admin_purchases(
    pq: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> AdminPageResponse<PurchaseRowView> {
    let query_purchase = clean_query_value(pq).unwrap_or_default();
    let (where_sql, binds) = if query_purchase.is_empty() {
        (String::new(), Vec::new())
    } else {
        let like = format!("%{query_purchase}%");
        (
            " WHERE purchase_name LIKE ? OR COALESCE(discount_reason, '') LIKE ?".to_string(),
            vec![like.clone(), like],
        )
    };

    let count_sql = format!("SELECT COUNT(*) FROM purchase{where_sql}");
    let total = bind_strings_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &binds)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT purchase_name, price, orig_price, discount_from, discount_to, discount_reason
         FROM purchase{where_sql}
         ORDER BY purchase_name ASC
         LIMIT ? OFFSET ?"
    );
    let purchase_rows = bind_strings_as(sqlx::query_as::<_, PurchaseDbRow>(&row_sql), &binds)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let mut item_summaries: HashMap<String, Vec<String>> = HashMap::new();
    if !purchase_rows.is_empty() {
        let purchase_names = purchase_rows
            .iter()
            .map(|row| row.purchase_name.clone())
            .collect::<Vec<_>>();
        let placeholders = vec!["?"; purchase_names.len()].join(", ");
        let item_sql = format!(
            "SELECT purchase_name, item_id, type, amount
             FROM purchase_item
             WHERE purchase_name IN ({placeholders})
             ORDER BY purchase_name ASC, item_id ASC, type ASC"
        );
        let purchase_item_rows = bind_strings_as(
            sqlx::query_as::<_, PurchaseItemDbRow>(&item_sql),
            &purchase_names,
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        for item in purchase_item_rows {
            item_summaries
                .entry(item.purchase_name.clone())
                .or_default()
                .push(format!(
                    "{}:{}x{}",
                    item.item_id,
                    item.r#type,
                    item.amount.unwrap_or(1)
                ));
        }
    }

    let purchases = purchase_rows
        .into_iter()
        .map(|purchase| {
            let item_summary = item_summaries
                .remove(&purchase.purchase_name)
                .map(|items| items.join(", "))
                .unwrap_or_else(|| "-".to_string());
            purchase_db_row_to_purchase_view(purchase, item_summary)
        })
        .collect();

    page_response(purchases, total, page, page_size)
}

async fn load_admin_purchase_items(
    iq: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> AdminPageResponse<PurchaseItemRowView> {
    let query_purchase_item = clean_query_value(iq).unwrap_or_default();
    let (where_sql, binds) = if query_purchase_item.is_empty() {
        (String::new(), Vec::new())
    } else {
        let like = format!("%{query_purchase_item}%");
        (
            " WHERE purchase_name LIKE ? OR item_id LIKE ? OR type LIKE ?".to_string(),
            vec![like.clone(), like.clone(), like],
        )
    };

    let count_sql = format!("SELECT COUNT(*) FROM purchase_item{where_sql}");
    let total = bind_strings_scalar(sqlx::query_scalar::<_, i64>(&count_sql), &binds)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT purchase_name, item_id, type, amount
         FROM purchase_item{where_sql}
         ORDER BY purchase_name ASC, item_id ASC, type ASC
         LIMIT ? OFFSET ?"
    );
    let purchase_items = bind_strings_as(sqlx::query_as::<_, PurchaseItemDbRow>(&row_sql), &binds)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(purchase_item_db_row_to_view)
        .collect();

    page_response(purchase_items, total, page, page_size)
}

async fn create_song(
    pool: &DbPool,
    sid_raw: &str,
    name_en_raw: &str,
    rating_pst_raw: &str,
    rating_prs_raw: &str,
    rating_ftr_raw: &str,
    rating_byd_raw: &str,
    rating_etr_raw: &str,
) -> Result<(), String> {
    let sid = normalize_chart_text(sid_raw, "song_id")?;
    let name_en = normalize_chart_text(name_en_raw, "name_en")?;
    let rating_pst = parse_rating_input_tenths(rating_pst_raw, "rating_pst")?;
    let rating_prs = parse_rating_input_tenths(rating_prs_raw, "rating_prs")?;
    let rating_ftr = parse_rating_input_tenths(rating_ftr_raw, "rating_ftr")?;
    let rating_byd = parse_rating_input_tenths(rating_byd_raw, "rating_byd")?;
    let rating_etr = parse_rating_input_tenths(rating_etr_raw, "rating_etr")?;

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM chart WHERE song_id = ?",
        sid
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;

    if exists > 0 {
        return Err("歌曲已存在".to_string());
    }

    sqlx::query!(
        "INSERT INTO chart (song_id, name, rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        sid,
        name_en,
        rating_pst,
        rating_prs,
        rating_ftr,
        rating_byd,
        rating_etr
    )
    .execute(pool)
    .await
    .map_err(|err| format!("新增失败: {err}"))?;

    Ok(())
}

async fn update_song(
    pool: &DbPool,
    sid_raw: &str,
    name_en_raw: &str,
    rating_pst_raw: &str,
    rating_prs_raw: &str,
    rating_ftr_raw: &str,
    rating_byd_raw: &str,
    rating_etr_raw: &str,
) -> Result<(), String> {
    let sid = normalize_chart_text(sid_raw, "song_id")?;
    let name_en = normalize_chart_text(name_en_raw, "name_en")?;
    let rating_pst = parse_rating_input_tenths(rating_pst_raw, "rating_pst")?;
    let rating_prs = parse_rating_input_tenths(rating_prs_raw, "rating_prs")?;
    let rating_ftr = parse_rating_input_tenths(rating_ftr_raw, "rating_ftr")?;
    let rating_byd = parse_rating_input_tenths(rating_byd_raw, "rating_byd")?;
    let rating_etr = parse_rating_input_tenths(rating_etr_raw, "rating_etr")?;

    let done = sqlx::query!(
        "UPDATE chart
         SET name = ?,
             rating_pst = ?,
             rating_prs = ?,
             rating_ftr = ?,
             rating_byn = ?,
             rating_etr = ?
         WHERE song_id = ?",
        name_en,
        rating_pst,
        rating_prs,
        rating_ftr,
        rating_byd,
        rating_etr,
        sid
    )
    .execute(pool)
    .await
    .map_err(|err| format!("更新失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("歌曲不存在".to_string());
    }

    Ok(())
}

async fn delete_song(pool: &DbPool, sid_raw: &str) -> Result<(), String> {
    let sid = normalize_chart_text(sid_raw, "song_id")?;
    let done = sqlx::query!("DELETE FROM chart WHERE song_id = ?", sid)
        .execute(pool)
        .await
        .map_err(|err| format!("删除失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("歌曲不存在".to_string());
    }

    Ok(())
}

async fn create_item(
    pool: &DbPool,
    item_id_raw: &str,
    item_type_raw: &str,
    is_available_raw: Option<i32>,
) -> Result<(), String> {
    let item_id = normalize_chart_text(item_id_raw, "item_id")?;
    let item_type = normalize_chart_text(item_type_raw, "type")?;
    let is_available = normalize_item_available(is_available_raw);

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64`
         FROM item
         WHERE item_id = ? AND type = ?",
        item_id,
        item_type
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;

    if exists > 0 {
        return Err("物品已存在".to_string());
    }

    sqlx::query!(
        "INSERT INTO item (item_id, type, is_available)
         VALUES (?, ?, ?)",
        item_id,
        item_type,
        is_available
    )
    .execute(pool)
    .await
    .map_err(|err| format!("新增失败: {err}"))?;

    Ok(())
}

async fn update_item(
    pool: &DbPool,
    item_id_raw: &str,
    item_type_raw: &str,
    is_available_raw: Option<i32>,
) -> Result<(), String> {
    let item_id = normalize_chart_text(item_id_raw, "item_id")?;
    let item_type = normalize_chart_text(item_type_raw, "type")?;
    let is_available = normalize_item_available(is_available_raw);

    let done = sqlx::query!(
        "UPDATE item
         SET is_available = ?
         WHERE item_id = ? AND type = ?",
        is_available,
        item_id,
        item_type
    )
    .execute(pool)
    .await
    .map_err(|err| format!("更新失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("物品不存在".to_string());
    }

    Ok(())
}

async fn delete_item(pool: &DbPool, item_id_raw: &str, item_type_raw: &str) -> Result<(), String> {
    let item_id = normalize_chart_text(item_id_raw, "item_id")?;
    let item_type = normalize_chart_text(item_type_raw, "type")?;

    let done = sqlx::query!(
        "DELETE FROM item
         WHERE item_id = ? AND type = ?",
        item_id,
        item_type
    )
    .execute(pool)
    .await
    .map_err(|err| format!("删除失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("物品不存在".to_string());
    }

    Ok(())
}

async fn create_purchase(
    pool: &DbPool,
    purchase_name_raw: &str,
    price_raw: Option<&str>,
    orig_price_raw: Option<&str>,
    discount_from_raw: Option<&str>,
    discount_to_raw: Option<&str>,
    discount_reason_raw: Option<&str>,
) -> Result<(), String> {
    let purchase_name = normalize_chart_text(purchase_name_raw, "purchase_name")?;
    let price = parse_optional_i32_input(price_raw, "price")?;
    let orig_price = parse_optional_i32_input(orig_price_raw, "orig_price")?;
    let discount_from = parse_discount_datetime_input(discount_from_raw, "discount_from")?;
    let discount_to = parse_discount_datetime_input(discount_to_raw, "discount_to")?;
    let discount_reason = normalize_optional_text(discount_reason_raw, 255);

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64`
         FROM purchase
         WHERE purchase_name = ?",
        &purchase_name
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;

    if exists > 0 {
        return Err("购买项已存在".to_string());
    }

    sqlx::query!(
        "INSERT INTO purchase (purchase_name, price, orig_price, discount_from, discount_to, discount_reason)
         VALUES (?, ?, ?, ?, ?, ?)",
        purchase_name,
        price,
        orig_price,
        discount_from,
        discount_to,
        discount_reason
    )
    .execute(pool)
    .await
    .map_err(|err| format!("新增失败: {err}"))?;

    Ok(())
}

async fn update_purchase(
    pool: &DbPool,
    purchase_name_raw: &str,
    price_raw: Option<&str>,
    orig_price_raw: Option<&str>,
    discount_from_raw: Option<&str>,
    discount_to_raw: Option<&str>,
    discount_reason_raw: Option<&str>,
) -> Result<(), String> {
    let purchase_name = normalize_chart_text(purchase_name_raw, "purchase_name")?;
    let price = parse_optional_i32_input(price_raw, "price")?;
    let orig_price = parse_optional_i32_input(orig_price_raw, "orig_price")?;
    let discount_from = parse_discount_datetime_input(discount_from_raw, "discount_from")?;
    let discount_to = parse_discount_datetime_input(discount_to_raw, "discount_to")?;
    let discount_reason = normalize_optional_text(discount_reason_raw, 255);

    let done = sqlx::query!(
        "UPDATE purchase
         SET price = ?,
             orig_price = ?,
             discount_from = ?,
             discount_to = ?,
             discount_reason = ?
         WHERE purchase_name = ?",
        price,
        orig_price,
        discount_from,
        discount_to,
        discount_reason,
        purchase_name
    )
    .execute(pool)
    .await
    .map_err(|err| format!("更新失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("购买项不存在".to_string());
    }

    Ok(())
}

async fn delete_purchase(pool: &DbPool, purchase_name_raw: &str) -> Result<(), String> {
    let purchase_name = normalize_chart_text(purchase_name_raw, "purchase_name")?;
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64`
         FROM purchase
         WHERE purchase_name = ?",
        &purchase_name
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;

    if exists <= 0 {
        return Err("购买项不存在".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| format!("事务创建失败: {err}"))?;

    sqlx::query!(
        "DELETE FROM purchase_item
         WHERE purchase_name = ?",
        &purchase_name
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("删除失败: {err}"))?;

    sqlx::query!(
        "DELETE FROM purchase
         WHERE purchase_name = ?",
        &purchase_name
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("删除失败: {err}"))?;

    tx.commit()
        .await
        .map_err(|err| format!("删除失败: {err}"))?;

    Ok(())
}

async fn create_purchase_item(
    pool: &DbPool,
    purchase_name_raw: &str,
    item_id_raw: &str,
    item_type_raw: &str,
    amount_raw: Option<&str>,
) -> Result<(), String> {
    let purchase_name = normalize_chart_text(purchase_name_raw, "purchase_name")?;
    let item_id = normalize_chart_text(item_id_raw, "item_id")?;
    let item_type = normalize_chart_text(item_type_raw, "type")?;
    let amount = parse_positive_i32_input(amount_raw, "amount")?;

    let purchase_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64`
         FROM purchase
         WHERE purchase_name = ?",
        &purchase_name
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;
    if purchase_exists <= 0 {
        return Err("购买项不存在".to_string());
    }

    let item_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64`
         FROM item
         WHERE item_id = ? AND type = ?",
        &item_id,
        &item_type
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;
    if item_exists <= 0 {
        return Err("物品不存在".to_string());
    }

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64`
         FROM purchase_item
         WHERE purchase_name = ? AND item_id = ? AND type = ?",
        &purchase_name,
        &item_id,
        &item_type
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;

    if exists > 0 {
        return Err("购买项物品已存在".to_string());
    }

    sqlx::query!(
        "INSERT INTO purchase_item (purchase_name, item_id, type, amount)
         VALUES (?, ?, ?, ?)",
        purchase_name,
        item_id,
        item_type,
        amount
    )
    .execute(pool)
    .await
    .map_err(|err| format!("新增失败: {err}"))?;

    Ok(())
}

async fn update_purchase_item(
    pool: &DbPool,
    purchase_name_raw: &str,
    item_id_raw: &str,
    item_type_raw: &str,
    amount_raw: Option<&str>,
) -> Result<(), String> {
    let purchase_name = normalize_chart_text(purchase_name_raw, "purchase_name")?;
    let item_id = normalize_chart_text(item_id_raw, "item_id")?;
    let item_type = normalize_chart_text(item_type_raw, "type")?;
    let amount = parse_positive_i32_input(amount_raw, "amount")?;

    let done = sqlx::query!(
        "UPDATE purchase_item
         SET amount = ?
         WHERE purchase_name = ? AND item_id = ? AND type = ?",
        amount,
        purchase_name,
        item_id,
        item_type
    )
    .execute(pool)
    .await
    .map_err(|err| format!("更新失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("购买项物品不存在".to_string());
    }

    Ok(())
}

async fn delete_purchase_item(
    pool: &DbPool,
    purchase_name_raw: &str,
    item_id_raw: &str,
    item_type_raw: &str,
) -> Result<(), String> {
    let purchase_name = normalize_chart_text(purchase_name_raw, "purchase_name")?;
    let item_id = normalize_chart_text(item_id_raw, "item_id")?;
    let item_type = normalize_chart_text(item_type_raw, "type")?;

    let done = sqlx::query!(
        "DELETE FROM purchase_item
         WHERE purchase_name = ? AND item_id = ? AND type = ?",
        purchase_name,
        item_id,
        item_type
    )
    .execute(pool)
    .await
    .map_err(|err| format!("删除失败: {err}"))?;

    if done.rows_affected() <= 0 {
        return Err("购买项物品不存在".to_string());
    }

    Ok(())
}

fn admin_api_input_error(message: String) -> ArcError {
    ArcError::input(message)
}

#[get("/api/session")]
pub fn admin_api_session(cookies: &CookieJar<'_>) -> RouteResult<AdminSessionResponse> {
    Ok(success_return(AdminSessionResponse {
        logged_in: is_admin_logged_in(cookies),
    }))
}

#[post("/api/login", format = "json", data = "<payload>")]
pub fn admin_api_login(
    payload: Json<AdminLoginRequest>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminSessionResponse> {
    let (username, password) = admin_credentials();
    if payload.username == username && payload.password == password {
        set_admin_cookie(cookies);
        Ok(success_return(AdminSessionResponse { logged_in: true }))
    } else {
        Err(ArcError::no_access("Incorrect username or password", 401))
    }
}

#[post("/api/logout")]
pub fn admin_api_logout(cookies: &CookieJar<'_>) -> RouteResult<EmptyResponse> {
    clear_admin_cookie(cookies);
    Ok(success_return_no_value())
}

#[get("/api/dashboard")]
pub async fn admin_api_dashboard(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminDashboardApiResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(load_dashboard_api(pool.inner()).await))
}

#[post("/api/operations/<operation_name>")]
pub async fn admin_api_operation(
    operation_name: &str,
    operation_manager: &State<OperationManager>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;

    match operation_name {
        "refresh_song_file_cache" | "refresh_content_bundle_cache" | "refresh_all_score_rating" => {
            operation_manager
                .execute_operation(operation_name, None)
                .await?;
            Ok(success_return_no_value())
        }
        _ => Err(ArcError::input("Unsupported admin operation")),
    }
}

#[get("/api/users?<q>&<status>&<page>&<page_size>")]
pub async fn admin_api_users(
    q: Option<&str>,
    status: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<UserListView>> {
    require_admin_api(cookies)?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_users(q, status, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/songs?<q>&<page>&<page_size>")]
pub async fn admin_api_songs(
    q: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<SongRowView>> {
    require_admin_api(cookies)?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_songs(q, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/items?<q>&<page>&<page_size>")]
pub async fn admin_api_items(
    q: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<ItemRowView>> {
    require_admin_api(cookies)?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_items(q, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/purchases?<pq>&<page>&<page_size>")]
pub async fn admin_api_purchases(
    pq: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<PurchaseRowView>> {
    require_admin_api(cookies)?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_purchases(pq, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/purchase-items?<iq>&<page>&<page_size>")]
pub async fn admin_api_purchase_items(
    iq: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<PurchaseItemRowView>> {
    require_admin_api(cookies)?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_purchase_items(iq, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/user-scores?<user_id>&<name>&<user_code>&<limit>")]
pub async fn admin_api_user_scores(
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    limit: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminUserScoresResponse> {
    require_admin_api(cookies)?;
    let query = AdminUserScoreQuery {
        user_id,
        name,
        user_code,
        limit,
    };
    Ok(success_return(
        load_admin_user_scores(&query, pool.inner()).await?,
    ))
}

#[get("/api/chart-top?<sid>&<difficulty>&<limit>")]
pub async fn admin_api_chart_top(
    sid: Option<&str>,
    difficulty: Option<i32>,
    limit: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminChartTopResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        load_admin_chart_top(sid, difficulty.unwrap_or(0), limit, pool.inner()).await?,
    ))
}

#[get("/api/redeem-users?<code>")]
pub async fn admin_api_redeem_users(
    code: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminRedeemUsersResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        load_admin_redeem_users(code, pool.inner()).await?,
    ))
}

#[post("/api/admin-actions/user-ticket", format = "json", data = "<payload>")]
pub async fn admin_api_user_ticket(
    payload: Json<AdminUserTicketPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        update_admin_user_ticket(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/user-password",
    format = "json",
    data = "<payload>"
)]
pub async fn admin_api_user_password(
    payload: Json<AdminUserPasswordPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        update_admin_user_password(&payload, pool.inner()).await?,
    ))
}

#[post("/api/admin-actions/user-ban", format = "json", data = "<payload>")]
pub async fn admin_api_user_ban(
    payload: Json<AdminUserSelectorPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        ban_admin_user(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/user-purchase",
    format = "json",
    data = "<payload>"
)]
pub async fn admin_api_user_purchase(
    payload: Json<AdminUserPurchasePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        update_admin_user_purchase(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/scores/delete",
    format = "json",
    data = "<payload>"
)]
pub async fn admin_api_scores_delete(
    payload: Json<AdminScoreDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        delete_admin_scores(&payload, pool.inner()).await?,
    ))
}

#[post("/api/admin-actions/presents", format = "json", data = "<payload>")]
pub async fn admin_api_present_create(
    payload: Json<AdminPresentPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        create_admin_present(&payload, pool.inner()).await?,
    ))
}

#[delete("/api/admin-actions/presents", format = "json", data = "<payload>")]
pub async fn admin_api_present_delete(
    payload: Json<AdminPresentDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        delete_admin_present(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/presents/deliver",
    format = "json",
    data = "<payload>"
)]
pub async fn admin_api_present_deliver(
    payload: Json<AdminPresentDeliverPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        deliver_admin_present(&payload, pool.inner()).await?,
    ))
}

#[post("/api/admin-actions/redeems", format = "json", data = "<payload>")]
pub async fn admin_api_redeem_create(
    payload: Json<AdminRedeemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        create_admin_redeem(&payload, pool.inner()).await?,
    ))
}

#[delete("/api/admin-actions/redeems", format = "json", data = "<payload>")]
pub async fn admin_api_redeem_delete(
    payload: Json<AdminRedeemDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        delete_admin_redeem(&payload, pool.inner()).await?,
    ))
}

#[post("/api/songs", format = "json", data = "<payload>")]
pub async fn admin_api_song_create(
    payload: Json<AdminSongPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    create_song(
        pool.inner(),
        &payload.sid,
        &payload.name_en,
        &payload.rating_pst,
        &payload.rating_prs,
        &payload.rating_ftr,
        &payload.rating_byd,
        &payload.rating_etr,
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[patch("/api/songs/<sid>", format = "json", data = "<payload>")]
pub async fn admin_api_song_update(
    sid: &str,
    payload: Json<AdminSongPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    update_song(
        pool.inner(),
        sid,
        &payload.name_en,
        &payload.rating_pst,
        &payload.rating_prs,
        &payload.rating_ftr,
        &payload.rating_byd,
        &payload.rating_etr,
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[delete("/api/songs", format = "json", data = "<payload>")]
pub async fn admin_api_song_delete(
    payload: Json<AdminSongDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    delete_song(pool.inner(), &payload.sid)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[post("/api/items", format = "json", data = "<payload>")]
pub async fn admin_api_item_create(
    payload: Json<AdminItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    create_item(
        pool.inner(),
        &payload.item_id,
        &payload.item_type,
        payload.is_available,
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[patch("/api/items", format = "json", data = "<payload>")]
pub async fn admin_api_item_update(
    payload: Json<AdminItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    update_item(
        pool.inner(),
        &payload.item_id,
        &payload.item_type,
        payload.is_available,
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[delete("/api/items", format = "json", data = "<payload>")]
pub async fn admin_api_item_delete(
    payload: Json<AdminItemDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    delete_item(pool.inner(), &payload.item_id, &payload.item_type)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[post("/api/purchases", format = "json", data = "<payload>")]
pub async fn admin_api_purchase_create(
    payload: Json<AdminPurchasePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    create_purchase(
        pool.inner(),
        &payload.purchase_name,
        payload.price.as_deref(),
        payload.orig_price.as_deref(),
        payload.discount_from.as_deref(),
        payload.discount_to.as_deref(),
        payload.discount_reason.as_deref(),
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[patch("/api/purchases/<purchase_name>", format = "json", data = "<payload>")]
pub async fn admin_api_purchase_update(
    purchase_name: &str,
    payload: Json<AdminPurchasePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    update_purchase(
        pool.inner(),
        purchase_name,
        payload.price.as_deref(),
        payload.orig_price.as_deref(),
        payload.discount_from.as_deref(),
        payload.discount_to.as_deref(),
        payload.discount_reason.as_deref(),
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[delete("/api/purchases", format = "json", data = "<payload>")]
pub async fn admin_api_purchase_delete(
    payload: Json<AdminPurchaseDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    delete_purchase(pool.inner(), &payload.purchase_name)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[post("/api/purchase-items", format = "json", data = "<payload>")]
pub async fn admin_api_purchase_item_create(
    payload: Json<AdminPurchaseItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    create_purchase_item(
        pool.inner(),
        &payload.purchase_name,
        &payload.item_id,
        &payload.item_type,
        payload.amount.as_deref(),
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[patch("/api/purchase-items", format = "json", data = "<payload>")]
pub async fn admin_api_purchase_item_update(
    payload: Json<AdminPurchaseItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    update_purchase_item(
        pool.inner(),
        &payload.purchase_name,
        &payload.item_id,
        &payload.item_type,
        payload.amount.as_deref(),
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[delete("/api/purchase-items", format = "json", data = "<payload>")]
pub async fn admin_api_purchase_item_delete(
    payload: Json<AdminPurchaseItemDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies)?;
    delete_purchase_item(
        pool.inner(),
        &payload.purchase_name,
        &payload.item_id,
        &payload.item_type,
    )
    .await
    .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

pub fn routes() -> Vec<Route> {
    routes![
        admin_api_session,
        admin_api_login,
        admin_api_logout,
        admin_api_dashboard,
        admin_api_operation,
        admin_api_users,
        admin_api_songs,
        admin_api_items,
        admin_api_purchases,
        admin_api_purchase_items,
        admin_api_user_scores,
        admin_api_chart_top,
        admin_api_redeem_users,
        admin_api_user_ticket,
        admin_api_user_password,
        admin_api_user_ban,
        admin_api_user_purchase,
        admin_api_scores_delete,
        admin_api_present_create,
        admin_api_present_delete,
        admin_api_present_deliver,
        admin_api_redeem_create,
        admin_api_redeem_delete,
        admin_api_song_create,
        admin_api_song_update,
        admin_api_song_delete,
        admin_api_item_create,
        admin_api_item_update,
        admin_api_item_delete,
        admin_api_purchase_create,
        admin_api_purchase_update,
        admin_api_purchase_delete,
        admin_api_purchase_item_create,
        admin_api_purchase_item_update,
        admin_api_purchase_item_delete,
    ]
}
