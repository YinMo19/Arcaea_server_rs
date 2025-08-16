use crate::error::ArcError;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{Request, Response};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::io::Cursor;

/// Standard API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

/// API error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub success: bool,
    pub error_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Create a successful response
    pub fn success(value: T) -> Self {
        Self {
            success: true,
            value: Some(value),
            error_code: None,
            extra: None,
        }
    }

    /// Create an error response
    pub fn error(
        error_code: i32,
        extra: Option<HashMap<String, serde_json::Value>>,
    ) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            value: None,
            error_code: Some(error_code),
            extra,
        }
    }
}

/// Implement Responder for ApiResponse
impl<'r, T> Responder<'r, 'static> for ApiResponse<T>
where
    T: Serialize,
{
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = serde_json::to_string(&self).map_err(|_| Status::InternalServerError)?;

        Response::build()
            .status(Status::Ok)
            .header(ContentType::JSON)
            .sized_body(json.len(), Cursor::new(json))
            .ok()
    }
}

/// Implement Responder for ArcError
impl<'r> Responder<'r, 'static> for ArcError {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = match self.status() {
            200 => Status::Ok,
            400 => Status::BadRequest,
            401 => Status::Unauthorized,
            403 => Status::Forbidden,
            404 => Status::NotFound,
            429 => Status::TooManyRequests,
            500 => Status::InternalServerError,
            _ => Status::InternalServerError,
        };

        let error_response = ApiErrorResponse {
            success: false,
            error_code: self.error_code(),
            extra: self.extra_data().cloned(),
        };

        let json =
            serde_json::to_string(&error_response).map_err(|_| Status::InternalServerError)?;

        Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(json.len(), Cursor::new(json))
            .ok()
    }
}

/// Helper function to create success response
pub fn success_return<T>(value: T) -> ApiResponse<T>
where
    T: Serialize,
{
    ApiResponse::success(value)
}

/// Helper function to create error response
pub fn error_return() -> ApiResponse<()> {
    ApiResponse::<()>::error(108, None)
}

/// Helper function to create error response with custom error code
pub fn error_return_with_code(error_code: i32) -> ApiResponse<()> {
    ApiResponse::<()>::error(error_code, None)
}

/// Helper function to create error response with extra data
pub fn error_return_with_extra(
    error_code: i32,
    extra: HashMap<String, serde_json::Value>,
) -> ApiResponse<()> {
    ApiResponse::<()>::error(error_code, Some(extra))
}

use rocket::outcome::Outcome;
/// Request guard for authentication
use rocket::request::{self, FromRequest};

/// Authentication guard that extracts user ID from Authorization header
pub struct AuthGuard {
    pub user_id: i32,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ArcError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let authorization = request.headers().get_one("Authorization");

        match authorization {
            Some(token) => {
                // Extract Bearer token
                let token = if token.starts_with("Bearer ") {
                    &token[7..]
                } else {
                    token
                };

                // Get UserService from Rocket state
                let user_service = match request.rocket().state::<crate::service::UserService>() {
                    Some(service) => service,
                    None => {
                        return Outcome::Error((
                            Status::InternalServerError,
                            ArcError::no_access("UserService not available", -4),
                        ));
                    }
                };

                // Validate the token
                match user_service.authenticate_token(token).await {
                    Ok(user_id) => Outcome::Success(AuthGuard { user_id }),
                    Err(e) => Outcome::Error((Status::Unauthorized, e)),
                }
            }
            None => Outcome::Error((
                Status::Unauthorized,
                ArcError::no_access("Missing Authorization header", -4),
            )),
        }
    }
}

/// CORS fairing for handling cross-origin requests
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(
        &self,
        _request: &'r rocket::Request<'_>,
        response: &mut rocket::Response<'r>,
    ) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

/// Result type alias for route handlers
pub type RouteResult<T> = Result<ApiResponse<T>, ArcError>;

/// Macro for handling async route operations with automatic error conversion
#[macro_export]
macro_rules! arc_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(e.into()),
        }
    };
}

/// Game info response structure (from Python version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfoResponse {
    pub version: String,
    pub database_version: String,
    pub log_database_version: String,
}

/// Empty response for endpoints that don't return data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponse {}

impl Default for EmptyResponse {
    fn default() -> Self {
        Self {}
    }
}
