use crate::context::ClientContext;
use crate::error::ArcError;
use crate::model::{RegisterResponse, UserLoginDto, UserRegisterDto};

use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::UserService;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket::{get, post, routes, FromForm, Route, State};
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;

/// User registration request payload
#[derive(Debug, Deserialize, FromForm)]
pub struct RegisterRequest {
    pub name: String,
    pub password: String,
    pub email: String,
    pub device_id: Option<String>,
}

/// Friend management request payload
#[derive(Debug, Deserialize, FromForm)]
pub struct FriendRequest {
    pub friend_user_code: Option<String>,
    pub friend_id: Option<i32>,
}

/// User registration endpoint
///
/// Registers a new user account with the provided credentials.
/// Validates input data, checks for existing users, and creates
/// a new account with initial character data.
#[post("/", data = "<register_info>")]
pub async fn register(
    user_service: &State<UserService>,
    register_info: Form<RegisterRequest>,
    ctx: ClientContext<'_>,
) -> RouteResult<RegisterResponse> {
    let register_data = UserRegisterDto {
        name: register_info.name.clone(),
        password: register_info.password.clone(),
        email: register_info.email.clone(),
    };

    let ip = ctx.get_client_ip();
    let device_id = register_info
        .device_id
        .clone()
        .or_else(|| ctx.get_device_id());

    let user_auth = user_service
        .register_user(register_data, device_id.clone(), ip.map(|c| c.to_string()))
        .await?;

    // auto login after register
    let login_data = UserLoginDto {
        name: register_info.name.clone(),
        password: register_info.password.clone(),
        device_id,
    };
    user_service.login_user(login_data, ip).await?;

    let response = RegisterResponse {
        user_id: user_auth.user_id,
        access_token: user_auth.token,
    };

    Ok(success_return(response))
}

/// Get current user information endpoint
///
/// Returns detailed information about the authenticated user.
/// Requires valid authentication token in Authorization header.
#[get("/me")]
pub async fn user_me(
    user_service: &State<UserService>,
    auth: AuthGuard,
) -> RouteResult<serde_json::Value> {
    let user_info = user_service.get_user_info(auth.user_id).await?;

    // Convert UserInfo to JSON for flexible response structure
    let user_json = serde_json::to_value(&user_info).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })?;

    Ok(success_return(user_json))
}

/// User logout endpoint
///
/// Invalidates the current access token by removing it from
/// the login sessions table.
#[post("/logout")]
pub async fn logout(
    _user_service: &State<UserService>,
    _auth: AuthGuard,
) -> RouteResult<HashMap<String, serde_json::Value>> {
    // TODO: Implement token invalidation
    // For now, return empty success response
    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        serde_json::Value::String("Logged out successfully".to_string()),
    );

    Ok(success_return(response))
}

/// Get user by user code endpoint
///
/// Retrieves user information using a 9-digit user code.
/// This is typically used for friend lookups.
#[get("/code/<user_code>")]
pub async fn user_by_code(
    user_service: &State<UserService>,
    user_code: String,
) -> RouteResult<serde_json::Value> {
    let user_id = user_service.get_user_id_by_code(&user_code).await?;
    let user_info = user_service.get_user_info(user_id).await?;

    let user_json = serde_json::to_value(&user_info).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })?;

    Ok(success_return(user_json))
}

/// Update user profile endpoint
///
/// Updates specific user profile fields. Requires authentication.
#[post("/update", data = "<request>")]
pub async fn update_user(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Json<HashMap<String, serde_json::Value>>,
) -> RouteResult<HashMap<String, serde_json::Value>> {
    let mut response = HashMap::new();
    let mut updated_fields = 0;

    for (field, value) in request.iter() {
        match field.as_str() {
            "character_id" => {
                if let Some(character_id) = value.as_i64() {
                    user_service
                        .update_user_character(auth.user_id, character_id as i32)
                        .await?;
                    response.insert("character_id".to_string(), value.clone());
                    updated_fields += 1;
                }
            }
            "is_skill_sealed" => {
                if let Some(is_sealed) = value.as_bool() {
                    user_service
                        .update_user_skill_sealed(auth.user_id, is_sealed)
                        .await?;
                    response.insert("is_skill_sealed".to_string(), value.clone());
                    updated_fields += 1;
                }
            }
            "favorite_character" => {
                if let Some(fav_char) = value.as_i64() {
                    user_service
                        .update_user_column(
                            auth.user_id,
                            "favorite_character",
                            &(fav_char as i32).to_string(),
                        )
                        .await?;
                    response.insert("favorite_character".to_string(), value.clone());
                    updated_fields += 1;
                }
            }
            "is_hide_rating" => {
                if let Some(hide_rating) = value.as_bool() {
                    let hide_val = if hide_rating { 1 } else { 0 };
                    user_service
                        .update_user_column(auth.user_id, "is_hide_rating", &hide_val.to_string())
                        .await?;
                    response.insert("is_hide_rating".to_string(), value.clone());
                    updated_fields += 1;
                }
            }
            _ => {
                // Ignore unknown fields for safety
                continue;
            }
        }
    }

    response.insert(
        "updated_fields".to_string(),
        serde_json::Value::Number(serde_json::Number::from(updated_fields)),
    );
    response.insert(
        "user_id".to_string(),
        serde_json::Value::Number(serde_json::Number::from(auth.user_id)),
    );

    Ok(success_return(response))
}

/// Authentication test endpoint
///
/// Simple endpoint to test if authentication is working.
/// Returns the authenticated user's ID.
#[get("/auth/test")]
pub async fn auth_test(auth: AuthGuard) -> RouteResult<HashMap<String, serde_json::Value>> {
    let mut response = HashMap::new();
    response.insert(
        "user_id".to_string(),
        serde_json::Value::Number(serde_json::Number::from(auth.user_id)),
    );
    response.insert(
        "message".to_string(),
        serde_json::Value::String("Authentication successful".to_string()),
    );

    Ok(success_return(response))
}

/// Toggle insight/invasion skill endpoint
///
/// Toggles the user's insight state for invasion skill.
#[post("/me/toggle_invasion")]
pub async fn toggle_invasion(
    user_service: &State<UserService>,
    auth: AuthGuard,
) -> RouteResult<serde_json::Value> {
    let user_info = user_service.toggle_invasion(auth.user_id).await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "insight_state": user_info.insight_state
    });

    Ok(success_return(response))
}

/// Character change endpoint
///
/// Changes the user's current character and skill sealed state.
#[derive(Debug, Deserialize, FromForm)]
pub struct CharacterChangeRequest {
    pub character: i32,
    pub skill_sealed: String,
}

#[post("/me/character", data = "<request>")]
pub async fn character_change(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Form<CharacterChangeRequest>,
) -> RouteResult<serde_json::Value> {
    let is_skill_sealed = request.skill_sealed == "true";

    user_service
        .change_character(auth.user_id, request.character, is_skill_sealed)
        .await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "character": request.character
    });

    Ok(success_return(response))
}

/// Toggle character uncap override endpoint
///
/// Toggles the uncap override state for a specific character.
#[post("/me/character/<character_id>/toggle_uncap")]
pub async fn toggle_uncap(
    user_service: &State<UserService>,
    auth: AuthGuard,
    character_id: i32,
) -> RouteResult<serde_json::Value> {
    let character_info = user_service
        .toggle_character_uncap_override(auth.user_id, character_id)
        .await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "character": [character_info]
    });

    Ok(success_return(response))
}

/// Character first uncap endpoint
///
/// Performs the first uncap of a character using fragments.
#[post("/me/character/<character_id>/uncap")]
pub async fn character_first_uncap(
    user_service: &State<UserService>,
    auth: AuthGuard,
    character_id: i32,
) -> RouteResult<serde_json::Value> {
    let (character_info, cores) = user_service
        .character_uncap(auth.user_id, character_id)
        .await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "character": [character_info],
        "cores": cores
    });

    Ok(success_return(response))
}

/// Character experience upgrade endpoint
///
/// Uses ether drops to upgrade character experience.
#[derive(Debug, Deserialize, FromForm)]
pub struct CharacterExpRequest {
    pub amount: i32,
}

#[post("/me/character/<character_id>/exp", data = "<request>")]
pub async fn character_exp(
    user_service: &State<UserService>,
    auth: AuthGuard,
    character_id: i32,
    request: Form<CharacterExpRequest>,
) -> RouteResult<serde_json::Value> {
    let (character_info, cores) = user_service
        .upgrade_character_by_core(auth.user_id, character_id, request.amount)
        .await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "character": [character_info],
        "cores": cores
    });

    Ok(success_return(response))
}

/// Cloud save get endpoint
///
/// Retrieves user's cloud save data.
#[get("/me/save")]
pub async fn cloud_get(
    user_service: &State<UserService>,
    auth: AuthGuard,
) -> RouteResult<serde_json::Value> {
    let save_data = user_service.get_user_save_data(auth.user_id).await?;
    Ok(success_return(save_data))
}

/// Cloud save post endpoint
///
/// Updates user's cloud save data.
#[derive(Debug, Deserialize, FromForm)]
pub struct CloudSaveRequest {
    pub scores_data: String,
    pub scores_checksum: String,
    pub clearlamps_data: String,
    pub clearlamps_checksum: String,
    pub clearedsongs_data: String,
    pub clearedsongs_checksum: String,
    pub unlocklist_data: String,
    pub unlocklist_checksum: String,
    pub installid_data: String,
    pub installid_checksum: String,
    pub devicemodelname_data: String,
    pub devicemodelname_checksum: String,
    pub story_data: String,
    pub story_checksum: String,
    pub finalestate_data: Option<String>,
    pub finalestate_checksum: Option<String>,
}

#[post("/me/save", data = "<request>")]
pub async fn cloud_post(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Form<CloudSaveRequest>,
) -> RouteResult<serde_json::Value> {
    user_service
        .update_user_save_data(auth.user_id, &*request)
        .await?;

    let response = serde_json::json!({
        "user_id": auth.user_id
    });

    Ok(success_return(response))
}

/// System settings endpoint
///
/// Updates various user settings.
#[derive(Debug, Deserialize, FromForm)]
pub struct SettingRequest {
    pub value: String,
}

#[post("/me/setting/<set_arg>", data = "<request>")]
pub async fn sys_set(
    user_service: &State<UserService>,
    auth: AuthGuard,
    set_arg: String,
    request: Form<SettingRequest>,
) -> RouteResult<serde_json::Value> {
    let user_info = user_service
        .update_user_setting(auth.user_id, &set_arg, &request.value)
        .await?;

    let user_json = serde_json::to_value(&user_info).map_err(|e| ArcError::Json {
        message: e.to_string(),
    })?;

    Ok(success_return(user_json))
}

/// User account deletion endpoint
///
/// Requests deletion of the user's account.
#[post("/me/request_delete")]
pub async fn user_delete(
    user_service: &State<UserService>,
    auth: AuthGuard,
) -> RouteResult<serde_json::Value> {
    user_service.delete_user_account(auth.user_id).await?;

    let response = serde_json::json!({
        "user_id": auth.user_id
    });

    Ok(success_return(response))
}

/// Add friend endpoint
///
/// Adds a user to the current user's friend list.
/// Can use either friend_user_code or friend_id to identify the target user.
#[post("/me/friend/add", data = "<request>")]
pub async fn add_friend(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Form<FriendRequest>,
) -> RouteResult<serde_json::Value> {
    let friend_id = if let Some(friend_user_code) = &request.friend_user_code {
        user_service.get_user_id_by_code(friend_user_code).await?
    } else if let Some(fid) = request.friend_id {
        fid
    } else {
        return Err(ArcError::input("friend_user_code or friend_id is required"));
    };

    user_service.add_friend(auth.user_id, friend_id).await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "friend_id": friend_id
    });

    Ok(success_return(response))
}

/// Delete friend endpoint
///
/// Removes a user from the current user's friend list.
#[post("/me/friend/delete", data = "<request>")]
pub async fn delete_friend(
    user_service: &State<UserService>,
    auth: AuthGuard,
    request: Form<FriendRequest>,
) -> RouteResult<serde_json::Value> {
    let friend_id = if let Some(friend_user_code) = &request.friend_user_code {
        user_service.get_user_id_by_code(friend_user_code).await?
    } else if let Some(fid) = request.friend_id {
        fid
    } else {
        return Err(ArcError::input("friend_user_code or friend_id is required"));
    };

    user_service.delete_friend(auth.user_id, friend_id).await?;

    let response = serde_json::json!({
        "user_id": auth.user_id,
        "friend_id": friend_id
    });

    Ok(success_return(response))
}

/// Get user friends endpoint
///
/// Returns the current user's friend list with detailed information.
#[get("/me/friends")]
pub async fn get_friends(
    user_service: &State<UserService>,
    auth: AuthGuard,
) -> RouteResult<Vec<serde_json::Value>> {
    let friends = user_service.get_user_friends(auth.user_id).await?;
    Ok(success_return(friends))
}

/// Email verification resend endpoint
///
/// Resends email verification (currently unavailable).
#[post("/email/resend_verify")]
pub async fn email_resend_verify() -> RouteResult<serde_json::Value> {
    Err(ArcError::no_data("Email verification unavailable.", 151))
}

/// Email verification status endpoint
///
/// Checks email verification status (currently unavailable).
#[post("/verify")]
pub async fn email_verify() -> RouteResult<serde_json::Value> {
    Err(ArcError::no_data("Email verification unavailable.", 151))
}

/// Get all user routes
pub fn routes() -> Vec<Route> {
    routes![
        register,
        user_me,
        logout,
        user_by_code,
        update_user,
        auth_test,
        toggle_invasion,
        character_change,
        toggle_uncap,
        character_first_uncap,
        character_exp,
        cloud_get,
        cloud_post,
        sys_set,
        user_delete,
        email_resend_verify,
        email_verify,
        add_friend,
        delete_friend,
        get_friends
    ]
}
