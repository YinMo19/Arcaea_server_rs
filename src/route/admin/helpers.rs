//! Cross-cutting helpers shared across admin domain modules: timestamp
//! formatting, pagination, SQL filter builders, user resolution and ban
//! detection.

use chrono::{Local, TimeZone, Utc};

use crate::error::ArcError;
use crate::DbPool;

use super::models::{
    AdminPageResponse, AdminUserDbSummary, AdminUserSelectorPayload, AdminUserSummary,
};

/// Format an optional millisecond timestamp as `YYYY-MM-DD HH:MM`, accepting
/// either millisecond or second precision.
pub(super) fn format_timestamp(ts: Option<i64>) -> String {
    let Some(ts) = ts else {
        return "-".to_string();
    };

    let sec = if ts > 10_000_000_000 { ts / 1000 } else { ts };

    Local
        .timestamp_opt(sec, 0)
        .single()
        .map(|x| x.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string())
}

/// A ban flag is `ban:<end_timestamp_ms>`; it is active while that timestamp
/// is still in the future.
pub(super) fn is_ban_flag_active(ban_flag: Option<&str>) -> bool {
    let Some(flag) = ban_flag.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let Some(end_timestamp) = flag
        .split(':')
        .nth(1)
        .and_then(|value| value.parse::<i64>().ok())
    else {
        return false;
    };
    end_timestamp > Utc::now().timestamp_millis()
}

/// A web user is considered banned when the password was cleared or an active
/// ban flag is present.
pub(super) fn is_admin_user_banned(password: Option<&str>, ban_flag: Option<&str>) -> bool {
    password.map(str::is_empty).unwrap_or(true) || is_ban_flag_active(ban_flag)
}

pub(super) fn clean_optional_payload_text(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Trim an optional string and cap its length, defaulting to an empty string.
pub(super) fn normalize_optional_text(raw: Option<&str>, max_len: usize) -> String {
    let text = raw.unwrap_or("").trim();
    text.chars().take(max_len).collect()
}

/// Resolve a user by `user_id` / `name` / `user_code` (first one wins).
pub(super) async fn resolve_admin_user(
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

/// Resolve a user from a selector payload, trimming the optional text fields.
pub(super) async fn resolve_admin_user_from_selector(
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

// Pagination & query helpers

pub(super) fn clean_query_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

pub(super) fn normalize_page(page: Option<i64>, page_size: Option<i64>) -> (i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(25).clamp(10, 100);
    (page, page_size)
}

pub(super) fn clamp_page(page: i64, page_size: i64, total: i64) -> (i64, i64) {
    let page_count = ((total.max(1) + page_size - 1) / page_size).max(1);
    let page = page.clamp(1, page_count);
    (page, (page - 1) * page_size)
}

pub(super) fn page_response<T>(rows: Vec<T>, total: i64, page: i64, page_size: i64) -> AdminPageResponse<T> {
    AdminPageResponse {
        rows,
        total,
        page,
        page_size,
    }
}

pub(super) fn filter_sql(filters: &[&str]) -> String {
    if filters.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", filters.join(" AND "))
    }
}

/// Build a `WHERE col LIKE ? OR ...` clause plus the bind values.
pub(super) fn like_filter(query: Option<&str>, columns: &[&str]) -> (String, Vec<String>) {
    let Some(query) = clean_query_value(query) else {
        return (String::new(), Vec::new());
    };

    let filters = columns
        .iter()
        .map(|column| format!("{column} LIKE ?"))
        .collect::<Vec<_>>();
    let like = format!("%{query}%");
    (
        format!(" WHERE {}", filters.join(" OR ")),
        vec![like; columns.len()],
    )
}

/// Adapter turning a `String` error from the catalog CRUD helpers into an
/// [`ArcError`] for use with `map_err`.
pub(super) fn admin_api_input_error(message: String) -> ArcError {
    ArcError::input(message)
}
