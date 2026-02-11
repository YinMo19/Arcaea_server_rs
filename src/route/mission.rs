use crate::error::ArcError;
use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::MissionService;
use crate::service::UserService;
use crate::DbPool;
use rocket::form::Form;
use rocket::serde::json::Value;
use rocket::{post, routes, Route, State};
use serde_json::json;
use std::collections::HashMap;

/// Mission clear endpoint
///
/// Python baseline: `POST /mission/me/clear`
#[post("/mission/me/clear", data = "<request_form>")]
pub async fn mission_clear(
    pool: &State<DbPool>,
    auth: AuthGuard,
    request_form: Form<HashMap<String, String>>,
) -> RouteResult<Value> {
    let mission_service = MissionService::new(pool.inner().clone());
    let mission_ids = MissionService::parse_mission_form(&request_form);

    let mut missions = Vec::with_capacity(mission_ids.len());
    for (idx, mission_id) in mission_ids.into_iter().enumerate() {
        let mut mission = mission_service
            .clear_mission(auth.user_id, &mission_id)
            .await?;
        mission["request_id"] = json!((idx as i32) + 1);
        missions.push(mission);
    }

    Ok(success_return(json!({ "missions": missions })))
}

/// Mission claim endpoint
///
/// Python baseline: `POST /mission/me/claim`
#[post("/mission/me/claim", data = "<request_form>")]
pub async fn mission_claim(
    pool: &State<DbPool>,
    user_service: &State<UserService>,
    auth: AuthGuard,
    request_form: Form<HashMap<String, String>>,
) -> RouteResult<Value> {
    let mission_service = MissionService::new(pool.inner().clone());
    let mission_ids = MissionService::parse_mission_form(&request_form);

    let mut missions = Vec::with_capacity(mission_ids.len());
    for (idx, mission_id) in mission_ids.into_iter().enumerate() {
        let mut mission = mission_service
            .claim_mission(auth.user_id, &mission_id)
            .await?;
        mission["request_id"] = json!((idx as i32) + 1);
        missions.push(mission);
    }

    let user = user_service.get_user_info(auth.user_id).await?;
    let user_json = serde_json::to_value(user).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })?;

    Ok(success_return(json!({
        "missions": missions,
        "user": user_json,
    })))
}

/// Get all mission routes
pub fn routes() -> Vec<Route> {
    routes![mission_clear, mission_claim]
}
