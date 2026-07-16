//! Data types for the admin web panel: response views, request payloads,
//! database row structs and the shared response envelopes.
//!
//! Fields are `pub(super)` so the sibling domain modules under `super::admin`
//! can construct and read them.

use rocket::http::{ContentType, Status};
use rocket::response::{Responder, Response};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::io::Cursor;

use super::{ADMIN_ROLE, USER_ROLE};

// Response view structs

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RecentOpView {
    pub(super) name: String,
    pub(super) operator: String,
    pub(super) time: String,
    pub(super) status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UserListView {
    pub(super) user_id: i32,
    pub(super) name: String,
    pub(super) user_code: String,
    pub(super) rating_ptt: i32,
    pub(super) ticket: i32,
    pub(super) last_play: String,
    pub(super) banned: bool,
    pub(super) is_admin: bool,
    pub(super) can_edit_chart_constants: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SongRowView {
    pub(super) song_id: String,
    pub(super) name_en: String,
    pub(super) rating_pst: String,
    pub(super) rating_prs: String,
    pub(super) rating_ftr: String,
    pub(super) rating_byd: String,
    pub(super) rating_etr: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ItemRowView {
    pub(super) item_id: String,
    pub(super) item_type: String,
    pub(super) is_available: i32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PurchaseRowView {
    pub(super) purchase_name: String,
    pub(super) price: String,
    pub(super) orig_price: String,
    pub(super) discount_from: String,
    pub(super) discount_to: String,
    pub(super) discount_reason: String,
    pub(super) item_summary: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PurchaseItemRowView {
    pub(super) purchase_name: String,
    pub(super) item_id: String,
    pub(super) item_type: String,
    pub(super) amount: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminUserSummary {
    pub(super) user_id: i32,
    pub(super) name: String,
    pub(super) user_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminScoreRowView {
    pub(super) user_id: i32,
    pub(super) name: Option<String>,
    pub(super) song_id: String,
    pub(super) difficulty: i32,
    pub(super) score: i32,
    pub(super) shiny_perfect_count: i32,
    pub(super) perfect_count: i32,
    pub(super) near_count: i32,
    pub(super) miss_count: i32,
    pub(super) clear_type: i32,
    pub(super) best_clear_type: i32,
    pub(super) rating: f64,
    pub(super) time_played: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminUserScoreStats {
    pub(super) best_30_sum: f64,
    pub(super) recent_10_sum: f64,
    pub(super) potential: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminUserScoresResponse {
    pub(super) user: AdminUserSummary,
    pub(super) stats: AdminUserScoreStats,
    pub(super) b30: Vec<AdminScoreRowView>,
    pub(super) r10: Vec<AdminScoreRowView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminChartTopResponse {
    pub(super) song_id: String,
    pub(super) name_en: String,
    pub(super) difficulty: i32,
    pub(super) scores: Vec<AdminScoreRowView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminActionResponse {
    pub(super) message: String,
    pub(super) affected_rows: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminRedeemUsersResponse {
    pub(super) code: String,
    pub(super) users: Vec<AdminUserSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UserCheckinResponse {
    pub(super) user: AdminUserSummary,
    pub(super) today: String,
    pub(super) checked_in_today: bool,
    pub(super) claimed: bool,
    pub(super) reward: Option<i32>,
    pub(super) current_ticket: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScoreImageView {
    pub(super) mode: String,
    pub(super) title: String,
    pub(super) entry_count: usize,
    pub(super) url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScoreImagesResponse {
    pub(super) user: AdminUserSummary,
    pub(super) images: Vec<ScoreImageView>,
}

pub(super) struct PngResponse {
    pub(super) bytes: Vec<u8>,
}

impl<'r> Responder<'r, 'static> for PngResponse {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .status(Status::Ok)
            .header(ContentType::PNG)
            .sized_body(self.bytes.len(), Cursor::new(self.bytes))
            .ok()
    }
}

// Request payloads

#[derive(Debug, Deserialize)]
pub(super) struct AdminSongPayload {
    pub(super) sid: String,
    pub(super) name_en: String,
    pub(super) rating_pst: String,
    pub(super) rating_prs: String,
    pub(super) rating_ftr: String,
    pub(super) rating_byd: String,
    pub(super) rating_etr: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChartConstantsPayload {
    pub(super) rating_pst: String,
    pub(super) rating_prs: String,
    pub(super) rating_ftr: String,
    pub(super) rating_byd: String,
    pub(super) rating_etr: String,
}

pub(super) struct AdminSongInput<'a> {
    pub(super) sid: &'a str,
    pub(super) name_en: &'a str,
    pub(super) rating_pst: &'a str,
    pub(super) rating_prs: &'a str,
    pub(super) rating_ftr: &'a str,
    pub(super) rating_byd: &'a str,
    pub(super) rating_etr: &'a str,
}

impl<'a> From<&'a AdminSongPayload> for AdminSongInput<'a> {
    fn from(payload: &'a AdminSongPayload) -> Self {
        Self {
            sid: &payload.sid,
            name_en: &payload.name_en,
            rating_pst: &payload.rating_pst,
            rating_prs: &payload.rating_prs,
            rating_ftr: &payload.rating_ftr,
            rating_byd: &payload.rating_byd,
            rating_etr: &payload.rating_etr,
        }
    }
}

impl<'a> AdminSongInput<'a> {
    pub(super) fn with_sid(mut self, sid: &'a str) -> Self {
        self.sid = sid;
        self
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminSongDeletePayload {
    pub(super) sid: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminItemPayload {
    pub(super) item_id: String,
    pub(super) item_type: String,
    pub(super) is_available: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminItemDeletePayload {
    pub(super) item_id: String,
    pub(super) item_type: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPurchasePayload {
    pub(super) purchase_name: String,
    pub(super) price: Option<String>,
    pub(super) orig_price: Option<String>,
    pub(super) discount_from: Option<String>,
    pub(super) discount_to: Option<String>,
    pub(super) discount_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPurchaseDeletePayload {
    pub(super) purchase_name: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPurchaseItemPayload {
    pub(super) purchase_name: String,
    pub(super) item_id: String,
    pub(super) item_type: String,
    pub(super) amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPurchaseItemDeletePayload {
    pub(super) purchase_name: String,
    pub(super) item_id: String,
    pub(super) item_type: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUserSelectorPayload {
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChartEditorPermissionPayload {
    pub(super) enabled: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUserTicketPayload {
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) ticket: i32,
    pub(super) all_users: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUserPasswordPayload {
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) password: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUserCreatePayload {
    pub(super) name: String,
    pub(super) password: String,
    pub(super) email: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUserPurchasePayload {
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) method: String,
    pub(super) all_users: Option<bool>,
    pub(super) item_types: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminScoreDeletePayload {
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) song_id: Option<String>,
    pub(super) difficulty: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPresentPayload {
    pub(super) present_id: String,
    pub(super) expire_ts: Option<String>,
    pub(super) description: Option<String>,
    pub(super) item_id: String,
    pub(super) item_type: String,
    pub(super) amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPresentDeletePayload {
    pub(super) present_id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminPresentDeliverPayload {
    pub(super) present_id: String,
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) all_users: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminRedeemPayload {
    pub(super) code: Option<String>,
    pub(super) random_amount: Option<i32>,
    pub(super) redeem_type: i32,
    pub(super) item_id: String,
    pub(super) item_type: String,
    pub(super) amount: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminRedeemDeletePayload {
    pub(super) code: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUserScoreQuery {
    pub(super) user_id: Option<i32>,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminLoginRequest {
    pub(super) username: String,
    pub(super) password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminSessionResponse {
    pub(super) logged_in: bool,
    pub(super) role: i8,
    pub(super) permissions: Vec<String>,
    pub(super) app_title: String,
    pub(super) login_background: Option<String>,
    pub(super) login_position: String,
    pub(super) login_card_opacity: f64,
    pub(super) web_surface_opacity: f64,
    pub(super) user: Option<AdminUserSummary>,
}

#[derive(Debug, Clone)]
pub(super) struct WebSession {
    pub(super) user: AdminUserSummary,
    pub(super) role: i8,
    pub(super) can_edit_chart_constants: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminDashboardApiResponse {
    pub(super) online_users: i64,
    pub(super) online_growth: f64,
    pub(super) score_submits: i64,
    pub(super) score_error_rate: f64,
    pub(super) present_count: i64,
    pub(super) alert_count: i64,
    pub(super) recent_ops: Vec<RecentOpView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AdminPageResponse<T> {
    pub(super) rows: Vec<T>,
    pub(super) total: i64,
    pub(super) page: i64,
    pub(super) page_size: i64,
}

// Database row structs

#[derive(FromRow)]
pub(super) struct RecentLoginRow {
    pub(super) name: Option<String>,
    pub(super) login_time: Option<i64>,
}

pub(super) struct UserListDbRow {
    pub(super) user_id: i32,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) rating_ptt: Option<i32>,
    pub(super) ticket: Option<i32>,
    pub(super) time_played: Option<i64>,
    pub(super) password: Option<String>,
    pub(super) ban_flag: Option<String>,
    pub(super) is_admin: i64,
    pub(super) can_edit_chart_constants: i64,
}

#[derive(FromRow)]
pub(super) struct ChartDbRow {
    pub(super) song_id: String,
    pub(super) name: Option<String>,
    pub(super) rating_pst: Option<i32>,
    pub(super) rating_prs: Option<i32>,
    pub(super) rating_ftr: Option<i32>,
    pub(super) rating_byn: Option<i32>,
    pub(super) rating_etr: Option<i32>,
}

#[derive(FromRow)]
pub(super) struct ItemDbRow {
    pub(super) item_id: String,
    pub(super) r#type: String,
    pub(super) is_available: Option<i8>,
}

#[derive(FromRow)]
pub(super) struct PurchaseDbRow {
    pub(super) purchase_name: String,
    pub(super) price: Option<i32>,
    pub(super) orig_price: Option<i32>,
    pub(super) discount_from: Option<i64>,
    pub(super) discount_to: Option<i64>,
    pub(super) discount_reason: Option<String>,
}

#[derive(Clone, FromRow)]
pub(super) struct PurchaseItemDbRow {
    pub(super) purchase_name: String,
    pub(super) item_id: String,
    pub(super) r#type: String,
    pub(super) amount: Option<i32>,
}

pub(super) struct AdminUserDbSummary {
    pub(super) user_id: i32,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
}

#[derive(FromRow)]
pub(super) struct WebLoginUserRow {
    pub(super) user_id: i32,
    pub(super) name: Option<String>,
    pub(super) user_code: Option<String>,
    pub(super) password: Option<String>,
    pub(super) ban_flag: Option<String>,
    pub(super) role: i64,
    pub(super) can_edit_chart_constants: i64,
}

impl WebLoginUserRow {
    pub(super) fn web_role(&self) -> i8 {
        if self.role > 0 {
            ADMIN_ROLE
        } else {
            USER_ROLE
        }
    }

    pub(super) fn can_edit_chart_constants(&self) -> bool {
        self.web_role() == ADMIN_ROLE || self.can_edit_chart_constants > 0
    }
}
