//! Dashboard overview metrics, daily check-in and admin maintenance operations.

use chrono::{Local, NaiveDate};
use rand::Rng;
use rocket::http::CookieJar;
use rocket::{get, post, State};

use crate::error::ArcError;
use crate::route::common::{success_return, success_return_no_value, EmptyResponse, RouteResult};
use crate::service::OperationManager;
use crate::DbPool;

use super::helpers::format_timestamp;
use super::models::{
    AdminDashboardApiResponse, RecentLoginRow, RecentOpView, UserCheckinResponse, WebSession,
};
use super::session::{require_admin_api, require_web_session};

async fn load_dashboard_api(pool: &DbPool) -> AdminDashboardApiResponse {
    let now_ms = Local::now().timestamp_millis();
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

fn checkin_today() -> NaiveDate {
    Local::now().date_naive()
}

fn format_checkin_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

async fn load_user_checkin_status(
    session: &WebSession,
    pool: &DbPool,
) -> Result<UserCheckinResponse, ArcError> {
    let today = checkin_today();
    let record = sqlx::query!(
        r#"
        SELECT reward_ticket as `reward_ticket!: i32`
        FROM user_checkin
        WHERE user_id = ? AND checkin_date = ?
        "#,
        session.user.user_id,
        today
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询签到状态失败: {err}")))?;

    let user = sqlx::query!(
        "SELECT ticket FROM user WHERE user_id = ?",
        session.user.user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|err| ArcError::input(format!("查询记忆源点失败: {err}")))?;

    Ok(UserCheckinResponse {
        user: session.user.clone(),
        today: format_checkin_date(today),
        checked_in_today: record.is_some(),
        claimed: false,
        reward: record.map(|row| row.reward_ticket),
        current_ticket: user.ticket.unwrap_or(0),
    })
}

async fn claim_user_checkin(
    session: &WebSession,
    pool: &DbPool,
) -> Result<UserCheckinResponse, ArcError> {
    let today = checkin_today();
    let reward = rand::thread_rng().gen_range(200..=500);
    let created_at = Local::now().timestamp_millis();
    let mut tx = pool
        .begin()
        .await
        .map_err(|err| ArcError::input(format!("开始签到事务失败: {err}")))?;

    let inserted = sqlx::query!(
        r#"
        INSERT IGNORE INTO user_checkin (user_id, checkin_date, reward_ticket, created_at)
        VALUES (?, ?, ?, ?)
        "#,
        session.user.user_id,
        today,
        reward,
        created_at
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| ArcError::input(format!("写入签到记录失败: {err}")))?;

    let claimed = inserted.rows_affected() > 0;
    if claimed {
        let updated = sqlx::query!(
            "UPDATE user SET ticket = COALESCE(ticket, 0) + ? WHERE user_id = ?",
            reward,
            session.user.user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| ArcError::input(format!("发放签到源点失败: {err}")))?;

        if updated.rows_affected() == 0 {
            return Err(ArcError::no_data("玩家不存在", -2));
        }
    }

    let record = sqlx::query!(
        r#"
        SELECT reward_ticket as `reward_ticket!: i32`
        FROM user_checkin
        WHERE user_id = ? AND checkin_date = ?
        "#,
        session.user.user_id,
        today
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ArcError::input(format!("读取签到记录失败: {err}")))?;

    let user = sqlx::query!(
        "SELECT ticket FROM user WHERE user_id = ?",
        session.user.user_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ArcError::input(format!("查询记忆源点失败: {err}")))?;

    tx.commit()
        .await
        .map_err(|err| ArcError::input(format!("提交签到事务失败: {err}")))?;

    Ok(UserCheckinResponse {
        user: session.user.clone(),
        today: format_checkin_date(today),
        checked_in_today: true,
        claimed,
        reward: Some(record.reward_ticket),
        current_ticket: user.ticket.unwrap_or(0),
    })
}

#[get("/api/dashboard")]
pub(super) async fn admin_api_dashboard(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminDashboardApiResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(load_dashboard_api(pool.inner()).await))
}

#[get("/api/checkin")]
pub(super) async fn admin_api_checkin_status(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<UserCheckinResponse> {
    let session = require_web_session(cookies, pool.inner()).await?;
    Ok(success_return(
        load_user_checkin_status(&session, pool.inner()).await?,
    ))
}

#[post("/api/checkin")]
pub(super) async fn admin_api_checkin_claim(
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<UserCheckinResponse> {
    let session = require_web_session(cookies, pool.inner()).await?;
    Ok(success_return(
        claim_user_checkin(&session, pool.inner()).await?,
    ))
}

#[post("/api/operations/<operation_name>")]
pub(super) async fn admin_api_operation(
    operation_name: &str,
    operation_manager: &State<OperationManager>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;

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
