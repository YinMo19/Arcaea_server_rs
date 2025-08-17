use lazy_static::lazy_static;
use std::collections::HashMap;

/// Game server configuration constants
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,

    // Game API settings
    pub game_api_prefix: String,
    pub old_game_api_prefix: Vec<String>,
    pub allow_appversion: Vec<String>,

    // Bundle settings
    pub bundle_strict_mode: bool,

    // World settings
    pub world_rank_max: i32,
    pub available_map: Vec<String>,

    // Authentication
    pub username: String,
    pub password: String,
    pub secret_key: String,
    pub api_token: String,

    // Download settings
    pub download_link_prefix: String,
    pub bundle_download_link_prefix: String,
    pub download_use_nginx_x_accel_redirect: bool,
    pub nginx_x_accel_redirect_prefix: String,
    pub bundle_nginx_x_accel_redirect_prefix: String,

    // Rate limiting
    pub download_times_limit: i32,
    pub download_time_gap_limit: i64,
    pub download_forbid_when_no_item: bool,
    pub bundle_download_times_limit: String,
    pub bundle_download_time_gap_limit: i64,

    // Login settings
    pub login_device_number_limit: i32,
    pub allow_login_same_device: bool,
    pub allow_ban_multidevice_user_auto: bool,

    // Game settings
    pub allow_score_with_no_song: bool,
    pub default_memories: i32,
    pub update_with_new_character_data: bool,
    pub character_full_unlock: bool,
    pub world_song_full_unlock: bool,
    pub world_scenery_full_unlock: bool,
    pub save_full_unlock: bool,
    pub allow_self_account_delete: bool,

    // PTT calculation weights
    pub best30_weight: f64,
    pub recent10_weight: f64,
    pub invasion_start_weight: f64,
    pub invasion_hard_weight: f64,

    // Social settings
    pub max_friend_count: i32,

    // Logging
    pub allow_info_log: bool,
    pub allow_warning_log: bool,

    // File paths (for reference, might not be used in Rust version)
    pub world_map_folder_path: String,
    pub song_file_folder_path: String,
    pub songlist_file_path: String,
    pub content_bundle_folder_path: String,
    pub database_init_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 80,

            game_api_prefix: "/coldwind/35".to_string(),
            old_game_api_prefix: Vec::new(),
            allow_appversion: Vec::new(),

            bundle_strict_mode: true,

            world_rank_max: 200,
            available_map: Vec::new(),

            username: "admin".to_string(),
            password: "admin".to_string(),
            secret_key: "1145141919810".to_string(),
            api_token: String::new(),

            download_link_prefix: String::new(),
            bundle_download_link_prefix: String::new(),
            download_use_nginx_x_accel_redirect: false,
            nginx_x_accel_redirect_prefix: "/nginx_download/".to_string(),
            bundle_nginx_x_accel_redirect_prefix: "/nginx_bundle_download/".to_string(),

            download_times_limit: 3000,
            download_time_gap_limit: 1000,
            download_forbid_when_no_item: false,
            bundle_download_times_limit: "100/60 minutes".to_string(),
            bundle_download_time_gap_limit: 3000,

            login_device_number_limit: 1,
            allow_login_same_device: false,
            allow_ban_multidevice_user_auto: true,

            allow_score_with_no_song: true,
            default_memories: 0,
            update_with_new_character_data: true,
            character_full_unlock: true,
            world_song_full_unlock: true,
            world_scenery_full_unlock: true,
            save_full_unlock: false,
            allow_self_account_delete: false,

            best30_weight: 1.0 / 40.0,
            recent10_weight: 1.0 / 40.0,
            invasion_start_weight: 0.1,
            invasion_hard_weight: 0.1,

            max_friend_count: 50,

            allow_info_log: false,
            allow_warning_log: false,

            world_map_folder_path: "./database/map/".to_string(),
            song_file_folder_path: "./database/songs/".to_string(),
            songlist_file_path: "./database/songs/songlist".to_string(),
            content_bundle_folder_path: "./database/bundle/".to_string(),
            database_init_path: "./database/init/".to_string(),
        }
    }
}

/// Game constants
pub struct Constants;

impl Constants {
    /// Ban duration in days for repeated violations
    pub const BAN_TIME: [i32; 5] = [1, 3, 7, 15, 31];

    /// Maximum stamina value
    pub const MAX_STAMINA: i32 = 12;

    /// Insight toggle states
    pub const INSIGHT_TOGGLE_STATES: [i32; 4] = [3, 4, 5, 6];

    /// Stamina recovery time in milliseconds (30 minutes)
    pub const STAMINA_RECOVER_TICK: i64 = 1800000;

    /// Fragment stamina recovery time in milliseconds (23 hours)
    pub const FRAGSTAM_RECOVER_TICK: i64 = 23 * 3600 * 1000;

    /// Course stamina cost
    pub const COURSE_STAMINA_COST: i32 = 4;

    /// Core experience points
    pub const CORE_EXP: i32 = 250;

    /// World value names
    pub const WORLD_VALUE_NAME_ENUM: [&'static str; 3] = ["frag", "prog", "over"];

    /// Free pack name
    pub const FREE_PACK_NAME: &'static str = "base";

    /// Single pack name
    pub const SINGLE_PACK_NAME: &'static str = "single";

    /// Character uncap bonus progress values
    pub const ETO_UNCAP_BONUS_PROGRESS: i32 = 7;
    pub const LUNA_UNCAP_BONUS_PROGRESS: i32 = 7;
    pub const AYU_UNCAP_BONUS_PROGRESS: i32 = 5;

    /// Skill related constants
    pub const SKILL_FATALIS_WORLD_LOCKED_TIME: i64 = 3600000; // 1 hour
    pub const FATALIS_MAX_VALUE: i32 = 100;

    /// Mika skill songs
    pub const SKILL_MIKA_SONGS: [&'static str; 8] = [
        "aprilshowers",
        "seventhsense",
        "oshamascramble",
        "amazingmightyyyy",
        "cycles",
        "maxrage",
        "infinity",
        "temptation",
    ];

    /// Ranking constants
    pub const MY_RANK_MAX_LOCAL_POSITION: i32 = 5;
    pub const MY_RANK_MAX_GLOBAL_POSITION: i32 = 9999;

    /// Character level experience requirements
    pub fn get_level_steps() -> HashMap<i32, i32> {
        let mut steps = HashMap::new();
        steps.insert(1, 0);
        steps.insert(2, 50);
        steps.insert(3, 100);
        steps.insert(4, 150);
        steps.insert(5, 200);
        steps.insert(6, 300);
        steps.insert(7, 450);
        steps.insert(8, 650);
        steps.insert(9, 900);
        steps.insert(10, 1200);
        steps.insert(11, 1600);
        steps.insert(12, 2100);
        steps.insert(13, 2700);
        steps.insert(14, 3400);
        steps.insert(15, 4200);
        steps.insert(16, 5100);
        steps.insert(17, 6100);
        steps.insert(18, 7200);
        steps.insert(19, 8500);
        steps.insert(20, 10000);
        steps.insert(21, 11500);
        steps.insert(22, 13000);
        steps.insert(23, 14500);
        steps.insert(24, 16000);
        steps.insert(25, 17500);
        steps.insert(26, 19000);
        steps.insert(27, 20500);
        steps.insert(28, 22000);
        steps.insert(29, 23500);
        steps.insert(30, 25000);
        steps
    }
}

lazy_static! {
    /// Global configuration instance
    pub static ref CONFIG: Config = {
        // Load from environment variables or configuration file
        // For now, use default values
        Config::default()
    };

    /// Character level experience steps
    pub static ref LEVEL_STEPS: HashMap<i32, i32> = Constants::get_level_steps();
}

/// Server version information
pub const ARCAEA_SERVER_VERSION: &str = "v0.1.0";
pub const ARCAEA_DATABASE_VERSION: &str = "v0.1.0";
pub const ARCAEA_LOG_DATABASE_VERSION: &str = "v0.1.0";

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub game_register_ip_rate_limit: String,
    pub game_register_device_rate_limit: String,
    pub game_login_rate_limit: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            game_register_ip_rate_limit: "5/minute".to_string(),
            game_register_device_rate_limit: "5/minute".to_string(),
            game_login_rate_limit: "10/minute".to_string(),
        }
    }
}

lazy_static! {
    /// Global rate limiting configuration
    pub static ref RATE_LIMIT_CONFIG: RateLimitConfig = RateLimitConfig::default();
}
