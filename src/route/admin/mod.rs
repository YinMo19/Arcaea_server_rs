//! Web admin panel routes (`/web`).
//!
//! The module is split by domain:
//! - [`mod@models`] — request/response and DB-row types.
//! - [`mod@helpers`] — shared formatting, pagination and query helpers.
//! - [`mod@session`] — authentication, cookies and the `require_*` guards.
//! - [`mod@dashboard`] — overview metrics, check-in, maintenance operations.
//! - [`mod@users`] — player management and per-player scores.
//! - [`mod@scores`] — score images and the chart leaderboard.
//! - [`mod@presents`] — presents and redeem codes.
//! - [`mod@catalog`] — song / item / purchase / purchase-item data tables.

mod catalog;
mod dashboard;
mod helpers;
mod models;
mod presents;
mod scores;
mod session;
mod users;

use std::sync::{OnceLock, RwLock};

use rocket::{routes, Route};

use crate::config::CONFIG;

/// Cookie name holding the signed web session value.
pub(super) const ADMIN_COOKIE: &str = "arcaea_web_session";
/// Role id stored on users granted admin privileges.
pub(super) const ADMIN_ROLE: i8 = 1;
/// Default role id for regular players.
pub(super) const USER_ROLE: i8 = 0;
pub(super) const CHART_EDITOR_ROLE: &str = "chart_editor";
pub(super) const CHART_CONSTANT_EDIT_POWER: &str = "web_chart_constant_edit";

/// Credentials used for first-time admin bootstrap and as a fallback.
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

/// Override the configured admin credentials at runtime.
pub fn set_admin_config(config: AdminConfig) {
    let lock = ADMIN_CONFIG.get_or_init(|| RwLock::new(AdminConfig::default()));
    if let Ok(mut guard) = lock.write() {
        *guard = config;
    }
}

/// All admin web routes, mounted under `/web`.
pub fn routes() -> Vec<Route> {
    routes![
        // session
        session::admin_api_session,
        session::admin_api_login,
        session::admin_api_logout,
        // dashboard / check-in / operations
        dashboard::admin_api_dashboard,
        dashboard::admin_api_checkin_status,
        dashboard::admin_api_checkin_claim,
        dashboard::admin_api_operation,
        // listings
        users::admin_api_users,
        users::admin_api_chart_editor_permission,
        catalog::admin_api_songs,
        catalog::admin_api_items,
        catalog::admin_api_purchases,
        catalog::admin_api_purchase_items,
        // queries
        users::admin_api_user_scores,
        scores::admin_api_score_images,
        scores::admin_api_score_image_png,
        scores::admin_api_chart_top,
        presents::admin_api_redeem_users,
        // player actions
        users::admin_api_user_ticket,
        users::admin_api_user_password,
        users::admin_api_user_create,
        users::admin_api_user_ban,
        users::admin_api_user_purchase,
        users::admin_api_scores_delete,
        // presents / redeems
        presents::admin_api_present_create,
        presents::admin_api_present_delete,
        presents::admin_api_present_deliver,
        presents::admin_api_redeem_create,
        presents::admin_api_redeem_delete,
        // catalog CRUD
        catalog::admin_api_song_create,
        catalog::admin_api_song_update,
        catalog::admin_api_chart_constants_update,
        catalog::admin_api_song_delete,
        catalog::admin_api_item_create,
        catalog::admin_api_item_update,
        catalog::admin_api_item_delete,
        catalog::admin_api_purchase_create,
        catalog::admin_api_purchase_update,
        catalog::admin_api_purchase_delete,
        catalog::admin_api_purchase_item_create,
        catalog::admin_api_purchase_item_update,
        catalog::admin_api_purchase_item_delete,
    ]
}
