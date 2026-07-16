//! Score visualisation: B30/AP30/Sex30 score images and the per-chart
//! leaderboard.

use rocket::http::CookieJar;
use rocket::{get, State};

use crate::error::ArcError;
use crate::route::common::{success_return, RouteResult};
use crate::service::{
    generate_score_image_png, generate_score_images, parse_score_image_mode, ScoreImageMode,
};
use crate::DbPool;

use super::helpers::clean_optional_payload_text;
use super::helpers::format_timestamp;
use super::models::{
    AdminChartTopResponse, AdminScoreRowView, PngResponse, ScoreImageView, ScoreImagesResponse,
};
use super::session::{require_web_session, resolve_score_image_user};

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

#[get("/api/chart-top?<sid>&<difficulty>&<limit>")]
pub(super) async fn admin_api_chart_top(
    sid: Option<&str>,
    difficulty: Option<i32>,
    limit: Option<i64>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<AdminChartTopResponse> {
    require_web_session(cookies, pool.inner()).await?;
    Ok(success_return(
        load_admin_chart_top(sid, difficulty.unwrap_or(0), limit, pool.inner()).await?,
    ))
}

#[get("/api/score-images?<user_id>&<name>&<user_code>")]
pub(super) async fn admin_api_score_images(
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> RouteResult<ScoreImagesResponse> {
    let session = require_web_session(cookies, pool.inner()).await?;
    let user = resolve_score_image_user(
        &session,
        user_id,
        clean_optional_payload_text(&name),
        clean_optional_payload_text(&user_code),
        pool.inner(),
    )
    .await?;

    let images = generate_score_images(
        pool.inner(),
        user.user_id,
        &[
            ScoreImageMode::B30,
            ScoreImageMode::Ap30,
            ScoreImageMode::Sex30,
        ],
    )
    .await?
    .into_iter()
    .map(|image| ScoreImageView {
        mode: image.mode.slug().to_string(),
        title: image.mode.title().to_string(),
        entry_count: image.entry_count,
        url: image.url,
    })
    .collect();

    Ok(success_return(ScoreImagesResponse { user, images }))
}

#[get("/api/score-images/<file_name>?<user_id>&<name>&<user_code>")]
pub(super) async fn admin_api_score_image_png(
    file_name: &str,
    user_id: Option<i32>,
    name: Option<String>,
    user_code: Option<String>,
    pool: &State<DbPool>,
    cookies: &CookieJar<'_>,
) -> Result<PngResponse, ArcError> {
    let mode_slug = file_name.strip_suffix(".png").unwrap_or(file_name);
    let mode = parse_score_image_mode(mode_slug)
        .ok_or_else(|| ArcError::input("Unsupported score image mode"))?;
    let session = require_web_session(cookies, pool.inner()).await?;
    let user = resolve_score_image_user(
        &session,
        user_id,
        clean_optional_payload_text(&name),
        clean_optional_payload_text(&user_code),
        pool.inner(),
    )
    .await?;
    let bytes = generate_score_image_png(pool.inner(), user.user_id, mode).await?;
    Ok(PngResponse { bytes })
}
