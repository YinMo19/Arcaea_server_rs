use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// World map structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMap {
    pub map_id: String,
    pub is_legacy: bool,
    pub is_beyond: bool,
    pub is_breached: bool,
    pub beyond_health: Option<i32>,
    pub character_affinity: Vec<i32>,
    pub affinity_multiplier: Vec<f64>,
    pub chapter: Option<i32>,
    pub available_from: i64,
    pub available_to: i64,
    pub is_repeatable: bool,
    pub require_id: Option<String>,
    pub require_type: Option<String>,
    pub require_value: i32,
    pub coordinate: Option<String>,
    pub custom_bg: Option<String>,
    pub stamina_cost: Option<i32>,
    pub step_count: i32,
    pub require_localunlock_songid: Option<String>,
    pub require_localunlock_challengeid: Option<String>,
    pub chain_info: Option<serde_json::Value>,
    pub disable_over: Option<bool>,
    pub new_law: Option<String>,
    pub requires_any: Option<Vec<serde_json::Value>>,
    pub steps: Vec<WorldStep>,
}

impl WorldMap {
    /// Get rewards from all steps
    pub fn get_rewards(&self) -> Vec<StepReward> {
        let mut rewards = Vec::new();
        for step in &self.steps {
            if !step.items.is_empty() {
                rewards.push(StepReward {
                    position: step.position,
                    items: step.items.clone(),
                });
            }
        }
        rewards
    }

    /// Convert to dictionary format for API response
    pub fn to_dict(&self) -> serde_json::Value {
        let mut result = serde_json::json!({
            "map_id": self.map_id,
            "is_legacy": self.is_legacy,
            "is_beyond": self.is_beyond,
            "is_breached": self.is_breached,
            "beyond_health": self.beyond_health,
            "character_affinity": self.character_affinity,
            "affinity_multiplier": self.affinity_multiplier,
            "chapter": self.chapter,
            "available_from": self.available_from,
            "available_to": self.available_to,
            "is_repeatable": self.is_repeatable,
            "require_id": self.require_id,
            "require_type": self.require_type,
            "require_value": self.require_value,
            "coordinate": self.coordinate,
            "custom_bg": self.custom_bg,
            "stamina_cost": self.stamina_cost,
            "step_count": self.step_count,
            "require_localunlock_songid": self.require_localunlock_songid,
            "require_localunlock_challengeid": self.require_localunlock_challengeid,
            "steps": self.steps.iter().map(|s| s.to_dict()).collect::<Vec<_>>()
        });

        if let Some(ref chain_info) = self.chain_info {
            result["chain_info"] = chain_info.clone();
        }
        if let Some(disable_over) = self.disable_over {
            result["disable_over"] = serde_json::json!(disable_over);
        }
        if let Some(ref new_law) = self.new_law {
            if !new_law.is_empty() {
                result["new_law"] = serde_json::json!(new_law);
            }
        }
        if let Some(ref requires_any) = self.requires_any {
            result["requires_any"] = serde_json::json!(requires_any);
        }

        result
    }
}

/// User-specific world map data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMap {
    #[serde(flatten)]
    pub map: WorldMap,
    pub curr_position: i32,
    pub curr_capture: i32,
    pub is_locked: bool,
    pub user_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_capture: Option<i32>,
}

impl UserMap {
    /// Convert to dictionary format with optional sections
    pub fn to_dict(
        &self,
        has_map_info: bool,
        has_steps: bool,
        has_rewards: bool,
    ) -> serde_json::Value {
        if has_map_info {
            let mut result = self.map.to_dict();
            result["curr_position"] = serde_json::json!(self.curr_position);
            result["curr_capture"] = serde_json::json!(self.curr_capture);
            result["is_locked"] = serde_json::json!(self.is_locked);
            result["user_id"] = serde_json::json!(self.user_id);

            if !has_steps {
                result.as_object_mut().unwrap().remove("steps");
            }
            if has_rewards {
                result["rewards"] = serde_json::json!(self
                    .map
                    .get_rewards()
                    .iter()
                    .map(|r| r.to_dict())
                    .collect::<Vec<_>>());
            }

            result
        } else {
            serde_json::json!({
                "map_id": self.map.map_id,
                "curr_position": self.curr_position,
                "curr_capture": self.curr_capture,
                "is_locked": self.is_locked,
                "user_id": self.user_id
            })
        }
    }

    /// Get rewards for climbing from previous position to current position
    pub fn get_rewards_for_climbing(&self) -> Vec<StepReward> {
        let mut rewards = Vec::new();
        if let Some(prev_pos) = self.prev_position {
            for i in (prev_pos + 1)..=(self.curr_position) {
                if let Some(step) = self.map.steps.get(i as usize) {
                    if !step.items.is_empty() {
                        rewards.push(StepReward {
                            position: step.position,
                            items: step.items.clone(),
                        });
                    }
                }
            }
        }
        rewards
    }
}

/// World step structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStep {
    pub position: i32,
    pub capture: i32,
    pub items: Vec<StepItem>,
    pub restrict_id: Option<String>,
    pub restrict_ids: Option<Vec<String>>,
    pub restrict_type: Option<String>,
    pub restrict_difficulty: Option<i32>,
    pub step_type: Vec<String>,
    pub speed_limit_value: Option<i32>,
    pub plus_stamina_value: Option<i32>,
}

impl WorldStep {
    /// Convert to dictionary format for API response
    pub fn to_dict(&self) -> serde_json::Value {
        let mut result = serde_json::json!({
            "position": self.position,
            "capture": self.capture
        });

        if !self.items.is_empty() {
            result["items"] =
                serde_json::json!(self.items.iter().map(|i| i.to_dict()).collect::<Vec<_>>());
        }
        if let Some(ref restrict_type) = self.restrict_type {
            result["restrict_type"] = serde_json::json!(restrict_type);
            if let Some(ref restrict_id) = self.restrict_id {
                result["restrict_id"] = serde_json::json!(restrict_id);
            }
            if let Some(ref restrict_ids) = self.restrict_ids {
                result["restrict_ids"] = serde_json::json!(restrict_ids);
            }
            if let Some(restrict_difficulty) = self.restrict_difficulty {
                result["restrict_difficulty"] = serde_json::json!(restrict_difficulty);
            }
        }
        if !self.step_type.is_empty() {
            result["step_type"] = serde_json::json!(self.step_type);
        }
        if let Some(speed_limit_value) = self.speed_limit_value {
            result["speed_limit_value"] = serde_json::json!(speed_limit_value);
        }
        if let Some(plus_stamina_value) = self.plus_stamina_value {
            result["plus_stamina_value"] = serde_json::json!(plus_stamina_value);
        }

        result
    }
}

/// Step item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepItem {
    #[serde(rename = "id")]
    pub item_id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

impl StepItem {
    /// Convert to dictionary format for API response
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.item_id,
            "type": self.item_type,
            "amount": self.amount
        })
    }
}

/// Step reward structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepReward {
    pub position: i32,
    pub items: Vec<StepItem>,
}

impl StepReward {
    /// Convert to dictionary format for API response
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "position": self.position,
            "items": self.items.iter().map(|i| i.to_dict()).collect::<Vec<_>>()
        })
    }
}

/// World map info structure (for database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMapInfo {
    pub map_id: String,
    pub chapter: Option<i32>,
    pub is_repeatable: bool,
    pub is_beyond: bool,
    pub is_legacy: bool,
    pub step_count: i32,
}

/// User world entry from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorldEntry {
    pub user_id: i32,
    pub map_id: String,
    pub curr_position: i32,
    pub curr_capture: i32,
    pub is_locked: bool,
}

/// World all response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldAllResponse {
    pub current_map: String,
    pub user_id: i32,
    pub maps: Vec<serde_json::Value>,
}

/// World single map response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMapResponse {
    pub user_id: i32,
    pub current_map: String,
    pub maps: Vec<serde_json::Value>,
}

/// Map enter response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEnterResponse {
    pub map_id: String,
    pub curr_position: i32,
    pub curr_capture: i32,
    pub is_locked: bool,
    pub user_id: i32,
}

/// Stamina structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stamina {
    pub stamina: i32,
    pub max_stamina_ts: i64,
}

impl Stamina {
    /// Calculate current stamina based on time passed
    pub fn calculate_current_stamina(&self, max_stamina: i32, recover_rate: i64) -> i32 {
        if self.stamina >= max_stamina {
            return self.stamina;
        }

        let current_time = chrono::Utc::now().timestamp_millis();
        if current_time < self.max_stamina_ts {
            return self.stamina;
        }

        let time_passed = current_time - self.max_stamina_ts;
        let stamina_recovered = (time_passed / recover_rate) as i32;

        std::cmp::min(self.stamina + stamina_recovered, max_stamina)
    }
}

/// Map parser constants (would be loaded from files)
#[derive(Debug, Clone)]
pub struct MapParser {
    pub map_id_path: HashMap<String, String>,
    pub world_info: HashMap<String, WorldMapInfo>,
    pub chapter_info: HashMap<i32, Vec<String>>,
    pub chapter_info_without_repeatable: HashMap<i32, Vec<String>>,
}

impl Default for MapParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MapParser {
    /// Create new map parser instance
    pub fn new() -> Self {
        Self {
            map_id_path: HashMap::new(),
            world_info: HashMap::new(),
            chapter_info: HashMap::new(),
            chapter_info_without_repeatable: HashMap::new(),
        }
    }

    /// Parse world map files (placeholder - would load from JSON files)
    pub fn parse(&mut self) {
        // This would scan the world map folder and load all JSON files
        // For now, this is a placeholder
    }

    /// Get world info for a specific map (placeholder)
    pub fn get_world_info(&self, map_id: &str) -> Option<&WorldMapInfo> {
        self.world_info.get(map_id)
    }
}
