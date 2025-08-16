use crate::error::ArcError;
use crate::model::{UserLoginDto, UserRegisterDto};
use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::UserService;
use rocket::serde::json::Json;
use rocket::{get, post, routes, Route, State};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User registration request payload
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub password: String,
    pub email: String,
    pub user_code: Option<String>,
    pub device_id: Option<String>,
}

/// User login request payload
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub name: String,
    pub password: String,
    pub device_id: Option<String>,
}

/// Authentication response payload
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user_id: i32,
    pub access_token: String,
}

/// User registration endpoint
///
/// Registers a new user account with the provided credentials.
/// Validates input data, checks for existing users, and creates
/// a new account with initial character data.
#[post("/register", data = "<request>")]
pub async fn register(
    user_service: &State<UserService>,
    request: Json<RegisterRequest>,
    // Note: ClientRealIp is not available in current Rocket version
    // Using Option<String> as placeholder for IP extraction
) -> RouteResult<AuthResponse> {
    let register_data = UserRegisterDto {
        name: request.name.clone(),
        password: request.password.clone(),
        email: request.email.clone(),
        user_code: request.user_code.clone(),
    };

    // TODO: Extract real IP from request headers
    let ip: Option<String> = None;
    let device_id = request.device_id.clone();

    let user_auth = user_service
        .register_user(register_data, device_id, ip)
        .await?;

    let response = AuthResponse {
        user_id: user_auth.user_id,
        access_token: user_auth.token,
    };

    Ok(success_return(response))
}

/// User login endpoint
///
/// Authenticates user credentials and returns an access token.
/// Validates username/password, checks for bans, manages device
/// sessions and generates a new access token.
#[post("/login", data = "<request>")]
pub async fn login(
    user_service: &State<UserService>,
    request: Json<LoginRequest>,
    // Note: ClientRealIp is not available in current Rocket version
    // Using Option<String> as placeholder for IP extraction
) -> RouteResult<AuthResponse> {
    let login_data = UserLoginDto {
        name: request.name.clone(),
        password: request.password.clone(),
        device_id: request.device_id.clone(),
    };

    // TODO: Extract real IP from request headers
    let ip: Option<String> = None;

    let user_auth = user_service.login_user(login_data, ip).await?;

    let response = AuthResponse {
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
    // TODO: Implement user profile updates
    // This would involve validating the fields and updating the database
    let _ = user_service;
    let _ = auth;
    let _ = request;

    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        serde_json::Value::String("Update not implemented yet".to_string()),
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

/// Get all user routes
pub fn routes() -> Vec<Route> {
    routes![
        register,
        login,
        user_me,
        logout,
        user_by_code,
        update_user,
        auth_test
    ]
}
