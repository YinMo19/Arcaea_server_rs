use crate::error::ArcError;

use crate::service::{
    DownloadService, PresentService, PurchaseService, ScoreService, UserService, WorldService,
};

use crate::Constants;

use std::collections::HashMap;

fn python_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(|v| v != 0)
            .or_else(|| n.as_u64().map(|v| v != 0))
            .or_else(|| n.as_f64().map(|v| v != 0.0))
            .unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

/// Handle /user/me endpoint
pub async fn handle_user_me(
    user_service: &UserService,
    user_id: i32,
) -> Result<serde_json::Value, ArcError> {
    let user_info = user_service.get_user_info(user_id).await?;
    serde_json::to_value(&user_info).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })
}

/// Handle /game/info endpoint
pub async fn handle_game_info() -> Result<serde_json::Value, ArcError> {
    let level_step = Constants::get_level_steps();
    let mut level_step: Vec<HashMap<&str, &i32>> = level_step
        .iter()
        .map(|(level, level_exp)| HashMap::from([("level", level), ("level_exp", level_exp)]))
        .collect();
    level_step.sort_by(|a, b| a["level"].cmp(b["level"]));

    Ok(serde_json::json!({"max_stamina": 12,
    "stamina_recover_tick": 1800000,
    "core_exp": 250,
    "curr_ts": chrono::Utc::now().timestamp_millis(),
    "level_steps": level_step,
    "world_ranking_enabled": true,
    "is_byd_chapter_unlocked": true}))
}

/// Handle /present/me endpoint
pub async fn handle_present_info(
    present_service: &PresentService,
    user_id: i32,
) -> Result<serde_json::Value, ArcError> {
    let presents = present_service.get_user_presents(user_id).await?;
    let present_list = presents.iter().map(|p| p.to_dict(true)).collect::<Vec<_>>();
    serde_json::to_value(&present_list).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })
}

/// Handle /world/map/me endpoint
pub async fn handle_world_all(
    world_service: &WorldService,
    user_id: i32,
) -> Result<serde_json::Value, ArcError> {
    world_service.get_user_world_all(user_id).await
}

/// Handle /score/song/friend endpoint
pub async fn handle_song_score_friend(
    score_service: &ScoreService,
    _user_service: &UserService,
    user_id: i32,
    query_params: &HashMap<String, String>,
) -> Result<serde_json::Value, ArcError> {
    let song_id = query_params
        .get("song_id")
        .ok_or_else(|| ArcError::input("song_id is required"))?;
    let difficulty = query_params
        .get("difficulty")
        .and_then(|v| v.parse::<i32>().ok())
        .ok_or_else(|| ArcError::input("difficulty is required"))?;

    let scores = score_service
        .get_friend_song_ranks(user_id, song_id, difficulty)
        .await?;

    Ok(serde_json::to_value(scores)?)
}

/// Handle /serve/download/me/song endpoint
pub async fn handle_download_song(
    download_service: &DownloadService,
    user_service: &UserService,
    user_id: i32,
    query_params: &HashMap<String, String>,
) -> Result<serde_json::Value, ArcError> {
    // Get user info for permission checking
    let user_info = user_service.get_user_info(user_id).await?;

    // Parse song IDs from query parameters
    let song_ids = query_params.get("sid").map(|s| {
        s.split(',')
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
    });

    // Parse URL flag (defaults to true)
    let url_flag = match query_params.get("url") {
        Some(raw) => {
            let parsed: serde_json::Value = serde_json::from_str(raw)
                .map_err(|_| ArcError::input("Invalid `url` query value"))?;
            python_truthy(&parsed)
        }
        None => true,
    };

    // Check rate limiting if URLs are requested
    if url_flag && download_service.check_download_limit(user_id).await? {
        return Err(ArcError::rate_limit(
            "You have reached the download limit.".to_string(),
            903,
        ));
    }

    // Generate download list
    let download_songs = download_service
        .generate_download_list(&user_info, song_ids, url_flag)
        .await?;

    // Convert to the expected format
    Ok(serde_json::to_value(download_songs)?)
}

/// Handle /purchase/bundle/pack endpoint
pub async fn handle_bundle_pack(
    purchase_service: &PurchaseService,
    user_id: i32,
) -> Result<serde_json::Value, ArcError> {
    let packs = purchase_service.get_pack_purchases(user_id).await?;
    serde_json::to_value(&packs).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })
}

/// Handle /purchase/bundle/bundle endpoint
pub async fn handle_bundle_bundle() -> Result<serde_json::Value, ArcError> {
    Ok(serde_json::json!([]))
}

/// Handle /purchase/bundle/single endpoint
pub async fn handle_bundle_single(
    purchase_service: &PurchaseService,
    user_id: i32,
) -> Result<serde_json::Value, ArcError> {
    let singles = purchase_service.get_single_purchases(user_id).await?;
    serde_json::to_value(&singles).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })
}

/// Handle /finale/progress endpoint
pub async fn handle_finale_progress() -> Result<serde_json::Value, ArcError> {
    Ok(serde_json::json!({
        "percentage": 100000
    }))
}
