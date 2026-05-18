use crate::context::ClientContext;
use crate::model::UserLoginDto;
use crate::model::{AuthResponse, LoginRequest};
use crate::service::UserService;
use crate::ArcError;

use rocket::form::Form;

use rocket::{post, routes, Route, State};

/// User login endpoint
///
/// Authenticates user credentials and returns an access token.
/// Validates username/password, checks for bans, manages device
/// sessions and generates a new access token.
#[post("/login", data = "<request>")]
pub async fn login<'a>(
    user_service: &State<UserService>,
    request: Form<LoginRequest>,
    ctx: ClientContext<'_>,
) -> Result<AuthResponse<'a>, ArcError> {
    if request.grant_type.as_deref() != Some("client_credentials") {
        return Err(ArcError::input("Invalid grant_type"));
    }

    let authorization = ctx
        .authorization
        .ok_or_else(|| ArcError::no_access("Missing Authorization header", -4))?;
    let encoded_auth = authorization
        .strip_prefix("Basic ")
        .ok_or_else(|| ArcError::no_access("Invalid Authorization header", -4))?;
    let auth_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded_auth)
            .map_err(|_| ArcError::no_access("Invalid Authorization header", -4))?;
    let auth_str = String::from_utf8(auth_bytes)
        .map_err(|_| ArcError::no_access("Invalid Authorization header", -4))?;

    let (name, password) = auth_str
        .split_once(':')
        .ok_or_else(|| ArcError::no_access("Invalid Authorization header", -4))?;

    let login_data = UserLoginDto {
        name: name.to_string(),
        password: password.to_string(),
        device_id: ctx.get_header("DeviceId").cloned(),
    };

    let ip = ctx.get_client_ip();

    let user_auth = user_service.login_user(login_data, ip).await?;

    let response = AuthResponse {
        success: true,
        token_type: "Bearer",
        user_id: user_auth.user_id,
        access_token: user_auth.token,
    };

    Ok(response)
}

/// Get all others routes
pub fn routes() -> Vec<Route> {
    routes![login]
}
