//! Presents (gift rewards) and redeem codes: creation, deletion, delivery and
//! lookup of redeem-code users.

use chrono::{Local, NaiveDateTime, TimeZone};
use rand::Rng;
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};

use crate::error::ArcError;
use crate::route::common::{success_return, RouteResult};
use crate::DbPool;

use super::helpers::{clean_optional_payload_text, resolve_admin_user};
use super::models::{
    AdminActionResponse, AdminPresentDeletePayload, AdminPresentDeliverPayload,
    AdminPresentPayload, AdminRedeemDeletePayload, AdminRedeemPayload, AdminRedeemUsersResponse,
    AdminUserDbSummary, AdminUserSummary,
};
use super::session::require_admin_api;

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
    let description = super::helpers::normalize_optional_text(payload.description.as_deref(), 200);
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

#[post("/api/admin-actions/presents", format = "json", data = "<payload>")]
pub(super) async fn admin_api_present_create(
    payload: Json<AdminPresentPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        create_admin_present(&payload, pool.inner()).await?,
    ))
}

#[delete("/api/admin-actions/presents", format = "json", data = "<payload>")]
pub(super) async fn admin_api_present_delete(
    payload: Json<AdminPresentDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        delete_admin_present(&payload, pool.inner()).await?,
    ))
}

#[post(
    "/api/admin-actions/presents/deliver",
    format = "json",
    data = "<payload>"
)]
pub(super) async fn admin_api_present_deliver(
    payload: Json<AdminPresentDeliverPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        deliver_admin_present(&payload, pool.inner()).await?,
    ))
}

#[post("/api/admin-actions/redeems", format = "json", data = "<payload>")]
pub(super) async fn admin_api_redeem_create(
    payload: Json<AdminRedeemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        create_admin_redeem(&payload, pool.inner()).await?,
    ))
}

#[delete("/api/admin-actions/redeems", format = "json", data = "<payload>")]
pub(super) async fn admin_api_redeem_delete(
    payload: Json<AdminRedeemDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminActionResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        delete_admin_redeem(&payload, pool.inner()).await?,
    ))
}

#[get("/api/redeem-users?<code>")]
pub(super) async fn admin_api_redeem_users(
    code: Option<&str>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminRedeemUsersResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    Ok(success_return(
        load_admin_redeem_users(code, pool.inner()).await?,
    ))
}
