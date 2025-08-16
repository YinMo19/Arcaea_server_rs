//! Arcaea Server Rust Implementation
//!
//! This is a Rust reimplementation of the Arcaea game server,
//! originally written in Python Flask. It provides the backend
//! API for the Arcaea rhythm game.

pub mod config;
pub mod error;
pub mod model;
pub mod route;
pub mod service;

// Re-export commonly used types for convenience
pub use config::{Constants, ARCAEA_SERVER_VERSION, CONFIG};
pub use error::{ArcError, ArcResult};

use sqlx::{MySql, Pool};
use std::env;

/// Database connection pool type alias
pub type DbPool = Pool<MySql>;

/// Database connection manager
pub struct Database;

impl Database {
    /// Create a new database connection pool
    ///
    /// Reads the DATABASE_URL environment variable to establish
    /// the connection. Falls back to a default MySQL connection
    /// string if the environment variable is not set.
    pub async fn new() -> Result<DbPool, sqlx::Error> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "mysql://arcaea:yinmo19sprivite@localhost:3306/arcaea_core".to_string()
        });

        let pool = sqlx::MySqlPool::connect(&database_url).await?;

        // Run any pending migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(pool)
    }

    /// Check if the database connection is healthy
    pub async fn check_health(pool: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(pool).await?;
        Ok(())
    }
}

/// Application state that will be managed by Rocket
pub struct AppState {
    pub db_pool: DbPool,
}

impl AppState {
    /// Create a new application state instance
    pub async fn new() -> Result<Self, sqlx::Error> {
        let db_pool = Database::new().await?;
        Ok(Self { db_pool })
    }
}

/// Utility functions for the application
pub mod utils {
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Get current timestamp in milliseconds
    pub fn current_timestamp_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Get current timestamp in seconds
    pub fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Validate email format (basic validation)
    pub fn is_valid_email(email: &str) -> bool {
        email.len() >= 4 && email.len() <= 64 && email.contains('@') && email.contains('.')
    }

    /// Validate user code format (9 digits)
    pub fn is_valid_user_code(user_code: &str) -> bool {
        user_code.len() == 9 && user_code.chars().all(|c| c.is_ascii_digit())
    }

    /// Validate username format
    pub fn is_valid_username(username: &str) -> bool {
        username.len() >= 3 && username.len() <= 16
    }

    /// Validate password format
    pub fn is_valid_password(password: &str) -> bool {
        password.len() >= 8 && password.len() <= 32
    }
}

/// Constants used throughout the application
pub mod constants {
    pub use crate::config::Constants;

    /// API version paths
    pub const GAME_API_PREFIX: &str = "/coldwind/35";
    pub const OLD_GAME_API_PREFIX: &[&str] = &[];

    /// Default values
    pub const DEFAULT_CHARACTER_ID: i32 = 0;
    pub const DEFAULT_RATING_PTT: i32 = 0;
    pub const DEFAULT_FAVORITE_CHARACTER: i32 = -1;
    pub const DEFAULT_INSIGHT_STATE: i32 = 4;

    /// HTTP status codes
    pub const STATUS_OK: u16 = 200;
    pub const STATUS_BAD_REQUEST: u16 = 400;
    pub const STATUS_UNAUTHORIZED: u16 = 401;
    pub const STATUS_FORBIDDEN: u16 = 403;
    pub const STATUS_NOT_FOUND: u16 = 404;
    pub const STATUS_TOO_MANY_REQUESTS: u16 = 429;
    pub const STATUS_INTERNAL_SERVER_ERROR: u16 = 500;
}

/// Prelude module for commonly used imports
pub mod prelude {
    pub use crate::config::{Constants, CONFIG};
    pub use crate::error::{ArcError, ArcResult};
    pub use crate::model::{
        Character, CharacterInfo, CharacterItem, Login, NewUserCharacter, User, UserAuth,
        UserCharacter, UserCharacterFull, UserCodeMapping, UserCredentials, UserExists, UserInfo,
        UserLoginDevice, UserLoginDto, UserRegisterDto,
    };
    pub use crate::route::{success_return, ApiResponse, RouteResult};
    pub use crate::service::UserService;
    pub use crate::utils;
    pub use crate::DbPool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(utils::is_valid_email("test@example.com"));
        assert!(utils::is_valid_email("user@domain.co.uk"));
        assert!(!utils::is_valid_email("invalid"));
        assert!(!utils::is_valid_email("@domain.com"));
        assert!(!utils::is_valid_email("user@"));
        assert!(!utils::is_valid_email("a@b")); // too short
    }

    #[test]
    fn test_user_code_validation() {
        assert!(utils::is_valid_user_code("123456789"));
        assert!(!utils::is_valid_user_code("12345678")); // too short
        assert!(!utils::is_valid_user_code("1234567890")); // too long
        assert!(!utils::is_valid_user_code("12345678a")); // contains letter
    }

    #[test]
    fn test_username_validation() {
        assert!(utils::is_valid_username("test"));
        assert!(utils::is_valid_username("testuser123"));
        assert!(!utils::is_valid_username("ab")); // too short
        assert!(!utils::is_valid_username("thisusernameistoolong")); // too long
    }

    #[test]
    fn test_password_validation() {
        assert!(utils::is_valid_password("password123"));
        assert!(utils::is_valid_password("verylongpassword"));
        assert!(!utils::is_valid_password("short")); // too short
        assert!(!utils::is_valid_password(
            "thispasswordisdefinitelytoolongtobevalid"
        )); // too long
    }
}
