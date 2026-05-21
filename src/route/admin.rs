use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::serde::json::Json;
use rocket::{delete, get, patch, post, routes, Route, State};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
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
pub struct AdminPurchaseApiResponse {
    purchases: Vec<PurchaseRowView>,
    purchase_items: Vec<PurchaseItemRowView>,
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

fn admin_unauthorized() -> ArcError {
    ArcError::no_access("Admin login required", 401)
}

fn require_admin_api(cookies: &CookieJar<'_>) -> Result<(), ArcError> {
    is_admin_logged_in(cookies)
        .then_some(())
        .ok_or_else(admin_unauthorized)
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
    pool: &DbPool,
) -> Vec<UserListView> {
    let keyword = clean_query_value(q);
    let status = status.map(str::trim);

    let rows = match (keyword.as_deref(), status) {
        (Some(kw), Some("banned")) => {
            let like = format!("%{kw}%");
            sqlx::query_as!(
                UserListDbRow,
                "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
                 FROM user
                 WHERE (name LIKE ? OR user_code LIKE ?) AND COALESCE(password, '') = ''
                 ORDER BY rating_ptt DESC, user_id ASC
                 LIMIT 300",
                &like,
                &like
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        }
        (Some(kw), Some("normal")) => {
            let like = format!("%{kw}%");
            sqlx::query_as!(
                UserListDbRow,
                "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
                 FROM user
                 WHERE (name LIKE ? OR user_code LIKE ?) AND COALESCE(password, '') <> ''
                 ORDER BY rating_ptt DESC, user_id ASC
                 LIMIT 300",
                &like,
                &like
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        }
        (Some(kw), _) => {
            let like = format!("%{kw}%");
            sqlx::query_as!(
                UserListDbRow,
                "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
                 FROM user
                 WHERE (name LIKE ? OR user_code LIKE ?)
                 ORDER BY rating_ptt DESC, user_id ASC
                 LIMIT 300",
                &like,
                &like
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        }
        (None, Some("banned")) => sqlx::query_as!(
            UserListDbRow,
            "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
             FROM user
             WHERE COALESCE(password, '') = ''
             ORDER BY rating_ptt DESC, user_id ASC
             LIMIT 300"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default(),
        (None, Some("normal")) => sqlx::query_as!(
            UserListDbRow,
            "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
             FROM user
             WHERE COALESCE(password, '') <> ''
             ORDER BY rating_ptt DESC, user_id ASC
             LIMIT 300"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default(),
        (None, _) => sqlx::query_as!(
            UserListDbRow,
            "SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password
             FROM user
             ORDER BY rating_ptt DESC, user_id ASC
             LIMIT 300"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default(),
    };

    rows.into_iter()
        .map(|row| UserListView {
            user_id: row.user_id,
            name: row.name.unwrap_or_default(),
            user_code: row.user_code.unwrap_or_default(),
            rating_ptt: row.rating_ptt.unwrap_or(0),
            ticket: row.ticket.unwrap_or(0),
            last_play: format_timestamp(row.time_played),
            banned: row.password.unwrap_or_default().is_empty(),
        })
        .collect()
}

async fn load_admin_songs(q: Option<&str>, pool: &DbPool) -> Vec<SongRowView> {
    let query = clean_query_value(q).unwrap_or_default();

    let db_rows = if query.is_empty() {
        sqlx::query_as!(
            ChartDbRow,
            "SELECT song_id, name, rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr
             FROM chart
             ORDER BY song_id ASC"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        let like = format!("%{query}%");
        sqlx::query_as!(
            ChartDbRow,
            "SELECT song_id, name, rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr
             FROM chart
             WHERE song_id LIKE ? OR name LIKE ?
             ORDER BY song_id ASC",
            like,
            like
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    db_rows.into_iter().map(chart_db_row_to_song_view).collect()
}

async fn load_admin_items(q: Option<&str>, pool: &DbPool) -> Vec<ItemRowView> {
    let query = clean_query_value(q).unwrap_or_default();

    let db_rows = if query.is_empty() {
        sqlx::query_as!(
            ItemDbRow,
            "SELECT item_id, type, is_available
             FROM item
             ORDER BY type, item_id",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        let like = format!("%{query}%");
        sqlx::query_as!(
            ItemDbRow,
            "SELECT item_id, type, is_available
             FROM item
             WHERE item_id LIKE ? OR type LIKE ?
             ORDER BY type, item_id",
            like,
            like
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    db_rows.into_iter().map(item_db_row_to_item_view).collect()
}

async fn load_admin_purchases(
    pq: Option<&str>,
    iq: Option<&str>,
    pool: &DbPool,
) -> AdminPurchaseApiResponse {
    let query_purchase = clean_query_value(pq).unwrap_or_default();
    let query_purchase_item = clean_query_value(iq).unwrap_or_default();

    let purchase_rows = if query_purchase.is_empty() {
        sqlx::query_as!(
            PurchaseDbRow,
            "SELECT purchase_name, price, orig_price, discount_from, discount_to, discount_reason
             FROM purchase
             ORDER BY purchase_name ASC",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        let like = format!("%{query_purchase}%");
        sqlx::query_as!(
            PurchaseDbRow,
            "SELECT purchase_name, price, orig_price, discount_from, discount_to, discount_reason
             FROM purchase
             WHERE purchase_name LIKE ? OR COALESCE(discount_reason, '') LIKE ?
             ORDER BY purchase_name ASC",
            like,
            like
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    let all_purchase_item_rows = sqlx::query_as!(
        PurchaseItemDbRow,
        "SELECT purchase_name, item_id, type, amount
         FROM purchase_item
         ORDER BY purchase_name ASC, item_id ASC, type ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let purchase_names = purchase_rows
        .iter()
        .map(|row| row.purchase_name.clone())
        .collect::<HashSet<_>>();
    let mut item_summaries: HashMap<String, Vec<String>> = HashMap::new();
    for item in &all_purchase_item_rows {
        if !purchase_names.contains(&item.purchase_name) {
            continue;
        }
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

    let purchase_item_rows = if query_purchase_item.is_empty() {
        all_purchase_item_rows
    } else {
        let like = format!("%{query_purchase_item}%");
        sqlx::query_as!(
            PurchaseItemDbRow,
            "SELECT purchase_name, item_id, type, amount
             FROM purchase_item
             WHERE purchase_name LIKE ? OR item_id LIKE ? OR type LIKE ?
             ORDER BY purchase_name ASC, item_id ASC, type ASC",
            like,
            like,
            like
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    let purchase_items = purchase_item_rows
        .into_iter()
        .map(purchase_item_db_row_to_view)
        .collect();

    AdminPurchaseApiResponse {
        purchases,
        purchase_items,
    }
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

async fn delete_item(
    pool: &DbPool,
    item_id_raw: &str,
    item_type_raw: &str,
) -> Result<(), String> {
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

    let mut tx = pool.begin().await.map_err(|err| format!("事务创建失败: {err}"))?;

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
        "refresh_song_file_cache"
        | "refresh_content_bundle_cache"
        | "refresh_all_score_rating" => {
            operation_manager
                .execute_operation(operation_name, None)
                .await?;
            Ok(success_return_no_value())
        }
        _ => Err(ArcError::input("Unsupported admin operation")),
    }
}

#[get("/api/users?<q>&<status>")]
pub async fn admin_api_users(
    q: Option<&str>,
    status: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<Vec<UserListView>> {
    require_admin_api(cookies)?;
    Ok(success_return(load_admin_users(q, status, pool.inner()).await))
}

#[get("/api/songs?<q>")]
pub async fn admin_api_songs(
    q: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<Vec<SongRowView>> {
    require_admin_api(cookies)?;
    Ok(success_return(load_admin_songs(q, pool.inner()).await))
}

#[get("/api/items?<q>")]
pub async fn admin_api_items(
    q: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<Vec<ItemRowView>> {
    require_admin_api(cookies)?;
    Ok(success_return(load_admin_items(q, pool.inner()).await))
}

#[get("/api/purchases?<pq>&<iq>")]
pub async fn admin_api_purchases(
    pq: Option<&str>,
    iq: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPurchaseApiResponse> {
    require_admin_api(cookies)?;
    Ok(success_return(
        load_admin_purchases(pq, iq, pool.inner()).await,
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
