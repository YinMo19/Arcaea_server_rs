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
    assert_eq!(request.grant_type, Some("client_credentials".to_string()));

    let auth_str = String::from_utf8(
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            // Authorization: Basic xxxxx
            ctx.clone().authorization.unwrap()[6..].to_string(),
        )
        .unwrap(),
    )
    .unwrap();

    let mut id_pwd = auth_str.split(":");

    let (name, password) = (id_pwd.next().unwrap(), id_pwd.next().unwrap());

    let login_data = UserLoginDto {
        name: name.to_string(),
        password: password.to_string(),
        device_id: ctx.headers.get("Deviceid").map(|s| s.to_owned()),
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
