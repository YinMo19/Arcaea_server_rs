use serde::{Deserialize, Serialize};

/// Present/Gift structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Present {
    pub present_id: String,
    pub expire_ts: Option<i64>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<PresentItem>>,
}

impl Present {
    /// Check if the present has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expire_ts) = self.expire_ts {
            let current_ts = chrono::Utc::now().timestamp_millis();
            expire_ts < current_ts
        } else {
            false
        }
    }

    /// Convert to dictionary format for API response
    pub fn to_dict(&self, has_items: bool) -> serde_json::Value {
        let mut result = serde_json::json!({
            "present_id": self.present_id,
            "expire_ts": self.expire_ts,
            "description": self.description.as_ref().unwrap_or(&String::new())
        });

        if has_items {
            if let Some(ref items) = self.items {
                result["items"] =
                    serde_json::json!(items.iter().map(|i| i.to_dict()).collect::<Vec<_>>());
            } else {
                result["items"] = serde_json::json!([]);
            }
        }

        result
    }
}

/// Present item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentItem {
    pub present_id: String,
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

impl PresentItem {
    /// Convert to dictionary format for API response
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "item_id": self.item_id,
            "type": self.item_type,
            "amount": self.amount
        })
    }
}

/// User present association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresent {
    pub user_id: i32,
    pub present_id: String,
}

/// Present list response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentListResponse {
    pub presents: Vec<Present>,
}

impl PresentListResponse {
    /// Convert to list of dictionaries for API response
    pub fn to_dict_list(&self) -> Vec<serde_json::Value> {
        self.presents.iter().map(|p| p.to_dict(true)).collect()
    }
}

/// Present creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePresentRequest {
    pub present_id: String,
    pub expire_ts: Option<i64>,
    pub description: Option<String>,
    pub items: Vec<CreatePresentItem>,
}

/// Present item creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePresentItem {
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

impl From<CreatePresentItem> for PresentItem {
    fn from(item: CreatePresentItem) -> Self {
        Self {
            present_id: String::new(), // Will be set when creating the present
            item_id: item.item_id,
            item_type: item.item_type,
            amount: item.amount,
        }
    }
}
