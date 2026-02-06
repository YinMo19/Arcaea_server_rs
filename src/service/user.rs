use crate::config::{Constants, CONFIG};
use crate::error::{ArcError, ArcResult};
use crate::model::user::UserRecentScore;
use crate::model::{
    Item, Stamina, UpdateCharacter, User, UserAuth, UserCodeMapping, UserCredentials, UserExists,
    UserInfo, UserLoginDevice, UserLoginDto, UserRegisterDto,
};
use crate::service::{CharacterService, ItemService};
use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{MySql, Pool};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// User service for handling user operations
pub struct UserService {
    pool: Pool<MySql>,
    character_service: CharacterService,
    item_service: ItemService,
}

impl UserService {
    /// Create a new user service instance
    pub fn new(pool: Pool<MySql>) -> Self {
        let character_service = CharacterService::new(pool.clone());
        let item_service = ItemService::new(pool.clone());
        Self {
            pool,
            character_service,
            item_service,
        }
    }

    /// Get current timestamp in milliseconds
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Hash password using SHA-256
    fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate access token using SHA-256 and random data
    fn generate_token(user_id: i32, timestamp: i64) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{user_id}{timestamp}").as_bytes());
        hasher.update(rand::thread_rng().gen::<[u8; 8]>());
        general_purpose::STANDARD.encode(hasher.finalize())
    }

    /// Validate username format and uniqueness
    async fn validate_username(&self, name: &str) -> ArcResult<()> {
        if name.len() < 3 || name.len() > 16 {
            return Err(ArcError::input("Username is invalid."));
        }

        let exists = sqlx::query_as!(
            UserExists,
            "SELECT EXISTS(SELECT 1 FROM user WHERE name = ?) as `exists`",
            name
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists != 0 {
            return Err(ArcError::data_exist("Username exists.", 101, -210));
        }

        Ok(())
    }

    /// Validate password format
    fn validate_password(password: &str) -> ArcResult<()> {
        if password.len() < 8 || password.len() > 32 {
            return Err(ArcError::input("Password is invalid."));
        }
        Ok(())
    }

    /// Validate email format and uniqueness
    async fn validate_email(&self, email: &str) -> ArcResult<()> {
        if email.len() < 4 || email.len() > 64 || !email.contains('@') || !email.contains('.') {
            return Err(ArcError::input("Email address is invalid."));
        }

        let exists = sqlx::query_as!(
            UserExists,
            "SELECT EXISTS(SELECT 1 FROM user WHERE email = ?) as `exists`",
            email
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists != 0 {
            return Err(ArcError::data_exist("Email address exists.", 102, -211));
        }

        Ok(())
    }

    /// Validate user code format and uniqueness
    async fn validate_user_code(&self, user_code: &str) -> ArcResult<()> {
        if user_code.len() != 9 || !user_code.chars().all(|c| c.is_ascii_digit()) {
            return Err(ArcError::input("User code is invalid."));
        }

        let exists = sqlx::query_as!(
            UserExists,
            "SELECT EXISTS(SELECT 1 FROM user WHERE user_code = ?) as `exists`",
            user_code
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists != 0 {
            return Err(ArcError::data_exist("User code exists.", 103, -212));
        }

        Ok(())
    }

    /// Generate a unique 9-digit user code
    async fn generate_user_code(&self) -> ArcResult<String> {
        for _ in 0..1000 {
            let user_code: String = (0..9)
                .map(|_| rand::thread_rng().gen_range(0..10).to_string())
                .collect();

            let exists = sqlx::query_as!(
                UserExists,
                "SELECT EXISTS(SELECT 1 FROM user WHERE user_code = ?) as `exists`",
                user_code
            )
            .fetch_one(&self.pool)
            .await?;

            if exists.exists == 0 {
                return Ok(user_code);
            }
        }

        Err(ArcError::Base {
            message: "No available user code.".to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 500,
        })
    }

    /// Generate a new user ID
    async fn generate_user_id(&self) -> ArcResult<i32> {
        let result = sqlx::query!("SELECT MAX(user_id) as max_id FROM user")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.max_id.unwrap_or(2000000) + 1)
    }

    /// Insert initial characters for a new user
    async fn insert_initial_characters(&self, user_id: i32) -> ArcResult<()> {
        // Insert initial characters (0 and 1)
        sqlx::query!(
            "INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag) VALUES (?, ?, ?, ?, ?, ?, ?)",
            user_id, 0, 1, 0.0, 0, 0, 0
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!(
            "INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag) VALUES (?, ?, ?, ?, ?, ?, ?)",
            user_id, 1, 1, 0.0, 0, 0, 0
        )
        .execute(&self.pool)
        .await?;

        // Insert all characters into user_char_full
        let characters = sqlx::query_as!(
            UpdateCharacter,
            "SELECT character_id, max_level, is_uncapped FROM `character`"
        )
        .fetch_all(&self.pool)
        .await?;

        for character in characters {
            let exp = if character.max_level.unwrap_or(20) == 30 {
                25000.0
            } else {
                10000.0
            };

            sqlx::query!(
                "INSERT INTO user_char_full (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag) VALUES (?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE level = VALUES(level), exp = VALUES(exp), is_uncapped = VALUES(is_uncapped)",
                user_id,
                character.character_id,
                character.max_level.unwrap_or(20),
                exp,
                character.is_uncapped.unwrap_or(0),
                0,
                0
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Register a new user
    ///
    /// Validates input data, generates user ID and code if needed,
    /// creates the user account and inserts initial character data.
    pub async fn register_user(
        &self,
        user_data: UserRegisterDto,
        _device_id: Option<String>,
        _ip: Option<String>,
    ) -> ArcResult<UserAuth> {
        // TODO: Implement rate limiting for IP and device
        // if let Some(ip) = &ip {
        //     self.check_ip_rate_limit(ip).await?;
        // }

        // if let Some(device_id) = &device_id {
        //     self.check_device_rate_limit(device_id).await?;
        // }
        // Validate input data
        self.validate_username(&user_data.name).await?;
        Self::validate_password(&user_data.password)?;
        self.validate_email(&user_data.email).await?;

        let user_code = self.generate_user_code().await?;

        let user_id = self.generate_user_id().await?;
        let join_date = Self::current_timestamp();
        let hashed_password = Self::hash_password(&user_data.password);

        // Insert user
        sqlx::query!(
            r#"INSERT INTO user (
                user_id, name, password, join_date, user_code, rating_ptt,
                character_id, is_skill_sealed, is_char_uncapped, is_char_uncapped_override,
                is_hide_rating, favorite_character, max_stamina_notification_enabled,
                current_map, ticket, prog_boost, email
            ) VALUES (?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, -1, 0, '', ?, 0, ?)"#,
            user_id,
            user_data.name,
            hashed_password,
            join_date,
            user_code,
            CONFIG.default_memories,
            user_data.email
        )
        .execute(&self.pool)
        .await?;

        // Insert initial characters
        self.insert_initial_characters(user_id).await?;

        // Generate token for immediate login
        let token = Self::generate_token(user_id, join_date);

        Ok(UserAuth { user_id, token })
    }

    /// Check device login limits and manage existing sessions
    async fn check_device_limits(&self, user_id: i32, device_id: &str) -> ArcResult<()> {
        let current_time = Self::current_timestamp();

        // Get existing login devices
        let devices = sqlx::query_as!(
            UserLoginDevice,
            "SELECT login_device FROM login WHERE user_id = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let device_list: Vec<String> = devices
            .into_iter()
            .map(|d| d.login_device.unwrap_or_default())
            .collect();

        let mut should_delete_num = device_list.len() as i32 + 1 - CONFIG.login_device_number_limit;

        if !CONFIG.allow_login_same_device && device_list.contains(&device_id.to_string()) {
            // Delete existing sessions for the same device
            sqlx::query!(
                "DELETE FROM login WHERE login_device = ? AND user_id = ?",
                device_id,
                user_id
            )
            .execute(&self.pool)
            .await?;

            should_delete_num = device_list.len() as i32 + 1
                - device_list.iter().filter(|&d| d == device_id).count() as i32
                - CONFIG.login_device_number_limit;
        }

        if should_delete_num >= 1 {
            if !CONFIG.allow_login_same_device && CONFIG.allow_ban_multidevice_user_auto {
                // Check for auto-ban condition
                let login_count = sqlx::query!(
                    "SELECT COUNT(*) as count FROM login WHERE user_id = ? AND login_time > ?",
                    user_id,
                    current_time - 86400000
                )
                .fetch_one(&self.pool)
                .await?;

                if login_count.count >= CONFIG.login_device_number_limit as i64 {
                    let remaining_ts = self.auto_ban_user(user_id, current_time).await?;
                    let mut extra_data = HashMap::new();
                    extra_data.insert(
                        "remaining_ts".to_string(),
                        Value::Number(serde_json::Number::from(remaining_ts)),
                    );
                    return Err(ArcError::user_ban(
                        "Too many devices logging in during 24 hours.",
                        105,
                        Some(extra_data),
                    ));
                }
            }

            // Delete excess tokens (MariaDB compatible approach)
            sqlx::query!(
                "DELETE FROM login WHERE user_id = ? ORDER BY login_time ASC LIMIT ?",
                user_id,
                should_delete_num
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Apply auto-ban to user for multi-device violation
    async fn auto_ban_user(&self, user_id: i32, current_time: i64) -> ArcResult<i64> {
        // Delete all login sessions
        sqlx::query!("DELETE FROM login WHERE user_id = ?", user_id)
            .execute(&self.pool)
            .await?;

        // Get current ban flag
        let user = sqlx::query!("SELECT ban_flag FROM user WHERE user_id = ?", user_id)
            .fetch_one(&self.pool)
            .await?;

        let ban_time = if let Some(ban_flag) = user.ban_flag {
            if !ban_flag.is_empty() {
                if let Some(last_ban_time_str) = ban_flag.split(':').next() {
                    if let Ok(last_ban_time) = last_ban_time_str.parse::<i32>() {
                        let mut i = 0;
                        while i < Constants::BAN_TIME.len() - 1
                            && Constants::BAN_TIME[i] <= last_ban_time
                        {
                            i += 1;
                        }
                        Constants::BAN_TIME[i]
                    } else {
                        Constants::BAN_TIME[0]
                    }
                } else {
                    Constants::BAN_TIME[0]
                }
            } else {
                Constants::BAN_TIME[0]
            }
        } else {
            Constants::BAN_TIME[0]
        };

        let ban_end_time = current_time + (ban_time as i64 * 86400000);
        let ban_flag = format!("{ban_time}:{ban_end_time}");

        sqlx::query!(
            "UPDATE user SET ban_flag = ? WHERE user_id = ?",
            ban_flag,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(ban_end_time - current_time)
    }

    /// Login user with credentials
    ///
    /// Validates username and password, checks for bans,
    /// manages device sessions and generates access token.
    pub async fn login_user(
        &self,
        login_data: UserLoginDto,
        ip: Option<&str>,
    ) -> ArcResult<UserAuth> {
        // TODO: Implement rate limiting
        // self.check_login_rate_limit(&login_data.name).await?;

        let current_time = Self::current_timestamp();

        // Get user credentials
        let user = sqlx::query_as!(
            UserCredentials,
            "SELECT user_id, password, ban_flag FROM user WHERE name = ?",
            login_data.name
        )
        .fetch_optional(&self.pool)
        .await?;

        let user = user.ok_or_else(|| {
            ArcError::no_data(
                format!("Username `{}` does not exist.", login_data.name),
                104,
            )
        })?;

        // Check for ban
        if let Some(ban_flag) = &user.ban_flag {
            if !ban_flag.is_empty() {
                if let Some(ban_timestamp_str) = ban_flag.split(':').nth(1) {
                    if let Ok(ban_timestamp) = ban_timestamp_str.parse::<i64>() {
                        if ban_timestamp > current_time {
                            let mut extra_data = HashMap::new();
                            extra_data.insert(
                                "remaining_ts".to_string(),
                                Value::Number(serde_json::Number::from(
                                    ban_timestamp - current_time,
                                )),
                            );
                            return Err(ArcError::user_ban(
                                format!(
                                    "Too many devices user `{}` logging in during 24 hours.",
                                    user.user_id
                                ),
                                105,
                                Some(extra_data),
                            ));
                        }
                    }
                }
            }
        }

        // Check for account ban (empty password)
        let password = user.password.as_ref().ok_or_else(|| {
            ArcError::user_ban(
                format!("The account `{}` has been banned.", user.user_id),
                106,
                None,
            )
        })?;

        if password.is_empty() {
            return Err(ArcError::user_ban(
                format!("The account `{}` has been banned.", user.user_id),
                106,
                None,
            ));
        }

        // Verify password
        let hashed_input = Self::hash_password(&login_data.password);
        if password != &hashed_input {
            return Err(ArcError::no_access(
                format!("Wrong password of user `{}`", user.user_id),
                104,
            ));
        }

        // Generate token
        let token = Self::generate_token(user.user_id, current_time);

        // Check device limits
        if let Some(device_id) = &login_data.device_id {
            self.check_device_limits(user.user_id, device_id).await?;
        }

        // Insert login record
        sqlx::query!(
            "INSERT INTO login (access_token, user_id, login_time, login_ip, login_device) VALUES (?, ?, ?, ?, ?)",
            token,
            user.user_id,
            current_time,
            ip,
            login_data.device_id
        )
        .execute(&self.pool)
        .await?;

        Ok(UserAuth {
            user_id: user.user_id,
            token,
        })
    }

    /// Get user ID from access token
    ///
    /// Validates the access token and returns the associated user ID.
    pub async fn authenticate_token(&self, token: &str) -> ArcResult<i32> {
        log::info!("Authenticating token: {token}");
        let result = sqlx::query_as!(
            UserCodeMapping,
            "SELECT user_id FROM login WHERE access_token = ?",
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        result
            .map(|r| r.user_id)
            .ok_or_else(|| ArcError::no_access("Wrong token.", -4))
    }

    /// Get user information by user ID
    ///
    /// Retrieves complete user information for API responses.
    pub async fn get_user_info(&self, user_id: i32) -> ArcResult<UserInfo> {
        let user = sqlx::query_as!(User, "SELECT user_id, name, password, join_date, user_code, rating_ptt, character_id, is_skill_sealed, is_char_uncapped, is_char_uncapped_override, is_hide_rating, song_id, difficulty, score, shiny_perfect_count, perfect_count, near_count, miss_count, health, modifier, time_played, clear_type, rating, favorite_character, max_stamina_notification_enabled, current_map, ticket, prog_boost, email, world_rank_score, ban_flag, next_fragstam_ts, max_stamina_ts, stamina, world_mode_locked_end_ts, beyond_boost_gauge, kanae_stored_prog, mp_notification_enabled, highest_rating_ptt, insight_state FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&self.pool)
            .await?;

        let user = user.ok_or_else(|| ArcError::no_data("User not found.", 401))?;

        // Load additional user data
        let mut user_info = UserInfo::from(user);

        // Load character stats from character service
        user_info.character_stats = self
            .character_service
            .get_user_character_stats(user_id)
            .await?;
        user_info.characters = self
            .character_service
            .get_user_character_ids(user_id)
            .await?;

        // Load user cores
        user_info.cores = self.get_user_cores(user_id).await?;

        // Load user packs, singles, and world songs
        user_info.packs = self.get_user_packs(user_id).await?;
        user_info.singles = self.get_user_singles(user_id).await?;
        user_info.world_songs = self.get_user_world_songs(user_id).await?;
        user_info.world_unlocks = self.get_user_world_unlocks(user_id).await?;

        user_info.stamina = self.get_user_stamina(user_id).await?;

        // Load recent score
        user_info.recent_score = self.get_user_recent_scores(user_id).await?;

        Ok(user_info)
    }

    /// Get user ID from user code
    ///
    /// Converts a 9-digit user code to the corresponding user ID.
    pub async fn get_user_id_by_code(&self, user_code: &str) -> ArcResult<i32> {
        let result = sqlx::query_as!(
            UserCodeMapping,
            "SELECT user_id FROM user WHERE user_code = ?",
            user_code
        )
        .fetch_optional(&self.pool)
        .await?;

        result
            .map(|r| r.user_id)
            .ok_or_else(|| ArcError::no_data("No user.", 401))
    }

    /// Update a single column for a user
    ///
    /// Updates one specific field in the user table.
    pub async fn update_user_column(
        &self,
        user_id: i32,
        column: &str,
        value: &str,
    ) -> ArcResult<()> {
        // Note: This is potentially unsafe due to SQL injection risk
        // In a real implementation, you should validate the column name
        // and use a match statement or enum for allowed columns
        let query = format!("UPDATE user SET {column} = ? WHERE user_id = ?");
        sqlx::query(&query)
            .bind(value)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update user's insight state
    pub async fn update_user_insight_state(
        &self,
        user_id: i32,
        insight_state: i32,
    ) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE user SET insight_state = ? WHERE user_id = ?",
            insight_state,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update user's character ID
    pub async fn update_user_character(&self, user_id: i32, character_id: i32) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE user SET character_id = ? WHERE user_id = ?",
            character_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update user's skill sealed status
    pub async fn update_user_skill_sealed(&self, user_id: i32, is_sealed: bool) -> ArcResult<()> {
        let is_sealed_val = if is_sealed { 1 } else { 0 };
        sqlx::query!(
            "UPDATE user SET is_skill_sealed = ? WHERE user_id = ?",
            is_sealed_val,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Toggle user's invasion/insight state
    ///
    /// Cycles through the insight state values according to the game logic.
    pub async fn toggle_invasion(&self, user_id: i32) -> ArcResult<UserInfo> {
        // Get current insight state
        let current_state =
            sqlx::query!("SELECT insight_state FROM user WHERE user_id = ?", user_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| ArcError::no_data("No user.", 108))?;

        // Toggle insight state (4 -> 0 -> 1 -> 2 -> 3 -> 4)
        let insight_toggle_states = [4, 0, 1, 2, 3];
        let current_index = insight_toggle_states
            .iter()
            .position(|&x| x == current_state.insight_state.unwrap_or(4))
            .unwrap_or(0);
        let new_state = insight_toggle_states[(current_index + 1) % insight_toggle_states.len()];

        sqlx::query!(
            "UPDATE user SET insight_state = ? WHERE user_id = ?",
            new_state,
            user_id
        )
        .execute(&self.pool)
        .await?;

        self.get_user_info(user_id).await
    }

    /// Change user's character and skill sealed state
    ///
    /// Updates the user's current character and whether skills are sealed.
    pub async fn change_character(
        &self,
        user_id: i32,
        character_id: i32,
        is_skill_sealed: bool,
    ) -> ArcResult<()> {
        // Get character uncap status
        let char_info = sqlx::query!(
            "SELECT is_uncapped, is_uncapped_override FROM user_char WHERE user_id = ? AND character_id = ?",
            user_id,
            character_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let (is_uncapped, is_uncapped_override) = if let Some(info) = char_info {
            (
                info.is_uncapped.unwrap_or(0),
                info.is_uncapped_override.unwrap_or(0),
            )
        } else {
            (0, 0) // Default values if character not found
        };

        let skill_sealed_val = if is_skill_sealed { 1 } else { 0 };

        sqlx::query!(
            "UPDATE user SET character_id = ?, is_skill_sealed = ?, is_char_uncapped = ?, is_char_uncapped_override = ? WHERE user_id = ?",
            character_id,
            skill_sealed_val,
            is_uncapped,
            is_uncapped_override,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Toggle character uncap override
    ///
    /// Toggles the uncap override state for a specific character.
    pub async fn toggle_character_uncap_override(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<serde_json::Value> {
        let character_info = self
            .character_service
            .toggle_character_uncap_override(user_id, character_id)
            .await?;

        Ok(serde_json::to_value(character_info.to_dict())?)
    }

    /// Perform character uncap
    ///
    /// Uncaps a character using required fragments/cores.
    pub async fn character_uncap(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<(serde_json::Value, serde_json::Value)> {
        let character_info = self
            .character_service
            .character_uncap(user_id, character_id)
            .await?;

        // Get user cores after uncap
        let cores = self.get_user_cores_json(user_id).await?;

        Ok((
            serde_json::to_value(character_info.to_dict())?,
            serde_json::json!(cores),
        ))
    }

    /// Upgrade character using cores
    ///
    /// Uses ether drops (core_generic) to upgrade character experience.
    pub async fn upgrade_character_by_core(
        &self,
        user_id: i32,
        character_id: i32,
        amount: i32,
    ) -> ArcResult<(serde_json::Value, serde_json::Value)> {
        let character_info = self
            .character_service
            .upgrade_character_by_core(user_id, character_id, -amount)
            .await?;

        // Get user cores after upgrade
        let cores = self.get_user_cores_json(user_id).await?;

        Ok((
            serde_json::to_value(character_info.to_dict())?,
            serde_json::json!(cores),
        ))
    }

    /// Get user's cloud save data
    ///
    /// Retrieves all cloud save data for the user.
    pub async fn get_user_save_data(&self, user_id: i32) -> ArcResult<serde_json::Value> {
        let save_data = sqlx::query!("SELECT * FROM user_save WHERE user_id = ?", user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(data) = save_data {
            let response = serde_json::json!({
                "user_id": user_id,
                "story": {
                    "": serde_json::from_str::<serde_json::Value>(&data.story_data.unwrap_or_default()).unwrap_or(serde_json::json!([]))
                },
                "devicemodelname": {
                    "val": serde_json::from_str::<serde_json::Value>(&data.devicemodelname_data.unwrap_or_default()).unwrap_or(serde_json::json!(""))
                },
                "installid": {
                    "val": serde_json::from_str::<serde_json::Value>(&data.installid_data.unwrap_or_default()).unwrap_or(serde_json::json!(""))
                },
                "unlocklist": {
                    "": serde_json::from_str::<serde_json::Value>(&data.unlocklist_data.unwrap_or_default()).unwrap_or(serde_json::json!([]))
                },
                "clearedsongs": {
                    "": serde_json::from_str::<serde_json::Value>(&data.clearedsongs_data.unwrap_or_default()).unwrap_or(serde_json::json!([]))
                },
                "clearlamps": {
                    "": serde_json::from_str::<serde_json::Value>(&data.clearlamps_data.unwrap_or_default()).unwrap_or(serde_json::json!([]))
                },
                "scores": {
                    "": serde_json::from_str::<serde_json::Value>(&data.scores_data.unwrap_or_default()).unwrap_or(serde_json::json!([]))
                },
                "version": {
                    "val": 1
                },
                "createdAt": data.createdAt.unwrap_or(0),
                "finalestate": {
                    "val": data.finalestate_data.unwrap_or_default()
                }
            });
            Ok(response)
        } else {
            Err(ArcError::no_data("User has no cloud save data", 108))
        }
    }

    /// Get user cores information
    async fn get_user_cores(&self, user_id: i32) -> ArcResult<Vec<Item>> {
        let cores = sqlx::query!(
            r#"
            SELECT item_id, amount
            FROM user_item
            WHERE user_id = ? AND type = 'core'
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        cores
            .into_iter()
            .map(|core| {
                let amount = core.amount.unwrap_or(1);
                self.item_service.create_item_from_dict(&HashMap::from([
                    ("item_id", Value::from(core.item_id)),
                    ("amount", Value::from(amount)),
                    ("item_type", Value::from("core")),
                ]))
            })
            .collect::<ArcResult<Vec<Item>>>()
    }

    /// Get user pack unlocks
    async fn get_user_packs(&self, user_id: i32) -> ArcResult<Vec<String>> {
        let packs = sqlx::query_scalar!(
            "SELECT item_id FROM user_item WHERE user_id = ? AND type = 'pack'",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(packs)
    }

    /// Get user single song unlocks
    async fn get_user_singles(&self, user_id: i32) -> ArcResult<Vec<String>> {
        let singles = sqlx::query_scalar!(
            "SELECT item_id FROM user_item WHERE user_id = ? AND type = 'single'",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(singles)
    }

    /// Get user world song unlocks
    async fn get_user_world_songs(&self, user_id: i32) -> ArcResult<Vec<String>> {
        let world_songs = if CONFIG.world_song_full_unlock {
            sqlx::query_scalar!("SELECT item_id FROM item WHERE type = 'world_song'")
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_scalar!(
                "SELECT item_id FROM user_item WHERE user_id = ? AND type = 'world_song'",
                user_id
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(world_songs)
    }

    /// Get user world unlocks
    async fn get_user_world_unlocks(&self, user_id: i32) -> ArcResult<Vec<String>> {
        let world_unlocks = if CONFIG.world_scenery_full_unlock {
            sqlx::query_scalar!("SELECT item_id FROM item WHERE type = 'world_unlock'",)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_scalar!(
                "SELECT item_id FROM user_item WHERE user_id = ? AND type = 'world_unlock'",
                user_id
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(world_unlocks)
    }

    /// Get user recent scores
    async fn get_user_recent_scores(&self, user_id: i32) -> ArcResult<Vec<UserRecentScore>> {
        let user = sqlx::query!(
            r#"
            SELECT song_id, difficulty, score, shiny_perfect_count, perfect_count,
                   near_count, miss_count, health, modifier, time_played, clear_type, rating
            FROM user
            WHERE user_id = ? AND song_id IS NOT NULL
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = user {
            if let Some(song_id) = record.song_id {
                let recent_score = UserRecentScore {
                    song_id,
                    difficulty: record.difficulty.unwrap_or(0),
                    score: record.score.unwrap_or(0),
                    shiny_perfect_count: record.shiny_perfect_count.unwrap_or(0),
                    perfect_count: record.perfect_count.unwrap_or(0),
                    near_count: record.near_count.unwrap_or(0),
                    miss_count: record.miss_count.unwrap_or(0),
                    health: record.health.unwrap_or(100),
                    modifier: record.modifier.unwrap_or(0),
                    time_played: record.time_played.unwrap_or(0),
                    clear_type: record.clear_type.unwrap_or(0),
                    rating: record.rating.unwrap_or(0.0),
                };
                return Ok(vec![recent_score]);
            }
        }

        Ok(Vec::new())
    }

    /// Update user's cloud save data
    ///
    /// Updates the user's cloud save data with new values.
    pub async fn update_user_save_data(
        &self,
        user_id: i32,
        save_request: &crate::route::user::CloudSaveRequest,
    ) -> ArcResult<()> {
        // TODO: Implement checksum validation like in Python version
        // For now, just update the data

        let current_time = Self::current_timestamp();

        sqlx::query!(
            "INSERT INTO user_save (user_id, scores_data, clearlamps_data, clearedsongs_data, unlocklist_data, installid_data, devicemodelname_data, story_data, createdAt, finalestate_data)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
             scores_data = VALUES(scores_data),
             clearlamps_data = VALUES(clearlamps_data),
             clearedsongs_data = VALUES(clearedsongs_data),
             unlocklist_data = VALUES(unlocklist_data),
             installid_data = VALUES(installid_data),
             devicemodelname_data = VALUES(devicemodelname_data),
             story_data = VALUES(story_data),
             createdAt = VALUES(createdAt),
             finalestate_data = VALUES(finalestate_data)",
            user_id,
            save_request.scores_data,
            save_request.clearlamps_data,
            save_request.clearedsongs_data,
            save_request.unlocklist_data,
            save_request.installid_data,
            save_request.devicemodelname_data,
            save_request.story_data,
            current_time,
            save_request.finalestate_data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update user setting
    ///
    /// Updates a specific user setting based on the setting argument.
    pub async fn update_user_setting(
        &self,
        user_id: i32,
        set_arg: &str,
        value: &str,
    ) -> ArcResult<UserInfo> {
        match set_arg {
            "favorite_character" => {
                let character_id: i32 = value
                    .parse()
                    .map_err(|_| ArcError::input("Invalid character ID"))?;
                sqlx::query!(
                    "UPDATE user SET favorite_character = ? WHERE user_id = ?",
                    character_id,
                    user_id
                )
                .execute(&self.pool)
                .await?;
            }
            "is_hide_rating" | "max_stamina_notification_enabled" | "mp_notification_enabled" => {
                let bool_value = value == "true";
                let int_value = if bool_value { 1 } else { 0 };

                match set_arg {
                    "is_hide_rating" => {
                        sqlx::query!(
                            "UPDATE user SET is_hide_rating = ? WHERE user_id = ?",
                            int_value,
                            user_id
                        )
                        .execute(&self.pool)
                        .await?;
                    }
                    "max_stamina_notification_enabled" => {
                        sqlx::query!(
                            "UPDATE user SET max_stamina_notification_enabled = ? WHERE user_id = ?",
                            int_value,
                            user_id
                        )
                        .execute(&self.pool)
                        .await?;
                    }
                    "mp_notification_enabled" => {
                        sqlx::query!(
                            "UPDATE user SET mp_notification_enabled = ? WHERE user_id = ?",
                            int_value,
                            user_id
                        )
                        .execute(&self.pool)
                        .await?;
                    }
                    _ => {}
                }
            }
            _ => return Err(ArcError::input("Invalid setting argument")),
        }

        self.get_user_info(user_id).await
    }

    /// Delete user account
    ///
    /// Deletes a user account and all associated data.
    pub async fn delete_user_account(&self, user_id: i32) -> ArcResult<()> {
        // Check if account deletion is allowed based on config
        if !CONFIG.allow_self_account_delete {
            return Err(ArcError::no_data("Cannot delete the account.", 151));
        }

        // Start a transaction for atomic deletion
        let mut transaction = self.pool.begin().await?;

        // Delete from all related tables
        sqlx::query!("DELETE FROM login WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!(
            "DELETE FROM friend WHERE user_id_me = ? OR user_id_other = ?",
            user_id,
            user_id
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!("DELETE FROM best_score WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_char WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_char_full WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM recent30 WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_world WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_item WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_save WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_present WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_redeem WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_role WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_course WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_mission WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_kvdata WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM user_custom_course WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM download_token WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        // Finally delete from user table
        sqlx::query!("DELETE FROM user WHERE user_id = ?", user_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Get user cores as JSON (for API responses)
    ///
    /// Returns user's core inventory as JSON.
    async fn get_user_cores_json(&self, user_id: i32) -> ArcResult<Vec<Item>> {
        let user_cores = self.get_user_cores(user_id).await?;
        Ok(user_cores)
    }

    /// Update a single column for a user
    ///
    /// Updates a specific column in the user table with the provided value.
    pub async fn update_user_one_column<T>(
        &self,
        user_id: i32,
        column: &str,
        value: &T,
    ) -> ArcResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        let value_str = serde_json::to_string(value)?;
        let sql = format!("UPDATE user SET {column} = ? WHERE user_id = ?");

        sqlx::query(&sql)
            .bind(value_str)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add stamina to a user
    ///
    /// Increases the user's stamina by the specified amount.
    pub async fn add_stamina(&self, user_id: i32, amount: i32) -> ArcResult<()> {
        let current_stamina = sqlx::query!("SELECT stamina FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&self.pool)
            .await?;

        match current_stamina {
            Some(stamina_row) => {
                let new_stamina = stamina_row.stamina.unwrap_or(0) + amount;
                sqlx::query!(
                    "UPDATE user SET stamina = ? WHERE user_id = ?",
                    new_stamina,
                    user_id
                )
                .execute(&self.pool)
                .await?;
            }
            None => {
                return Err(ArcError::no_data(
                    "User not found for stamina update".to_string(),
                    108,
                ));
            }
        }

        Ok(())
    }

    /// get user's stamina
    async fn get_user_stamina(&self, user_id: i32) -> ArcResult<i32> {
        let stamina_info = sqlx::query!(
            "select max_stamina_ts, stamina from user where user_id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        let stamina = Stamina {
            stamina: stamina_info.stamina.unwrap_or(12),
            max_stamina_ts: stamina_info.max_stamina_ts.unwrap_or(0),
        };

        Ok(stamina.calculate_current_stamina(12, 1800000))
    }

    /// Add a friend to the user's friend list
    ///
    /// Creates a friendship relationship between the current user and the target user.
    pub async fn add_friend(&self, user_id: i32, friend_id: i32) -> ArcResult<()> {
        if user_id == friend_id {
            return Err(ArcError::friend("Add yourself as a friend.", 604, -1));
        }

        // Check if friendship already exists
        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM friend WHERE user_id_me = ? AND user_id_other = ?) as `exists`",
            user_id,
            friend_id
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists != 0 {
            return Err(ArcError::friend("The user has been your friend.", 602, -1));
        }

        // Add friend relationship
        sqlx::query!(
            "INSERT INTO friend (user_id_me, user_id_other) VALUES (?, ?)",
            user_id,
            friend_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a friend from the user's friend list
    ///
    /// Removes the friendship relationship between the current user and the target user.
    pub async fn delete_friend(&self, user_id: i32, friend_id: i32) -> ArcResult<()> {
        // Check if friendship exists
        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM friend WHERE user_id_me = ? AND user_id_other = ?) as `exists`",
            user_id,
            friend_id
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists == 0 {
            return Err(ArcError::friend(
                "No user or the user is not your friend.",
                401,
                -1,
            ));
        }

        // Remove friend relationship
        sqlx::query!(
            "DELETE FROM friend WHERE user_id_me = ? AND user_id_other = ?",
            user_id,
            friend_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user's global ranking
    ///
    /// Returns the user's position in the global ranking based on world_rank_score.
    /// Returns 0 if the user is not ranked or exceeds the maximum rank limit.
    pub async fn get_global_rank(&self, user_id: i32) -> ArcResult<i32> {
        // First get user's world_rank_score
        let user_score = sqlx::query!(
            "SELECT world_rank_score FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match user_score {
            Some(score_row) => {
                let world_rank_score = score_row.world_rank_score.unwrap_or(0);
                if world_rank_score == 0 {
                    return Ok(0);
                }

                // Count how many users have higher scores
                let rank_result = sqlx::query!(
                    "SELECT COUNT(*) as count FROM user WHERE world_rank_score > ?",
                    world_rank_score
                )
                .fetch_one(&self.pool)
                .await?;

                let rank = rank_result.count as i32 + 1;
                if rank <= CONFIG.world_rank_max {
                    Ok(rank)
                } else {
                    Ok(0)
                }
            }
            None => Ok(0),
        }
    }

    /// Update user's global ranking score
    ///
    /// Calculates and updates the user's world_rank_score based on their best scores
    /// across FTR, BYN, and ETR difficulties.
    pub async fn update_global_rank(&self, user_id: i32) -> ArcResult<()> {
        let score_result = sqlx::query!(
            r#"
            WITH user_scores AS (
                SELECT song_id, difficulty, score_v2
                FROM best_score
                WHERE user_id = ? AND difficulty IN (2, 3, 4)
            )
            SELECT SUM(a) as total_score FROM (
                SELECT SUM(score_v2) as a
                FROM user_scores
                WHERE difficulty = 2
                AND song_id IN (SELECT song_id FROM chart WHERE rating_ftr > 0)
                UNION
                SELECT SUM(score_v2) as a
                FROM user_scores
                WHERE difficulty = 3
                AND song_id IN (SELECT song_id FROM chart WHERE rating_byn > 0)
                UNION
                SELECT SUM(score_v2) as a
                FROM user_scores
                WHERE difficulty = 4
                AND song_id IN (SELECT song_id FROM chart WHERE rating_etr > 0)
            ) totals
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if let Some(total_score) = score_result.total_score {
            sqlx::query!(
                "UPDATE user SET world_rank_score = ? WHERE user_id = ?",
                total_score,
                user_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Update user world mode completion information
    ///
    /// Updates user's world mode completion data for skill calculations.
    pub async fn update_user_world_complete_info(&self, user_id: i32) -> ArcResult<()> {
        // Note: This requires world map parsing logic and user_kvdata table operations
        // For now, we'll implement a placeholder that can be extended later
        // TODO: Implement full world mode completion tracking

        // Get total step count for user
        let step_result = sqlx::query!(
            "SELECT CAST(COALESCE(SUM(curr_position), 0) + COUNT(*) AS SIGNED) as total_steps FROM user_world WHERE user_id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Store in user_kvdata table for fatalis skill
        sqlx::query!(
            r#"
                INSERT INTO user_kvdata (user_id, class, `key`, idx, value)
                VALUES (?, 'world', 'total_step_count', 0, ?)
                ON DUPLICATE KEY UPDATE value = VALUES(value)
                "#,
            user_id,
            step_result.total_steps
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Change user's favorite character
    ///
    /// Updates the user's favorite character setting.
    pub async fn change_favorite_character(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE user SET favorite_character = ? WHERE user_id = ?",
            character_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user's friend list with detailed information
    ///
    /// Returns a list of friends with their characters and recent scores.
    pub async fn get_user_friends(&self, user_id: i32) -> ArcResult<Vec<serde_json::Value>> {
        let friend_ids = sqlx::query!(
            "SELECT user_id_other FROM friend WHERE user_id_me = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut friends = Vec::new();

        for friend_row in friend_ids {
            let friend_id = friend_row.user_id_other;

            // Check if mutual friendship exists
            let is_mutual = sqlx::query!(
                "SELECT EXISTS(SELECT 1 FROM friend WHERE user_id_me = ? AND user_id_other = ?) as `exists`",
                friend_id,
                user_id
            )
            .fetch_one(&self.pool)
            .await?
            .exists != 0;

            // Get friend's basic info
            let friend_info = sqlx::query!(
                r#"
                SELECT name, user_id, rating_ptt, is_hide_rating, join_date,
                       character_id, is_skill_sealed, is_char_uncapped, is_char_uncapped_override,
                       favorite_character, song_id, difficulty, score, shiny_perfect_count,
                       perfect_count, near_count, miss_count, health, modifier, time_played, clear_type, rating
                FROM user WHERE user_id = ?
                "#,
                friend_id
            )
            .fetch_optional(&self.pool)
            .await?;

            if let Some(friend) = friend_info {
                let character_id = friend
                    .favorite_character
                    .unwrap_or(friend.character_id.unwrap_or(0));

                // Get best clear type for recent score if exists
                let best_clear_type = if let Some(ref song_id) = friend.song_id {
                    let best_clear = sqlx::query!(
                        "SELECT best_clear_type FROM best_score WHERE user_id = ? AND song_id = ? AND difficulty = ?",
                        friend_id,
                        song_id,
                        friend.difficulty
                    )
                    .fetch_optional(&self.pool)
                    .await?;
                    best_clear
                        .and_then(|bc| bc.best_clear_type)
                        .unwrap_or(friend.clear_type.unwrap_or(0))
                } else {
                    friend.clear_type.unwrap_or(0)
                };

                let recent_score = if friend.song_id.is_some() {
                    vec![serde_json::json!({
                        "song_id": friend.song_id,
                        "difficulty": friend.difficulty,
                        "score": friend.score,
                        "shiny_perfect_count": friend.shiny_perfect_count,
                        "perfect_count": friend.perfect_count,
                        "near_count": friend.near_count,
                        "miss_count": friend.miss_count,
                        "health": friend.health,
                        "modifier": friend.modifier,
                        "time_played": friend.time_played,
                        "clear_type": friend.clear_type,
                        "rating": friend.rating,
                        "best_clear_type": best_clear_type
                    })]
                } else {
                    Vec::new()
                };

                let friend_json = serde_json::json!({
                    "is_mutual": is_mutual,
                    "is_char_uncapped_override": friend.is_char_uncapped_override.unwrap_or(0) != 0,
                    "is_char_uncapped": friend.is_char_uncapped.unwrap_or(0) != 0,
                    "is_skill_sealed": friend.is_skill_sealed.unwrap_or(0) != 0,
                    "rating": if friend.is_hide_rating.unwrap_or(0) != 0 { -1 } else { friend.rating_ptt.unwrap_or(0) },
                    "join_date": friend.join_date,
                    "character": character_id,
                    "recent_score": recent_score,
                    "name": friend.name,
                    "user_id": friend.user_id
                });

                friends.push(friend_json);
            }
        }

        // Sort by recent score time_played (most recent first)
        friends.sort_by(|a, b| {
            let time_a = a["recent_score"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|score| score["time_played"].as_i64())
                .unwrap_or(0);
            let time_b = b["recent_score"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|score| score["time_played"].as_i64())
                .unwrap_or(0);
            time_b.cmp(&time_a)
        });

        Ok(friends)
    }
}
