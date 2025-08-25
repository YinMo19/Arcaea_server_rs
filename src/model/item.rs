use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;

/// Base item structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    #[serde(rename = "id")]
    pub item_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_available: Option<bool>,
}

/// Database item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbItem {
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub is_available: bool,
}

/// User item structure from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbUserItem {
    pub user_id: i32,
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

/// Core item structure with special formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreItem {
    pub core_type: String,
    pub amount: i32,
}

/// Character item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterItem {
    pub character_id: i32,
    pub amount: i32,
}

/// Item creation request
#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: Option<i32>,
    pub is_available: Option<bool>,
}

/// Item update request
#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub is_available: bool,
}

/// User item list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserItemListResponse {
    pub items: Vec<Item>,
}

/// Collection item for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionItem {
    pub collection_id: String,
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

/// Item factory creation parameters
#[derive(Debug, Deserialize)]
pub struct ItemFactoryParams {
    pub item_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: Option<i32>,
    pub is_available: Option<bool>,
}

/// Item from string parameters
#[derive(Debug)]
pub struct ItemFromString {
    pub item_type: String,
    pub item_id: String,
    pub amount: i32,
}

impl Item {
    /// Create new item with specified parameters
    pub fn new(
        item_id: Option<String>,
        item_type: String,
        amount: Option<i32>,
        is_available: Option<bool>,
    ) -> Self {
        Self {
            item_id,
            item_type,
            amount,
            is_available,
        }
    }

    /// Convert to dictionary format with optional fields
    pub fn to_dict(
        &self,
        has_is_available: bool,
        has_amount: bool,
    ) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();

        if let Some(ref id) = self.item_id {
            result.insert("id".to_string(), Value::String(id.clone()));
        }
        result.insert("type".to_string(), Value::String(self.item_type.clone()));

        if has_amount {
            if let Some(amount) = self.amount {
                result.insert("amount".to_string(), Value::Number(amount.into()));
            }
        }

        if has_is_available {
            if let Some(available) = self.is_available {
                result.insert("is_available".to_string(), Value::Bool(available));
            }
        }

        result
    }

    /// Convert to character format for core items
    pub fn to_character_format(&self) -> Option<CoreItem> {
        if self.item_type == "core" {
            Some(CoreItem {
                core_type: self.item_id.clone().unwrap_or_default(),
                amount: self.amount.unwrap_or(0),
            })
        } else {
            None
        }
    }
}

/// Item exists check result
#[derive(Debug)]
pub struct ItemExists {
    pub exists: i32,
}

/// Character name to ID mapping
#[derive(Debug)]
pub struct CharacterMapping {
    pub character_id: i32,
}

/// User ticket info
#[derive(Debug)]
pub struct UserTicket {
    pub ticket: Option<i32>,
}

/// Item constants and type definitions
pub struct ItemTypes;

impl ItemTypes {
    pub const CORE: &'static str = "core";
    pub const CHARACTER: &'static str = "character";
    pub const MEMORY: &'static str = "memory";
    pub const FRAGMENT: &'static str = "fragment";
    pub const ANNI5TIX: &'static str = "anni5tix";
    pub const PICK_TICKET: &'static str = "pick_ticket";
    pub const WORLD_SONG: &'static str = "world_song";
    pub const WORLD_UNLOCK: &'static str = "world_unlock";
    pub const SINGLE: &'static str = "single";
    pub const PACK: &'static str = "pack";
    pub const PROG_BOOST_300: &'static str = "prog_boost_300";
    pub const STAMINA6: &'static str = "stamina6";
    pub const STAMINA: &'static str = "stamina";
    pub const COURSE_BANNER: &'static str = "course_banner";
}

/// Item configuration trait
pub trait ItemConfig {
    fn get_item_type(&self) -> &str;
    fn get_default_amount(&self) -> i32;
    fn is_available_by_default(&self) -> bool;
}

/// Core item implementation
pub struct CoreItemConfig;
impl ItemConfig for CoreItemConfig {
    fn get_item_type(&self) -> &str {
        ItemTypes::CORE
    }
    fn get_default_amount(&self) -> i32 {
        0
    }
    fn is_available_by_default(&self) -> bool {
        true
    }
}

/// Character item implementation
pub struct CharacterItemConfig;
impl ItemConfig for CharacterItemConfig {
    fn get_item_type(&self) -> &str {
        ItemTypes::CHARACTER
    }
    fn get_default_amount(&self) -> i32 {
        1
    }
    fn is_available_by_default(&self) -> bool {
        true
    }
}

/// Memory item implementation
pub struct MemoryItemConfig;
impl ItemConfig for MemoryItemConfig {
    fn get_item_type(&self) -> &str {
        ItemTypes::MEMORY
    }
    fn get_default_amount(&self) -> i32 {
        1
    }
    fn is_available_by_default(&self) -> bool {
        true
    }
}

/// Fragment item implementation
pub struct FragmentItemConfig;
impl ItemConfig for FragmentItemConfig {
    fn get_item_type(&self) -> &str {
        ItemTypes::FRAGMENT
    }
    fn get_default_amount(&self) -> i32 {
        0
    }
    fn is_available_by_default(&self) -> bool {
        true
    }
}

/// Normal item types
pub struct NormalItemTypes;
impl NormalItemTypes {
    pub const WORLD_SONG: &'static str = ItemTypes::WORLD_SONG;
    pub const WORLD_UNLOCK: &'static str = ItemTypes::WORLD_UNLOCK;
    pub const COURSE_BANNER: &'static str = ItemTypes::COURSE_BANNER;
    pub const SINGLE: &'static str = ItemTypes::SINGLE;
    pub const PACK: &'static str = ItemTypes::PACK;
}

/// Positive item types
pub struct PositiveItemTypes;
impl PositiveItemTypes {
    pub const CORE: &'static str = ItemTypes::CORE;
    pub const ANNI5TIX: &'static str = ItemTypes::ANNI5TIX;
    pub const PICK_TICKET: &'static str = ItemTypes::PICK_TICKET;
}

/// Special item types that require custom handling
pub struct SpecialItemTypes;
impl SpecialItemTypes {
    pub const PROG_BOOST_300: &'static str = ItemTypes::PROG_BOOST_300;
    pub const STAMINA6: &'static str = ItemTypes::STAMINA6;
    pub const STAMINA: &'static str = ItemTypes::STAMINA;
}
