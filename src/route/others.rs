use crate::config::{ARCAEA_DATABASE_VERSION, ARCAEA_LOG_DATABASE_VERSION, ARCAEA_SERVER_VERSION};
use crate::error::ArcError;
use crate::route::common::{success_return, AuthGuard, EmptyResponse, RouteResult};
use crate::service::UserService;
use rocket::serde::json::Json;
use rocket::{get, post, routes, Route, State};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Game information response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub version: String,
    pub database_version: String,
    pub log_database_version: String,
}

/// Notification response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub title: String,
    pub message: String,
    pub timestamp: i64,
}

/// Bundle download response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleResponse {
    #[serde(rename = "orderedResults")]
    pub ordered_results: Vec<BundleItem>,
}

/// Bundle item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleItem {
    pub name: String,
    pub version: String,
    pub url: String,
    pub size: u64,
}

/// Insight complete response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightCompleteResponse {
    pub insight_state: i32,
}

/// Aggregate request structure
#[derive(Debug, Deserialize)]
pub struct AggregateCall {
    pub endpoint: String,
    pub id: Option<String>,
}

/// Aggregate response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Vec<AggregateValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

/// Aggregate value structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateValue {
    pub id: Option<String>,
    pub value: serde_json::Value,
}

/// Game information endpoint
///
/// Returns system information including server version,
/// database version, and log database version.
#[get("/game/info")]
pub async fn game_info() -> RouteResult<GameInfo> {
    let info = GameInfo {
        version: ARCAEA_SERVER_VERSION.to_string(),
        database_version: ARCAEA_DATABASE_VERSION.to_string(),
        log_database_version: ARCAEA_LOG_DATABASE_VERSION.to_string(),
    };

    Ok(success_return(info))
}

/// User notifications endpoint
///
/// Returns a list of notifications for the authenticated user.
/// Currently returns an empty list as notification system is not implemented.
#[get("/notification/me")]
pub async fn notification_me(
    _user_service: &State<UserService>,
    _auth: AuthGuard,
) -> RouteResult<Vec<NotificationResponse>> {
    // TODO: Implement notification system
    // For now, return empty list
    let notifications = Vec::new();
    Ok(success_return(notifications))
}

/// Content bundle endpoint
///
/// Returns hot update/bundle information for the client.
/// Handles app version, bundle version, and device ID from headers.
#[get("/game/content_bundle")]
pub async fn game_content_bundle(// TODO: Extract headers for app_version, bundle_version, device_id
) -> RouteResult<BundleResponse> {
    // TODO: Implement bundle download system
    // For now, return empty bundle list
    let response = BundleResponse {
        ordered_results: Vec::new(),
    };

    Ok(success_return(response))
}

/// Song download endpoint
///
/// Provides download URLs for requested songs.
/// Requires authentication and handles rate limiting.
#[get("/serve/download/me/song?<sid>&<url>")]
pub async fn download_song(
    _user_service: &State<UserService>,
    _auth: AuthGuard,
    sid: Vec<String>,
    url: Option<bool>,
) -> RouteResult<Vec<String>> {
    // TODO: Implement download system with rate limiting
    // Check if user has reached download limit
    // Generate download URLs for requested songs

    let url_flag = url.unwrap_or(true);

    if url_flag {
        // TODO: Check rate limit
        // if is_limited {
        //     return Err(ArcError::rate_limit("You have reached the download limit.", 903, -999));
        // }
    }

    // For now, return empty URLs based on requested song IDs
    let urls: Vec<String> = sid.into_iter().map(|_| String::new()).collect();
    Ok(success_return(urls))
}

/// Finale end endpoint
///
/// Handles testify end event, grants Hikari & Tairitsu (Reunion) character.
/// This is related to the Testify storyline conclusion in Arcaea.
#[post("/finale/finale_end")]
pub async fn finale_end(
    _user_service: &State<UserService>,
    _auth: AuthGuard,
) -> RouteResult<EmptyResponse> {
    // TODO: Implement character item system
    // Grant Hikari & Tairitsu (Reunion) character (ID: 5) to user

    Ok(success_return(EmptyResponse::default()))
}

/// Insight completion endpoint
///
/// Handles insight state changes and character unlocks.
/// Different pack IDs trigger different rewards and state changes.
#[post("/insight/me/complete/<pack_id>")]
pub async fn insight_complete(
    _user_service: &State<UserService>,
    _auth: AuthGuard,
    pack_id: String,
) -> RouteResult<InsightCompleteResponse> {
    let insight_state = match pack_id.as_str() {
        "eden_append_1" => {
            // TODO: Implement character item system
            // Grant Insight (Ascendant - 8th Seeker) character (ID: 72)
            // Update user insight_state to 1
            1
        }
        "lephon" => {
            // TODO: Update user insight_state to 3
            3
        }
        _ => {
            return Err(ArcError::Base {
                message: "Invalid pack_id".to_string(),
                error_code: 151,
                api_error_code: -999,
                extra_data: None,
                status: 404,
            });
        }
    };

    let response = InsightCompleteResponse { insight_state };
    Ok(success_return(response))
}

/// Application log endpoint
///
/// Receives client-side exception logs but doesn't process them.
/// Always returns success to acknowledge receipt.
#[post("/applog/me/log", data = "<_log_data>")]
pub async fn applog_me(_log_data: Json<serde_json::Value>) -> RouteResult<EmptyResponse> {
    // Exception logs are received but not processed
    Ok(success_return(EmptyResponse::default()))
}

/// Aggregate request endpoint
///
/// Handles integrated requests that combine multiple API calls.
/// Processes up to 10 requests in a single call for efficiency.
#[get("/compose/aggregate?<calls>")]
pub async fn aggregate(calls: String) -> RouteResult<AggregateResponse> {
    // Parse the calls parameter as JSON
    let call_list: Vec<AggregateCall> = match serde_json::from_str(&calls) {
        Ok(calls) => calls,
        Err(_) => {
            return Ok(success_return(AggregateResponse {
                success: false,
                value: None,
                error_code: Some(108),
                id: None,
                extra: None,
            }));
        }
    };

    // Limit to 10 requests maximum
    if call_list.len() > 10 {
        return Ok(success_return(AggregateResponse {
            success: false,
            value: None,
            error_code: Some(108),
            id: None,
            extra: None,
        }));
    }

    // TODO: Implement actual request processing
    // For each call in call_list:
    // 1. Parse the endpoint URL
    // 2. Extract query parameters
    // 3. Route to appropriate handler
    // 4. Collect responses

    // For now, return empty success response
    let response = AggregateResponse {
        success: true,
        value: Some(Vec::new()),
        error_code: None,
        id: None,
        extra: None,
    };

    Ok(success_return(response))
}

/// Get all others routes
pub fn routes() -> Vec<Route> {
    routes![
        game_info,
        notification_me,
        game_content_bundle,
        finale_end,
        insight_complete,
        applog_me,
        aggregate
    ]
}
