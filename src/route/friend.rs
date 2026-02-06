use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::UserService;
use rocket::form::Form;
use rocket::{post, routes, FromForm, Route, State};
use serde::Deserialize;

/// Friend add request payload (Python baseline: `friend_code`)
#[derive(Debug, Deserialize, FromForm)]
pub struct FriendAddRequest {
    pub friend_code: String,
}

/// Friend delete request payload (Python baseline: `friend_id`)
#[derive(Debug, Deserialize, FromForm)]
pub struct FriendDeleteRequest {
    pub friend_id: i32,
}

/// Add friend endpoint
///
/// Python baseline: `POST /friend/me/add` with form field `friend_code`.
/// Returns a payload containing the updated friend list.
#[post("/friend/me/add", data = "<request>")]
pub async fn add_friend(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Form<FriendAddRequest>,
) -> RouteResult<serde_json::Value> {
    let friend_id = user_service.get_user_id_by_code(&request.friend_code).await?;

    user_service.add_friend(auth.user_id, friend_id).await?;

    let friends = user_service.get_user_friends(auth.user_id).await?;

    Ok(success_return(serde_json::json!({
        "user_id": auth.user_id,
        "updatedAt": "2020-09-07T07:32:12.740Z",
        "createdAt": "2020-09-06T10:05:18.471Z",
        "friends": friends
    })))
}

/// Delete friend endpoint
///
/// Python baseline: `POST /friend/me/delete` with form field `friend_id`.
/// Returns a payload containing the updated friend list.
#[post("/friend/me/delete", data = "<request>")]
pub async fn delete_friend(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Form<FriendDeleteRequest>,
) -> RouteResult<serde_json::Value> {
    user_service
        .delete_friend(auth.user_id, request.friend_id)
        .await?;

    let friends = user_service.get_user_friends(auth.user_id).await?;

    Ok(success_return(serde_json::json!({
        "user_id": auth.user_id,
        "updatedAt": "2020-09-07T07:32:12.740Z",
        "createdAt": "2020-09-06T10:05:18.471Z",
        "friends": friends
    })))
}

/// Get all friend routes
pub fn routes() -> Vec<Route> {
    routes![add_friend, delete_friend]
}

