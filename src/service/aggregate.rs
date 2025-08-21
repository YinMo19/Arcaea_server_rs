use crate::config::{ARCAEA_DATABASE_VERSION, ARCAEA_LOG_DATABASE_VERSION, ARCAEA_SERVER_VERSION};
use crate::error::ArcError;

use crate::service::{
    DownloadService, ItemService, PresentService, PurchaseService, ScoreService, UserService,
    WorldService,
};

use crate::model::GameInfo;

use std::collections::HashMap;

/// Handle /user/me endpoint
pub async fn handle_user_me(
    user_service: &UserService,
    item_service: &ItemService,
    user_id: i32,
) -> Result<serde_json::Value, ArcError> {
    let user_info = user_service.get_user_info(user_id).await?;
    serde_json::to_value(&user_info).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })
}

/// Handle /game/info endpoint
pub async fn handle_game_info() -> Result<serde_json::Value, ArcError> {
    let info = GameInfo {
        version: ARCAEA_SERVER_VERSION.to_string(),
        database_version: ARCAEA_DATABASE_VERSION.to_string(),
        log_database_version: ARCAEA_LOG_DATABASE_VERSION.to_string(),
    };
    serde_json::to_value(&info).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })
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
    _score_service: &ScoreService,
    _user_service: &UserService,
    _user_id: i32,
    query_params: &HashMap<String, String>,
) -> Result<serde_json::Value, ArcError> {
    // Get song_id and difficulty from query params
    let _song_id = query_params.get("song_id");
    let _difficulty = query_params.get("difficulty");

    // For now, return empty friend score list since friend system is not implemented
    // In the future, this should query friend scores from the database
    Ok(serde_json::json!([]))
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
    let url_flag = query_params
        .get("url")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

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
