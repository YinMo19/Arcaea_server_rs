use lazy_static::lazy_static;
use rocket::figment::{
    providers::{Format, Toml},
    Figment, Profile,
};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

macro_rules! set_from_figment {
    ($config:expr, $figment:expr, $field:ident, $key:expr, $ty:ty) => {
        if let Ok(value) = $figment.extract_inner::<$ty>($key) {
            $config.$field = value;
        }
    };
}

macro_rules! set_from_env {
    ($config:expr, $field:ident, $ty:ty) => {
        set_from_env_key!(
            $config,
            $field,
            stringify!($field).to_ascii_uppercase(),
            $ty
        );
    };
}

macro_rules! set_from_env_key {
    ($config:expr, $field:ident, $key:expr, $ty:ty) => {
        if let Some(value) = env_config_value::<$ty>($key.as_ref()) {
            $config.$field = value;
        }
    };
}

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
    pub bundle_download_link_prefix: Option<String>,
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
    pub disable_registration: bool,
    pub login_device_number_limit: i32,
    pub allow_login_same_device: bool,
    pub allow_ban_multidevice_user_auto: bool,

    // Game settings
    pub allow_score_with_no_song: bool,
    pub trace_complete_ticket_reward_enabled: bool,
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
            bundle_download_link_prefix: Some(String::from(
                "https://arc.yinmo.site/bundle_download/",
            )),
            download_use_nginx_x_accel_redirect: false,
            nginx_x_accel_redirect_prefix: "/nginx_download/".to_string(),
            bundle_nginx_x_accel_redirect_prefix: "/nginx_bundle_download/".to_string(),

            download_times_limit: 3000,
            download_time_gap_limit: 1000,
            download_forbid_when_no_item: false,
            bundle_download_times_limit: "100/60 minutes".to_string(),
            bundle_download_time_gap_limit: 3000,

            disable_registration: false,
            login_device_number_limit: 1,
            allow_login_same_device: false,
            allow_ban_multidevice_user_auto: true,

            allow_score_with_no_song: true,
            trace_complete_ticket_reward_enabled: false,
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

impl Config {
    /// Load configuration using the precedence: environment/.env > Rocket.toml > defaults.
    pub fn load() -> Self {
        dotenv::dotenv().ok();

        let mut config = Self::default();
        let figment = rocket_toml_figment();

        config.apply_rocket_toml(&figment);
        config.apply_env();

        config
    }

    fn apply_rocket_toml(&mut self, figment: &Figment) {
        // Standard Rocket key aliases.
        set_from_figment!(self, figment, host, "address", String);

        set_from_figment!(self, figment, host, "host", String);
        set_from_figment!(self, figment, port, "port", u16);
        set_from_figment!(self, figment, game_api_prefix, "game_api_prefix", String);
        set_from_figment!(
            self,
            figment,
            old_game_api_prefix,
            "old_game_api_prefix",
            Vec<String>
        );
        set_from_figment!(
            self,
            figment,
            allow_appversion,
            "allow_appversion",
            Vec<String>
        );
        set_from_figment!(
            self,
            figment,
            bundle_strict_mode,
            "bundle_strict_mode",
            bool
        );
        set_from_figment!(self, figment, world_rank_max, "world_rank_max", i32);
        set_from_figment!(self, figment, available_map, "available_map", Vec<String>);
        set_from_figment!(self, figment, username, "username", String);
        set_from_figment!(self, figment, password, "password", String);
        set_from_figment!(self, figment, secret_key, "secret_key", String);
        set_from_figment!(self, figment, api_token, "api_token", String);
        set_from_figment!(
            self,
            figment,
            download_link_prefix,
            "download_link_prefix",
            String
        );
        set_from_figment!(
            self,
            figment,
            bundle_download_link_prefix,
            "bundle_download_link_prefix",
            Option<String>
        );
        set_from_figment!(
            self,
            figment,
            download_use_nginx_x_accel_redirect,
            "download_use_nginx_x_accel_redirect",
            bool
        );
        set_from_figment!(
            self,
            figment,
            nginx_x_accel_redirect_prefix,
            "nginx_x_accel_redirect_prefix",
            String
        );
        set_from_figment!(
            self,
            figment,
            bundle_nginx_x_accel_redirect_prefix,
            "bundle_nginx_x_accel_redirect_prefix",
            String
        );
        set_from_figment!(
            self,
            figment,
            download_times_limit,
            "download_times_limit",
            i32
        );
        set_from_figment!(
            self,
            figment,
            download_time_gap_limit,
            "download_time_gap_limit",
            i64
        );
        set_from_figment!(
            self,
            figment,
            download_forbid_when_no_item,
            "download_forbid_when_no_item",
            bool
        );
        set_from_figment!(
            self,
            figment,
            bundle_download_times_limit,
            "bundle_download_times_limit",
            String
        );
        set_from_figment!(
            self,
            figment,
            bundle_download_time_gap_limit,
            "bundle_download_time_gap_limit",
            i64
        );
        set_from_figment!(
            self,
            figment,
            disable_registration,
            "disable_registration",
            bool
        );
        set_from_figment!(
            self,
            figment,
            login_device_number_limit,
            "login_device_number_limit",
            i32
        );
        set_from_figment!(
            self,
            figment,
            allow_login_same_device,
            "allow_login_same_device",
            bool
        );
        set_from_figment!(
            self,
            figment,
            allow_ban_multidevice_user_auto,
            "allow_ban_multidevice_user_auto",
            bool
        );
        set_from_figment!(
            self,
            figment,
            allow_score_with_no_song,
            "allow_score_with_no_song",
            bool
        );
        set_from_figment!(
            self,
            figment,
            trace_complete_ticket_reward_enabled,
            "trace_complete_ticket_reward_enabled",
            bool
        );
        set_from_figment!(self, figment, default_memories, "default_memories", i32);
        set_from_figment!(
            self,
            figment,
            update_with_new_character_data,
            "update_with_new_character_data",
            bool
        );
        set_from_figment!(
            self,
            figment,
            character_full_unlock,
            "character_full_unlock",
            bool
        );
        set_from_figment!(
            self,
            figment,
            world_song_full_unlock,
            "world_song_full_unlock",
            bool
        );
        set_from_figment!(
            self,
            figment,
            world_scenery_full_unlock,
            "world_scenery_full_unlock",
            bool
        );
        set_from_figment!(self, figment, save_full_unlock, "save_full_unlock", bool);
        set_from_figment!(
            self,
            figment,
            allow_self_account_delete,
            "allow_self_account_delete",
            bool
        );
        set_from_figment!(self, figment, best30_weight, "best30_weight", f64);
        set_from_figment!(self, figment, recent10_weight, "recent10_weight", f64);
        set_from_figment!(
            self,
            figment,
            invasion_start_weight,
            "invasion_start_weight",
            f64
        );
        set_from_figment!(
            self,
            figment,
            invasion_hard_weight,
            "invasion_hard_weight",
            f64
        );
        set_from_figment!(self, figment, max_friend_count, "max_friend_count", i32);
        set_from_figment!(self, figment, allow_info_log, "allow_info_log", bool);
        set_from_figment!(self, figment, allow_warning_log, "allow_warning_log", bool);
        set_from_figment!(
            self,
            figment,
            world_map_folder_path,
            "world_map_folder_path",
            String
        );
        set_from_figment!(
            self,
            figment,
            song_file_folder_path,
            "song_file_folder_path",
            String
        );
        set_from_figment!(
            self,
            figment,
            songlist_file_path,
            "songlist_file_path",
            String
        );
        set_from_figment!(
            self,
            figment,
            content_bundle_folder_path,
            "content_bundle_folder_path",
            String
        );
        set_from_figment!(
            self,
            figment,
            database_init_path,
            "database_init_path",
            String
        );
    }

    fn apply_env(&mut self) {
        // Standard Rocket key aliases. Field-specific variables below take precedence.
        set_from_env_key!(self, host, "ROCKET_ADDRESS", String);
        set_from_env_key!(self, host, "ADDRESS", String);
        set_from_env_key!(self, port, "ROCKET_PORT", u16);

        set_from_env!(self, host, String);
        set_from_env!(self, port, u16);
        set_from_env!(self, game_api_prefix, String);
        set_from_env!(self, old_game_api_prefix, Vec<String>);
        set_from_env!(self, allow_appversion, Vec<String>);
        set_from_env!(self, bundle_strict_mode, bool);
        set_from_env!(self, world_rank_max, i32);
        set_from_env!(self, available_map, Vec<String>);
        set_from_env!(self, username, String);
        set_from_env!(self, password, String);
        set_from_env!(self, secret_key, String);
        set_from_env!(self, api_token, String);
        set_from_env!(self, download_link_prefix, String);
        set_from_env!(self, bundle_download_link_prefix, Option<String>);
        set_from_env!(self, download_use_nginx_x_accel_redirect, bool);
        set_from_env!(self, nginx_x_accel_redirect_prefix, String);
        set_from_env!(self, bundle_nginx_x_accel_redirect_prefix, String);
        set_from_env!(self, download_times_limit, i32);
        set_from_env!(self, download_time_gap_limit, i64);
        set_from_env!(self, download_forbid_when_no_item, bool);
        set_from_env!(self, bundle_download_times_limit, String);
        set_from_env!(self, bundle_download_time_gap_limit, i64);
        set_from_env!(self, disable_registration, bool);
        set_from_env!(self, login_device_number_limit, i32);
        set_from_env!(self, allow_login_same_device, bool);
        set_from_env!(self, allow_ban_multidevice_user_auto, bool);
        set_from_env!(self, allow_score_with_no_song, bool);
        set_from_env!(self, trace_complete_ticket_reward_enabled, bool);
        set_from_env!(self, default_memories, i32);
        set_from_env!(self, update_with_new_character_data, bool);
        set_from_env!(self, character_full_unlock, bool);
        set_from_env!(self, world_song_full_unlock, bool);
        set_from_env!(self, world_scenery_full_unlock, bool);
        set_from_env!(self, save_full_unlock, bool);
        set_from_env!(self, allow_self_account_delete, bool);
        set_from_env!(self, best30_weight, f64);
        set_from_env!(self, recent10_weight, f64);
        set_from_env!(self, invasion_start_weight, f64);
        set_from_env!(self, invasion_hard_weight, f64);
        set_from_env!(self, max_friend_count, i32);
        set_from_env!(self, allow_info_log, bool);
        set_from_env!(self, allow_warning_log, bool);
        set_from_env!(self, world_map_folder_path, String);
        set_from_env!(self, song_file_folder_path, String);
        set_from_env!(self, songlist_file_path, String);
        set_from_env!(self, content_bundle_folder_path, String);
        set_from_env!(self, database_init_path, String);
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
    pub static ref CONFIG: Config = Config::load();

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

impl RateLimitConfig {
    /// Load rate-limit configuration using the precedence: environment/.env > Rocket.toml > defaults.
    pub fn load() -> Self {
        dotenv::dotenv().ok();

        let mut config = Self::default();
        let figment = rocket_toml_figment();

        config.apply_rocket_toml(&figment);
        config.apply_env();

        config
    }

    fn apply_rocket_toml(&mut self, figment: &Figment) {
        set_from_figment!(
            self,
            figment,
            game_register_ip_rate_limit,
            "game_register_ip_rate_limit",
            String
        );
        set_from_figment!(
            self,
            figment,
            game_register_device_rate_limit,
            "game_register_device_rate_limit",
            String
        );
        set_from_figment!(
            self,
            figment,
            game_login_rate_limit,
            "game_login_rate_limit",
            String
        );
    }

    fn apply_env(&mut self) {
        set_from_env!(self, game_register_ip_rate_limit, String);
        set_from_env!(self, game_register_device_rate_limit, String);
        set_from_env!(self, game_login_rate_limit, String);
    }
}

fn rocket_toml_figment() -> Figment {
    let config_file = env::var("ROCKET_CONFIG").unwrap_or_else(|_| "Rocket.toml".to_string());
    let default_profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let profile = Profile::from_env_or("ROCKET_PROFILE", default_profile);

    Figment::from(Toml::file(config_file).nested()).select(profile)
}

trait EnvConfigValue: Sized {
    fn parse_env(key: &str, value: &str) -> Option<Self>;
}

impl EnvConfigValue for String {
    fn parse_env(_key: &str, value: &str) -> Option<Self> {
        Some(value.to_string())
    }
}

impl EnvConfigValue for bool {
    fn parse_env(key: &str, value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "y" | "on" => Some(true),
            "false" | "0" | "no" | "n" | "off" => Some(false),
            _ => {
                log::warn!("Ignoring invalid boolean config env {key}={value:?}");
                None
            }
        }
    }
}

impl EnvConfigValue for Vec<String> {
    fn parse_env(key: &str, value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if let Ok(values) = serde_json::from_str::<Vec<String>>(trimmed) {
            return Some(values);
        }

        let list = trimmed.trim_start_matches('[').trim_end_matches(']');
        let values = list
            .split(',')
            .map(|part| part.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if values.is_empty() && trimmed.starts_with('[') {
            log::warn!("Ignoring invalid string-list config env {key}={value:?}");
            None
        } else {
            Some(values)
        }
    }
}

impl EnvConfigValue for Option<String> {
    fn parse_env(_key: &str, value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" | "null" => Some(None),
            _ => Some(Some(value.to_string())),
        }
    }
}

macro_rules! impl_from_str_env_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl EnvConfigValue for $ty {
                fn parse_env(key: &str, value: &str) -> Option<Self> {
                    match <$ty>::from_str(value.trim()) {
                        Ok(value) => Some(value),
                        Err(_) => {
                            log::warn!("Ignoring invalid config env {key}={value:?}");
                            None
                        }
                    }
                }
            }
        )*
    };
}

impl_from_str_env_value!(u16, i32, i64, f64);

fn env_config_value<T: EnvConfigValue>(key: &str) -> Option<T> {
    let value = env::var(key).ok()?;
    if value.trim().is_empty() {
        return None;
    }

    T::parse_env(key, &value)
}

lazy_static! {
    /// Global rate limiting configuration
    pub static ref RATE_LIMIT_CONFIG: RateLimitConfig = RateLimitConfig::load();
}
