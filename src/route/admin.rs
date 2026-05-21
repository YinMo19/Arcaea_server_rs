use askama::Template;
use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use rocket::form::{Form, FromForm};
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::request::FlashMessage;
use rocket::response::content::{RawCss, RawHtml};
use rocket::response::{Flash, Redirect};
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
use crate::service::{OperationManager, ScoreService, UserService};
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

#[derive(Debug, Clone)]
struct UserEditView {
    user_id: i32,
    name: String,
    user_code: String,
    ticket: i32,
    rating_ptt: i32,
}

#[derive(Debug, Clone, Default)]
struct ScoreRowView {
    rank: i32,
    song_id: String,
    difficulty: String,
    score: i32,
    pure: String,
    far: i32,
    lost: i32,
    clear_type: String,
    best_clear_type: String,
    rating: String,
    time_played: String,
}

#[derive(Debug, Clone, Default)]
struct RecentRowView {
    index: i32,
    song_id: String,
    difficulty: String,
    rating: String,
    time_played: String,
}

#[derive(Debug, Clone, Default)]
struct ChartTopEntryView {
    rank: i32,
    user_id: i32,
    name: String,
    score: i32,
    pure: String,
    far: i32,
    lost: i32,
    clear_type: String,
    best_clear_type: String,
    time_played: String,
}

#[derive(Debug, Clone, Default)]
struct UserSummaryView {
    user_id: i32,
    name: String,
    user_code: String,
    ticket: i32,
    ptt: String,
    banned: bool,
}

#[derive(Debug, Clone, Default)]
struct UserPttView {
    user_id: i32,
    name: String,
    user_code: String,
    join_date: String,
    last_play_at: String,
    ticket: i32,
    ptt: String,
    banned: bool,

    last_song_id: String,
    last_difficulty: String,
    last_score: i32,
    last_pure: String,
    last_far: i32,
    last_lost: i32,
    last_clear_type: String,
    last_rating: String,
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

#[derive(Template)]
#[template(path = "admin/login.html")]
struct AdminLoginTemplate {
    flash_error: String,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct AdminDashboardTemplate {
    active_nav: &'static str,
    flash_kind: String,
    flash_message: String,
    online_users: i64,
    online_growth: f64,
    score_submits: i64,
    score_error_rate: f64,
    present_count: i64,
    alert_count: i64,
    recent_ops: Vec<RecentOpView>,
}

#[derive(Template)]
#[template(path = "admin/users.html")]
struct AdminUsersTemplate {
    active_nav: &'static str,
    users: Vec<UserListView>,
}

#[derive(Template)]
#[template(path = "admin/user_edit.html")]
struct AdminUserEditTemplate {
    active_nav: &'static str,
    user: UserEditView,
    flash_kind: String,
    flash_message: String,
}

#[derive(Template)]
#[template(path = "admin/simple_table.html")]
struct AdminSimpleTableTemplate {
    active_nav: &'static str,
    page_title: String,
    page_subtitle: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    empty_text: String,
}

#[derive(Template)]
#[template(path = "admin/allsong.html")]
struct AdminAllSongTemplate {
    active_nav: &'static str,
    query: String,
    flash_kind: String,
    flash_message: String,
    songs: Vec<SongRowView>,
}

#[derive(Template)]
#[template(path = "admin/allitem.html")]
struct AdminAllItemTemplate {
    active_nav: &'static str,
    query: String,
    flash_kind: String,
    flash_message: String,
    items: Vec<ItemRowView>,
}

#[derive(Template)]
#[template(path = "admin/allpurchase.html")]
struct AdminAllPurchaseTemplate {
    active_nav: &'static str,
    query_purchase: String,
    query_purchase_item: String,
    flash_kind: String,
    flash_message: String,
    purchases: Vec<PurchaseRowView>,
    purchase_items: Vec<PurchaseItemRowView>,
}

#[derive(Template)]
#[template(path = "admin/singleplayer.html")]
struct AdminSinglePlayerTemplate {
    active_nav: &'static str,
    query_name: String,
    query_user_code: String,
    message_kind: String,
    message: String,
    show_user: bool,
    user: UserSummaryView,
    scores: Vec<ScoreRowView>,
}

#[derive(Template)]
#[template(path = "admin/singleplayerptt.html")]
struct AdminSinglePlayerPttTemplate {
    active_nav: &'static str,
    query_name: String,
    query_user_code: String,
    message_kind: String,
    message: String,
    show_user: bool,
    user: UserPttView,
    user_has_last_play: bool,
    best30_avg: String,
    recent10_avg: String,
    best30: Vec<ScoreRowView>,
    recent30: Vec<RecentRowView>,
}

#[derive(Template)]
#[template(path = "admin/singlecharttop.html")]
struct AdminSingleChartTopTemplate {
    active_nav: &'static str,
    query_sid: String,
    difficulty: i32,
    limit: i32,
    message_kind: String,
    message: String,
    show_song: bool,
    song_id: String,
    song_name: String,
    difficulty_label: String,
    entries: Vec<ChartTopEntryView>,
}

#[derive(FromForm)]
pub struct AdminLoginForm {
    pub username: String,
    pub password: String,
}

#[derive(FromForm)]
pub struct UserTicketForm {
    pub ticket: i32,
}

#[derive(FromForm)]
pub struct PlayerLookupForm {
    pub name: Option<String>,
    pub user_code: Option<String>,
}

#[derive(FromForm)]
pub struct ChartTopForm {
    pub sid: Option<String>,
    pub difficulty: Option<i32>,
    pub limit: Option<i32>,
}

#[derive(FromForm)]
pub struct SongCrudForm {
    pub sid: String,
    pub name_en: String,
    pub rating_pst: String,
    pub rating_prs: String,
    pub rating_ftr: String,
    pub rating_byd: String,
    pub rating_etr: String,
}

#[derive(FromForm)]
pub struct SongDeleteForm {
    pub sid: String,
}

#[derive(FromForm)]
pub struct ItemAddForm {
    pub item_id: String,
    pub item_type: String,
    pub is_available: Option<i32>,
}

#[derive(FromForm)]
pub struct ItemUpdateForm {
    pub item_id: String,
    pub item_type: String,
    pub is_available: Option<i32>,
}

#[derive(FromForm)]
pub struct ItemDeleteForm {
    pub item_id: String,
    pub item_type: String,
}

#[derive(FromForm)]
pub struct PurchaseAddForm {
    pub purchase_name: String,
    pub price: Option<String>,
    pub orig_price: Option<String>,
    pub discount_from: Option<String>,
    pub discount_to: Option<String>,
    pub discount_reason: Option<String>,
}

#[derive(FromForm)]
pub struct PurchaseUpdateForm {
    pub purchase_name: String,
    pub price: Option<String>,
    pub orig_price: Option<String>,
    pub discount_from: Option<String>,
    pub discount_to: Option<String>,
    pub discount_reason: Option<String>,
}

#[derive(FromForm)]
pub struct PurchaseDeleteForm {
    pub purchase_name: String,
}

#[derive(FromForm)]
pub struct PurchaseItemAddForm {
    pub purchase_name: String,
    pub item_id: String,
    pub item_type: String,
    pub amount: Option<String>,
}

#[derive(FromForm)]
pub struct PurchaseItemUpdateForm {
    pub purchase_name: String,
    pub item_id: String,
    pub item_type: String,
    pub amount: Option<String>,
}

#[derive(FromForm)]
pub struct PurchaseItemDeleteForm {
    pub purchase_name: String,
    pub item_id: String,
    pub item_type: String,
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
struct UserDetailDbRow {
    user_id: i32,
    name: Option<String>,
    user_code: Option<String>,
    rating_ptt: Option<i32>,
    ticket: Option<i32>,
}

#[derive(FromRow)]
struct UserBasicQueryDbRow {
    user_id: i32,
    name: Option<String>,
    user_code: Option<String>,
    rating_ptt: Option<i32>,
    ticket: Option<i32>,
    password: Option<String>,
}

#[derive(FromRow)]
struct UserPttQueryDbRow {
    user_id: i32,
    name: Option<String>,
    user_code: Option<String>,
    join_date: Option<i64>,
    rating_ptt: Option<i32>,
    ticket: Option<i32>,
    password: Option<String>,
    song_id: Option<String>,
    difficulty: Option<i32>,
    score: Option<i32>,
    shiny_perfect_count: Option<i32>,
    perfect_count: Option<i32>,
    near_count: Option<i32>,
    miss_count: Option<i32>,
    time_played: Option<i64>,
    clear_type: Option<i32>,
    rating: Option<f64>,
}

#[derive(FromRow)]
struct BestScoreQueryDbRow {
    song_id: String,
    difficulty: i32,
    score: Option<i32>,
    shiny_perfect_count: Option<i32>,
    perfect_count: Option<i32>,
    near_count: Option<i32>,
    miss_count: Option<i32>,
    time_played: Option<i64>,
    best_clear_type: Option<i32>,
    clear_type: Option<i32>,
    rating: Option<f64>,
}

#[derive(FromRow)]
struct Recent30QueryDbRow {
    song_id: Option<String>,
    difficulty: Option<i32>,
    rating: Option<f64>,
    time_played: Option<i64>,
}

#[derive(FromRow)]
struct ChartLookupDbRow {
    song_id: String,
    name: Option<String>,
}

#[derive(FromRow)]
struct ChartTopEntryDbRow {
    user_id: i32,
    name: Option<String>,
    score: Option<i32>,
    shiny_perfect_count: Option<i32>,
    perfect_count: Option<i32>,
    near_count: Option<i32>,
    miss_count: Option<i32>,
    time_played: Option<i64>,
    clear_type: Option<i32>,
    best_clear_type: Option<i32>,
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
struct CharacterDbRow {
    character_id: i32,
    name: Option<String>,
    max_level: Option<i32>,
    skill_id: Option<String>,
    skill_id_uncap: Option<String>,
    is_uncapped: Option<i8>,
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

#[derive(FromRow)]
struct PurchaseItemDbRow {
    purchase_name: String,
    item_id: String,
    r#type: String,
    amount: Option<i32>,
}

#[derive(FromRow)]
struct PresentDbRow {
    present_id: String,
    expire_ts: Option<i64>,
    description: Option<String>,
}

#[derive(FromRow)]
struct RedeemDbRow {
    code: String,
    r#type: Option<i32>,
}

#[derive(FromRow)]
struct CollectionItemDbRow {
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

fn render_template<T: Template>(template: &T) -> RawHtml<String> {
    match template.render() {
        Ok(html) => RawHtml(html),
        Err(err) => RawHtml(format!("<h1>Template render error</h1><pre>{err}</pre>")),
    }
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

fn format_ptt_hundredths(value: Option<i32>) -> String {
    match value {
        Some(v) if v >= 0 => format!("{:.2}", v as f64 / 100.0),
        _ => "-".to_string(),
    }
}

fn format_rating(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v:.4}"),
        None => "-".to_string(),
    }
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

fn difficulty_label(difficulty: i32) -> &'static str {
    match difficulty {
        0 => "PST",
        1 => "PRS",
        2 => "FTR",
        3 => "BYD",
        4 => "ETR",
        _ => "?",
    }
}

fn clear_type_label(clear_type: i32) -> &'static str {
    match clear_type {
        3 => "Pure Memory",
        2 => "Full Recall",
        5 => "Hard Clear",
        1 => "Normal Clear",
        4 => "Easy Clear",
        _ => "Track Lost",
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
            name: row.name.unwrap_or_else(|| "".to_string()),
            user_code: row.user_code.unwrap_or_else(|| "".to_string()),
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

    let mut purchases = Vec::with_capacity(purchase_rows.len());
    for purchase in purchase_rows {
        let items = sqlx::query_as!(
            CollectionItemDbRow,
            "SELECT item_id, type, amount
             FROM purchase_item
             WHERE purchase_name = ?
             ORDER BY item_id ASC, type ASC",
            &purchase.purchase_name
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let item_summary = if items.is_empty() {
            "-".to_string()
        } else {
            items
                .into_iter()
                .map(|i| format!("{}:{}x{}", i.item_id, i.r#type, i.amount.unwrap_or(1)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        purchases.push(purchase_db_row_to_purchase_view(purchase, item_summary));
    }

    let purchase_item_rows = if query_purchase_item.is_empty() {
        sqlx::query_as!(
            PurchaseItemDbRow,
            "SELECT purchase_name, item_id, type, amount
             FROM purchase_item
             ORDER BY purchase_name ASC, item_id ASC, type ASC",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
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

struct AdminRequired;

impl From<AdminRequired> for Redirect {
    fn from(_: AdminRequired) -> Self {
        Redirect::to("/web/login")
    }
}

fn require_admin(cookies: &CookieJar<'_>) -> Result<(), AdminRequired> {
    is_admin_logged_in(cookies)
        .then_some(())
        .ok_or(AdminRequired)
}

#[get("/static/admin.css")]
pub fn admin_css() -> RawCss<&'static str> {
    RawCss(include_str!("../../templates/admin/admin.css"))
}

#[get("/login")]
pub fn admin_login_get(
    flash: Option<FlashMessage<'_>>,
    cookies: &CookieJar<'_>,
) -> RawHtml<String> {
    let flash_error = flash
        .filter(|m| m.kind() == "error")
        .map(|m| m.message().to_string())
        .unwrap_or_default();

    if is_admin_logged_in(cookies) {
        let template = AdminLoginTemplate {
            flash_error: "已登录，可直接访问 /web".to_string(),
        };
        return render_template(&template);
    }

    let template = AdminLoginTemplate { flash_error };
    render_template(&template)
}

#[post("/login", data = "<form>")]
pub fn admin_login_post(form: Form<AdminLoginForm>, cookies: &CookieJar<'_>) -> Flash<Redirect> {
    let (username, password) = admin_credentials();
    if form.username == username && form.password == password {
        set_admin_cookie(cookies);
        Flash::success(Redirect::to("/web"), "登录成功")
    } else {
        Flash::error(
            Redirect::to("/web/login"),
            "错误的用户名或密码 Incorrect username or password.",
        )
    }
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
        Err(ArcError::no_access(
            "Incorrect username or password",
            401,
        ))
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

#[get("/logout")]
pub fn admin_logout_get(cookies: &CookieJar<'_>) -> Flash<Redirect> {
    clear_admin_cookie(cookies);
    Flash::success(Redirect::to("/web/login"), "成功登出")
}

#[get("/")]
pub async fn admin_dashboard(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;
    let data = load_dashboard_api(pool.inner()).await;

    let (flash_kind, flash_message) = flash
        .map(|msg| (msg.kind().to_string(), msg.message().to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));

    let template = AdminDashboardTemplate {
        active_nav: "dashboard",
        flash_kind,
        flash_message,
        online_users: data.online_users,
        online_growth: data.online_growth,
        score_submits: data.score_submits,
        score_error_rate: data.score_error_rate,
        present_count: data.present_count,
        alert_count: data.alert_count,
        recent_ops: data.recent_ops,
    };

    Ok(render_template(&template))
}

#[get("/index")]
pub fn admin_dashboard_index() -> Redirect {
    Redirect::to("/web")
}

#[post("/updatedatabase/refreshsonghash")]
pub async fn admin_refresh_song_hash_post(
    operation_manager: &State<OperationManager>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match operation_manager
        .execute_operation("refresh_song_file_cache", None)
        .await
    {
        Ok(_) => Flash::success(Redirect::to("/web"), "Song Hash 刷新成功"),
        Err(err) => Flash::error(Redirect::to("/web"), format!("Song Hash 刷新失败: {err}")),
    }
}

#[post("/updatedatabase/refreshsbundle")]
pub async fn admin_refresh_bundle_post(
    operation_manager: &State<OperationManager>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match operation_manager
        .execute_operation("refresh_content_bundle_cache", None)
        .await
    {
        Ok(_) => Flash::success(Redirect::to("/web"), "Bundle 刷新成功"),
        Err(err) => Flash::error(Redirect::to("/web"), format!("Bundle 刷新失败: {err}")),
    }
}

#[post("/updatedatabase/refreshsongrating")]
pub async fn admin_refresh_song_rating_post(
    operation_manager: &State<OperationManager>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match operation_manager
        .execute_operation("refresh_all_score_rating", None)
        .await
    {
        Ok(_) => Flash::success(Redirect::to("/web"), "Rating 全量重算完成"),
        Err(err) => Flash::error(Redirect::to("/web"), format!("Rating 重算失败: {err}")),
    }
}

#[get("/users?<q>&<status>")]
pub async fn admin_users_get(
    q: Option<&str>,
    status: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;
    let users = load_admin_users(q, status, pool.inner()).await;

    let template = AdminUsersTemplate {
        active_nav: "users",
        users,
    };

    Ok(render_template(&template))
}

#[get("/allplayer?<q>&<status>")]
pub async fn admin_allplayer_get(
    q: Option<&str>,
    status: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    admin_users_get(q, status, pool, cookies).await
}

#[get("/users/<user_id>")]
pub async fn admin_user_detail_get(
    user_id: i32,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
    flash: Option<FlashMessage<'_>>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let user_row = sqlx::query_as!(
        UserDetailDbRow,
        "SELECT user_id, name, user_code, rating_ptt, ticket FROM user WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    let Some(user_row) = user_row else {
        return Err(Redirect::to("/web/users"));
    };

    let (flash_kind, flash_message) = flash
        .map(|msg| (msg.kind().to_string(), msg.message().to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));

    let template = AdminUserEditTemplate {
        active_nav: "users",
        user: UserEditView {
            user_id: user_row.user_id,
            name: user_row.name.unwrap_or_default(),
            user_code: user_row.user_code.unwrap_or_default(),
            ticket: user_row.ticket.unwrap_or(0),
            rating_ptt: user_row.rating_ptt.unwrap_or(0),
        },
        flash_kind,
        flash_message,
    };

    Ok(render_template(&template))
}

#[get("/allsong?<q>")]
pub async fn admin_allsong_get(
    q: Option<&str>,
    flash: Option<FlashMessage<'_>>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let query = clean_query_value(q).unwrap_or_default();
    let songs = load_admin_songs(q, pool.inner()).await;

    let (flash_kind, flash_message) = flash
        .map(|msg| (msg.kind().to_string(), msg.message().to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));

    let template = AdminAllSongTemplate {
        active_nav: "scores",
        query,
        flash_kind,
        flash_message,
        songs,
    };

    Ok(render_template(&template))
}

#[get("/changesong")]
pub async fn admin_changesong_get(cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    require_admin(cookies)?;
    Ok(Redirect::to("/web/allsong"))
}

#[post("/changesong/addsong", data = "<form>")]
pub async fn admin_changesong_add_post(
    form: Form<SongCrudForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match create_song(
        pool.inner(),
        &form.sid,
        &form.name_en,
        &form.rating_pst,
        &form.rating_prs,
        &form.rating_ftr,
        &form.rating_byd,
        &form.rating_etr,
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allsong"), "歌曲新增成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allsong"), msg),
    }
}

#[post("/changesong/updatesong", data = "<form>")]
pub async fn admin_changesong_update_post(
    form: Form<SongCrudForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match update_song(
        pool.inner(),
        &form.sid,
        &form.name_en,
        &form.rating_pst,
        &form.rating_prs,
        &form.rating_ftr,
        &form.rating_byd,
        &form.rating_etr,
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allsong"), "歌曲更新成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allsong"), msg),
    }
}

#[post("/changesong/deletesong", data = "<form>")]
pub async fn admin_changesong_delete_post(
    form: Form<SongDeleteForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match delete_song(pool.inner(), &form.sid).await {
        Ok(()) => Flash::success(Redirect::to("/web/allsong"), "歌曲删除成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allsong"), msg),
    }
}

#[get("/singleplayer")]
pub async fn admin_singleplayer_get(cookies: &CookieJar<'_>) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let template = AdminSinglePlayerTemplate {
        active_nav: "scores",
        query_name: String::new(),
        query_user_code: String::new(),
        message_kind: String::new(),
        message: String::new(),
        show_user: false,
        user: UserSummaryView::default(),
        scores: Vec::new(),
    };

    Ok(render_template(&template))
}

#[post("/singleplayer", data = "<form>")]
pub async fn admin_singleplayer_post(
    form: Form<PlayerLookupForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let query_name = clean_query_value(form.name.as_deref());
    let query_user_code = clean_query_value(form.user_code.as_deref());

    if query_name.is_none() && query_user_code.is_none() {
        let template = AdminSinglePlayerTemplate {
            active_nav: "scores",
            query_name: String::new(),
            query_user_code: String::new(),
            message_kind: "error".to_string(),
            message: "输入为空 Null Input.".to_string(),
            show_user: false,
            user: UserSummaryView::default(),
            scores: Vec::new(),
        };
        return Ok(render_template(&template));
    }

    let user_row = if let Some(code) = &query_user_code {
        sqlx::query_as!(
            UserBasicQueryDbRow,
            "SELECT user_id, name, user_code, rating_ptt, ticket, password
             FROM user
             WHERE user_code = ?
             LIMIT 1",
            code
        )
        .fetch_optional(pool.inner())
        .await
        .unwrap_or(None)
    } else if let Some(name) = &query_name {
        sqlx::query_as!(
            UserBasicQueryDbRow,
            "SELECT user_id, name, user_code, rating_ptt, ticket, password
             FROM user
             WHERE name = ?
             LIMIT 1",
            name
        )
        .fetch_optional(pool.inner())
        .await
        .unwrap_or(None)
    } else {
        None
    };

    let Some(user_row) = user_row else {
        let template = AdminSinglePlayerTemplate {
            active_nav: "scores",
            query_name: query_name.unwrap_or_default(),
            query_user_code: query_user_code.unwrap_or_default(),
            message_kind: "error".to_string(),
            message: "玩家不存在 The player does not exist.".to_string(),
            show_user: false,
            user: UserSummaryView::default(),
            scores: Vec::new(),
        };
        return Ok(render_template(&template));
    };

    let score_rows = sqlx::query_as!(
        BestScoreQueryDbRow,
        "SELECT song_id, difficulty, score, shiny_perfect_count, perfect_count, near_count, miss_count,
                time_played, best_clear_type, clear_type, rating
         FROM best_score
         WHERE user_id = ?
         ORDER BY rating DESC",
        user_row.user_id
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let scores = score_rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| ScoreRowView {
            rank: (idx + 1) as i32,
            song_id: row.song_id,
            difficulty: difficulty_label(row.difficulty).to_string(),
            score: row.score.unwrap_or(0),
            pure: format!(
                "{} ({})",
                row.perfect_count.unwrap_or(0),
                row.shiny_perfect_count.unwrap_or(0)
            ),
            far: row.near_count.unwrap_or(0),
            lost: row.miss_count.unwrap_or(0),
            clear_type: clear_type_label(row.clear_type.unwrap_or(0)).to_string(),
            best_clear_type: clear_type_label(row.best_clear_type.unwrap_or(0)).to_string(),
            rating: format_rating(row.rating),
            time_played: format_timestamp(row.time_played),
        })
        .collect::<Vec<_>>();

    let user = UserSummaryView {
        user_id: user_row.user_id,
        name: user_row.name.unwrap_or_default(),
        user_code: user_row.user_code.unwrap_or_default(),
        ticket: user_row.ticket.unwrap_or(0),
        ptt: format_ptt_hundredths(user_row.rating_ptt),
        banned: user_row.password.unwrap_or_default().is_empty(),
    };

    let (message_kind, message) = if scores.is_empty() {
        ("error".to_string(), "无成绩 No score.".to_string())
    } else {
        (String::new(), String::new())
    };

    let template = AdminSinglePlayerTemplate {
        active_nav: "scores",
        query_name: query_name.unwrap_or_default(),
        query_user_code: query_user_code.unwrap_or_default(),
        message_kind,
        message,
        show_user: true,
        user,
        scores,
    };

    Ok(render_template(&template))
}

#[get("/singleplayerptt")]
pub async fn admin_singleplayerptt_get(
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let template = AdminSinglePlayerPttTemplate {
        active_nav: "scores",
        query_name: String::new(),
        query_user_code: String::new(),
        message_kind: String::new(),
        message: String::new(),
        show_user: false,
        user: UserPttView::default(),
        user_has_last_play: false,
        best30_avg: "-".to_string(),
        recent10_avg: "-".to_string(),
        best30: Vec::new(),
        recent30: Vec::new(),
    };

    Ok(render_template(&template))
}

#[post("/singleplayerptt", data = "<form>")]
pub async fn admin_singleplayerptt_post(
    form: Form<PlayerLookupForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let query_name = clean_query_value(form.name.as_deref());
    let query_user_code = clean_query_value(form.user_code.as_deref());

    if query_name.is_none() && query_user_code.is_none() {
        let template = AdminSinglePlayerPttTemplate {
            active_nav: "scores",
            query_name: String::new(),
            query_user_code: String::new(),
            message_kind: "error".to_string(),
            message: "输入为空 Null Input.".to_string(),
            show_user: false,
            user: UserPttView::default(),
            user_has_last_play: false,
            best30_avg: "-".to_string(),
            recent10_avg: "-".to_string(),
            best30: Vec::new(),
            recent30: Vec::new(),
        };
        return Ok(render_template(&template));
    }

    let user_row = if let Some(code) = &query_user_code {
        sqlx::query_as!(
            UserPttQueryDbRow,
            "SELECT user_id, name, user_code, join_date, rating_ptt, ticket, password,
                    song_id, difficulty, score, shiny_perfect_count, perfect_count, near_count,
                    miss_count, time_played, clear_type, rating
             FROM user
             WHERE user_code = ?
             LIMIT 1",
            code
        )
        .fetch_optional(pool.inner())
        .await
        .unwrap_or(None)
    } else if let Some(name) = &query_name {
        sqlx::query_as!(
            UserPttQueryDbRow,
            "SELECT user_id, name, user_code, join_date, rating_ptt, ticket, password,
                    song_id, difficulty, score, shiny_perfect_count, perfect_count, near_count,
                    miss_count, time_played, clear_type, rating
             FROM user
             WHERE name = ?
             LIMIT 1",
            name
        )
        .fetch_optional(pool.inner())
        .await
        .unwrap_or(None)
    } else {
        None
    };

    let Some(user_row) = user_row else {
        let template = AdminSinglePlayerPttTemplate {
            active_nav: "scores",
            query_name: query_name.unwrap_or_default(),
            query_user_code: query_user_code.unwrap_or_default(),
            message_kind: "error".to_string(),
            message: "玩家不存在 The player does not exist.".to_string(),
            show_user: false,
            user: UserPttView::default(),
            user_has_last_play: false,
            best30_avg: "-".to_string(),
            recent10_avg: "-".to_string(),
            best30: Vec::new(),
            recent30: Vec::new(),
        };
        return Ok(render_template(&template));
    };

    let best30_rows = sqlx::query_as!(
        BestScoreQueryDbRow,
        "SELECT song_id, difficulty, score, shiny_perfect_count, perfect_count, near_count, miss_count,
                time_played, best_clear_type, clear_type, rating
         FROM best_score
         WHERE user_id = ?
         ORDER BY rating DESC
         LIMIT 30",
        user_row.user_id
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let best30_sum: f64 = best30_rows.iter().map(|r| r.rating.unwrap_or(0.0)).sum();
    let best30_avg = best30_sum / 30.0;

    let best30 = best30_rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| ScoreRowView {
            rank: (idx + 1) as i32,
            song_id: row.song_id,
            difficulty: difficulty_label(row.difficulty).to_string(),
            score: row.score.unwrap_or(0),
            pure: format!(
                "{} ({})",
                row.perfect_count.unwrap_or(0),
                row.shiny_perfect_count.unwrap_or(0)
            ),
            far: row.near_count.unwrap_or(0),
            lost: row.miss_count.unwrap_or(0),
            clear_type: clear_type_label(row.clear_type.unwrap_or(0)).to_string(),
            best_clear_type: clear_type_label(row.best_clear_type.unwrap_or(0)).to_string(),
            rating: format_rating(row.rating),
            time_played: format_timestamp(row.time_played),
        })
        .collect::<Vec<_>>();

    let recent30_rows = sqlx::query_as!(
        Recent30QueryDbRow,
        "SELECT song_id, difficulty, rating, time_played
         FROM recent30
         WHERE user_id = ? AND song_id != ''
         ORDER BY time_played DESC",
        user_row.user_id
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let mut max_ratings: HashMap<(String, i32), f64> = HashMap::new();
    for row in &recent30_rows {
        let Some(song_id) = row.song_id.as_ref().filter(|s| !s.is_empty()) else {
            continue;
        };
        let difficulty = row.difficulty.unwrap_or(0);
        let rating = row.rating.unwrap_or(0.0);
        let key = (song_id.to_string(), difficulty);
        let current_max = max_ratings.get(&key).copied().unwrap_or(0.0);
        if rating > current_max {
            max_ratings.insert(key, rating);
        }
    }

    let mut recent_ratings: Vec<f64> = max_ratings.values().copied().collect();
    recent_ratings.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let recent10_sum: f64 = recent_ratings.into_iter().take(10).sum();
    let recent10_avg = recent10_sum / 10.0;

    let recent30 = recent30_rows
        .into_iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let song_id = row.song_id?;
            if song_id.is_empty() {
                return None;
            }
            Some(RecentRowView {
                index: (idx + 1) as i32,
                song_id,
                difficulty: difficulty_label(row.difficulty.unwrap_or(0)).to_string(),
                rating: format_rating(row.rating),
                time_played: format_timestamp(row.time_played),
            })
        })
        .collect::<Vec<_>>();

    let user_has_last_play = user_row
        .song_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();

    let user = UserPttView {
        user_id: user_row.user_id,
        name: user_row.name.unwrap_or_default(),
        user_code: user_row.user_code.unwrap_or_default(),
        join_date: format_timestamp(user_row.join_date),
        last_play_at: format_timestamp(user_row.time_played),
        ticket: user_row.ticket.unwrap_or(0),
        ptt: format_ptt_hundredths(user_row.rating_ptt),
        banned: user_row.password.unwrap_or_default().is_empty(),
        last_song_id: user_row.song_id.clone().unwrap_or_default(),
        last_difficulty: difficulty_label(user_row.difficulty.unwrap_or(0)).to_string(),
        last_score: user_row.score.unwrap_or(0),
        last_pure: format!(
            "{} ({})",
            user_row.perfect_count.unwrap_or(0),
            user_row.shiny_perfect_count.unwrap_or(0)
        ),
        last_far: user_row.near_count.unwrap_or(0),
        last_lost: user_row.miss_count.unwrap_or(0),
        last_clear_type: clear_type_label(user_row.clear_type.unwrap_or(0)).to_string(),
        last_rating: format_rating(user_row.rating),
    };

    let (message_kind, message) = if best30.is_empty() {
        ("error".to_string(), "无成绩 No score.".to_string())
    } else {
        (String::new(), String::new())
    };

    let template = AdminSinglePlayerPttTemplate {
        active_nav: "scores",
        query_name: query_name.unwrap_or_default(),
        query_user_code: query_user_code.unwrap_or_default(),
        message_kind,
        message,
        show_user: true,
        user,
        user_has_last_play,
        best30_avg: format!("{best30_avg:.4}"),
        recent10_avg: format!("{recent10_avg:.4}"),
        best30,
        recent30,
    };

    Ok(render_template(&template))
}

#[get("/singlecharttop")]
pub async fn admin_singlecharttop_get(
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let template = AdminSingleChartTopTemplate {
        active_nav: "scores",
        query_sid: String::new(),
        difficulty: 0,
        limit: 200,
        message_kind: String::new(),
        message: String::new(),
        show_song: false,
        song_id: String::new(),
        song_name: String::new(),
        difficulty_label: difficulty_label(0).to_string(),
        entries: Vec::new(),
    };

    Ok(render_template(&template))
}

#[post("/singlecharttop", data = "<form>")]
pub async fn admin_singlecharttop_post(
    form: Form<ChartTopForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let query_sid = clean_query_value(form.sid.as_deref()).unwrap_or_default();
    let difficulty = form.difficulty.unwrap_or(0).clamp(0, 4);
    let limit = form.limit.unwrap_or(200).clamp(1, 5000);

    if query_sid.is_empty() {
        let template = AdminSingleChartTopTemplate {
            active_nav: "scores",
            query_sid,
            difficulty,
            limit,
            message_kind: "error".to_string(),
            message: "输入为空 Null Input.".to_string(),
            show_song: false,
            song_id: String::new(),
            song_name: String::new(),
            difficulty_label: difficulty_label(difficulty).to_string(),
            entries: Vec::new(),
        };
        return Ok(render_template(&template));
    }

    let like = format!("%{query_sid}%");
    let chart = sqlx::query_as!(
        ChartLookupDbRow,
        "SELECT song_id, name
         FROM chart
         WHERE song_id LIKE ?
         LIMIT 1",
        like
    )
    .fetch_optional(pool.inner())
    .await
    .unwrap_or(None);

    let Some(chart) = chart else {
        let template = AdminSingleChartTopTemplate {
            active_nav: "scores",
            query_sid,
            difficulty,
            limit,
            message_kind: "error".to_string(),
            message: "查询为空 No song.".to_string(),
            show_song: false,
            song_id: String::new(),
            song_name: String::new(),
            difficulty_label: difficulty_label(difficulty).to_string(),
            entries: Vec::new(),
        };
        return Ok(render_template(&template));
    };

    let entry_rows = sqlx::query_as!(
        ChartTopEntryDbRow,
        "SELECT b.user_id as user_id, u.name as name,
                b.score as score, b.shiny_perfect_count as shiny_perfect_count, b.perfect_count as perfect_count,
                b.near_count as near_count, b.miss_count as miss_count, b.time_played as time_played,
                b.clear_type as clear_type, b.best_clear_type as best_clear_type
         FROM best_score b
         JOIN user u ON u.user_id = b.user_id
         WHERE b.song_id = ? AND b.difficulty = ?
         ORDER BY b.score DESC, b.time_played DESC
         LIMIT ?",
        &chart.song_id,
        difficulty,
        limit
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let entries = entry_rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| ChartTopEntryView {
            rank: (idx + 1) as i32,
            user_id: row.user_id,
            name: row.name.unwrap_or_default(),
            score: row.score.unwrap_or(0),
            pure: format!(
                "{} ({})",
                row.perfect_count.unwrap_or(0),
                row.shiny_perfect_count.unwrap_or(0)
            ),
            far: row.near_count.unwrap_or(0),
            lost: row.miss_count.unwrap_or(0),
            clear_type: clear_type_label(row.clear_type.unwrap_or(0)).to_string(),
            best_clear_type: clear_type_label(row.best_clear_type.unwrap_or(0)).to_string(),
            time_played: format_timestamp(row.time_played),
        })
        .collect::<Vec<_>>();

    let template = AdminSingleChartTopTemplate {
        active_nav: "scores",
        query_sid,
        difficulty,
        limit,
        message_kind: String::new(),
        message: String::new(),
        show_song: true,
        song_id: chart.song_id,
        song_name: chart.name.unwrap_or_default(),
        difficulty_label: difficulty_label(difficulty).to_string(),
        entries,
    };

    Ok(render_template(&template))
}

#[get("/allchar")]
pub async fn admin_allchar_get(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let db_rows = sqlx::query_as!(
        CharacterDbRow,
        "SELECT character_id, name, max_level, skill_id, skill_id_uncap, is_uncapped
         FROM `character`
         ORDER BY character_id ASC",
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let rows = db_rows
        .into_iter()
        .map(|x| {
            vec![
                x.character_id.to_string(),
                x.name.unwrap_or_default(),
                x.max_level.unwrap_or(0).to_string(),
                x.skill_id.unwrap_or_default(),
                x.skill_id_uncap.unwrap_or_default(),
                if x.is_uncapped.unwrap_or(0) != 0 {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                },
            ]
        })
        .collect();

    let template = AdminSimpleTableTemplate {
        active_nav: "scores",
        page_title: "全部角色".to_string(),
        page_subtitle: "对应 Python: /web/allchar".to_string(),
        headers: vec![
            "character_id".to_string(),
            "name".to_string(),
            "max_level".to_string(),
            "skill_id".to_string(),
            "skill_id_uncap".to_string(),
            "is_uncapped".to_string(),
        ],
        rows,
        empty_text: "没有角色数据".to_string(),
    };

    Ok(render_template(&template))
}

#[get("/allitem?<q>")]
pub async fn admin_allitem_get(
    q: Option<&str>,
    flash: Option<FlashMessage<'_>>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let query = clean_query_value(q).unwrap_or_default();
    let items = load_admin_items(q, pool.inner()).await;

    let (flash_kind, flash_message) = flash
        .map(|msg| (msg.kind().to_string(), msg.message().to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));

    let template = AdminAllItemTemplate {
        active_nav: "items",
        query,
        flash_kind,
        flash_message,
        items,
    };

    Ok(render_template(&template))
}

#[get("/changeitem")]
pub async fn admin_changeitem_get(cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    require_admin(cookies)?;
    Ok(Redirect::to("/web/allitem"))
}

#[post("/changeitem/add", data = "<form>")]
pub async fn admin_changeitem_add_post(
    form: Form<ItemAddForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match create_item(pool.inner(), &form.item_id, &form.item_type, form.is_available).await {
        Ok(()) => Flash::success(Redirect::to("/web/allitem"), "物品新增成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allitem"), msg),
    }
}

#[post("/changeitem/update", data = "<form>")]
pub async fn admin_changeitem_update_post(
    form: Form<ItemUpdateForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match update_item(pool.inner(), &form.item_id, &form.item_type, form.is_available).await {
        Ok(()) => Flash::success(Redirect::to("/web/allitem"), "物品更新成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allitem"), msg),
    }
}

#[post("/changeitem/delete", data = "<form>")]
pub async fn admin_changeitem_delete_post(
    form: Form<ItemDeleteForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match delete_item(pool.inner(), &form.item_id, &form.item_type).await {
        Ok(()) => Flash::success(Redirect::to("/web/allitem"), "物品删除成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allitem"), msg),
    }
}

#[get("/allpurchase?<pq>&<iq>")]
pub async fn admin_allpurchase_get(
    pq: Option<&str>,
    iq: Option<&str>,
    flash: Option<FlashMessage<'_>>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let query_purchase = clean_query_value(pq).unwrap_or_default();
    let query_purchase_item = clean_query_value(iq).unwrap_or_default();
    let data = load_admin_purchases(pq, iq, pool.inner()).await;

    let (flash_kind, flash_message) = flash
        .map(|msg| (msg.kind().to_string(), msg.message().to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));

    let template = AdminAllPurchaseTemplate {
        active_nav: "items",
        query_purchase,
        query_purchase_item,
        flash_kind,
        flash_message,
        purchases: data.purchases,
        purchase_items: data.purchase_items,
    };

    Ok(render_template(&template))
}

#[get("/changepurchase")]
pub async fn admin_changepurchase_get(cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    require_admin(cookies)?;
    Ok(Redirect::to("/web/allpurchase"))
}

#[post("/changepurchase/add", data = "<form>")]
pub async fn admin_changepurchase_add_post(
    form: Form<PurchaseAddForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match create_purchase(
        pool.inner(),
        &form.purchase_name,
        form.price.as_deref(),
        form.orig_price.as_deref(),
        form.discount_from.as_deref(),
        form.discount_to.as_deref(),
        form.discount_reason.as_deref(),
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allpurchase"), "购买项新增成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allpurchase"), msg),
    }
}

#[post("/changepurchase", data = "<form>")]
pub async fn admin_changepurchase_post(
    form: Form<PurchaseAddForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    admin_changepurchase_add_post(form, pool, cookies).await
}

#[post("/changepurchase/update", data = "<form>")]
pub async fn admin_changepurchase_update_post(
    form: Form<PurchaseUpdateForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match update_purchase(
        pool.inner(),
        &form.purchase_name,
        form.price.as_deref(),
        form.orig_price.as_deref(),
        form.discount_from.as_deref(),
        form.discount_to.as_deref(),
        form.discount_reason.as_deref(),
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allpurchase"), "购买项更新成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allpurchase"), msg),
    }
}

#[post("/changepurchase/delete", data = "<form>")]
pub async fn admin_changepurchase_delete_post(
    form: Form<PurchaseDeleteForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match delete_purchase(pool.inner(), &form.purchase_name).await {
        Ok(()) => Flash::success(Redirect::to("/web/allpurchase"), "购买项删除成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allpurchase"), msg),
    }
}

#[get("/changepurchaseitem")]
pub async fn admin_changepurchaseitem_get(cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    require_admin(cookies)?;
    Ok(Redirect::to("/web/allpurchase"))
}

#[post("/changepurchaseitem/add", data = "<form>")]
pub async fn admin_changepurchaseitem_add_post(
    form: Form<PurchaseItemAddForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match create_purchase_item(
        pool.inner(),
        &form.purchase_name,
        &form.item_id,
        &form.item_type,
        form.amount.as_deref(),
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allpurchase"), "购买项物品新增成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allpurchase"), msg),
    }
}

#[post("/changepurchaseitem", data = "<form>")]
pub async fn admin_changepurchaseitem_post(
    form: Form<PurchaseItemAddForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    admin_changepurchaseitem_add_post(form, pool, cookies).await
}

#[post("/changepurchaseitem/update", data = "<form>")]
pub async fn admin_changepurchaseitem_update_post(
    form: Form<PurchaseItemUpdateForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match update_purchase_item(
        pool.inner(),
        &form.purchase_name,
        &form.item_id,
        &form.item_type,
        form.amount.as_deref(),
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allpurchase"), "购买项物品更新成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allpurchase"), msg),
    }
}

#[post("/changepurchaseitem/delete", data = "<form>")]
pub async fn admin_changepurchaseitem_delete_post(
    form: Form<PurchaseItemDeleteForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    match delete_purchase_item(
        pool.inner(),
        &form.purchase_name,
        &form.item_id,
        &form.item_type,
    )
    .await
    {
        Ok(()) => Flash::success(Redirect::to("/web/allpurchase"), "购买项物品删除成功"),
        Err(msg) => Flash::error(Redirect::to("/web/allpurchase"), msg),
    }
}

#[get("/allpresent")]
pub async fn admin_allpresent_get(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let presents = sqlx::query_as!(
        PresentDbRow,
        "SELECT present_id, expire_ts, description
         FROM present
         ORDER BY expire_ts DESC, present_id ASC",
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let mut rows = Vec::new();
    for p in presents {
        let items = sqlx::query_as!(
            CollectionItemDbRow,
            "SELECT item_id, type, amount FROM present_item WHERE present_id = ?",
            &p.present_id
        )
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

        let item_summary = if items.is_empty() {
            "-".to_string()
        } else {
            items
                .into_iter()
                .map(|i| format!("{}:{}x{}", i.item_id, i.r#type, i.amount.unwrap_or(1)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        rows.push(vec![
            p.present_id,
            format_timestamp(p.expire_ts),
            p.description.unwrap_or_default(),
            item_summary,
        ]);
    }

    let template = AdminSimpleTableTemplate {
        active_nav: "items",
        page_title: "全部奖励".to_string(),
        page_subtitle: "对应 Python: /web/allpresent".to_string(),
        headers: vec![
            "present_id".to_string(),
            "expire_ts".to_string(),
            "description".to_string(),
            "items".to_string(),
        ],
        rows,
        empty_text: "没有奖励数据".to_string(),
    };

    Ok(render_template(&template))
}

#[get("/allredeem")]
pub async fn admin_allredeem_get(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<RawHtml<String>, Redirect> {
    require_admin(cookies)?;

    let redeems = sqlx::query_as!(
        RedeemDbRow,
        "SELECT code, type FROM redeem ORDER BY code ASC"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let mut rows = Vec::new();
    for r in redeems {
        let items = sqlx::query_as!(
            CollectionItemDbRow,
            "SELECT item_id, type, amount FROM redeem_item WHERE code = ?",
            &r.code
        )
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

        let item_summary = if items.is_empty() {
            "-".to_string()
        } else {
            items
                .into_iter()
                .map(|i| format!("{}:{}x{}", i.item_id, i.r#type, i.amount.unwrap_or(1)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        rows.push(vec![
            r.code,
            r.r#type.unwrap_or(0).to_string(),
            item_summary,
        ]);
    }

    let template = AdminSimpleTableTemplate {
        active_nav: "items",
        page_title: "全部兑换码".to_string(),
        page_subtitle: "对应 Python: /web/allredeem".to_string(),
        headers: vec!["code".to_string(), "type".to_string(), "items".to_string()],
        rows,
        empty_text: "没有兑换码数据".to_string(),
    };

    Ok(render_template(&template))
}

#[post("/users/<user_id>/ticket", data = "<form>")]
pub async fn admin_user_ticket_post(
    user_id: i32,
    form: Form<UserTicketForm>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    let result = sqlx::query!(
        "UPDATE user SET ticket = ? WHERE user_id = ?",
        form.ticket,
        user_id
    )
    .execute(pool.inner())
    .await;

    match result {
        Ok(done) if done.rows_affected() > 0 => Flash::success(
            Redirect::to(format!("/web/users/{user_id}")),
            "Ticket 更新成功",
        ),
        Ok(_) => Flash::error(Redirect::to(format!("/web/users/{user_id}")), "玩家不存在"),
        Err(err) => Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("更新失败: {err}"),
        ),
    }
}

#[post("/users/<user_id>/ban")]
pub async fn admin_user_ban_post(
    user_id: i32,
    pool: &State<DbPool>,
    user_service: &State<UserService>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    let old_tokens = match sqlx::query!("SELECT access_token FROM login WHERE user_id = ?", user_id)
        .fetch_all(pool.inner())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return Flash::error(
                Redirect::to(format!("/web/users/{user_id}")),
                format!("封禁失败: {err}"),
            )
        }
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            return Flash::error(
                Redirect::to(format!("/web/users/{user_id}")),
                format!("事务创建失败: {err}"),
            )
        }
    };

    let update = sqlx::query!("UPDATE user SET password = '' WHERE user_id = ?", user_id)
        .execute(&mut *tx)
        .await;

    if let Err(err) = update {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("封禁失败: {err}"),
        );
    }

    if let Err(err) = sqlx::query!("DELETE FROM login WHERE user_id = ?", user_id)
        .execute(&mut *tx)
        .await
    {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("封禁失败: {err}"),
        );
    }

    if let Err(err) = tx.commit().await {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("封禁失败: {err}"),
        );
    }

    user_service
        .invalidate_auth_tokens(old_tokens.into_iter().map(|row| row.access_token))
        .await;
    user_service.invalidate_user_info_cache(user_id).await;

    Flash::success(Redirect::to(format!("/web/users/{user_id}")), "封禁成功")
}

#[post("/users/<user_id>/scores/delete")]
pub async fn admin_user_scores_delete_post(
    user_id: i32,
    pool: &State<DbPool>,
    score_service: &State<ScoreService>,
    cookies: &CookieJar<'_>,
) -> Flash<Redirect> {
    if !is_admin_logged_in(cookies) {
        return Flash::error(Redirect::to("/web/login"), "请先登录");
    }

    let score_keys = match sqlx::query!(
        "SELECT song_id, difficulty FROM best_score WHERE user_id = ?",
        user_id
    )
    .fetch_all(pool.inner())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| (row.song_id, row.difficulty))
            .collect::<Vec<_>>(),
        Err(err) => {
            return Flash::error(
                Redirect::to(format!("/web/users/{user_id}")),
                format!("读取成绩缓存键失败: {err}"),
            )
        }
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            return Flash::error(
                Redirect::to(format!("/web/users/{user_id}")),
                format!("事务创建失败: {err}"),
            )
        }
    };

    if let Err(err) = sqlx::query!(
        "UPDATE user
         SET rating_ptt = 0,
             song_id = '',
             difficulty = 0,
             score = 0,
             shiny_perfect_count = 0,
             perfect_count = 0,
             near_count = 0,
             miss_count = 0,
             health = 0,
             modifier = 0,
             time_played = 0,
             clear_type = 0,
             rating = 0,
             world_rank_score = 0
         WHERE user_id = ?",
        user_id
    )
    .execute(&mut *tx)
    .await
    {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("更新用户失败: {err}"),
        );
    }

    if let Err(err) = sqlx::query!("DELETE FROM best_score WHERE user_id = ?", user_id)
        .execute(&mut *tx)
        .await
    {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("删除 best_score 失败: {err}"),
        );
    }

    if let Err(err) = sqlx::query!("DELETE FROM recent30 WHERE user_id = ?", user_id)
        .execute(&mut *tx)
        .await
    {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("删除 recent30 失败: {err}"),
        );
    }

    if let Err(err) = sqlx::query!(
        r#"DELETE FROM user_kvdata
           WHERE user_id = ? AND class = 'score' AND `key` = 'world_rank_score_raw' AND idx = 0"#,
        user_id
    )
    .execute(&mut *tx)
    .await
    {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("删除 world_rank_score_raw 失败: {err}"),
        );
    }

    if let Err(err) = tx.commit().await {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("提交失败: {err}"),
        );
    }

    if let Err(err) = score_service
        .reset_user_score_caches_for_scores(user_id, score_keys)
        .await
    {
        return Flash::error(
            Redirect::to(format!("/web/users/{user_id}")),
            format!("清理成绩缓存失败: {err}"),
        );
    }

    Flash::success(
        Redirect::to(format!("/web/users/{user_id}")),
        "用户成绩已删除",
    )
}

pub fn routes() -> Vec<Route> {
    routes![
        admin_css,
        admin_login_get,
        admin_login_post,
        admin_api_session,
        admin_api_login,
        admin_api_logout,
        admin_api_dashboard,
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
        admin_logout_get,
        admin_dashboard,
        admin_dashboard_index,
        admin_refresh_song_hash_post,
        admin_refresh_bundle_post,
        admin_refresh_song_rating_post,
        admin_users_get,
        admin_allplayer_get,
        admin_allsong_get,
        admin_changesong_get,
        admin_changesong_add_post,
        admin_changesong_update_post,
        admin_changesong_delete_post,
        admin_singleplayer_get,
        admin_singleplayer_post,
        admin_singleplayerptt_get,
        admin_singleplayerptt_post,
        admin_singlecharttop_get,
        admin_singlecharttop_post,
        admin_allchar_get,
        admin_allitem_get,
        admin_changeitem_get,
        admin_changeitem_add_post,
        admin_changeitem_update_post,
        admin_changeitem_delete_post,
        admin_allpurchase_get,
        admin_changepurchase_get,
        admin_changepurchase_post,
        admin_changepurchase_add_post,
        admin_changepurchase_update_post,
        admin_changepurchase_delete_post,
        admin_changepurchaseitem_get,
        admin_changepurchaseitem_post,
        admin_changepurchaseitem_add_post,
        admin_changepurchaseitem_update_post,
        admin_changepurchaseitem_delete_post,
        admin_allpresent_get,
        admin_allredeem_get,
        admin_user_detail_get,
        admin_user_ticket_post,
        admin_user_ban_post,
        admin_user_scores_delete_post
    ]
}
