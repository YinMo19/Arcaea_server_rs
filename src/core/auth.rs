use base64::{engine::general_purpose, Engine as _};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use rand::Rng;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::{post, State};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::core::error::{ArcError, ArcResult, SuccessResponse};
use crate::core::models::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user: User,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token_type: String,
    pub user_id: i32,
    pub access_token: String,
}

#[derive(Debug, FromForm)]
pub struct LoginForm {
    pub grant_type: String,
}

pub struct UserAuth {
    pool: SqlitePool,
}

impl UserAuth {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn login(
        &self,
        name: &str,
        password: &str,
        device_id: &str,
        remote_addr: &str,
    ) -> ArcResult<LoginResponse> {
        // First, find the user by name
        let user_row = sqlx::query("SELECT * FROM user WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        let user_row = match user_row {
            Some(row) => row,
            None => {
                return Err(ArcError::with_error_code(
                    "Username or password is incorrect.",
                    104,
                ))
            }
        };

        let stored_password: String = user_row.get("password");
        let user_id: i32 = user_row.get("user_id");

        // Verify password
        if !verify_password(password, &stored_password)? {
            return Err(ArcError::with_error_code(
                "Username or password is incorrect.",
                104,
            ));
        }

        // Check if user is banned
        let ban_flag: Option<String> = user_row.get("ban_flag");
        if let Some(ban_msg) = ban_flag {
            if !ban_msg.is_empty() {
                return Err(ArcError::with_error_code(
                    &format!("Account banned: {}", ban_msg),
                    121,
                ));
            }
        }

        // Generate access token
        let access_token = generate_access_token();
        let login_time = Utc::now().timestamp_millis();

        // Check for existing login and handle device limits
        self.check_device_login_limits(user_id, device_id, remote_addr)
            .await?;

        // Insert or update login record
        sqlx::query(
            "INSERT OR REPLACE INTO login (access_token, user_id, login_time, login_ip, login_device) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&access_token)
        .bind(user_id)
        .bind(login_time)
        .bind(remote_addr)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(LoginResponse {
            success: true,
            token_type: "Bearer".to_string(),
            user_id,
            access_token,
        })
    }

    pub async fn verify_token(&self, token: &str) -> ArcResult<User> {
        let row = sqlx::query(
            "SELECT u.* FROM user u
             JOIN login l ON u.user_id = l.user_id
             WHERE l.access_token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let user = User::from_row(&row)?;

                // Check if user is banned
                if let Some(ban_msg) = &user.ban_flag {
                    if !ban_msg.is_empty() {
                        return Err(ArcError::user_ban(&format!("Account banned: {}", ban_msg)));
                    }
                }

                Ok(user)
            }
            None => Err(ArcError::no_access("Invalid or expired token")),
        }
    }

    pub async fn register_user(
        &self,
        name: &str,
        password: &str,
        email: &str,
        device_id: &str,
        remote_addr: &str,
    ) -> ArcResult<LoginResponse> {
        // Check if name already exists
        let existing_user = sqlx::query("SELECT user_id FROM user WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        if existing_user.is_some() {
            return Err(ArcError::with_error_code("Username is already taken.", 101));
        }

        // Check if email already exists
        let existing_email = sqlx::query("SELECT user_id FROM user WHERE email = ?")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        if existing_email.is_some() {
            return Err(ArcError::with_error_code(
                "Email is already registered.",
                102,
            ));
        }

        // Check device creation limits (one account per device per day)
        self.check_device_creation_limits(device_id, remote_addr)
            .await?;

        // Generate user details
        let user_code = generate_user_code();
        let hashed_password = hash_password(password)?;
        let join_date = Utc::now().timestamp_millis();

        // Get next user ID
        let next_user_id = self.get_next_user_id().await?;

        // Insert user
        sqlx::query(
            r#"INSERT INTO user (
                user_id, name, password, join_date, user_code, rating_ptt,
                character_id, is_skill_sealed, is_char_uncapped, is_char_uncapped_override,
                is_hide_rating, favorite_character, max_stamina_notification_enabled,
                current_map, ticket, prog_boost, email, world_rank_score, ban_flag,
                next_fragstam_ts, max_stamina_ts, stamina, world_mode_locked_end_ts,
                beyond_boost_gauge, kanae_stored_prog, mp_notification_enabled, insight_state
            ) VALUES (?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, -1, 0, '', 25, 0, ?, 0, '',
                     0, 0, 6, 0, 0.0, 0.0, 1, 4)"#,
        )
        .bind(next_user_id)
        .bind(name)
        .bind(hashed_password)
        .bind(join_date)
        .bind(&user_code)
        .bind(email)
        .execute(&self.pool)
        .await?;

        // Initialize user characters (add starter character)
        self.init_user_characters(next_user_id).await?;

        // Generate access token and login
        let access_token = generate_access_token();
        let login_time = Utc::now().timestamp_millis();

        sqlx::query(
            "INSERT INTO login (access_token, user_id, login_time, login_ip, login_device) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&access_token)
        .bind(next_user_id)
        .bind(login_time)
        .bind(remote_addr)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(LoginResponse {
            success: true,
            token_type: "Bearer".to_string(),
            user_id: next_user_id,
            access_token,
        })
    }

    async fn check_device_login_limits(
        &self,
        user_id: i32,
        device_id: &str,
        _remote_addr: &str,
    ) -> ArcResult<()> {
        // Check if user logged in from different device in last 24 hours
        let last_24h = Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000);

        let different_device = sqlx::query(
            "SELECT COUNT(*) as count FROM login WHERE user_id = ? AND login_device != ? AND login_time > ?"
        )
        .bind(user_id)
        .bind(device_id)
        .bind(last_24h)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = different_device.get("count");
        if count > 0 {
            return Err(ArcError::with_error_code(
                "Logged in from two devices within 24 hours.",
                105,
            ));
        }

        Ok(())
    }

    async fn check_device_creation_limits(
        &self,
        device_id: &str,
        remote_addr: &str,
    ) -> ArcResult<()> {
        let last_24h = Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000);

        // Check device creation limit
        let device_count = sqlx::query(
            "SELECT COUNT(*) as count FROM login WHERE login_device = ? AND login_time > ?",
        )
        .bind(device_id)
        .bind(last_24h)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = device_count.get("count");
        if count > 0 {
            return Err(ArcError::with_error_code(
                "A user has already been created from this device.",
                103,
            ));
        }

        // Check IP creation limit
        let ip_count = sqlx::query(
            "SELECT COUNT(*) as count FROM login WHERE login_ip = ? AND login_time > ?",
        )
        .bind(remote_addr)
        .bind(last_24h)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = ip_count.get("count");
        if count >= 3 {
            return Err(ArcError::with_error_code(
                "You can't create more accounts from this IP address today.",
                124,
            ));
        }

        Ok(())
    }

    async fn get_next_user_id(&self) -> ArcResult<i32> {
        let row = sqlx::query("SELECT COALESCE(MAX(user_id), 9999999) + 1 as next_id FROM user")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("next_id"))
    }

    async fn init_user_characters(&self, user_id: i32) -> ArcResult<()> {
        // Initialize with starter characters (character 0 and 1)
        for char_id in 0..=1 {
            sqlx::query(
                "INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag) VALUES (?, ?, 1, 0, 0, 0, 0)"
            )
            .bind(user_id)
            .bind(char_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = ArcError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = match req.guard::<&State<SqlitePool>>().await {
            Outcome::Success(pool) => pool.inner().clone(),
            Outcome::Error((status, _)) => {
                return Outcome::Error((status, ArcError::new("Database pool unavailable")))
            }
            Outcome::Forward(status) => return Outcome::Forward(status),
        };

        let auth_header = match req.headers().get_one("Authorization") {
            Some(header) => header,
            None => {
                return Outcome::Error((
                    Status::Unauthorized,
                    ArcError::no_access("No token provided"),
                ))
            }
        };

        if !auth_header.starts_with("Bearer ") {
            return Outcome::Error((
                Status::Unauthorized,
                ArcError::no_access("Invalid token format"),
            ));
        }

        let token = &auth_header[7..];
        let user_auth = UserAuth::new(pool);

        match user_auth.verify_token(token).await {
            Ok(user) => Outcome::Success(AuthenticatedUser {
                user,
                token: token.to_string(),
            }),
            Err(e) => Outcome::Error((Status::Unauthorized, e)),
        }
    }
}

pub fn parse_basic_auth(auth_header: &str) -> ArcResult<(String, String)> {
    if !auth_header.starts_with("Basic ") {
        return Err(ArcError::no_access("Invalid authorization header"));
    }

    let encoded = &auth_header[6..];
    let decoded = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| ArcError::no_access("Invalid base64 encoding"))?;

    let decoded_str =
        String::from_utf8(decoded).map_err(|_| ArcError::no_access("Invalid UTF-8 encoding"))?;

    let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(ArcError::no_access("Invalid credentials format"));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn generate_user_code() -> String {
    let mut rng = rand::thread_rng();
    format!("{:09}", rng.gen_range(100_000_000..=999_999_999))
}

fn generate_access_token() -> String {
    Uuid::new_v4().to_string().replace('-', "")
}

pub fn hash_password(password: &str) -> ArcResult<String> {
    hash(password, DEFAULT_COST)
        .map_err(|e| ArcError::new(&format!("Password hashing failed: {}", e)))
}

pub fn verify_password(password: &str, hash: &str) -> ArcResult<bool> {
    verify(password, hash)
        .map_err(|e| ArcError::new(&format!("Password verification failed: {}", e)))
}

#[derive(FromForm)]
pub struct AuthHeader(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthHeader {
    type Error = ArcError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("Authorization") {
            Some(header) => Outcome::Success(AuthHeader(header.to_string())),
            None => Outcome::Error((
                Status::BadRequest,
                ArcError::no_access("Authorization header missing"),
            )),
        }
    }
}

#[derive(FromForm)]
pub struct DeviceIdHeader(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DeviceIdHeader {
    type Error = ArcError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("DeviceId") {
            Some(header) => Outcome::Success(DeviceIdHeader(header.to_string())),
            None => Outcome::Success(DeviceIdHeader("low_version".to_string())),
        }
    }
}

#[post("/login")]
pub async fn auth_login(
    pool: &State<SqlitePool>,
    auth_header: AuthHeader,
    device_id: DeviceIdHeader,
) -> Result<Json<LoginResponse>, ArcError> {
    let (username, password) = parse_basic_auth(&auth_header.0)?;

    let user_auth = UserAuth::new((*pool).clone());
    let response = user_auth
        .login(&username, &password, &device_id.0, "127.0.0.1")
        .await?;

    Ok(Json(response))
}

#[post("/verify")]
pub fn email_verify() -> Result<Json<SuccessResponse<()>>, ArcError> {
    Err(
        ArcError::with_error_code("Email verification unavailable.", 151)
            .with_extra_data(serde_json::json!({"status": 404})),
    )
}
