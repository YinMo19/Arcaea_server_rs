use crate::route::common::{success_return, AuthGuard, EmptyResponse, RouteResult};
use crate::service::{MatchmakingJoinRequest, MultiplayerService};
use rocket::serde::json::{Json, Value};
use rocket::{post, routes, Route, State};

/// Matchmaking join endpoint
///
/// Python baseline: `POST /multiplayer/me/matchmaking/join/`
#[post("/multiplayer/me/matchmaking/join", data = "<request>")]
pub async fn matchmaking_join(
    multiplayer_service: &State<MultiplayerService>,
    auth: AuthGuard,
    request: Json<MatchmakingJoinRequest>,
) -> RouteResult<Value> {
    let result = multiplayer_service
        .matchmaking_join(auth.user_id, &request.client_song_map)
        .await?;
    Ok(success_return(result))
}

/// Matchmaking status endpoint
///
/// Python baseline: `POST /multiplayer/me/matchmaking/status/`
#[post("/multiplayer/me/matchmaking/status")]
pub async fn matchmaking_status(
    multiplayer_service: &State<MultiplayerService>,
    auth: AuthGuard,
) -> RouteResult<Value> {
    let result = multiplayer_service.matchmaking_status(auth.user_id).await?;
    Ok(success_return(result))
}

/// Matchmaking leave endpoint
///
/// Python baseline: `POST /multiplayer/me/matchmaking/leave/`
#[post("/multiplayer/me/matchmaking/leave")]
pub async fn matchmaking_leave(
    multiplayer_service: &State<MultiplayerService>,
    auth: AuthGuard,
) -> RouteResult<EmptyResponse> {
    multiplayer_service.matchmaking_leave(auth.user_id).await?;
    Ok(success_return(EmptyResponse::default()))
}

/// Get all multiplayer routes
pub fn routes() -> Vec<Route> {
    routes![matchmaking_join, matchmaking_status, matchmaking_leave]
}
