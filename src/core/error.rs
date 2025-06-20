use rocket::response::{self, Responder, Response};
use rocket::{http::Status, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ArcError {
    #[error("ArcError: {message}")]
    General {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Input Error: {message}")]
    InputError {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Data Exists: {message}")]
    DataExist {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("No Data: {message}")]
    NoData {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Post Error: {message}")]
    PostError {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("User Ban: {message}")]
    UserBan {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Item Not Enough: {message}")]
    ItemNotEnough {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Item Unavailable: {message}")]
    ItemUnavailable {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Redeem Unavailable: {message}")]
    RedeemUnavailable {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Map Locked: {message}")]
    MapLocked {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Stamina Not Enough: {message}")]
    StaminaNotEnough {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Ticket Not Enough: {message}")]
    TicketNotEnough {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Friend Error: {message}")]
    FriendError {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("No Access: {message}")]
    NoAccess {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Low Version: {message}")]
    LowVersion {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Timeout: {message}")]
    Timeout {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },

    #[error("Rate Limit: {message}")]
    RateLimit {
        message: String,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<serde_json::Value>,
        status: u16,
    },
}

impl ArcError {
    pub fn new(message: &str) -> Self {
        Self::General {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn with_error_code(message: &str, error_code: i32) -> Self {
        Self::General {
            message: message.to_string(),
            error_code,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn input_error(message: &str) -> Self {
        Self::InputError {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -100,
            extra_data: None,
            status: 200,
        }
    }

    pub fn data_exist(message: &str) -> Self {
        Self::DataExist {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -4,
            extra_data: None,
            status: 200,
        }
    }

    pub fn no_data(message: &str) -> Self {
        Self::NoData {
            message: message.to_string(),
            error_code: 401,
            api_error_code: -3,
            extra_data: None,
            status: 200,
        }
    }

    pub fn post_error(message: &str) -> Self {
        Self::PostError {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -100,
            extra_data: None,
            status: 200,
        }
    }

    pub fn user_ban(message: &str) -> Self {
        Self::UserBan {
            message: message.to_string(),
            error_code: 121,
            api_error_code: -202,
            extra_data: None,
            status: 200,
        }
    }

    pub fn item_not_enough(message: &str) -> Self {
        Self::ItemNotEnough {
            message: message.to_string(),
            error_code: -6,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn item_unavailable(message: &str) -> Self {
        Self::ItemUnavailable {
            message: message.to_string(),
            error_code: -6,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn redeem_unavailable(message: &str) -> Self {
        Self::RedeemUnavailable {
            message: message.to_string(),
            error_code: 505,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn map_locked(message: &str) -> Self {
        Self::MapLocked {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn stamina_not_enough(message: &str) -> Self {
        Self::StaminaNotEnough {
            message: message.to_string(),
            error_code: 107,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn ticket_not_enough(message: &str) -> Self {
        Self::TicketNotEnough {
            message: message.to_string(),
            error_code: -6,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn friend_error(message: &str) -> Self {
        Self::FriendError {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn no_access(message: &str) -> Self {
        Self::NoAccess {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 403,
        }
    }

    pub fn low_version(message: &str) -> Self {
        Self::LowVersion {
            message: message.to_string(),
            error_code: 5,
            api_error_code: -999,
            extra_data: None,
            status: 403,
        }
    }

    pub fn timeout(message: &str) -> Self {
        Self::Timeout {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 200,
        }
    }

    pub fn rate_limit(message: &str) -> Self {
        Self::RateLimit {
            message: message.to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 429,
        }
    }

    pub fn with_extra_data(mut self, extra_data: serde_json::Value) -> Self {
        match &mut self {
            ArcError::General { extra_data: ed, .. }
            | ArcError::InputError { extra_data: ed, .. }
            | ArcError::DataExist { extra_data: ed, .. }
            | ArcError::NoData { extra_data: ed, .. }
            | ArcError::PostError { extra_data: ed, .. }
            | ArcError::UserBan { extra_data: ed, .. }
            | ArcError::ItemNotEnough { extra_data: ed, .. }
            | ArcError::ItemUnavailable { extra_data: ed, .. }
            | ArcError::RedeemUnavailable { extra_data: ed, .. }
            | ArcError::MapLocked { extra_data: ed, .. }
            | ArcError::StaminaNotEnough { extra_data: ed, .. }
            | ArcError::TicketNotEnough { extra_data: ed, .. }
            | ArcError::FriendError { extra_data: ed, .. }
            | ArcError::NoAccess { extra_data: ed, .. }
            | ArcError::LowVersion { extra_data: ed, .. }
            | ArcError::Timeout { extra_data: ed, .. }
            | ArcError::RateLimit { extra_data: ed, .. } => {
                *ed = Some(extra_data);
            }
        }
        self
    }

    pub fn get_error_code(&self) -> i32 {
        match self {
            ArcError::General { error_code, .. }
            | ArcError::InputError { error_code, .. }
            | ArcError::DataExist { error_code, .. }
            | ArcError::NoData { error_code, .. }
            | ArcError::PostError { error_code, .. }
            | ArcError::UserBan { error_code, .. }
            | ArcError::ItemNotEnough { error_code, .. }
            | ArcError::ItemUnavailable { error_code, .. }
            | ArcError::RedeemUnavailable { error_code, .. }
            | ArcError::MapLocked { error_code, .. }
            | ArcError::StaminaNotEnough { error_code, .. }
            | ArcError::TicketNotEnough { error_code, .. }
            | ArcError::FriendError { error_code, .. }
            | ArcError::NoAccess { error_code, .. }
            | ArcError::LowVersion { error_code, .. }
            | ArcError::Timeout { error_code, .. }
            | ArcError::RateLimit { error_code, .. } => *error_code,
        }
    }

    pub fn get_status(&self) -> u16 {
        match self {
            ArcError::General { status, .. }
            | ArcError::InputError { status, .. }
            | ArcError::DataExist { status, .. }
            | ArcError::NoData { status, .. }
            | ArcError::PostError { status, .. }
            | ArcError::UserBan { status, .. }
            | ArcError::ItemNotEnough { status, .. }
            | ArcError::ItemUnavailable { status, .. }
            | ArcError::RedeemUnavailable { status, .. }
            | ArcError::MapLocked { status, .. }
            | ArcError::StaminaNotEnough { status, .. }
            | ArcError::TicketNotEnough { status, .. }
            | ArcError::FriendError { status, .. }
            | ArcError::NoAccess { status, .. }
            | ArcError::LowVersion { status, .. }
            | ArcError::Timeout { status, .. }
            | ArcError::RateLimit { status, .. } => *status,
        }
    }

    pub fn get_extra_data(&self) -> &Option<serde_json::Value> {
        match self {
            ArcError::General { extra_data, .. }
            | ArcError::InputError { extra_data, .. }
            | ArcError::DataExist { extra_data, .. }
            | ArcError::NoData { extra_data, .. }
            | ArcError::PostError { extra_data, .. }
            | ArcError::UserBan { extra_data, .. }
            | ArcError::ItemNotEnough { extra_data, .. }
            | ArcError::ItemUnavailable { extra_data, .. }
            | ArcError::RedeemUnavailable { extra_data, .. }
            | ArcError::MapLocked { extra_data, .. }
            | ArcError::StaminaNotEnough { extra_data, .. }
            | ArcError::TicketNotEnough { extra_data, .. }
            | ArcError::FriendError { extra_data, .. }
            | ArcError::NoAccess { extra_data, .. }
            | ArcError::LowVersion { extra_data, .. }
            | ArcError::Timeout { extra_data, .. }
            | ArcError::RateLimit { extra_data, .. } => extra_data,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub success: bool,
    pub error_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SuccessResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
}

impl<T: Serialize> SuccessResponse<T> {
    pub fn new(value: T) -> Self {
        Self {
            success: true,
            value: Some(value),
        }
    }
}

impl SuccessResponse<()> {
    pub fn empty() -> Self {
        SuccessResponse {
            success: true,
            value: None,
        }
    }
}

impl<'r> Responder<'r, 'static> for ArcError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let status = Status::from_code(self.get_status()).unwrap_or(Status::InternalServerError);

        let error_response = ErrorResponse {
            success: false,
            error_code: self.get_error_code(),
            extra: self.get_extra_data().clone(),
        };

        let json = serde_json::to_string(&error_response).unwrap();

        Response::build()
            .status(status)
            .header(rocket::http::ContentType::JSON)
            .sized_body(json.len(), std::io::Cursor::new(json))
            .ok()
    }
}

// Convert from various error types
impl From<sqlx::Error> for ArcError {
    fn from(err: sqlx::Error) -> Self {
        ArcError::new(&format!("Database error: {}", err))
    }
}

impl From<serde_json::Error> for ArcError {
    fn from(err: serde_json::Error) -> Self {
        ArcError::new(&format!("JSON error: {}", err))
    }
}

impl From<std::num::ParseIntError> for ArcError {
    fn from(err: std::num::ParseIntError) -> Self {
        ArcError::input_error(&format!("Parse error: {}", err))
    }
}

pub type ArcResult<T> = Result<T, ArcError>;

// Helper function to create success responses
pub fn success_return<T: Serialize>(value: T) -> rocket::serde::json::Json<SuccessResponse<T>> {
    rocket::serde::json::Json(SuccessResponse::new(value))
}

pub fn empty_success() -> rocket::serde::json::Json<SuccessResponse<()>> {
    rocket::serde::json::Json(SuccessResponse::<()>::empty())
}
