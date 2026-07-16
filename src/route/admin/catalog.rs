//! Catalog data tables: songs (charts), items, purchases and purchase-items.
//! Covers list loading with pagination plus create/update/delete for each.

use chrono::{Local, NaiveDateTime, TimeZone};
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::{delete, get, patch, post, State};
use std::collections::HashMap;

use crate::route::common::{success_return, success_return_no_value, EmptyResponse, RouteResult};
use crate::utils::sql_placeholders;
use crate::DbPool;

use super::helpers::{
    admin_api_input_error, clamp_page, like_filter, normalize_page, page_response,
};
use super::models::{
    AdminItemDeletePayload, AdminItemPayload, AdminPageResponse, AdminPurchaseDeletePayload,
    AdminPurchaseItemDeletePayload, AdminPurchaseItemPayload, AdminPurchasePayload,
    AdminSongDeletePayload, AdminSongInput, AdminSongPayload, ChartConstantsPayload, ChartDbRow,
    ItemDbRow, ItemRowView, PurchaseDbRow, PurchaseItemDbRow, PurchaseItemRowView, PurchaseRowView,
    SongRowView,
};
use super::session::{require_admin_api, require_chart_constant_edit_api, require_web_session};

// Parsing helpers local to the catalog

fn normalize_chart_text(raw: &str, field: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{field} 不能为空"));
    }

    let truncated: String = value.chars().take(200).collect();
    Ok(truncated)
}

fn format_rating_input_tenths(value: Option<i32>) -> String {
    match value {
        Some(v) if v >= 0 => format!("{:.1}", v as f64 / 10.0),
        _ => "-1".to_string(),
    }
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

fn normalize_item_available(value: Option<i32>) -> i32 {
    if value.unwrap_or(0) != 0 {
        1
    } else {
        0
    }
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

// List loaders

async fn load_admin_songs(
    q: Option<&str>,
    page: i64,
    page_size: i64,
    pool: &DbPool,
) -> AdminPageResponse<SongRowView> {
    let (where_sql, binds) = like_filter(q, &["song_id", "name"]);

    let count_sql = format!("SELECT COUNT(*) FROM chart{where_sql}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for value in &binds {
        count_query = count_query.bind(value);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT *
         FROM chart{where_sql}
         ORDER BY song_id ASC
         LIMIT ? OFFSET ?"
    );
    let mut rows_query = sqlx::query_as::<_, ChartDbRow>(&row_sql);
    for value in &binds {
        rows_query = rows_query.bind(value);
    }
    let rows = rows_query
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
    let (where_sql, binds) = like_filter(q, &["item_id", "type"]);

    let count_sql = format!("SELECT COUNT(*) FROM item{where_sql}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for value in &binds {
        count_query = count_query.bind(value);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT *
         FROM item{where_sql}
         ORDER BY type, item_id
         LIMIT ? OFFSET ?"
    );
    let mut rows_query = sqlx::query_as::<_, ItemDbRow>(&row_sql);
    for value in &binds {
        rows_query = rows_query.bind(value);
    }
    let rows = rows_query
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
    let (where_sql, binds) = like_filter(pq, &["purchase_name", "COALESCE(discount_reason, '')"]);

    let count_sql = format!("SELECT COUNT(*) FROM purchase{where_sql}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for value in &binds {
        count_query = count_query.bind(value);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT *
         FROM purchase{where_sql}
         ORDER BY purchase_name ASC
         LIMIT ? OFFSET ?"
    );
    let mut purchase_query = sqlx::query_as::<_, PurchaseDbRow>(&row_sql);
    for value in &binds {
        purchase_query = purchase_query.bind(value);
    }
    let purchase_rows = purchase_query
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
        let purchase_placeholders = sql_placeholders(purchase_names.len());
        let item_sql = format!(
            "SELECT *
             FROM purchase_item
             WHERE purchase_name IN ({purchase_placeholders})
             ORDER BY purchase_name ASC, item_id ASC, type ASC"
        );
        let mut purchase_item_query = sqlx::query_as::<_, PurchaseItemDbRow>(&item_sql);
        for purchase_name in &purchase_names {
            purchase_item_query = purchase_item_query.bind(purchase_name);
        }
        let purchase_item_rows = purchase_item_query
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
    let (where_sql, binds) = like_filter(iq, &["purchase_name", "item_id", "type"]);

    let count_sql = format!("SELECT COUNT(*) FROM purchase_item{where_sql}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for value in &binds {
        count_query = count_query.bind(value);
    }
    let total = count_query.fetch_one(pool).await.unwrap_or(0);
    let (page, offset) = clamp_page(page, page_size, total);

    let row_sql = format!(
        "SELECT *
         FROM purchase_item{where_sql}
         ORDER BY purchase_name ASC, item_id ASC, type ASC
         LIMIT ? OFFSET ?"
    );
    let mut purchase_item_query = sqlx::query_as::<_, PurchaseItemDbRow>(&row_sql);
    for value in &binds {
        purchase_item_query = purchase_item_query.bind(value);
    }
    let purchase_items = purchase_item_query
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

// Song (chart) CRUD

async fn create_song(pool: &DbPool, input: AdminSongInput<'_>) -> Result<(), String> {
    let sid = normalize_chart_text(input.sid, "song_id")?;
    let name_en = normalize_chart_text(input.name_en, "name_en")?;
    let rating_pst = parse_rating_input_tenths(input.rating_pst, "rating_pst")?;
    let rating_prs = parse_rating_input_tenths(input.rating_prs, "rating_prs")?;
    let rating_ftr = parse_rating_input_tenths(input.rating_ftr, "rating_ftr")?;
    let rating_byd = parse_rating_input_tenths(input.rating_byd, "rating_byd")?;
    let rating_etr = parse_rating_input_tenths(input.rating_etr, "rating_etr")?;

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

async fn update_song(pool: &DbPool, input: AdminSongInput<'_>) -> Result<(), String> {
    let sid = normalize_chart_text(input.sid, "song_id")?;
    let name_en = normalize_chart_text(input.name_en, "name_en")?;
    let rating_pst = parse_rating_input_tenths(input.rating_pst, "rating_pst")?;
    let rating_prs = parse_rating_input_tenths(input.rating_prs, "rating_prs")?;
    let rating_ftr = parse_rating_input_tenths(input.rating_ftr, "rating_ftr")?;
    let rating_byd = parse_rating_input_tenths(input.rating_byd, "rating_byd")?;
    let rating_etr = parse_rating_input_tenths(input.rating_etr, "rating_etr")?;

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

    if done.rows_affected() == 0 {
        return Err("歌曲不存在".to_string());
    }

    Ok(())
}

async fn update_chart_constants(
    pool: &DbPool,
    sid_raw: &str,
    payload: &ChartConstantsPayload,
) -> Result<(), String> {
    let sid = normalize_chart_text(sid_raw, "song_id")?;
    let rating_pst = parse_rating_input_tenths(&payload.rating_pst, "rating_pst")?;
    let rating_prs = parse_rating_input_tenths(&payload.rating_prs, "rating_prs")?;
    let rating_ftr = parse_rating_input_tenths(&payload.rating_ftr, "rating_ftr")?;
    let rating_byd = parse_rating_input_tenths(&payload.rating_byd, "rating_byd")?;
    let rating_etr = parse_rating_input_tenths(&payload.rating_etr, "rating_etr")?;

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) as `count!: i64` FROM chart WHERE song_id = ?",
        sid
    )
    .fetch_one(pool)
    .await
    .map_err(|err| format!("查询失败: {err}"))?;
    if exists == 0 {
        return Err("歌曲不存在".to_string());
    }

    sqlx::query!(
        "UPDATE chart
         SET rating_pst = ?, rating_prs = ?, rating_ftr = ?, rating_byn = ?, rating_etr = ?
         WHERE song_id = ?",
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

    Ok(())
}

async fn delete_song(pool: &DbPool, sid_raw: &str) -> Result<(), String> {
    let sid = normalize_chart_text(sid_raw, "song_id")?;
    let done = sqlx::query!("DELETE FROM chart WHERE song_id = ?", sid)
        .execute(pool)
        .await
        .map_err(|err| format!("删除失败: {err}"))?;

    if done.rows_affected() == 0 {
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

    if done.rows_affected() == 0 {
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

    if done.rows_affected() == 0 {
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
    let discount_reason = super::helpers::normalize_optional_text(discount_reason_raw, 255);

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
    let discount_reason = super::helpers::normalize_optional_text(discount_reason_raw, 255);

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

    if done.rows_affected() == 0 {
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

    if done.rows_affected() == 0 {
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

    if done.rows_affected() == 0 {
        return Err("购买项物品不存在".to_string());
    }

    Ok(())
}

// Route handlers

#[get("/api/songs?<q>&<page>&<page_size>")]
pub(super) async fn admin_api_songs(
    q: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<SongRowView>> {
    require_chart_constant_edit_api(cookies, pool.inner()).await?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_songs(q, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/items?<q>&<page>&<page_size>")]
pub(super) async fn admin_api_items(
    q: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<ItemRowView>> {
    require_web_session(cookies, pool.inner()).await?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_items(q, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/purchases?<pq>&<page>&<page_size>")]
pub(super) async fn admin_api_purchases(
    pq: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<PurchaseRowView>> {
    require_web_session(cookies, pool.inner()).await?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_purchases(pq, page, page_size, pool.inner()).await,
    ))
}

#[get("/api/purchase-items?<iq>&<page>&<page_size>")]
pub(super) async fn admin_api_purchase_items(
    iq: Option<&str>,
    page: Option<i64>,
    page_size: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminPageResponse<PurchaseItemRowView>> {
    require_admin_api(cookies, pool.inner()).await?;
    let (page, page_size) = normalize_page(page, page_size);
    Ok(success_return(
        load_admin_purchase_items(iq, page, page_size, pool.inner()).await,
    ))
}

#[post("/api/songs", format = "json", data = "<payload>")]
pub(super) async fn admin_api_song_create(
    payload: Json<AdminSongPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    create_song(pool.inner(), AdminSongInput::from(&*payload))
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[patch("/api/songs/<sid>", format = "json", data = "<payload>")]
pub(super) async fn admin_api_song_update(
    sid: &str,
    payload: Json<AdminSongPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    update_song(pool.inner(), AdminSongInput::from(&*payload).with_sid(sid))
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[patch("/api/songs/<sid>/constants", format = "json", data = "<payload>")]
pub(super) async fn admin_api_chart_constants_update(
    sid: &str,
    payload: Json<ChartConstantsPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_chart_constant_edit_api(cookies, pool.inner()).await?;
    update_chart_constants(pool.inner(), sid, &payload)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[delete("/api/songs", format = "json", data = "<payload>")]
pub(super) async fn admin_api_song_delete(
    payload: Json<AdminSongDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    delete_song(pool.inner(), &payload.sid)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[post("/api/items", format = "json", data = "<payload>")]
pub(super) async fn admin_api_item_create(
    payload: Json<AdminItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
pub(super) async fn admin_api_item_update(
    payload: Json<AdminItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
pub(super) async fn admin_api_item_delete(
    payload: Json<AdminItemDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    delete_item(pool.inner(), &payload.item_id, &payload.item_type)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[post("/api/purchases", format = "json", data = "<payload>")]
pub(super) async fn admin_api_purchase_create(
    payload: Json<AdminPurchasePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
pub(super) async fn admin_api_purchase_update(
    purchase_name: &str,
    payload: Json<AdminPurchasePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
pub(super) async fn admin_api_purchase_delete(
    payload: Json<AdminPurchaseDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
    delete_purchase(pool.inner(), &payload.purchase_name)
        .await
        .map_err(admin_api_input_error)?;
    Ok(success_return_no_value())
}

#[post("/api/purchase-items", format = "json", data = "<payload>")]
pub(super) async fn admin_api_purchase_item_create(
    payload: Json<AdminPurchaseItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
pub(super) async fn admin_api_purchase_item_update(
    payload: Json<AdminPurchaseItemPayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
pub(super) async fn admin_api_purchase_item_delete(
    payload: Json<AdminPurchaseItemDeletePayload>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<EmptyResponse> {
    require_admin_api(cookies, pool.inner()).await?;
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
