use rocket::http::Status;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rocket::http::ContentType;
use rocket::response::Responder;
use rocket::{Request, Response};
use serde_json;
use std::io::Cursor;

/// Game information response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub version: String,
    pub database_version: String,
    pub log_database_version: String,
}

/// Notification response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub title: String,
    pub message: String,
    pub timestamp: i64,
}

/// Bundle download response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleResponse {
    #[serde(rename = "orderedResults")]
    pub ordered_results: Vec<BundleItem>,
}

/// Bundle item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleItem {
    pub name: String,
    pub version: String,
    pub url: String,
    pub size: u64,
}

/// Insight completion response
#[derive(Debug, Serialize, Deserialize)]
pub struct InsightCompleteResponse {
    pub insight_state: i32,
}

/// Aggregate request structure
#[derive(Debug, Deserialize)]
pub struct AggregateCall {
    pub endpoint: String,
    pub id: Option<serde_json::Value>,
}

/// Aggregate response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Vec<AggregateValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

/// Aggregate value structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateValue {
    pub id: Option<serde_json::Value>,
    pub value: serde_json::Value,
}

/// Implement Responder for AggregateResponse
impl<'r> Responder<'r, 'static> for AggregateResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = serde_json::to_string(&self).map_err(|_| Status::InternalServerError)?;

        Response::build()
            .status(Status::Ok)
            .header(ContentType::JSON)
            .sized_body(json.len(), Cursor::new(json))
            .ok()
    }
}
