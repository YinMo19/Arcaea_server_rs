use crate::config::Constants;
use crate::error::ArcError;

use crate::model::world::*;
use serde_json;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global map parser instance
static MAP_PARSER: OnceLock<MapParser> = OnceLock::new();

/// Map parser for loading and caching world map data
#[derive(Debug, Clone)]
pub struct MapParser {
    pub map_id_path: HashMap<String, String>,
    pub world_info: HashMap<String, WorldMapInfo>,
    pub chapter_info: HashMap<i32, Vec<String>>,
    pub chapter_info_without_repeatable: HashMap<i32, Vec<String>>,
}

impl MapParser {
    /// Create a new map parser and parse all map files
    pub fn new() -> Self {
        let mut parser = Self {
            map_id_path: HashMap::new(),
            world_info: HashMap::new(),
            chapter_info: HashMap::new(),
            chapter_info_without_repeatable: HashMap::new(),
        };
        parser.parse();
        parser
    }

    /// Parse all map files from the assets directory
    pub fn parse(&mut self) {
        let map_path = Path::new("src/assets/map");

        if let Ok(entries) = std::fs::read_dir(map_path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".json") {
                        let map_id = file_name.trim_end_matches(".json").to_string();
                        let path = entry.path().to_string_lossy().to_string();
                        self.map_id_path.insert(map_id.clone(), path);

                        if let Ok(map_data) = self.get_world_info(&map_id) {
                            if let Some(chapter) = map_data.chapter {
                                self.chapter_info
                                    .entry(chapter)
                                    .or_default()
                                    .push(map_id.clone());

                                if !map_data.is_repeatable {
                                    self.chapter_info_without_repeatable
                                        .entry(chapter)
                                        .or_default()
                                        .push(map_id.clone());
                                }
                            }

                            self.world_info.insert(map_id, map_data);
                        }
                    }
                }
            }
        }
    }

    /// Get world info for a specific map ID
    pub fn get_world_info(&self, map_id: &str) -> Result<WorldMapInfo, ArcError> {
        if let Some(cached) = self.world_info.get(map_id) {
            return Ok(cached.clone());
        }

        if let Some(path) = self.map_id_path.get(map_id) {
            let content = std::fs::read_to_string(path).map_err(|e| ArcError::Database {
                message: format!("Failed to read map file {path}: {e}"),
            })?;

            let map_data: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| ArcError::Database {
                    message: format!("Failed to parse map JSON {path}: {e}"),
                })?;

            let world_map_info = WorldMapInfo {
                map_id: map_id.to_string(),
                chapter: map_data
                    .get("chapter")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                is_repeatable: map_data
                    .get("is_repeatable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_beyond: map_data
                    .get("is_beyond")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_legacy: map_data
                    .get("is_legacy")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                step_count: map_data
                    .get("steps")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len() as i32)
                    .unwrap_or(0),
            };

            return Ok(world_map_info);
        }

        Err(ArcError::no_data(format!("Map {map_id} not found"), 404))
    }

    /// Load full world map data from JSON file
    pub fn load_world_map(&self, map_id: &str) -> Result<WorldMap, ArcError> {
        if let Some(path) = self.map_id_path.get(map_id) {
            let content = std::fs::read_to_string(path).map_err(|e| ArcError::Database {
                message: format!("Failed to read map file {path}: {e}"),
            })?;

            let map_data: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| ArcError::Database {
                    message: format!("Failed to parse map JSON {path}: {e}"),
                })?;

            let mut world_map = WorldMap {
                map_id: map_id.to_string(),
                is_legacy: map_data
                    .get("is_legacy")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_beyond: map_data
                    .get("is_beyond")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_breached: map_data
                    .get("is_breached")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                beyond_health: map_data
                    .get("beyond_health")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                character_affinity: map_data
                    .get("character_affinity")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_i64().map(|i| i as i32))
                            .collect()
                    })
                    .unwrap_or_default(),
                affinity_multiplier: map_data
                    .get("affinity_multiplier")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default(),
                chapter: map_data
                    .get("chapter")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                available_from: map_data
                    .get("available_from")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1),
                available_to: map_data
                    .get("available_to")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(9999999999999),
                is_repeatable: map_data
                    .get("is_repeatable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                require_id: map_data
                    .get("require_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                require_type: map_data
                    .get("require_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                require_value: map_data
                    .get("require_value")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32)
                    .unwrap_or(1),
                coordinate: map_data
                    .get("coordinate")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                custom_bg: map_data
                    .get("custom_bg")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                stamina_cost: map_data
                    .get("stamina_cost")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                step_count: 0,
                require_localunlock_songid: map_data
                    .get("require_localunlock_songid")
                    .and_then(|v| v.as_str())
                    .map_or(Some(String::from("")), |s| Some(s.to_string())),
                require_localunlock_challengeid: map_data
                    .get("require_localunlock_challengeid")
                    .and_then(|v| v.as_str())
                    .map_or(Some(String::from("")), |s| Some(s.to_string())),
                chain_info: map_data.get("chain_info").cloned(),
                disable_over: map_data.get("disable_over").and_then(|v| v.as_bool()),
                new_law: map_data
                    .get("new_law")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                requires_any: map_data
                    .get("requires_any")
                    .and_then(|v| v.as_array()).cloned(),
                steps: Vec::new(),
            };

            // Parse steps
            if let Some(steps_array) = map_data.get("steps").and_then(|v| v.as_array()) {
                for (index, step_data) in steps_array.iter().enumerate() {
                    let mut step = WorldStep {
                        position: index as i32,
                        capture: step_data
                            .get("capture")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32)
                            .unwrap_or(0),
                        items: Vec::new(),
                        restrict_id: step_data
                            .get("restrict_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        restrict_ids: step_data
                            .get("restrict_ids")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            }),
                        restrict_type: step_data
                            .get("restrict_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        restrict_difficulty: step_data
                            .get("restrict_difficulty")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32),
                        step_type: step_data
                            .get("step_type")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        speed_limit_value: step_data
                            .get("speed_limit_value")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32),
                        plus_stamina_value: step_data
                            .get("plus_stamina_value")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32),
                    };

                    // Parse items
                    if let Some(items_array) = step_data.get("items").and_then(|v| v.as_array()) {
                        for item_data in items_array {
                            let mut item_id = item_data
                                .get("item_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if item_id.is_empty() {
                                item_id = item_data
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                            }

                            let item = StepItem {
                                item_id,
                                item_type: item_data
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                amount: item_data
                                    .get("amount")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32)
                                    .unwrap_or(0),
                            };
                            step.items.push(item);
                        }
                    }

                    world_map.steps.push(step);
                }
            }

            world_map.step_count = world_map.steps.len() as i32;
            return Ok(world_map);
        }

        Err(ArcError::no_data(format!("Map {map_id} not found"), 404))
    }

    /// Get all map IDs
    pub fn get_all_map_ids(&self) -> Vec<String> {
        self.map_id_path.keys().cloned().collect()
    }
}

impl Default for MapParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global map parser instance
pub fn get_map_parser() -> &'static MapParser {
    MAP_PARSER.get_or_init(MapParser::new)
}

/// User map implementation with climbing logic
#[derive(Debug, Clone)]
pub struct UserMapImpl {
    pub map: WorldMap,
    pub curr_position: i32,
    pub curr_capture: i32,
    pub is_locked: bool,
    pub user_id: i32,
    pub prev_position: Option<i32>,
    pub prev_capture: Option<i32>,
}

impl UserMapImpl {
    /// Create a new user map instance
    pub fn new(map: WorldMap, user_id: i32) -> Self {
        Self {
            map,
            curr_position: 0,
            curr_capture: 0,
            is_locked: true,
            user_id,
            prev_position: None,
            prev_capture: None,
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

    /// Climb the map with the given step value
    pub fn climb(&mut self, step_value: f64) -> Result<(), ArcError> {
        if step_value < 0.0 {
            return Err(ArcError::input(
                "Step value must be non-negative".to_string(),
            ));
        }

        if self.is_locked {
            return Err(ArcError::input("Map is locked".to_string()));
        }

        self.prev_position = Some(self.curr_position);
        self.prev_capture = Some(self.curr_capture);

        if self.map.is_beyond {
            // Beyond map logic
            let beyond_health = self.map.beyond_health.unwrap_or(100);
            let dt = beyond_health - self.curr_capture;
            self.curr_capture = if dt >= step_value as i32 {
                self.curr_capture + step_value as i32
            } else {
                beyond_health
            };

            let mut i = 0;
            let mut t = self.prev_capture.unwrap_or(0) + step_value as i32;
            while i < self.map.step_count && t > 0 {
                if let Some(step) = self.map.steps.get(i as usize) {
                    let dt = step.capture;
                    if dt > t {
                        t = 0;
                    } else {
                        t -= dt;
                        i += 1;
                    }
                } else {
                    break;
                }
            }

            if i >= self.map.step_count {
                self.curr_position = self.map.step_count - 1;
            } else {
                self.curr_position = i;
            }
        } else {
            // Regular map logic
            let mut curr_position = self.curr_position;
            let mut curr_capture = self.curr_capture;
            let mut step_value = step_value;

            while step_value > 0.0 && curr_position < self.map.step_count {
                if let Some(step) = self.map.steps.get(curr_position as usize) {
                    let dt = step.capture - curr_capture;
                    if dt as f64 > step_value {
                        curr_capture += step_value as i32;
                        step_value = 0.0;
                    } else {
                        step_value -= dt as f64;
                        curr_capture = 0;
                        curr_position += 1;
                    }
                } else {
                    break;
                }
            }

            if curr_position >= self.map.step_count {
                self.curr_position = self.map.step_count - 1;
                self.curr_capture = 0;
            } else {
                self.curr_position = curr_position;
                self.curr_capture = curr_capture;
            }
        }

        Ok(())
    }

    /// Reclimb the map (reset to previous position and climb again)
    pub fn reclimb(&mut self, step_value: f64) -> Result<(), ArcError> {
        self.curr_position = self.prev_position.unwrap_or(0);
        self.curr_capture = self.prev_capture.unwrap_or(0);
        self.climb(step_value)
    }
}

/// Stamina calculation logic
#[derive(Debug, Clone)]
pub struct StaminaImpl {
    stamina: i32,
    max_stamina_ts: i64,
}

impl StaminaImpl {
    /// Create new stamina instance
    pub fn new(stamina: i32, max_stamina_ts: i64) -> Self {
        Self {
            stamina: if stamina > 0 {
                stamina
            } else {
                Constants::MAX_STAMINA
            },
            max_stamina_ts: if max_stamina_ts > 0 {
                max_stamina_ts
            } else {
                0
            },
        }
    }

    /// Get current stamina value based on time calculation
    pub fn get_current_stamina(&self) -> i32 {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let calculated_stamina = Constants::MAX_STAMINA
            - ((self.max_stamina_ts - current_time) / Constants::STAMINA_RECOVER_TICK) as i32;

        if calculated_stamina >= Constants::MAX_STAMINA {
            if self.stamina >= Constants::MAX_STAMINA {
                self.stamina
            } else {
                Constants::MAX_STAMINA
            }
        } else {
            calculated_stamina
        }
    }

    /// Set stamina value and update max_stamina_ts accordingly
    pub fn set_stamina(&mut self, value: i32) {
        self.stamina = value;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.max_stamina_ts = current_time
            - (self.stamina - Constants::MAX_STAMINA) as i64 * Constants::STAMINA_RECOVER_TICK;
    }
}

/// World service for handling world map system
pub struct WorldService {
    pool: MySqlPool,
}

impl WorldService {
    /// Create a new world service instance
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get all world maps for a user
    ///
    /// Returns comprehensive world map information including user progress,
    /// map details, and reward information for all available maps.
    pub async fn get_user_world_all(&self, user_id: i32) -> Result<serde_json::Value, ArcError> {
        let current_map = self.get_user_current_map(user_id).await?;
        let maps = self.get_all_user_maps(user_id).await?;

        Ok(serde_json::json!({
            "current_map": current_map,
            "user_id": user_id,
            "maps": maps
        }))
    }

    /// Get user's current map
    async fn get_user_current_map(&self, user_id: i32) -> Result<String, ArcError> {
        let current_map =
            sqlx::query_scalar!("SELECT current_map FROM user WHERE user_id = ?", user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ArcError::Database {
                    message: format!("Failed to get user current map: {e}"),
                })?;

        Ok(current_map
            .flatten()
            .unwrap_or_else(|| "tutorial".to_string()))
    }

    /// Get all maps with user progress
    async fn get_all_user_maps(&self, user_id: i32) -> Result<Vec<serde_json::Value>, ArcError> {
        let parser = get_map_parser();
        let mut maps = Vec::new();

        for map_id in parser.get_all_map_ids() {
            let user_map = self.load_user_map(user_id, &map_id).await?;
            let user_map_impl = UserMapImpl {
                map: user_map.map,
                curr_position: user_map.curr_position,
                curr_capture: user_map.curr_capture,
                is_locked: user_map.is_locked,
                user_id: user_map.user_id,
                prev_position: user_map.prev_position,
                prev_capture: user_map.prev_capture,
            };
            maps.push(user_map_impl.to_dict(true, false, true));
        }

        Ok(maps)
    }

    /// Load user map data
    async fn load_user_map(&self, user_id: i32, map_id: &str) -> Result<UserMap, ArcError> {
        let parser = get_map_parser();
        let world_map = parser.load_world_map(map_id)?;

        // Get user progress for this map
        let user_world = sqlx::query!(
            r#"
            SELECT user_id, map_id, curr_position, curr_capture, is_locked
            FROM user_world
            WHERE user_id = ? AND map_id = ?
            "#,
            user_id,
            map_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user world progress: {e}"),
        })?;

        let (curr_position, curr_capture, is_locked) = if let Some(record) = user_world {
            (
                record.curr_position.unwrap_or(0),
                record.curr_capture.unwrap_or(0.0) as i32,
                record.is_locked.unwrap_or(1) != 0,
            )
        } else {
            // Initialize new entry
            sqlx::query!(
                "INSERT INTO user_world (user_id, map_id, curr_position, curr_capture, is_locked) VALUES (?, ?, 0, 0, 1)",
                user_id,
                map_id
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to initialize user map: {e}"),
            })?;

            (0, 0, true)
        };

        Ok(UserMap {
            map: world_map,
            curr_position,
            curr_capture,
            is_locked,
            user_id,
            prev_position: None,
            prev_capture: None,
        })
    }

    /// Get single map information
    ///
    /// Returns detailed information about a specific map including
    /// user progress, steps, and rewards.
    pub async fn get_user_map(
        &self,
        user_id: i32,
        map_id: &str,
    ) -> Result<serde_json::Value, ArcError> {
        // Set user's current map to this map
        self.set_user_current_map(user_id, map_id).await?;

        // Load user map data
        let user_map = self.load_user_map(user_id, map_id).await?;
        let user_map_impl = UserMapImpl {
            map: user_map.map,
            curr_position: user_map.curr_position,
            curr_capture: user_map.curr_capture,
            is_locked: user_map.is_locked,
            user_id: user_map.user_id,
            prev_position: user_map.prev_position,
            prev_capture: user_map.prev_capture,
        };

        Ok(serde_json::json!({
            "user_id": user_id,
            "current_map": map_id,
            "maps": [user_map_impl.to_dict(true, true, true)]
        }))
    }

    /// Set user's current map
    async fn set_user_current_map(&self, user_id: i32, map_id: &str) -> Result<(), ArcError> {
        sqlx::query!(
            "UPDATE user SET current_map = ? WHERE user_id = ?",
            map_id,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to set user current map: {e}"),
        })?;

        Ok(())
    }

    /// Enter/unlock a map
    ///
    /// Attempts to unlock the specified map for the user.
    /// Returns map information if successful.
    pub async fn enter_map(
        &self,
        user_id: i32,
        map_id: &str,
    ) -> Result<serde_json::Value, ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {e}"),
        })?;

        let mut user_map = self.load_user_map(user_id, map_id).await?;

        // Check if map can be unlocked
        if user_map.is_locked {
            let can_unlock = self.check_map_unlock_requirements(user_id, map_id).await?;
            if can_unlock {
                user_map.is_locked = false;
                user_map.curr_position = 0;
                user_map.curr_capture = 0;

                sqlx::query!(
                    "UPDATE user_world SET is_locked = 0, curr_position = 0, curr_capture = 0 WHERE user_id = ? AND map_id = ?",
                    user_id,
                    map_id
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| ArcError::Database {
                    message: format!("Failed to unlock map: {e}"),
                })?;
            }
        }

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {e}"),
        })?;

        Ok(serde_json::json!({
            "map_id": map_id,
            "curr_position": user_map.curr_position,
            "curr_capture": user_map.curr_capture,
            "is_locked": user_map.is_locked,
            "user_id": user_id
        }))
    }

    /// Check if user meets requirements to unlock a map
    async fn check_map_unlock_requirements(
        &self,
        user_id: i32,
        map_id: &str,
    ) -> Result<bool, ArcError> {
        let parser = get_map_parser();
        let world_map = parser.load_world_map(map_id)?;

        if let Some(require_type) = &world_map.require_type {
            if !require_type.is_empty() && (require_type == "pack" || require_type == "single") {
                if let Some(require_id) = &world_map.require_id {
                    let item_count = sqlx::query_scalar!(
                        "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?",
                        user_id,
                        require_id,
                        require_type
                    )
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| ArcError::Database {
                        message: format!("Failed to check item requirement: {e}"),
                    })?;

                    if item_count.flatten().unwrap_or(0) <= 0 {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }
}

impl UserMapImpl {
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

            if let Some(prev_position) = self.prev_position {
                result["prev_position"] = serde_json::json!(prev_position);
            }
            if let Some(prev_capture) = self.prev_capture {
                result["prev_capture"] = serde_json::json!(prev_capture);
            }

            if !has_steps {
                result.as_object_mut().unwrap().remove("steps");
            }
            if has_rewards {
                result["rewards"] = serde_json::json!(self.map.get_rewards());
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
}
