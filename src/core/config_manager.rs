// config_manager.rs

pub const HOST: &str = "0.0.0.0";
pub const PORT: u16 = 80;

pub const DEPLOY_MODE: &str = "_waitress";
pub const USE_PROXY_FIX: bool = false;
pub const USE_CORS: bool = false;

pub const SONG_FILE_HASH_PRE_CALCULATE: bool = true;

pub const GAME_API_PREFIX: &str = "/natsugakuru/30";
pub const OLD_GAME_API_PREFIX: [&str; 0] = [];

pub const ALLOW_APPVERSION: [&str; 0] = [];

pub const BUNDLE_STRICT_MODE: bool = true;

pub const SET_LINKPLAY_SERVER_AS_SUB_PROCESS: bool = true;

pub const LINKPLAY_HOST: &str = "0.0.0.0";
pub const LINKPLAY_UDP_PORT: u16 = 10900;
pub const LINKPLAY_TCP_PORT: u16 = 10901;
pub const LINKPLAY_AUTHENTICATION: &str = "my_link_play_server";
pub const LINKPLAY_DISPLAY_HOST: &str = "";
pub const LINKPLAY_TCP_SECRET_KEY: &str = "1145141919810";

pub const SSL_CERT: &str = "";
pub const SSL_KEY: &str = "";

pub const IS_APRILFOOLS: bool = true;

pub const WORLD_RANK_MAX: i32 = 200;

pub const AVAILABLE_MAP: [&str; 0] = [];

pub const USERNAME: &str = "admin";
pub const PASSWORD: &str = "admin";

pub const SECRET_KEY: &str = "1145141919810";

pub const API_TOKEN: &str = "";

pub const DOWNLOAD_LINK_PREFIX: &str = "";
pub const BUNDLE_DOWNLOAD_LINK_PREFIX: &str = "";

pub const DOWNLOAD_USE_NGINX_X_ACCEL_REDIRECT: bool = false;
pub const NGINX_X_ACCEL_REDIRECT_PREFIX: &str = "/nginx_download/";
pub const BUNDLE_NGINX_X_ACCEL_REDIRECT_PREFIX: &str = "/nginx_bundle_download/";

pub const DOWNLOAD_TIMES_LIMIT: i32 = 3000;
pub const DOWNLOAD_TIME_GAP_LIMIT: i32 = 1000;

pub const DOWNLOAD_FORBID_WHEN_NO_ITEM: bool = false;

pub const BUNDLE_DOWNLOAD_TIMES_LIMIT: &str = "100/60 minutes";
pub const BUNDLE_DOWNLOAD_TIME_GAP_LIMIT: i32 = 3000;

pub const LOGIN_DEVICE_NUMBER_LIMIT: i32 = 1;
pub const ALLOW_LOGIN_SAME_DEVICE: bool = false;
pub const ALLOW_BAN_MULTIDEVICE_USER_AUTO: bool = true;

pub const ALLOW_SCORE_WITH_NO_SONG: bool = true;

pub const ALLOW_INFO_LOG: bool = false;
pub const ALLOW_WARNING_LOG: bool = false;

pub const DEFAULT_MEMORIES: i32 = 0;

pub const UPDATE_WITH_NEW_CHARACTER_DATA: bool = true;

pub const CHARACTER_FULL_UNLOCK: bool = true;
pub const WORLD_SONG_FULL_UNLOCK: bool = true;
pub const WORLD_SCENERY_FULL_UNLOCK: bool = true;

pub const SAVE_FULL_UNLOCK: bool = false;

pub const ALLOW_SELF_ACCOUNT_DELETE: bool = false;

pub const BEST30_WEIGHT: f32 = 1.0 / 40.0;
pub const RECENT10_WEIGHT: f32 = 1.0 / 40.0;

pub const MAX_FRIEND_COUNT: i32 = 50;

pub const WORLD_MAP_FOLDER_PATH: &str = "./database/map/";
pub const SONG_FILE_FOLDER_PATH: &str = "./database/songs/";
pub const SONGLIST_FILE_PATH: &str = "./database/songs/songlist";
pub const CONTENT_BUNDLE_FOLDER_PATH: &str = "./database/bundle/";
pub const SQLITE_DATABASE_PATH: &str = "./database/arcaea_database.db";
pub const SQLITE_DATABASE_BACKUP_FOLDER_PATH: &str = "./database/backup/";
pub const DATABASE_INIT_PATH: &str = "./database/init/";
pub const SQLITE_LOG_DATABASE_PATH: &str = "./database/arcaea_log.db";
pub const SQLITE_DATABASE_DELETED_PATH: &str = "./database/arcaea_database_deleted.db";

pub const GAME_LOGIN_RATE_LIMIT: &str = "30/5 minutes";
pub const API_LOGIN_RATE_LIMIT: &str = "10/5 minutes";
pub const GAME_REGISTER_IP_RATE_LIMIT: &str = "10/1 day";
pub const GAME_REGISTER_DEVICE_RATE_LIMIT: &str = "3/1 day";

pub const NOTIFICATION_EXPIRE_TIME: i32 = 3 * 60 * 1000;