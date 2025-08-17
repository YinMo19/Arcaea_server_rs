use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User database model representing the user table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub user_id: i32,
    pub name: Option<String>,
    pub password: Option<String>,
    pub join_date: Option<i64>,
    pub user_code: Option<String>,
    pub rating_ptt: Option<i32>,
    pub character_id: Option<i32>,
    pub is_skill_sealed: Option<i8>,
    pub is_char_uncapped: Option<i8>,
    pub is_char_uncapped_override: Option<i8>,
    pub is_hide_rating: Option<i8>,
    pub song_id: Option<String>,
    pub difficulty: Option<i32>,
    pub score: Option<i32>,
    pub shiny_perfect_count: Option<i32>,
    pub perfect_count: Option<i32>,
    pub near_count: Option<i32>,
    pub miss_count: Option<i32>,
    pub health: Option<i32>,
    pub modifier: Option<i32>,
    pub time_played: Option<i64>,
    pub clear_type: Option<i32>,
    pub rating: Option<f64>,
    pub favorite_character: Option<i32>,
    pub max_stamina_notification_enabled: Option<i8>,
    pub current_map: Option<String>,
    pub ticket: Option<i32>,
    pub prog_boost: Option<i32>,
    pub email: Option<String>,
    pub world_rank_score: Option<i32>,
    pub ban_flag: Option<String>,
    pub next_fragstam_ts: Option<i64>,
    pub max_stamina_ts: Option<i64>,
    pub stamina: Option<i32>,
    pub world_mode_locked_end_ts: Option<i64>,
    pub beyond_boost_gauge: Option<f64>,
    pub kanae_stored_prog: Option<f64>,
    pub mp_notification_enabled: Option<i8>,
    pub highest_rating_ptt: Option<i32>,
    pub insight_state: Option<i32>,
}

/// Login session model representing the login table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Login {
    pub access_token: String,
    pub user_id: i32,
    pub login_time: Option<i64>,
    pub login_ip: Option<String>,
    pub login_device: Option<String>,
}

/// User registration data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegisterDto {
    pub name: String,
    pub password: String,
    pub email: String,
}

/// User login data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoginDto {
    pub name: String,
    pub password: String,
    pub device_id: Option<String>,
}

/// User authentication token data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuth {
    pub user_id: i32,
    pub token: String,
}

/// Minimal user data for validation queries
#[derive(Debug, Clone, FromRow)]
pub struct UserCredentials {
    pub user_id: i32,
    pub password: Option<String>,
    pub ban_flag: Option<String>,
}

/// User existence check result
#[derive(Debug, Clone, FromRow)]
pub struct UserExists {
    pub exists: i64,
}

/// User code to ID mapping
#[derive(Debug, Clone, FromRow)]
pub struct UserCodeMapping {
    pub user_id: i32,
}

/// User login device list
#[derive(Debug, Clone, FromRow)]
pub struct UserLoginDevice {
    pub login_device: Option<String>,
}

/// User basic info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: i32,
    pub name: String,
    pub user_code: String,
    pub rating_ptt: i32,
    pub character_id: i32,
    pub is_skill_sealed: bool,
    pub is_char_uncapped: bool,
    pub is_char_uncapped_override: bool,
    pub is_hide_rating: bool,
    pub favorite_character: i32,
    pub max_stamina_notification_enabled: bool,
    pub current_map: String,
    pub ticket: i32,
    pub prog_boost: i32,
    pub world_rank_score: i32,
    pub next_fragstam_ts: i64,
    pub max_stamina_ts: i64,
    pub stamina: i32,
    pub world_mode_locked_end_ts: i64,
    pub beyond_boost_gauge: f64,
    pub kanae_stored_prog: f64,
    pub mp_notification_enabled: bool,
    pub highest_rating_ptt: i32,
    pub insight_state: i32,
}

/// User for insertion (new user registration)
#[derive(Debug, Clone)]
pub struct NewUser {
    pub user_id: i32,
    pub name: String,
    pub password: String,
    pub join_date: i64,
    pub user_code: String,
    pub email: String,
    pub ticket: i32,
}

impl User {
    /// Convert database boolean-like integers to actual booleans
    pub fn is_skill_sealed(&self) -> bool {
        self.is_skill_sealed.unwrap_or(0) != 0
    }

    pub fn is_char_uncapped(&self) -> bool {
        self.is_char_uncapped.unwrap_or(0) != 0
    }

    pub fn is_char_uncapped_override(&self) -> bool {
        self.is_char_uncapped_override.unwrap_or(0) != 0
    }

    pub fn is_hide_rating(&self) -> bool {
        self.is_hide_rating.unwrap_or(0) != 0
    }

    pub fn max_stamina_notification_enabled(&self) -> bool {
        self.max_stamina_notification_enabled.unwrap_or(0) != 0
    }

    pub fn mp_notification_enabled(&self) -> bool {
        self.mp_notification_enabled.unwrap_or(1) != 0
    }

    /// Check if user is currently banned
    pub fn is_banned(&self, current_time: i64) -> bool {
        if let Some(ban_flag) = &self.ban_flag {
            if !ban_flag.is_empty() {
                if let Some(ban_parts) = ban_flag.split(':').nth(1) {
                    if let Ok(ban_timestamp) = ban_parts.parse::<i64>() {
                        return ban_timestamp > current_time;
                    }
                }
            }
        }
        false
    }

    /// Get remaining ban time in milliseconds
    pub fn ban_remaining_time(&self, current_time: i64) -> Option<i64> {
        if let Some(ban_flag) = &self.ban_flag {
            if !ban_flag.is_empty() {
                if let Some(ban_parts) = ban_flag.split(':').nth(1) {
                    if let Ok(ban_timestamp) = ban_parts.parse::<i64>() {
                        if ban_timestamp > current_time {
                            return Some(ban_timestamp - current_time);
                        }
                    }
                }
            }
        }
        None
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            user_id: user.user_id,
            name: user.name.clone().unwrap_or_default(),
            user_code: user.user_code.clone().unwrap_or_default(),
            rating_ptt: user.rating_ptt.unwrap_or(0),
            character_id: user.character_id.unwrap_or(0),
            is_skill_sealed: user.is_skill_sealed(),
            is_char_uncapped: user.is_char_uncapped(),
            is_char_uncapped_override: user.is_char_uncapped_override(),
            is_hide_rating: user.is_hide_rating(),
            favorite_character: user.favorite_character.unwrap_or(-1),
            max_stamina_notification_enabled: user.max_stamina_notification_enabled(),
            current_map: user.current_map.clone().unwrap_or_default(),
            ticket: user.ticket.unwrap_or(0),
            prog_boost: user.prog_boost.unwrap_or(0),
            world_rank_score: user.world_rank_score.unwrap_or(0),
            next_fragstam_ts: user.next_fragstam_ts.unwrap_or(0),
            max_stamina_ts: user.max_stamina_ts.unwrap_or(0),
            stamina: user.stamina.unwrap_or(0),
            world_mode_locked_end_ts: user.world_mode_locked_end_ts.unwrap_or(0),
            beyond_boost_gauge: user.beyond_boost_gauge.unwrap_or(0.0),
            kanae_stored_prog: user.kanae_stored_prog.unwrap_or(0.0),
            mp_notification_enabled: user.mp_notification_enabled(),
            highest_rating_ptt: user.highest_rating_ptt.unwrap_or(0),
            insight_state: user.insight_state.unwrap_or(4),
        }
    }
}
