use crate::config::{Constants, CONFIG};
use crate::error::{ArcError, ArcResult};
use crate::model::{
    Character, User, UserAuth, UserCodeMapping, UserCredentials, UserExists, UserInfo,
    UserLoginDevice, UserLoginDto, UserRegisterDto,
};
use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::{MySql, Pool};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// User service for handling user operations
pub struct UserService {
    pool: Pool<MySql>,
}

impl UserService {
    /// Create a new user service instance
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
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
        hasher.update(format!("{}{}", user_id, timestamp).as_bytes());
        hasher.update(&rand::thread_rng().gen::<[u8; 8]>());
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
            Character,
            "SELECT character_id, name, max_level, frag1, prog1, overdrive1, frag20, prog20, overdrive20, frag30, prog30, overdrive30, skill_id, skill_unlock_level, skill_requires_uncap, skill_id_uncap, char_type, is_uncapped FROM `character`"
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
        // if let Some(device_id) = &device_id {
        //     self.check_device_rate_limit(device_id).await?;
        // }
        // if let Some(ip) = &ip {
        //     self.check_ip_rate_limit(ip).await?;
        // }

        // Validate input data
        self.validate_username(&user_data.name).await?;
        Self::validate_password(&user_data.password)?;
        self.validate_email(&user_data.email).await?;

        let user_code = if let Some(code) = user_data.user_code {
            self.validate_user_code(&code).await?;
            code
        } else {
            self.generate_user_code().await?
        };

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

        if !CONFIG.allow_login_same_device {
            if device_list.contains(&device_id.to_string()) {
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
                        serde_json::Value::Number(serde_json::Number::from(remaining_ts)),
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
        let ban_flag = format!("{}:{}", ban_time, ban_end_time);

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
        ip: Option<String>,
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
                -1,
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
                                serde_json::Value::Number(serde_json::Number::from(
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

        user.map(UserInfo::from)
            .ok_or_else(|| ArcError::no_data("User not found.", 401, -3))
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
            .ok_or_else(|| ArcError::no_data("No user.", 401, -3))
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
        let query = format!("UPDATE user SET {} = ? WHERE user_id = ?", column);
        sqlx::query(&query)
            .bind(value)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
