use rocket::FromForm;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{Request, Response};
use serde_json;
use std::io::Cursor;

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

/// User settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub favorite_character: i32,
    pub is_hide_rating: bool,
    pub max_stamina_notification_enabled: bool,
    pub mp_notification_enabled: bool,
    pub is_allow_marketing_email: bool,
}

/// User core item format for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCoreInfo {
    pub core_type: String,
    pub amount: i32,
}

/// User recent score information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecentScore {
    pub song_id: String,
    pub difficulty: i32,
    pub score: i32,
    pub shiny_perfect_count: i32,
    pub perfect_count: i32,
    pub near_count: i32,
    pub miss_count: i32,
    pub health: i32,
    pub modifier: i32,
    pub time_played: i64,
    pub clear_type: i32,
    pub rating: f64,
}

/// User core information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserCores {
    pub core_generic: i32,
    pub core_chunithm: i32,
    pub core_desolate: i32,
    pub core_hollow: i32,
    pub core_crimson: i32,
    pub core_ambivalent: i32,
    pub core_scarlet: i32,
    pub core_groove: i32,
    pub core_azure: i32,
    pub core_binary: i32,
    pub core_colorful: i32,
    pub core_course: i32,
}

/// User basic info for API responses matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: i32,
    pub name: String,
    pub user_code: String,
    pub display_name: String,
    pub ticket: i32,
    pub character: i32,
    pub is_locked_name_duplicate: bool,
    pub is_skill_sealed: bool,
    pub current_map: String,
    pub prog_boost: i32,
    pub beyond_boost_gauge: f64,
    pub kanae_stored_prog: f64,
    pub next_fragstam_ts: i64,
    pub max_stamina_ts: i64,
    pub stamina: i32,
    pub world_mode_locked_end_ts: i64,
    pub insight_state: i32,
    pub is_aprilfools: bool,
    pub max_friend: i32,
    pub rating: i32,
    pub join_date: i64,
    pub global_rank: Option<i32>,
    pub country: Option<String>,
    pub custom_banner: Option<String>,
    pub course_banners: Vec<serde_json::Value>,
    pub locked_char_ids: Vec<i32>,
    pub pick_ticket: i32,

    // Settings object
    pub settings: UserSettings,

    // Character and related info
    pub character_stats: Vec<serde_json::Value>,
    pub characters: Vec<i32>,

    // Social features
    pub friends: Vec<serde_json::Value>,

    // Collections
    pub packs: Vec<String>,
    pub singles: Vec<String>,
    pub world_songs: Vec<String>,
    pub world_unlocks: Vec<String>,
    pub curr_available_maps: Vec<String>,
    pub user_missions: Vec<serde_json::Value>,

    // Items and scores
    pub cores: Vec<UserCoreInfo>,
    pub recent_score: Vec<UserRecentScore>,
    pub has_email: bool,
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
        let favorite_character = user.favorite_character.unwrap_or(-1);

        Self {
            user_id: user.user_id,
            name: user.name.clone().unwrap_or_default(),
            user_code: user.user_code.clone().unwrap_or_default(),
            display_name: user.name.clone().unwrap_or_default(),
            ticket: user.ticket.unwrap_or(0),
            character: user.character_id.unwrap_or(0),
            is_locked_name_duplicate: false,
            is_skill_sealed: user.is_skill_sealed(),
            current_map: user.current_map.clone().unwrap_or_default(),
            prog_boost: user.prog_boost.unwrap_or(0),
            beyond_boost_gauge: user.beyond_boost_gauge.unwrap_or(0.0),
            kanae_stored_prog: user.kanae_stored_prog.unwrap_or(0.0),
            next_fragstam_ts: user.next_fragstam_ts.unwrap_or(0),
            max_stamina_ts: user.max_stamina_ts.unwrap_or(0),
            stamina: user.stamina.unwrap_or(0),
            world_mode_locked_end_ts: user.world_mode_locked_end_ts.unwrap_or(-1),
            insight_state: user.insight_state.unwrap_or(4),
            is_aprilfools: true, // TODO: Get from config
            max_friend: 50,      // TODO: Get from constants
            rating: user.rating_ptt.unwrap_or(0),
            join_date: user.join_date.unwrap_or(0),
            global_rank: Some(0),
            country: Some(String::new()),
            custom_banner: Some(String::new()),
            course_banners: Vec::new(),
            locked_char_ids: Vec::new(),
            pick_ticket: 0,

            settings: UserSettings {
                favorite_character,
                is_hide_rating: user.is_hide_rating(),
                max_stamina_notification_enabled: user.max_stamina_notification_enabled(),
                mp_notification_enabled: user.mp_notification_enabled(),
                is_allow_marketing_email: false,
            },

            character_stats: Vec::new(), // TODO: Load from character service
            characters: Vec::new(),      // TODO: Load from character service
            friends: Vec::new(),         // TODO: Load from friend service

            packs: Vec::new(),               // TODO: Load from user packs
            singles: Vec::new(),             // TODO: Load from user singles
            world_songs: Vec::new(),         // TODO: Load from user world songs
            world_unlocks: Vec::new(),       // TODO: Load from user world unlocks
            curr_available_maps: Vec::new(), // TODO: Load from world service
            user_missions: Vec::new(),       // TODO: Load from mission service

            cores: Vec::new(),        // TODO: Load from user cores
            recent_score: Vec::new(), // TODO: Load from recent scores
            has_email: user.email.as_ref().is_some_and(|e| !e.is_empty()),
        }
    }
}

/// User login request payload
#[derive(Debug, Deserialize, FromForm)]
pub struct LoginRequest {
    pub grant_type: Option<String>,
}

/// Authentication response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse<'r> {
    pub success: bool,
    pub token_type: &'r str,
    pub user_id: i32,
    pub access_token: String,
}

/// Implement Responder for AuthResponse
impl<'r> Responder<'r, 'static> for AuthResponse<'r> {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = serde_json::to_string(&self).map_err(|_| Status::InternalServerError)?;

        Response::build()
            .status(Status::Ok)
            .header(ContentType::JSON)
            .sized_body(json.len(), Cursor::new(json))
            .ok()
    }
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: i32,
    pub access_token: String,
}
