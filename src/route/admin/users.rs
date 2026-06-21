//! Player management: account actions (ticket / password / create / ban /
//! purchase), score deletion and per-player score queries.

use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::config::CONFIG;
use crate::error::ArcError;
use crate::model::UserRegisterDto;
use crate::route::common::{success_return, RouteResult};
use crate::service::UserService;
use crate::utils::sql_placeholders;
use crate::DbPool;

use super::helpers::{
    clean_optional_payload_text, clean_query_value, clamp_page, filter_sql, format_timestamp,
    is_admin_user_banned, page_response, resolve_admin_user,
};
use super::models::{
    AdminActionResponse, AdminPageResponse, AdminScoreDeletePayload, AdminScoreRowView,
    AdminUserCreatePayload, AdminUserPasswordPayload, AdminUserPurchasePayload, AdminUserScoreQuery,
    AdminUserScoresResponse, AdminUserScoreStats, AdminUserSelectorPayload, AdminUserSummary,
    AdminUserTicketPayload, UserListDbRow, UserListView,
};
use super::session::{require_admin_api, require_web_session};
use super::ADMIN_ROLE;

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
    let password_hash = UserService::hash_password(password);
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

async fn create_admin_user(
    payload: &AdminUserCreatePayload,
    pool: &DbPool,
    user_service: &UserService,
) -> Result<AdminUserSummary, ArcError> {
    let register_data = UserRegisterDto {
        name: payload.name.trim().to_string(),
        password: payload.password.clone(),
        email: payload.email.trim().to_string(),
        is_allow_marketing_email: false,
    };

    let user_auth = user_service
        .register_user(register_data, None, None)
        .await?;

    // Re-fetch the freshly created account so we can return its user_code.
    resolve_admin_user(Some(user_auth.user_id), None, None, pool).await
}

async fn ban_admin_user(
    payload: &AdminUserSelectorPayload,
    pool: &DbPool,
) -> Result<AdminActionResponse, ArcError> {
    let user = super::helpers::resolve_admin_user_from_selector(payload, pool).await?;
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

    let item_type_placeholders = sql_placeholders(item_types.len());
    let affected_rows = if method == "lock" {
        let user_placeholders = sql_placeholders(users.len());
        let sql = format!(
            "DELETE FROM user_item WHERE user_id IN ({user_placeholders}) AND type IN ({item_type_placeholders})"
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
        let item_sql =
            format!("SELECT item_id, type FROM item WHERE type IN ({item_type_placeholders})");
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
    if song_id.is_some() {
        filters.push("song_id = ?");
    }
    if difficulty.is_some() {
        filters.push("difficulty = ?");
    }
    if user.is_some() {
        filters.push("user_id = ?");
    }

    let sql = format!("DELETE FROM best_score{}", filter_sql(&filters));
    let mut query = sqlx::query(&sql);
    if let Some(song_id) = &song_id {
        query = query.bind(song_id);
    }
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

async fn load_admin_users(
    q: Option<&str>,
    status: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> Result<AdminPageResponse<UserListView>, ArcError> {
    let keyword = clean_query_value(q);
    let status = status
        .map(str::trim)
        .filter(|value| matches!(*value, "normal" | "banned"));

    let like = keyword.as_ref().map(|kw| format!("%{kw}%"));
    let keyword = keyword.as_deref();
    let like = like.as_deref();
    let has_keyword = keyword.is_some();
    let is_banned_filter = matches!(status, Some("banned"));
    let is_normal_filter = matches!(status, Some("normal"));

    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as `count!: i64`
        FROM user
        WHERE (? = 0 OR CAST(user_id AS CHAR) LIKE ? OR name LIKE ? OR user_code LIKE ?)
          AND (? = 0 OR COALESCE(password, '') = '' OR COALESCE(CAST(SUBSTRING_INDEX(NULLIF(ban_flag, ''), ':', -1) AS SIGNED), 0) > UNIX_TIMESTAMP(CURRENT_TIMESTAMP(3)) * 1000)
          AND (? = 0 OR (COALESCE(password, '') <> '' AND NOT (COALESCE(CAST(SUBSTRING_INDEX(NULLIF(ban_flag, ''), ':', -1) AS SIGNED), 0) > UNIX_TIMESTAMP(CURRENT_TIMESTAMP(3)) * 1000)))
        "#,
        has_keyword,
        like,
        like,
        like,
        is_banned_filter,
        is_normal_filter,
    )
    .fetch_one(pool)
    .await?;
    let (page, offset) = clamp_page(page, page_size, total);

    let rows = sqlx::query_as!(
        UserListDbRow,
        r#"
        SELECT user_id, name, user_code, rating_ptt, ticket, time_played, password, ban_flag
        FROM user
        WHERE (? = 0 OR CAST(user_id AS CHAR) LIKE ? OR name LIKE ? OR user_code LIKE ?)
          AND (? = 0 OR COALESCE(password, '') = '' OR COALESCE(CAST(SUBSTRING_INDEX(NULLIF(ban_flag, ''), ':', -1) AS SIGNED), 0) > UNIX_TIMESTAMP(CURRENT_TIMESTAMP(3)) * 1000)
          AND (? = 0 OR (COALESCE(password, '') <> '' AND NOT (COALESCE(CAST(SUBSTRING_INDEX(NULLIF(ban_flag, ''), ':', -1) AS SIGNED), 0) > UNIX_TIMESTAMP(CURRENT_TIMESTAMP(3)) * 1000)))
        ORDER BY rating_ptt DESC, user_id ASC
        LIMIT ? OFFSET ?
        "#,
        has_keyword,
        like,
        like,
        like,
        is_banned_filter,
        is_normal_filter,
        page_size,
        offset,
    )
    .fetch_all(pool)
    .await?
        .into_iter()
        .map(|row| UserListView {
            user_id: row.user_id,
            name: row.name.unwrap_or_default(),
            user_code: row.user_code.unwrap_or_default(),
            rating_ptt: row.rating_ptt.unwrap_or(0),
            ticket: row.ticket.unwrap_or(0),
            last_play: format_timestamp(row.time_played),
            banned: is_admin_user_banned(row.password.as_deref(), row.ban_flag.as_deref()),
        })
        .collect();

    Ok(page_response(rows, total, page, page_size))
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
    let b30 = sqlx::query!(
        "SELECT bs.user_id, u.name, bs.song_id, bs.difficulty, bs.score,
                bs.shiny_perfect_count, bs.perfect_count, bs.near_count, bs.miss_count,
                bs.clear_type, bs.best_clear_type, bs.rating, bs.time_played
         FROM best_score bs
         JOIN user u ON u.user_id = bs.user_id
         WHERE bs.user_id = ?
         ORDER BY bs.rating DESC, bs.score DESC
         LIMIT 30",
        user.user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询玩家 B30 失败: {err}")))?
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
    .collect::<Vec<_>>();

    let r10 = sqlx::query!(
        "WITH ranked_songs AS (
            SELECT r.*,
                   ROW_NUMBER() OVER (
                       PARTITION BY r.song_id
                       ORDER BY r.rating DESC, r.score DESC
                   ) AS song_rank
            FROM recent30 r
            WHERE r.user_id = ? AND r.song_id != ''
         )
         SELECT rs.user_id, u.name, rs.song_id, rs.difficulty, rs.score,
                rs.shiny_perfect_count, rs.perfect_count, rs.near_count, rs.miss_count,
                rs.clear_type, rs.rating, rs.time_played
         FROM ranked_songs rs
         JOIN user u ON u.user_id = rs.user_id
        WHERE rs.song_rank = 1
         ORDER BY rs.rating DESC, rs.score DESC
         LIMIT 10",
        user.user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询玩家 R10 失败: {err}")))?
    .into_iter()
    .map(|row| AdminScoreRowView {
        user_id: row.user_id.unwrap_or(user.user_id),
        name: row.name,
        song_id: row.song_id.unwrap_or_default(),
        difficulty: row.difficulty.unwrap_or_default(),
        score: row.score.unwrap_or_default(),
        shiny_perfect_count: row.shiny_perfect_count.unwrap_or_default(),
        perfect_count: row.perfect_count.unwrap_or_default(),
        near_count: row.near_count.unwrap_or_default(),
        miss_count: row.miss_count.unwrap_or_default(),
        clear_type: row.clear_type.unwrap_or_default(),
        best_clear_type: row.clear_type.unwrap_or_default(),
        rating: row.rating.unwrap_or(0.0),
        time_played: format_timestamp(row.time_played),
    })
    .collect::<Vec<_>>();

    let best_30_sum = b30.iter().map(|score| score.rating).sum();
    let recent_10_sum = r10.iter().map(|score| score.rating).sum();
    let stats = AdminUserScoreStats {
        best_30_sum,
        recent_10_sum,
        potential: best_30_sum * CONFIG.best30_weight + recent_10_sum * CONFIG.recent10_weight,
    };

    Ok(AdminUserScoresResponse {
        user,
        stats,
        b30,
        r10,
    })
}

#[get("/api/users?<q>&<status>&<page>&<page_size>")]
pub(super) async fn admin_api_users(
    q: Option<&str>,
    status: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<UserListView>> {
    require_admin_api(cookies, pool.inner()).await?;
    let (page, page_size) = super::helpers::normalize_page(page, page_size);
    Ok(success_return(
        load_admin_users(q, status, page, page_size, pool.inner()).await?,
    ))
}

#[post("/api/admin-actions/user-ticket", format = "json", data = "<payload>")]
pub(super) async fn admin_api_user_ticket(
    payload: Json<AdminUserTicketPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        update_admin_user_ticket(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/user-password",
    format = "json",
    data = "<payload>"
)]
pub(super) async fn admin_api_user_password(
    payload: Json<AdminUserPasswordPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        update_admin_user_password(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/user-create",
    format = "json",
    data = "<payload>"
)]
pub(super) async fn admin_api_user_create(
    payload: Json<AdminUserCreatePayload>,
    pool: &State<DbPool>,
    user_service: &State<UserService>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminUserSummary> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        create_admin_user(&payload, pool.inner(), user_service.inner()).await?,
    ))
}

#[post("/api/admin-actions/user-ban", format = "json", data = "<payload>")]
pub(super) async fn admin_api_user_ban(
    payload: Json<AdminUserSelectorPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        ban_admin_user(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/user-purchase",
    format = "json",
    data = "<payload>"
)]
pub(super) async fn admin_api_user_purchase(
    payload: Json<AdminUserPurchasePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        update_admin_user_purchase(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/scores/delete",
    format = "json",
    data = "<payload>"
)]
pub(super) async fn admin_api_scores_delete(
    payload: Json<AdminScoreDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        delete_admin_scores(&payload, pool.inner()).await?,
    ))
}

#[get("/api/user-scores?<user_id>&<name>&<user_code>")]
pub(super) async fn admin_api_user_scores(
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminUserScoresResponse> {
    let session = require_web_session(cookies, pool.inner()).await?;
    let query = if session.role == ADMIN_ROLE {
        AdminUserScoreQuery {
            user_id,
            name,
            user_code,
        }
    } else {
        AdminUserScoreQuery {
            user_id: Some(session.user.user_id),
            name: None,
            user_code: None,
        }
    };
    Ok(success_return(
        load_admin_user_scores(&query, pool.inner()).await?,
    ))
}
