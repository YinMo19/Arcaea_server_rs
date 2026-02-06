use crate::route::common::{
    success_return, success_return_no_value, AuthGuard, EmptyResponse, RouteResult,
};
use crate::service::PresentService;
use rocket::{get, post, routes, Route, State};
use serde::{Deserialize, Serialize};

/// Present information response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentInfoResponse {
    pub presents: Vec<serde_json::Value>,
}

/// Claim present request structure
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimPresentRequest {
    pub present_id: String,
}

/// Get user present information endpoint
///
/// Returns a list of available presents for the authenticated user.
/// Expired presents are automatically filtered out.
#[get("/present/me")]
pub async fn present_info(
    present_service: &State<PresentService>,
    auth: AuthGuard,
) -> RouteResult<Vec<serde_json::Value>> {
    let presents = present_service.get_user_presents(auth.user_id).await?;
    let present_list = presents.iter().map(|p| p.to_dict(true)).collect::<Vec<_>>();

    Ok(success_return(present_list))
}

/// Claim present endpoint
///
/// Claims a specific present for the authenticated user.
/// This will remove the present from the user's present list and
/// grant all items contained in the present to the user.
#[post("/present/me/claim/<present_id>")]
pub async fn claim_present(
    present_service: &State<PresentService>,
    auth: AuthGuard,
    present_id: String,
) -> RouteResult<EmptyResponse> {
    present_service
        .claim_present(auth.user_id, &present_id)
        .await?;

    Ok(success_return_no_value())
}

/// Get all present routes
pub fn routes() -> Vec<Route> {
    routes![present_info, claim_present]
}
