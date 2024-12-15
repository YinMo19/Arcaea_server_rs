// constants.rs
use crate::core::config_manager as Config;

pub const ARCAEA_SERVER_VERSION: &str = "v2.12.0.1";
pub const ARCAEA_DATABASE_VERSION: &str = "v2.12.0.1";
pub const ARCAEA_LOG_DATABASE_VERSION: &str = "v1.1";

pub const BAN_TIME: [i32; 5] = [1, 3, 7, 15, 31];

pub const MAX_STAMINA: i32 = 12;

pub const STAMINA_RECOVER_TICK: i32 = 1800000;
pub const FRAGSTAM_RECOVER_TICK: i32 = 23 * 3600 * 1000;

pub const COURSE_STAMINA_COST: i32 = 4;

pub const CORE_EXP: i32 = 250;

pub const LEVEL_STEPS: [(i32, i32); 30] = [
    (1, 0),
    (2, 50),
    (3, 100),
    (4, 150),
    (5, 200),
    (6, 300),
    (7, 450),
    (8, 650),
    (9, 900),
    (10, 1200),
    (11, 1600),
    (12, 2100),
    (13, 2700),
    (14, 3400),
    (15, 4200),
    (16, 5100),
    (17, 6100),
    (18, 7200),
    (19, 8500),
    (20, 10000),
    (21, 11500),
    (22, 13000),
    (23, 14500),
    (24, 16000),
    (25, 17500),
    (26, 19000),
    (27, 20500),
    (28, 22000),
    (29, 23500),
    (30, 25000),
];

pub const WORLD_VALUE_NAME_ENUM: [&str; 3] = ["frag", "prog", "over"];

pub const FREE_PACK_NAME: &str = "base";
pub const SINGLE_PACK_NAME: &str = "single";

pub const ETO_UNCAP_BONUS_PROGRESS: i32 = 7;
pub const LUNA_UNCAP_BONUS_PROGRESS: i32 = 7;
pub const AYU_UNCAP_BONUS_PROGRESS: i32 = 5;
pub const SKILL_FATALIS_WORLD_LOCKED_TIME: i32 = 3600000;
pub const SKILL_MIKA_SONGS: [&str; 8] = [
    "aprilshowers",
    "seventhsense",
    "oshamascramble",
    "amazingmightyyyy",
    "cycles",
    "maxrage",
    "infinity",
    "temptation",
];

pub const MY_RANK_MAX_LOCAL_POSITION: i32 = 5;
pub const MY_RANK_MAX_GLOBAL_POSITION: i32 = 9999;

pub const BEST30_WEIGHT: f32 = Config::BEST30_WEIGHT;
pub const RECENT10_WEIGHT: f32 = Config::RECENT10_WEIGHT;

pub const WORLD_MAP_FOLDER_PATH: &str = Config::WORLD_MAP_FOLDER_PATH;
pub const SONG_FILE_FOLDER_PATH: &str = Config::SONG_FILE_FOLDER_PATH;
pub const SONGLIST_FILE_PATH: &str = Config::SONGLIST_FILE_PATH;
pub const CONTENT_BUNDLE_FOLDER_PATH: &str = Config::CONTENT_BUNDLE_FOLDER_PATH;
pub const SQLITE_DATABASE_PATH: &str = Config::SQLITE_DATABASE_PATH;
pub const SQLITE_LOG_DATABASE_PATH: &str = Config::SQLITE_LOG_DATABASE_PATH;
pub const SQLITE_DATABASE_DELETED_PATH: &str = Config::SQLITE_DATABASE_DELETED_PATH;

pub const DOWNLOAD_TIMES_LIMIT: i32 = Config::DOWNLOAD_TIMES_LIMIT;
pub const DOWNLOAD_TIME_GAP_LIMIT: i32 = Config::DOWNLOAD_TIME_GAP_LIMIT;
pub const DOWNLOAD_LINK_PREFIX: &str = Config::DOWNLOAD_LINK_PREFIX;
pub const BUNDLE_DOWNLOAD_TIMES_LIMIT: &str = Config::BUNDLE_DOWNLOAD_TIMES_LIMIT;
pub const BUNDLE_DOWNLOAD_TIME_GAP_LIMIT: i32 = Config::BUNDLE_DOWNLOAD_TIME_GAP_LIMIT;
pub const BUNDLE_DOWNLOAD_LINK_PREFIX: &str = Config::BUNDLE_DOWNLOAD_LINK_PREFIX;

pub const LINKPLAY_UNLOCK_LENGTH: usize = 512; // 单位：字节
pub const LINKPLAY_TIMEOUT: i32 = 5; // 单位：秒

pub const LINKPLAY_HOST: &str = if Config::SET_LINKPLAY_SERVER_AS_SUB_PROCESS {
    "127.0.0.1"
} else {
    Config::LINKPLAY_HOST
};
pub const LINKPLAY_TCP_PORT: u16 = Config::LINKPLAY_TCP_PORT;
pub const LINKPLAY_UDP_PORT: u16 = Config::LINKPLAY_UDP_PORT;
pub const LINKPLAY_AUTHENTICATION: &str = Config::LINKPLAY_AUTHENTICATION;
pub const LINKPLAY_TCP_SECRET_KEY: &str = Config::LINKPLAY_TCP_SECRET_KEY;
pub const LINKPLAY_TCP_MAX_LENGTH: u32 = 0x0FFFFFFF;

pub const LINKPLAY_MATCH_GET_ROOMS_INTERVAL: i32 = 4; // 单位：秒
pub const LINKPLAY_MATCH_PTT_ABS: [i32; 8] = [5, 20, 50, 100, 200, 500, 1000, 2000];
pub const LINKPLAY_MATCH_UNLOCK_MIN: [i32; 8] = [1000, 800, 500, 300, 200, 100, 50, 1];
pub const LINKPLAY_MATCH_TIMEOUT: i32 = 15; // 单位：秒
pub const LINKPLAY_MATCH_MEMORY_CLEAN_INTERVAL: i32 = 60; // 单位：秒

pub const FINALE_SWITCH: [(u32, u32); 72] = [
    (0x0015F0, 0x00B032),
    (0x014C9A, 0x014408),
    (0x062585, 0x02783B),
    (0x02429E, 0x0449A4),
    (0x099C9C, 0x07CFB4),
    (0x0785BF, 0x019B2C),
    (0x0EFF43, 0x0841BF),
    (0x07C88B, 0x0DE9FC),
    (0x000778, 0x064815),
    (0x0E62E3, 0x079F02),
    (0x0188FE, 0x0923EB),
    (0x0E06CD, 0x0E1A26),
    (0x00669E, 0x0C8BE1),
    (0x0BEB7A, 0x05D635),
    (0x040E6F, 0x0B465B),
    (0x0568EC, 0x07ED2B),
    (0x189614, 0x00A3D2),
    (0x62D98D, 0x45E5CA),
    (0x6D8769, 0x473F0E),
    (0x922E4F, 0x667D6C),
    (0x021F5C, 0x298839),
    (0x2A1201, 0x49FB7E),
    (0x158B81, 0x8D905D),
    (0x2253A5, 0x7E7067),
    (0x3BEF79, 0x9368E9),
    (0x00669E, 0x0C8BE1),
    (0x0BEB7A, 0x05D635),
    (0x040E6F, 0x0B465B),
    (0x756276, 0x55CD57),
    (0x130055, 0x7010E7),
    (0x55E28D, 0x4477FB),
    (0x5E99CB, 0x81060E),
    (0x7F43A4, 0x8FEC56),
    (0x69412F, 0x32735C),
    (0x8FF846, 0x14B5A1),
    (0x8716BE, 0x5C78BE),
    (0x62ED0E, 0x348E4B),
    (0x4B20C8, 0x56E0C3),
    (0x0AF6BC, 0x872441),
    (0x8825BC, 0x94B315),
    (0x792784, 0x5B2C8E),
    (0x1AE3A7, 0x688E97),
    (0x0D630F, 0x06BE78),
    (0x792784, 0x5B2C8E),
    (0x314869, 0x41CCC1),
    (0x311934, 0x24DD94),
    (0x190EDB, 0x33993D),
    (0x25F5C5, 0x15FAE6),
    (0x18CA10, 0x1B761A),
    (0x51BE82, 0x120089),
    (0x51D3B6, 0x2C29A2),
    (0x402075, 0x4A89B2),
    (0x00697B, 0x0E6497),
    (0x6D872D, 0x618AE7),
    (0x3DC0BE, 0x4E2AC8),
    (0x8C6ACF, 0x9776CF),
    (0x84673B, 0x5CA060),
    (0x4B05EC, 0x97FDFE),
    (0x207258, 0x02BB9B),
    (0x20A9EE, 0x1BA4BB),
    (0x503D21, 0x6A41D0),
    (0x1C256C, 0x6DD3BC),
    (0x6E4E0C, 0x89FDAA),
    (0x3C5F95, 0x3BA786),
    (0x0FEA5, 0x2E4CA),
    (0x7BF653, 0x4BEFD11),
    (0x46BEA7B, 0x11D3684),
    (0x8BFB04, 0xA83D6C1),
    (0x5D6FC5, 0xAB97EF),
    (0x237206D, 0xDFEF2),
    (0xA3DEE, 0x6CB300),
    (0xA35687B, 0xE456CDEA),
];

pub const DATABASE_MIGRATE_TABLES: [&str; 22] = [
    "user",
    "friend",
    "best_score",
    "recent30",
    "user_world",
    "item",
    "user_item",
    "purchase",
    "purchase_item",
    "user_save",
    "login",
    "present",
    "user_present",
    "present_item",
    "redeem",
    "user_redeem",
    "redeem_item",
    "api_login",
    "chart",
    "user_course",
    "user_char",
    "user_role",
];

pub const LOG_DATABASE_MIGRATE_TABLES: [&str; 3] = ["cache", "user_score", "user_rating"];

pub const UPDATE_WITH_NEW_CHARACTER_DATA: bool = Config::UPDATE_WITH_NEW_CHARACTER_DATA;
