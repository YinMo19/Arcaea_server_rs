use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::WorldService;
use rocket::serde::json::Json;
use rocket::{get, post, routes, Route, State};
use serde::{Deserialize, Serialize};

/// World all maps response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldAllResponse {
    pub current_map: String,
    pub user_id: i32,
    pub maps: Vec<serde_json::Value>,
}

/// World single map response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMapResponse {
    pub user_id: i32,
    pub current_map: String,
    pub maps: Vec<serde_json::Value>,
}

/// Map enter request structure
#[derive(Debug, Clone, Deserialize)]
pub struct MapEnterRequest {
    pub map_id: String,
}

/// Map enter response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEnterResponse {
    pub map_id: String,
    pub curr_position: i32,
    pub curr_capture: i32,
    pub is_locked: bool,
    pub user_id: i32,
}

/// Get all world maps endpoint
///
/// Returns comprehensive world map information including user progress,
/// map details, and reward information for all available maps.
#[get("/world/map/me")]
pub async fn world_all(
    world_service: &State<WorldService>,
    auth: AuthGuard,
) -> RouteResult<serde_json::Value> {
    let world_data = world_service.get_user_world_all(auth.user_id).await?;
    Ok(success_return(world_data))
}

/// Enter/unlock a map endpoint
///
/// Attempts to unlock the specified map for the user.
/// Returns map information if successful.
#[post("/world/map/me", data = "<request>")]
pub async fn world_in(
    world_service: &State<WorldService>,
    auth: AuthGuard,
    request: Json<MapEnterRequest>,
) -> RouteResult<serde_json::Value> {
    let map_data = world_service
        .enter_map(auth.user_id, &request.map_id)
        .await?;
    Ok(success_return(map_data))
}

/// Get single map information endpoint
///
/// Returns detailed information about a specific map including
/// user progress, steps, and rewards.
#[get("/world/map/me/<map_id>")]
pub async fn world_one(
    world_service: &State<WorldService>,
    auth: AuthGuard,
    map_id: String,
) -> RouteResult<serde_json::Value> {
    let map_data = world_service.get_user_map(auth.user_id, &map_id).await?;
    Ok(success_return(map_data))
}

/// Get all world routes
pub fn routes() -> Vec<Route> {
    routes![world_all, world_in, world_one]
}
