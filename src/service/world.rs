use crate::error::ArcError;
use sqlx::MySqlPool;

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
        // Get user's current map
        let current_map = self.get_user_current_map(user_id).await?;

        // Get all maps with user progress
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
                    message: format!("Failed to get user current map: {}", e),
                })?;

        Ok(current_map
            .flatten()
            .unwrap_or_else(|| "tutorial".to_string()))
    }

    /// Get all maps with user progress
    async fn get_all_user_maps(&self, _user_id: i32) -> Result<Vec<serde_json::Value>, ArcError> {
        // This would typically load from JSON files like the Python version
        // For now, return empty array as world system is not fully implemented
        Ok(vec![])
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

        // Get map information (would load from JSON files)
        let map_info = self.get_map_info(map_id).await?;

        Ok(serde_json::json!({
            "user_id": user_id,
            "current_map": map_id,
            "maps": [map_info]
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
            message: format!("Failed to set user current map: {}", e),
        })?;

        Ok(())
    }

    /// Get map information
    async fn get_map_info(&self, map_id: &str) -> Result<serde_json::Value, ArcError> {
        // This would load map data from JSON files like Python version
        // For now, return basic structure
        Ok(serde_json::json!({
            "map_id": map_id,
            "is_locked": false,
            "curr_position": 0,
            "curr_capture": 0,
            "steps": []
        }))
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
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Get or create user map entry
        let user_map_record = sqlx::query!(
            r#"
            SELECT map_id, curr_position, curr_capture, is_locked
            FROM user_world
            WHERE user_id = ? AND map_id = ?
            "#,
            user_id,
            map_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user map: {}", e),
        })?;

        let user_map = match user_map_record {
            Some(record) => UserMapEntry {
                map_id: record.map_id,
                curr_position: record.curr_position.unwrap_or(0),
                curr_capture: record.curr_capture.unwrap_or(0.0) as i32,
                is_locked: record.is_locked.unwrap_or(1) == 1,
            },
            None => {
                // Initialize new map entry
                sqlx::query!(
                    "INSERT INTO user_world (user_id, map_id, curr_position, curr_capture, is_locked) VALUES (?, ?, 0, 0, 1)",
                    user_id,
                    map_id
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| ArcError::Database {
                    message: format!("Failed to initialize user map: {}", e),
                })?;

                UserMapEntry {
                    map_id: map_id.to_string(),
                    curr_position: 0,
                    curr_capture: 0,
                    is_locked: true,
                }
            }
        };

        // Check if map can be unlocked
        let can_unlock = self.check_map_unlock_requirements(user_id, map_id).await?;

        if user_map.is_locked && can_unlock {
            // Unlock the map
            sqlx::query!(
                "UPDATE user_world SET is_locked = 0, curr_position = 0, curr_capture = 0 WHERE user_id = ? AND map_id = ?",
                user_id,
                map_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to unlock map: {}", e),
            })?;
        }

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        // Return map information
        Ok(serde_json::json!({
            "map_id": map_id,
            "curr_position": if user_map.is_locked && can_unlock { 0 } else { user_map.curr_position },
            "curr_capture": if user_map.is_locked && can_unlock { 0 } else { user_map.curr_capture },
            "is_locked": user_map.is_locked && !can_unlock,
            "user_id": user_id
        }))
    }

    /// Check if user meets requirements to unlock a map
    async fn check_map_unlock_requirements(
        &self,
        _user_id: i32,
        _map_id: &str,
    ) -> Result<bool, ArcError> {
        // This would check map requirements from JSON files
        // For now, allow all maps to be unlocked
        Ok(true)
    }

    /// Climb/progress on a map
    ///
    /// Advances user position on the map based on step value.
    /// Handles both regular and Beyond maps with different mechanics.
    pub async fn climb_map(
        &self,
        user_id: i32,
        map_id: &str,
        step_value: f64,
    ) -> Result<serde_json::Value, ArcError> {
        if step_value < 0.0 {
            return Err(ArcError::input(
                "Step value must be non-negative".to_string(),
            ));
        }

        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Get current user map state
        let user_map_record = sqlx::query!(
            r#"
            SELECT map_id, curr_position, curr_capture, is_locked
            FROM user_world
            WHERE user_id = ? AND map_id = ?
            "#,
            user_id,
            map_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user map: {}", e),
        })?;

        let user_map_record =
            user_map_record.ok_or_else(|| ArcError::no_data("Map not found for user", 404, -2))?;

        let user_map = UserMapEntry {
            map_id: user_map_record.map_id,
            curr_position: user_map_record.curr_position.unwrap_or(0),
            curr_capture: user_map_record.curr_capture.unwrap_or(0.0) as i32,
            is_locked: user_map_record.is_locked.unwrap_or(1) == 1,
        };

        if user_map.is_locked {
            return Err(ArcError::input("Map is locked".to_string()));
        }

        // Load map information to get steps and determine climb logic
        let map_info = self.load_map_data(map_id).await?;

        // Calculate new position and capture
        let (new_position, new_capture) = self
            .calculate_climb_progress(&user_map, &map_info, step_value)
            .await?;

        // Update user map progress
        sqlx::query!(
            "UPDATE user_world SET curr_position = ?, curr_capture = ? WHERE user_id = ? AND map_id = ?",
            new_position,
            new_capture,
            user_id,
            map_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to update user map progress: {}", e),
        })?;

        // Get rewards for the climbed steps
        let rewards = self
            .get_step_rewards(&map_info, user_map.curr_position, new_position)
            .await?;

        // Grant rewards to user
        for reward in &rewards {
            self.grant_step_rewards(&mut tx, user_id, reward).await?;
        }

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(serde_json::json!({
            "map_id": map_id,
            "prev_position": user_map.curr_position,
            "prev_capture": user_map.curr_capture,
            "curr_position": new_position,
            "curr_capture": new_capture,
            "rewards": rewards,
            "user_id": user_id
        }))
    }

    /// Load map data from JSON files (placeholder)
    async fn load_map_data(&self, map_id: &str) -> Result<WorldMapData, ArcError> {
        // This would load from JSON files like Python version
        // For now return basic structure
        Ok(WorldMapData {
            map_id: map_id.to_string(),
            is_beyond: false,
            beyond_health: None,
            steps: vec![],
            step_count: 0,
        })
    }

    /// Calculate climb progress
    async fn calculate_climb_progress(
        &self,
        user_map: &UserMapEntry,
        map_info: &WorldMapData,
        step_value: f64,
    ) -> Result<(i32, i32), ArcError> {
        // Implement climbing logic based on whether it's a Beyond map or regular map
        if map_info.is_beyond {
            // Beyond map logic
            let beyond_health = map_info.beyond_health.unwrap_or(100);
            let dt = beyond_health - user_map.curr_capture;
            let new_capture = if dt >= step_value as i32 {
                user_map.curr_capture + step_value as i32
            } else {
                beyond_health
            };

            // Calculate position based on capture and steps
            let mut position = 0;
            let mut remaining_capture = new_capture;

            for (i, step) in map_info.steps.iter().enumerate() {
                if remaining_capture <= 0 {
                    break;
                }
                if step.capture <= remaining_capture {
                    remaining_capture -= step.capture;
                    position = i as i32 + 1;
                } else {
                    break;
                }
            }

            if position >= map_info.step_count {
                position = map_info.step_count - 1;
            }

            Ok((position, new_capture))
        } else {
            // Regular map logic
            let mut position = user_map.curr_position;
            let mut capture = user_map.curr_capture;
            let mut remaining_step = step_value;

            while remaining_step > 0.0 && position < map_info.step_count {
                let step_index = position as usize;
                if step_index >= map_info.steps.len() {
                    break;
                }

                let step = &map_info.steps[step_index];
                let needed_capture = step.capture - capture;

                if needed_capture <= remaining_step as i32 {
                    remaining_step -= needed_capture as f64;
                    capture = 0;
                    position += 1;
                } else {
                    capture += remaining_step as i32;
                    remaining_step = 0.0;
                }
            }

            if position >= map_info.step_count {
                position = map_info.step_count - 1;
                capture = 0;
            }

            Ok((position, capture))
        }
    }

    /// Get rewards for climbed steps
    async fn get_step_rewards(
        &self,
        map_info: &WorldMapData,
        prev_position: i32,
        new_position: i32,
    ) -> Result<Vec<StepReward>, ArcError> {
        let mut rewards = Vec::new();

        for pos in (prev_position + 1)..=(new_position) {
            let step_index = pos as usize;
            if step_index < map_info.steps.len() {
                let step = &map_info.steps[step_index];
                if !step.items.is_empty() {
                    rewards.push(StepReward {
                        position: pos,
                        items: step.items.clone(),
                    });
                }
            }
        }

        Ok(rewards)
    }

    /// Grant step rewards to user
    async fn grant_step_rewards(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        user_id: i32,
        reward: &StepReward,
    ) -> Result<(), ArcError> {
        for item in &reward.items {
            sqlx::query!(
                r#"
                INSERT INTO user_item (user_id, item_id, type, amount)
                VALUES (?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE amount = amount + VALUES(amount)
                "#,
                user_id,
                item.item_id,
                item.item_type,
                item.amount
            )
            .execute(&mut **tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to grant step reward: {}", e),
            })?;
        }

        Ok(())
    }
}

/// User map entry from database
#[derive(Debug, Clone)]
struct UserMapEntry {
    pub map_id: String,
    pub curr_position: i32,
    pub curr_capture: i32,
    pub is_locked: bool,
}

/// World map data structure
#[derive(Debug, Clone)]
struct WorldMapData {
    pub map_id: String,
    pub is_beyond: bool,
    pub beyond_health: Option<i32>,
    pub steps: Vec<WorldStepData>,
    pub step_count: i32,
}

/// World step data structure
#[derive(Debug, Clone)]
struct WorldStepData {
    pub position: i32,
    pub capture: i32,
    pub items: Vec<StepItem>,
}

/// Step item structure
#[derive(Debug, Clone, serde::Serialize)]
struct StepItem {
    pub item_id: String,
    pub item_type: String,
    pub amount: i32,
}

/// Step reward structure
#[derive(Debug, Clone, serde::Serialize)]
struct StepReward {
    pub position: i32,
    pub items: Vec<StepItem>,
}
