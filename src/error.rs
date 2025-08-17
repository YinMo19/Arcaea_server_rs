use rocket::serde::json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Main error type for the Arcaea server
#[derive(Error, Debug)]
pub enum ArcError {
    /// Base Arcaea error
    #[error("{message}")]
    Base {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Input validation error
    #[error("Input error: {message}")]
    Input {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Data already exists error
    #[error("Data exists: {message}")]
    DataExist {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Data not found error
    #[error("No data: {message}")]
    NoData {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Missing required input error
    #[error("Post error: {message}")]
    Post {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// User banned error
    #[error("User banned: {message}")]
    UserBan {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Item not enough error
    #[error("Item not enough: {message}")]
    ItemNotEnough {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Item unavailable error
    #[error("Item unavailable: {message}")]
    ItemUnavailable {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Redeem code unavailable error
    #[error("Redeem unavailable: {message}")]
    RedeemUnavailable {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Map locked error
    #[error("Map locked: {message}")]
    MapLocked {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Stamina not enough error
    #[error("Stamina not enough: {message}")]
    StaminaNotEnough {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Memory fragment not enough error
    #[error("Ticket not enough: {message}")]
    TicketNotEnough {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Friend system error
    #[error("Friend error: {message}")]
    Friend {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// No access permission error
    #[error("No access: {message}")]
    NoAccess {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Version too low error
    #[error("Low version: {message}")]
    LowVersion {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Timeout error
    #[error("Timeout: {message}")]
    Timeout {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Rate limit exceeded error
    #[error("Rate limit: {message}")]
    RateLimit {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
        status: u16,
    },

    /// Database error
    #[error("Database error: {message}")]
    Database { message: String },

    /// JSON serialization error
    #[error("JSON error: {message}")]
    Json { message: String },

    /// Rocket error
    #[error("Rocket error: {message}")]
    Rocket { message: String },

    /// IO error
    #[error("IO error: {message}")]
    Io { message: String },
}

impl ArcError {
    /// Create a new input validation error
    pub fn input<S: Into<String>>(message: S) -> Self {
        Self::Input {
            message: message.into(),
            error_code: 108,
            api_error_code: -100,
            extra_data: None,
            status: 200,
        }
    }

    /// Create a new data exists error
    pub fn data_exist<S: Into<String>>(message: S, error_code: i32, api_error_code: i32) -> Self {
        Self::DataExist {
            message: message.into(),
            error_code,
            api_error_code,
            extra_data: None,
            status: 200,
        }
    }

    /// Create a new no data error
    pub fn no_data<S: Into<String>>(message: S, error_code: i32, api_error_code: i32) -> Self {
        Self::NoData {
            message: message.into(),
            error_code,
            api_error_code,
            extra_data: None,
            status: 200,
        }
    }

    /// Create a new user ban error with extra data
    pub fn user_ban<S: Into<String>>(
        message: S,
        error_code: i32,
        extra_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self::UserBan {
            message: message.into(),
            error_code,
            api_error_code: -202,
            extra_data,
            status: 200,
        }
    }

    /// Create a new no access error
    pub fn no_access<S: Into<String>>(message: S, error_code: i32) -> Self {
        Self::NoAccess {
            message: message.into(),
            error_code,
            api_error_code: -999,
            extra_data: None,
            status: 403,
        }
    }

    /// Create a new rate limit error
    pub fn rate_limit<S: Into<String>>(message: S, error_code: i32, api_error_code: i32) -> Self {
        Self::RateLimit {
            message: message.into(),
            error_code,
            api_error_code,
            extra_data: None,
            status: 429,
        }
    }

    /// Create a rocket error
    pub fn rocket_err<S: Into<String>>(message: S) -> Self {
        Self::Rocket {
            message: message.into(),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status(&self) -> u16 {
        match self {
            Self::Base { status, .. }
            | Self::Input { status, .. }
            | Self::DataExist { status, .. }
            | Self::NoData { status, .. }
            | Self::Post { status, .. }
            | Self::UserBan { status, .. }
            | Self::ItemNotEnough { status, .. }
            | Self::ItemUnavailable { status, .. }
            | Self::RedeemUnavailable { status, .. }
            | Self::MapLocked { status, .. }
            | Self::StaminaNotEnough { status, .. }
            | Self::TicketNotEnough { status, .. }
            | Self::Friend { status, .. }
            | Self::NoAccess { status, .. }
            | Self::LowVersion { status, .. }
            | Self::Timeout { status, .. }
            | Self::RateLimit { status, .. } => *status,
            Self::Database { .. } | Self::Json { .. } | Self::Rocket { .. } | Self::Io { .. } => {
                500
            }
        }
    }

    /// Get the error code for this error
    pub fn error_code(&self) -> i32 {
        match self {
            Self::Base { error_code, .. }
            | Self::Input { error_code, .. }
            | Self::DataExist { error_code, .. }
            | Self::NoData { error_code, .. }
            | Self::Post { error_code, .. }
            | Self::UserBan { error_code, .. }
            | Self::ItemNotEnough { error_code, .. }
            | Self::ItemUnavailable { error_code, .. }
            | Self::RedeemUnavailable { error_code, .. }
            | Self::MapLocked { error_code, .. }
            | Self::StaminaNotEnough { error_code, .. }
            | Self::TicketNotEnough { error_code, .. }
            | Self::Friend { error_code, .. }
            | Self::NoAccess { error_code, .. }
            | Self::LowVersion { error_code, .. }
            | Self::Timeout { error_code, .. }
            | Self::RateLimit { error_code, .. } => *error_code,
            Self::Database { .. } | Self::Json { .. } | Self::Rocket { .. } | Self::Io { .. } => {
                108
            }
        }
    }

    /// Get the API error code for this error
    pub fn api_error_code(&self) -> i32 {
        match self {
            Self::Base { api_error_code, .. }
            | Self::Input { api_error_code, .. }
            | Self::DataExist { api_error_code, .. }
            | Self::NoData { api_error_code, .. }
            | Self::Post { api_error_code, .. }
            | Self::UserBan { api_error_code, .. }
            | Self::ItemNotEnough { api_error_code, .. }
            | Self::ItemUnavailable { api_error_code, .. }
            | Self::RedeemUnavailable { api_error_code, .. }
            | Self::MapLocked { api_error_code, .. }
            | Self::StaminaNotEnough { api_error_code, .. }
            | Self::TicketNotEnough { api_error_code, .. }
            | Self::Friend { api_error_code, .. }
            | Self::NoAccess { api_error_code, .. }
            | Self::LowVersion { api_error_code, .. }
            | Self::Timeout { api_error_code, .. }
            | Self::RateLimit { api_error_code, .. } => *api_error_code,
            Self::Database { .. } | Self::Json { .. } | Self::Rocket { .. } | Self::Io { .. } => {
                -999
            }
        }
    }

    /// Get the extra data for this error
    pub fn extra_data(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            Self::Base { extra_data, .. }
            | Self::Input { extra_data, .. }
            | Self::DataExist { extra_data, .. }
            | Self::NoData { extra_data, .. }
            | Self::Post { extra_data, .. }
            | Self::UserBan { extra_data, .. }
            | Self::ItemNotEnough { extra_data, .. }
            | Self::ItemUnavailable { extra_data, .. }
            | Self::RedeemUnavailable { extra_data, .. }
            | Self::MapLocked { extra_data, .. }
            | Self::StaminaNotEnough { extra_data, .. }
            | Self::TicketNotEnough { extra_data, .. }
            | Self::Friend { extra_data, .. }
            | Self::NoAccess { extra_data, .. }
            | Self::LowVersion { extra_data, .. }
            | Self::Timeout { extra_data, .. }
            | Self::RateLimit { extra_data, .. } => extra_data.as_ref(),
            Self::Database { .. } | Self::Json { .. } | Self::Rocket { .. } | Self::Io { .. } => {
                None
            }
        }
    }
}

impl From<sqlx::Error> for ArcError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ArcError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ArcError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
        }
    }
}

/// Alias for Result with ArcError
pub type ArcResult<T> = Result<T, ArcError>;

/// 404 Not Found handler
#[rocket::catch(404)]
pub fn not_found() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 404,
        "message": "Endpoint not found"
    })
}

/// 500 Internal Server Error handler
#[rocket::catch(500)]
pub fn internal_error() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 500,
        "message": "Internal server error"
    })
}

/// 400 Bad Request handler
#[rocket::catch(400)]
pub fn bad_request() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 400,
        "message": "Bad request"
    })
}

/// 401 Unauthorized handler
#[rocket::catch(401)]
pub fn unauthorized() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 401,
        "message": "Unauthorized"
    })
}

/// 403 Forbidden handler
#[rocket::catch(403)]
pub fn forbidden() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 403,
        "message": "Forbidden"
    })
}
