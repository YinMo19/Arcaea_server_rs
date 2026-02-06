use serde::{Deserialize, Serialize};

/// Purchase structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Purchase {
    pub purchase_name: String,
    pub price: i32,
    pub orig_price: i32,
    pub discount_from: Option<i64>,
    pub discount_to: Option<i64>,
    pub discount_reason: Option<String>,
    pub items: Vec<PurchaseItem>,
}

impl Purchase {
    /// Get the displayed price considering discounts and special tickets
    pub fn price_displayed(
        &self,
        user_special_items: &std::collections::HashMap<String, i32>,
    ) -> i32 {
        if let (Some(from), Some(to)) = (self.discount_from, self.discount_to) {
            let current_time = chrono::Utc::now().timestamp_millis();
            if from <= current_time && current_time <= to {
                if let Some(ref reason) = self.discount_reason {
                    match reason.as_str() {
                        "anni5tix" => {
                            if user_special_items.get("anni5tix").unwrap_or(&0) >= &1 {
                                return 0;
                            }
                        }
                        "pick_ticket" => {
                            if user_special_items.get("pick_ticket").unwrap_or(&0) >= &1 {
                                return 0;
                            }
                        }
                        _ => {}
                    }
                }
                return self.price;
            }
        }
        self.orig_price
    }

    /// Convert to dictionary format for API response
    pub fn to_dict(&self, has_items: bool, show_real_price: bool) -> serde_json::Value {
        let price = if show_real_price {
            // This would need user context to calculate properly
            // For now, use orig_price as default
            self.orig_price
        } else {
            self.price
        };

        let mut result = serde_json::json!({
            "name": self.purchase_name,
            "price": price,
            "orig_price": self.orig_price
        });

        if has_items {
            result["items"] = serde_json::json!(self
                .items
                .iter()
                .map(|i| i.to_dict(true))
                .collect::<Vec<_>>());
        }

        if let (Some(from), Some(to)) = (self.discount_from, self.discount_to) {
            if from > 0 && to > 0 {
                result["discount_from"] = serde_json::json!(from);
                result["discount_to"] = serde_json::json!(to);

                if !show_real_price
                    || (self
                        .discount_reason
                        .as_ref()
                        .is_some_and(|r| (r == "anni5tix" || r == "pick_ticket") && price == 0))
                {
                    if let Some(ref reason) = self.discount_reason {
                        result["discount_reason"] = serde_json::json!(reason);
                    }
                }
            }
        }

        result
    }

    /// Create from dictionary
    pub fn from_dict(d: serde_json::Value) -> Result<Self, String> {
        let obj = d.as_object().ok_or("Expected object")?;

        let purchase_name = obj
            .get("name")
            .or_else(|| obj.get("purchase_name"))
            .and_then(|v| v.as_str())
            .ok_or("purchase_name is required")?
            .to_string();

        let orig_price = obj
            .get("orig_price")
            .and_then(|v| v.as_i64())
            .ok_or("orig_price is required")? as i32;

        let price = obj
            .get("price")
            .and_then(|v| v.as_i64())
            .unwrap_or(orig_price as i64) as i32;

        let discount_from = obj.get("discount_from").and_then(|v| v.as_i64());

        let discount_to = obj.get("discount_to").and_then(|v| v.as_i64());

        let discount_reason = obj
            .get("discount_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let items = if let Some(items_val) = obj.get("items") {
            if let Some(items_array) = items_val.as_array() {
                items_array
                    .iter()
                    .map(|item| PurchaseItem::from_dict(item.clone()))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        Ok(Self {
            purchase_name,
            price,
            orig_price,
            discount_from,
            discount_to,
            discount_reason,
            items,
        })
    }
}

/// Purchase item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseItem {
    pub purchase_name: String,
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

impl PurchaseItem {
    /// Convert to dictionary format for API response
    pub fn to_dict(&self, has_is_available: bool) -> serde_json::Value {
        let mut result = serde_json::json!({
            "item_id": self.item_id,
            "type": self.item_type,
            "amount": self.amount
        });

        if has_is_available {
            result["is_available"] = serde_json::json!(true);
        }

        result
    }

    /// Create from dictionary
    pub fn from_dict(d: serde_json::Value) -> Result<Self, String> {
        let obj = d.as_object().ok_or("Expected object")?;

        let item_id = obj
            .get("item_id")
            .and_then(|v| v.as_str())
            .ok_or("item_id is required")?
            .to_string();

        let item_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("type is required")?
            .to_string();

        let amount = obj.get("amount").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

        Ok(Self {
            purchase_name: String::new(), // Will be set when creating purchase
            item_id,
            item_type,
            amount,
        })
    }
}

/// Purchase list structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseList {
    pub purchases: Vec<Purchase>,
}

impl Default for PurchaseList {
    fn default() -> Self {
        Self::new()
    }
}

impl PurchaseList {
    /// Create new empty purchase list
    pub fn new() -> Self {
        Self {
            purchases: Vec::new(),
        }
    }

    /// Add purchase to list
    pub fn add_purchase(&mut self, purchase: Purchase) {
        self.purchases.push(purchase);
    }

    /// Convert to list of dictionaries for API response
    pub fn to_dict_list(&self) -> Vec<serde_json::Value> {
        self.purchases
            .iter()
            .map(|p| p.to_dict(true, true))
            .collect()
    }

    /// Select purchases from database by type (placeholder method signature)
    pub fn select_from_type(&self, purchase_type: &str) -> Self {
        // This would filter purchases by type
        // For now, return self as placeholder
        self.clone()
    }
}

/// Pack/Single purchase response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSinglePurchaseResponse {
    pub user_id: i32,
    pub ticket: i32,
    pub packs: Vec<String>,
    pub singles: Vec<String>,
    pub characters: Vec<i32>,
}

/// Special item purchase response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialItemPurchaseResponse {
    pub user_id: i32,
    pub ticket: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stamina: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_stamina_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_mode_locked_end_ts: Option<i64>,
}

/// Stamina purchase response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaminaPurchaseResponse {
    pub user_id: i32,
    pub stamina: i32,
    pub max_stamina_ts: i64,
    pub next_fragstam_ts: i64,
    pub world_mode_locked_end_ts: i64,
}

/// Redeem response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemResponse {
    pub coupon: String,
}

/// Purchase request structures
#[derive(Debug, Clone, Deserialize)]
pub struct PackPurchaseRequest {
    pub pack_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SinglePurchaseRequest {
    pub single_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpecialItemPurchaseRequest {
    pub item_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedeemRequest {
    pub code: String,
}

/// Bundle purchase item (for bundle endpoints)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub id: String,
}

/// Bundle purchase structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePurchase {
    pub name: String,
    pub items: Vec<BundleItem>,
    pub orig_price: i32,
    pub price: i32,
    pub discount_from: i64,
    pub discount_to: i64,
    pub discount_reason: String,
}

/// Item factory helper (matches Python ItemFactory pattern)
pub struct ItemFactory;

impl ItemFactory {
    /// Create item from dictionary (placeholder)
    pub fn from_dict(d: serde_json::Value) -> Result<PurchaseItem, String> {
        PurchaseItem::from_dict(d)
    }

    /// Get item by type (placeholder)
    pub fn get_item(item_type: &str) -> serde_json::Value {
        serde_json::json!({
            "item_id": item_type,
            "type": item_type,
            "amount": 1
        })
    }
}
