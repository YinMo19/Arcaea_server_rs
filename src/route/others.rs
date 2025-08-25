use crate::context::{ClientContext, VersionContext};
use crate::error::ArcError;
use crate::model::{
    AggregateCall, AggregateResponse, AggregateValue, InsightCompleteResponse, NotificationResponse,
};
use crate::route::common::{success_return, AuthGuard, EmptyResponse, RouteResult};
use crate::service::aggregate::*;
use crate::service::bundle::BundleDownloadResponse;
use crate::service::{
    BundleService, CharacterService, DownloadService, NotificationService, PresentService,
    PurchaseService, ScoreService, UserService, WorldService,
};
use rocket::fs::NamedFile;
use rocket::http::Status;

use rocket::response::status;
use rocket::serde::json::{Json, Value};
use rocket::{get, post, routes, Route, State};
use std::collections::HashMap;
use url::Url;
use urlencoding::decode;

/// Game information endpoint
///
/// Returns system information including server version,
/// database version, and log database version.
#[get("/game/info")]
pub async fn game_info() -> RouteResult<Value> {
    let info = handle_game_info().await?;

    Ok(success_return(info))
}

/// User notifications endpoint
///
/// Returns a list of notifications for the authenticated user.
/// Currently returns an empty list as notification system is not implemented.
#[get("/notification/me")]
pub async fn notification_me(
    notification_service: &State<NotificationService>,
    auth: AuthGuard,
) -> RouteResult<Vec<NotificationResponse>> {
    let notifications = notification_service
        .get_user_notifications(auth.user_id)
        .await?;
    Ok(success_return(notifications))
}

/// Content bundle endpoint
///
/// Returns hot update/bundle information for the client.
/// Handles app version, bundle version, and device ID from headers.
#[get("/game/content_bundle")]
pub async fn game_content_bundle(
    version_ctx: VersionContext<'_>,
    bundle_service: &State<BundleService>,
) -> RouteResult<BundleDownloadResponse> {
    let app_version = match version_ctx.app_version {
        Some(version) => version,
        None => return Err(ArcError::no_data("no app version provided", 404)),
    };
    let ordered_results = bundle_service
        .get_bundle_list(
            app_version,
            version_ctx.bundle_version,
            version_ctx.device_id,
        )
        .await?;

    let response = BundleDownloadResponse { ordered_results };
    Ok(success_return(response))
}

/// Finale progress endpoint
///
/// return full percentage.
#[post("/finale/progress")]
pub async fn finale_progress(_auth: AuthGuard) -> RouteResult<Value> {
    // world boss percentage
    Ok(success_return(serde_json::json!({"percentage": 100000})))
}

/// Finale start endpoint
///
/// Grants Hikari (Fatalis) character to the user.
/// Used when the Testify finale begins.
#[post("/finale/finale_start")]
pub async fn finale_start(
    character_service: &State<CharacterService>,
    auth: AuthGuard,
) -> RouteResult<EmptyResponse> {
    // Grant Hikari (Fatalis) character (ID: 55) to user
    character_service.grant_hikari_fatalis(auth.user_id).await?;

    Ok(success_return(EmptyResponse::default()))
}

/// Finale end endpoint
///
/// Grants Hikari & Tairitsu (Reunion) character to the user.
#[post("/finale/finale_end")]
pub async fn finale_end(
    character_service: &State<CharacterService>,
    auth: AuthGuard,
) -> RouteResult<EmptyResponse> {
    // Grant Hikari & Tairitsu (Reunion) character (ID: 5) to user
    character_service
        .grant_hikari_tairitsu_reunion(auth.user_id)
        .await?;

    Ok(success_return(EmptyResponse::default()))
}

/// Insight completion endpoint
///
/// Handles insight state changes and character unlocks.
/// Different pack IDs trigger different rewards and state changes.
#[post("/insight/me/complete/<pack_id>")]
pub async fn insight_complete(
    character_service: &State<CharacterService>,
    user_service: &State<UserService>,
    auth: AuthGuard,
    pack_id: String,
) -> RouteResult<InsightCompleteResponse> {
    let new_insight_state = match pack_id.as_str() {
        "eden_append_1" => {
            // Grant Insight (Ascendant - 8th Seeker) character (ID: 72)
            character_service
                .grant_insight_ascendant(auth.user_id)
                .await?;
            // Update user insight_state to 1
            user_service
                .update_user_insight_state(auth.user_id, 1)
                .await?;
            1
        }
        "lephon" => {
            // Update user insight_state to 3
            user_service
                .update_user_insight_state(auth.user_id, 3)
                .await?;
            3
        }
        _ => {
            return Err(ArcError::input(format!("Invalid pack_id: {}", pack_id)));
        }
    };

    Ok(success_return(InsightCompleteResponse {
        insight_state: new_insight_state,
    }))
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
pub async fn aggregate(
    calls: String,
    user_service: &State<UserService>,
    score_service: &State<ScoreService>,
    download_service: &State<DownloadService>,
    present_service: &State<PresentService>,
    world_service: &State<WorldService>,
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
) -> Result<AggregateResponse, ArcError> {
    // Parse the calls parameter as JSON
    let call_list: Vec<AggregateCall> = match serde_json::from_str(&calls) {
        Ok(calls) => calls,
        Err(_) => {
            return Ok(AggregateResponse {
                success: false,
                value: None,
                error_code: Some(108),
                id: None,
                extra: None,
            });
        }
    };

    // Limit to 10 requests maximum
    if call_list.len() > 10 {
        return Ok(AggregateResponse {
            success: false,
            value: None,
            error_code: Some(108),
            id: None,
            extra: None,
        });
    }

    let mut response_values = Vec::new();

    // Process each request
    for call in call_list {
        let endpoint_url = match Url::parse(&format!("http://localhost{}", call.endpoint)) {
            Ok(url) => url,
            Err(_) => {
                continue; // Skip invalid URLs
            }
        };

        let path = endpoint_url.path();
        let query_params = parse_query_params(endpoint_url.query().unwrap_or(""));

        // Route to appropriate handler based on path
        let result = match path {
            "/user/me" => handle_user_me(user_service, auth.user_id).await,
            "/purchase/bundle/pack" => handle_bundle_pack(purchase_service, auth.user_id).await,
            "/serve/download/me/song" => {
                handle_download_song(download_service, user_service, auth.user_id, &query_params)
                    .await
            }
            "/game/info" => handle_game_info().await,
            "/present/me" => handle_present_info(present_service, auth.user_id).await,
            "/world/map/me" => handle_world_all(world_service, auth.user_id).await,
            "/score/song/friend" => {
                handle_song_score_friend(score_service, user_service, auth.user_id, &query_params)
                    .await
            }
            "/finale/progress" => handle_finale_progress().await,
            "/purchase/bundle/bundle" => handle_bundle_bundle().await,
            "/purchase/bundle/single" => handle_bundle_single(purchase_service, auth.user_id).await,
            _ => Err(ArcError::no_data("Endpoint not found in aggregate", 404)),
        };

        match result {
            Ok(value) => {
                response_values.push(AggregateValue {
                    id: call.id.clone(),
                    value,
                });
            }
            Err(e) => {
                log::warn!("{}", e);
                // Return error response immediately on first error
                return Ok(AggregateResponse {
                    success: false,
                    value: None,
                    error_code: Some(e.error_code()),
                    id: call.id,
                    extra: e.extra_data().cloned(),
                });
            }
        }
    }

    let response = AggregateResponse {
        success: true,
        value: Some(response_values),
        error_code: None,
        id: None,
        extra: None,
    };

    // log::info!("resp: {}", &serde_json::to_string(&response).unwrap());
    Ok(response)
}

/// Bundle download endpoint
///
/// Serves bundle files (JSON and CB) using download tokens.
/// Handles rate limiting for bundle files and token validation.
#[get("/bundle_download/<token>")]
pub async fn bundle_download(
    bundle_service: &State<BundleService>,
    token: &str,
    ctx: ClientContext<'_>,
) -> Result<NamedFile, status::Custom<String>> {
    // Get client IP for rate limiting
    let client_ip = ctx.get_client_ip().unwrap_or_else(|| "127.0.0.1");

    match bundle_service
        .get_file_path_by_token(&token, client_ip)
        .await
    {
        Ok(file_path) => {
            // Get the bundle file path directly from the service
            match bundle_service.get_bundle_file_path(&file_path).await {
                Ok(full_path) => match NamedFile::open(full_path).await {
                    Ok(file) => Ok(file),
                    Err(_) => Err(status::Custom(
                        Status::NotFound,
                        "File not found".to_string(),
                    )),
                },
                Err(e) => {
                    let status_code = match e.status() {
                        403 => Status::Forbidden,
                        404 => Status::NotFound,
                        429 => Status::TooManyRequests,
                        _ => Status::InternalServerError,
                    };
                    Err(status::Custom(status_code, e.to_string()))
                }
            }
        }
        Err(e) => {
            let status_code = match e.status() {
                403 => Status::Forbidden,
                404 => Status::NotFound,
                429 => Status::TooManyRequests,
                _ => Status::InternalServerError,
            };
            Err(status::Custom(status_code, e.to_string()))
        }
    }
}

/// Parse query parameters from URL query string
fn parse_query_params(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    if query.is_empty() {
        return params;
    }

    for pair in query.split('&') {
        if let Some(eq_pos) = pair.find('=') {
            let key = &pair[..eq_pos];
            let value = &pair[eq_pos + 1..];
            if let (Ok(decoded_key), Ok(decoded_value)) = (decode(key), decode(value)) {
                params.insert(decoded_key.to_string(), decoded_value.to_string());
            }
        }
    }
    params
}

/// Get all others routes
pub fn routes() -> Vec<Route> {
    routes![
        game_info,
        notification_me,
        game_content_bundle,
        finale_progress,
        finale_start,
        finale_end,
        insight_complete,
        applog_me,
        aggregate,
        bundle_download
    ]
}
