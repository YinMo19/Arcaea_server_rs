use chrono::Utc;

use rocket::serde::json::Json;
use rocket::{get, post, State};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::core::auth::AuthenticatedUser;
use crate::core::error::{success_return, ArcError, ArcResult, SuccessResponse};
use crate::core::notification::{init_notification_db, NotificationFactory};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameInfo {
    pub max_stamina: i32,
    pub stamina_recover_tick: i64,
    pub core_exp: i32,
    pub world_ranking_enabled: bool,
    pub is_byd_chapter_unlocked: bool,
    pub curr_ts: i64,
    pub server_update_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentBundleItem {
    pub name: String,
    pub version: String,
    pub url: String,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentBundleResponse {
    #[serde(rename = "orderedResults")]
    pub ordered_results: Vec<ContentBundleItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadSongResponse {
    pub urls: Vec<DownloadUrlItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadUrlItem {
    pub sid: String,
    pub url: String,
}

#[derive(Debug, FromForm)]
pub struct DownloadSongQuery {
    pub sid: Vec<String>,
    #[field(default = "true")]
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinaleProgressResponse {
    pub percentage: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsightCompleteResponse {
    pub insight_state: i32,
}

#[get("/game/info")]
pub async fn game_info() -> Result<Json<SuccessResponse<GameInfo>>, ArcError> {
    let game_info = GameInfo {
        max_stamina: 6,
        stamina_recover_tick: 1800000, // 30 minutes in milliseconds
        core_exp: 250,
        world_ranking_enabled: true,
        is_byd_chapter_unlocked: true,
        curr_ts: Utc::now().timestamp_millis(),
        server_update_available: false,
    };

    Ok(success_return(game_info))
}

#[get("/notification/me")]
pub async fn notification_me(
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<Vec<serde_json::Value>>>, ArcError> {
    // Initialize notification database if not already done
    let _ = init_notification_db().await;

    let notification_factory = NotificationFactory::new(user.user.user_id).await?;
    let notifications = notification_factory.get_notifications().await?;

    Ok(success_return(notifications))
}

#[get("/game/content_bundle")]
pub async fn game_content_bundle(// TODO: Parse headers like Python version
    // app_version: Option<String>,
    // bundle_version: Option<String>,
    // device_id: Option<String>,
) -> Result<Json<SuccessResponse<ContentBundleResponse>>, ArcError> {
    // For now, return empty bundle list
    // In production, this would check version compatibility and return appropriate bundles
    let response = ContentBundleResponse {
        ordered_results: vec![],
    };

    Ok(success_return(response))
}

#[get("/serve/download/me/song?<query..>")]
pub async fn download_song(
    _pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    query: DownloadSongQuery,
) -> Result<Json<SuccessResponse<DownloadSongResponse>>, ArcError> {
    let url_flag = query.url == "true";

    if !url_flag {
        // Just return empty URLs if URL flag is false
        let response = DownloadSongResponse { urls: vec![] };
        return Ok(success_return(response));
    }

    // Check download limits (simplified)
    // For demo purposes, assume no download limit reached
    let mut urls = Vec::new();

    for song_id in query.sid {
        // Generate download URL with token
        let token = format!("{}_{}", user.user.user_id, Utc::now().timestamp());
        let url = format!("/download/{}/song.pkg?t={}", song_id, token);

        urls.push(DownloadUrlItem { sid: song_id, url });
    }

    let response = DownloadSongResponse { urls };
    Ok(success_return(response))
}

#[get("/finale/progress")]
pub async fn finale_progress() -> Result<Json<SuccessResponse<FinaleProgressResponse>>, ArcError> {
    let response = FinaleProgressResponse {
        percentage: 100000, // World boss at 100%
    };

    Ok(success_return(response))
}

#[post("/finale/finale_start")]
pub async fn finale_start(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ArcError> {
    // Give Hikari (Fatalis) character (ID 55)
    give_character_to_user(pool, user.user.user_id, 55).await?;

    Ok(success_return(serde_json::json!({})))
}

#[post("/finale/finale_end")]
pub async fn finale_end(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ArcError> {
    // Give Hikari & Tairitsu (Reunion) character (ID 5)
    give_character_to_user(pool, user.user.user_id, 5).await?;

    Ok(success_return(serde_json::json!({})))
}

#[post("/insight/me/complete/<pack_id>")]
pub async fn insight_complete(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    pack_id: String,
) -> Result<Json<SuccessResponse<InsightCompleteResponse>>, ArcError> {
    let new_insight_state = match pack_id.as_str() {
        "eden_append_1" => {
            // Give Insight (Ascendant - 8th Seeker) character (ID 72)
            give_character_to_user(pool, user.user.user_id, 72).await?;

            // Update insight state to 1
            sqlx::query("UPDATE user SET insight_state = 1 WHERE user_id = ?")
                .bind(user.user.user_id)
                .execute(pool.inner())
                .await?;

            1
        }
        "lephon" => {
            // Update insight state to 3
            sqlx::query("UPDATE user SET insight_state = 3 WHERE user_id = ?")
                .bind(user.user.user_id)
                .execute(pool.inner())
                .await?;

            3
        }
        _ => {
            return Err(ArcError::with_error_code("Invalid pack_id", 151));
        }
    };

    let response = InsightCompleteResponse {
        insight_state: new_insight_state,
    };

    Ok(success_return(response))
}

#[post("/applog/me/log")]
pub async fn applog_me() -> Result<Json<SuccessResponse<serde_json::Value>>, ArcError> {
    // Just accept and ignore app logs
    Ok(success_return(serde_json::json!({})))
}

#[get("/compose/aggregate?<calls>")]
pub async fn aggregate(
    pool: &State<SqlitePool>,
    user: Option<AuthenticatedUser>,
    calls: String,
) -> Result<Json<SuccessResponse<Vec<serde_json::Value>>>, ArcError> {
    // Parse the calls JSON
    let call_list: Vec<serde_json::Value> =
        serde_json::from_str(&calls).map_err(|_| ArcError::input_error("Invalid calls format"))?;

    if call_list.len() > 10 {
        return Err(ArcError::input_error("Too many calls in aggregate request"));
    }

    let mut responses = Vec::new();

    for call in call_list {
        let id = call.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let endpoint = call.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");

        // Parse endpoint URL
        let url = url::Url::parse(&format!("http://localhost{}", endpoint))
            .map_err(|_| ArcError::input_error("Invalid endpoint URL"))?;

        let path = url.path();

        // Route to appropriate handler based on path
        let response_value = match path {
            "/user/me" => {
                if let Some(ref auth_user) = user {
                    // Call user_me equivalent
                    let cores = get_user_cores_simple(pool, auth_user.user.user_id)
                        .await
                        .unwrap_or_default();
                    serde_json::json!({
                        "user_id": auth_user.user.user_id,
                        "name": auth_user.user.name,
                        "user_code": auth_user.user.user_code,
                        "rating_ptt": auth_user.user.rating_ptt,
                        "character_id": auth_user.user.character_id,
                        "cores": cores
                    })
                } else {
                    return Err(ArcError::no_access("Authentication required"));
                }
            }
            "/game/info" => {
                serde_json::json!({
                    "max_stamina": 6,
                    "stamina_recover_tick": 1800000,
                    "core_exp": 250,
                    "world_ranking_enabled": true,
                    "is_byd_chapter_unlocked": true,
                    "curr_ts": Utc::now().timestamp_millis(),
                    "server_update_available": false
                })
            }
            "/finale/progress" => {
                serde_json::json!({
                    "percentage": 100000
                })
            }
            _ => {
                serde_json::json!({
                    "error": "Endpoint not supported in aggregate"
                })
            }
        };

        responses.push(serde_json::json!({
            "id": id,
            "value": response_value
        }));
    }

    Ok(success_return(responses))
}

// Helper functions

async fn give_character_to_user(
    pool: &State<SqlitePool>,
    user_id: i32,
    character_id: i32,
) -> ArcResult<()> {
    // Check if user already has this character
    let existing =
        sqlx::query("SELECT user_id FROM user_char WHERE user_id = ? AND character_id = ?")
            .bind(user_id)
            .bind(character_id)
            .fetch_optional(pool.inner())
            .await?;

    if existing.is_none() {
        // Give character to user
        sqlx::query(
            "INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag) VALUES (?, ?, 1, 0, 0, 0, 0)"
        )
        .bind(user_id)
        .bind(character_id)
        .execute(pool.inner())
        .await?;
    }

    Ok(())
}

async fn get_user_cores_simple(
    pool: &State<SqlitePool>,
    user_id: i32,
) -> ArcResult<Vec<serde_json::Value>> {
    let rows =
        sqlx::query("SELECT item_id, amount FROM user_item WHERE user_id = ? AND type = 'core'")
            .bind(user_id)
            .fetch_all(pool.inner())
            .await?;

    let mut cores = Vec::new();
    for row in rows {
        cores.push(serde_json::json!({
            "item_id": row.get::<String, _>("item_id"),
            "amount": row.get::<i32, _>("amount")
        }));
    }

    Ok(cores)
}
