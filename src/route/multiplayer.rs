use crate::route::common::{success_return, AuthGuard, EmptyResponse, RouteResult};
use crate::service::{
    MatchmakingJoinRequest, MultiplayerService, MultiplayerUpdateRequest, NotificationService,
};
use rocket::form::Form;
use rocket::serde::json::{Json, Value};
use rocket::{post, routes, FromForm, Route, State};

#[derive(Debug, FromForm)]
pub struct RoomInviteRequest {
    pub to: i32,
}

#[derive(Debug, FromForm)]
pub struct RoomStatusRequest {
    pub shareToken: String,
}

/// Room create endpoint
///
/// Python baseline: `POST /multiplayer/me/room/create`
#[post("/multiplayer/me/room/create", data = "<request>")]
pub async fn room_create(
    multiplayer_service: &State<MultiplayerService>,
    auth: AuthGuard,
    request: Json<MatchmakingJoinRequest>,
) -> RouteResult<Value> {
    let result = multiplayer_service
        .room_create(auth.user_id, &request.client_song_map)
        .await?;
    Ok(success_return(result))
}

/// Room join endpoint
///
/// Python baseline: `POST /multiplayer/me/room/join/<room_code>`
#[post("/multiplayer/me/room/join/<room_code>", data = "<request>")]
pub async fn room_join(
    multiplayer_service: &State<MultiplayerService>,
    auth: AuthGuard,
    room_code: String,
    request: Json<MatchmakingJoinRequest>,
) -> RouteResult<Value> {
    let result = multiplayer_service
        .room_join(auth.user_id, &room_code, &request.client_song_map)
        .await?;
    Ok(success_return(result))
}

/// Multiplayer room update endpoint
///
/// Python baseline: `POST /multiplayer/me/update`
#[post("/multiplayer/me/update", data = "<request>")]
pub async fn multiplayer_update(
    multiplayer_service: &State<MultiplayerService>,
    auth: AuthGuard,
    request: Json<MultiplayerUpdateRequest>,
) -> RouteResult<Value> {
    let token = request.token_u64()?;
    let result = multiplayer_service.room_update(auth.user_id, token).await?;
    Ok(success_return(result))
}

/// Room invite endpoint
///
/// Python baseline: `POST /multiplayer/me/room/<room_code>/invite`
#[post("/multiplayer/me/room/<room_code>/invite", data = "<request>")]
pub async fn room_invite(
    multiplayer_service: &State<MultiplayerService>,
    notification_service: &State<NotificationService>,
    auth: AuthGuard,
    room_code: String,
    request: Form<RoomInviteRequest>,
) -> RouteResult<EmptyResponse> {
    let share_token = multiplayer_service
        .room_invite_share_token(&room_code)
        .await?;
    let _ = multiplayer_service.user_linkplay_name(request.to).await?;
    let sender_name = multiplayer_service.user_linkplay_name(auth.user_id).await?;
    notification_service
        .create_room_invite(auth.user_id, sender_name, request.to, share_token)
        .await?;
    Ok(success_return(EmptyResponse::default()))
}

/// Room status endpoint
///
/// Python baseline: `POST /multiplayer/me/room/status`
#[post("/multiplayer/me/room/status", data = "<request>")]
pub async fn room_status(
    multiplayer_service: &State<MultiplayerService>,
    _auth: AuthGuard,
    request: Form<RoomStatusRequest>,
) -> RouteResult<Value> {
    let result = multiplayer_service.room_status(&request.shareToken).await?;
    Ok(success_return(result))
}

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
    routes![
        room_create,
        room_join,
        multiplayer_update,
        room_invite,
        room_status,
        matchmaking_join,
        matchmaking_status,
        matchmaking_leave
    ]
}
