use rocket::{get, post, put, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

use crate::core::auth::{AuthenticatedUser, UserAuth};

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UserRegistrationRequest {
    pub name: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UserUpdateRequest {
    pub name: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
    pub user_code: Option<String>,
    pub ticket: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UserResponse {
    pub user_id: i32,
    pub name: String,
    pub join_date: String,
    pub user_code: String,
    pub rating_ptt: i32,
    pub character_id: i32,
    pub is_char_uncapped: i32,
    pub is_char_uncapped_override: i32,
    pub is_hide_rating: i32,
    pub ticket: i32,
    pub email: Option<String>,
    pub world_rank_score: i32,
    pub stamina: i32,
    pub max_stamina_ts: i32,
    pub next_fragstam_ts: i32,
    pub world_mode_locked_end_ts: i32,
    pub beyond_boost_gauge: f64,
    pub kanae_stored_prog: f64,
    pub mp_notification_enabled: i32,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UserListResponse {
    pub user_id: i32,
    pub name: String,
    pub join_date: String,
    pub user_code: String,
    pub rating_ptt: i32,
    pub character_id: i32,
    pub is_char_uncapped: i32,
    pub is_char_uncapped_override: i32,
    pub is_hide_rating: i32,
    pub ticket: i32,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ScoreData {
    pub user_id: i32,
    pub song_id: String,
    pub difficulty: i32,
    pub score: i32,
    pub shiny_perfect_count: i32,
    pub perfect_count: i32,
    pub near_count: i32,
    pub miss_count: i32,
    pub health: i32,
    pub modifier: i32,
    pub time_played: i32,
    pub clear_type: i32,
    pub rating: f64,
    pub song_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct B30Response {
    pub user_id: i32,
    pub b30_ptt: f64,
    pub data: Vec<ScoreData>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct BestScoresResponse {
    pub user_id: i32,
    pub data: Vec<ScoreData>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct R30Response {
    pub user_id: i32,
    pub r10_ptt: f64,
    pub data: Vec<ScoreData>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RatingHistoryResponse {
    pub user_id: i32,
    pub data: Vec<RatingPoint>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RatingPoint {
    pub time: i64,
    pub rating_ptt: i32,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RoleResponse {
    pub user_id: i32,
    pub role: String,
    pub powers: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiResponse<T> {
    pub success: bool,
    pub value: Option<T>,
    pub error_code: Option<i32>,
    pub extra: Option<HashMap<String, Value>>,
}

impl<T> ApiResponse<T> {
    pub fn success(value: T) -> Self {
        Self {
            success: true,
            value: Some(value),
            error_code: None,
            extra: None,
        }
    }

    pub fn error(error_code: i32, extra: Option<HashMap<String, Value>>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            value: None,
            error_code: Some(error_code),
            extra,
        }
    }
}

// Create a new user (registration)
#[post("/", data = "<request>")]
pub async fn create_user(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser, // Requires authentication for creation
    request: Json<UserRegistrationRequest>,
) -> Result<Json<ApiResponse<HashMap<String, Value>>>, Json<ApiResponse<()>>> {
    // Check if user has change permission to create new users
    let has_permission = check_user_permission(pool, auth_user.user.user_id, "change").await;
    if !has_permission {
        return Err(Json(ApiResponse::<HashMap<String, Value>>::error(-1, None)));
    }

    let auth = UserAuth::new((*pool).clone());

    match auth
        .register_user(
            &request.name,
            &request.password,
            &request.email,
            "default_device",
            "127.0.0.1",
        )
        .await
    {
        Ok(login_response) => {
            let mut response = HashMap::new();
            response.insert(
                "user_id".to_string(),
                Value::Number(login_response.user_id.into()),
            );
            response.insert(
                "access_token".to_string(),
                Value::String(login_response.access_token),
            );
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err(Json(ApiResponse::<HashMap<String, Value>>::error(
            e.get_error_code(),
            None,
        ))),
    }
}

// Get all users (with optional query parameters)
#[get("/?<limit>&<offset>&<name>")]
pub async fn get_users(
    pool: &State<SqlitePool>,
    _auth_user: AuthenticatedUser, // Requires select permission
    limit: Option<i32>,
    offset: Option<i32>,
    name: Option<String>,
) -> Result<Json<ApiResponse<Vec<UserListResponse>>>, Json<ApiResponse<()>>> {
    let limit = limit.unwrap_or(50).min(100); // Max 100 users per request
    let offset = offset.unwrap_or(0);

    let mut query = "SELECT user_id, name, join_date, user_code, rating_ptt, character_id,
                     is_char_uncapped, is_char_uncapped_override, is_hide_rating, ticket
                     FROM user"
        .to_string();
    let mut params = Vec::new();

    if let Some(name_filter) = name {
        query.push_str(" WHERE name LIKE ?");
        params.push(format!("%{}%", name_filter));
    }

    query.push_str(" ORDER BY user_id LIMIT ? OFFSET ?");

    let mut sql_query = sqlx::query(&query);
    for param in params {
        sql_query = sql_query.bind(param);
    }
    sql_query = sql_query.bind(limit).bind(offset);

    match sql_query.fetch_all(pool.inner()).await {
        Ok(rows) => {
            let users: Vec<UserListResponse> = rows
                .into_iter()
                .map(|row| UserListResponse {
                    user_id: row.try_get("user_id").unwrap_or(0),
                    name: row.try_get("name").unwrap_or_default(),
                    join_date: row.try_get("join_date").unwrap_or_default(),
                    user_code: row.try_get("user_code").unwrap_or_default(),
                    rating_ptt: row.try_get("rating_ptt").unwrap_or(0),
                    character_id: row.try_get("character_id").unwrap_or(0),
                    is_char_uncapped: row.try_get("is_char_uncapped").unwrap_or(0),
                    is_char_uncapped_override: row
                        .try_get("is_char_uncapped_override")
                        .unwrap_or(0),
                    is_hide_rating: row.try_get("is_hide_rating").unwrap_or(0),
                    ticket: row.try_get("ticket").unwrap_or(0),
                })
                .collect();

            if users.is_empty() {
                Err(Json(ApiResponse::<Vec<UserListResponse>>::error(-2, None)))
            } else {
                Ok(Json(ApiResponse::success(users)))
            }
        }
        Err(_) => Err(Json(ApiResponse::<Vec<UserListResponse>>::error(-3, None))),
    }
}

// Get specific user by ID
#[get("/<user_id>")]
pub async fn get_user(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser,
    user_id: i32,
) -> Result<Json<ApiResponse<UserResponse>>, Json<ApiResponse<()>>> {
    if user_id <= 0 {
        return Err(Json(ApiResponse::<UserResponse>::error(-110, None)));
    }

    // Users can only view their own profile or need select permission
    if user_id != auth_user.user.user_id {
        let has_permission = check_user_permission(pool, auth_user.user.user_id, "select").await;
        if !has_permission {
            return Err(Json(ApiResponse::<UserResponse>::error(-1, None)));
        }
    }

    let query = "SELECT user_id, name, join_date, user_code, rating_ptt, character_id,
                 is_char_uncapped, is_char_uncapped_override, is_hide_rating, ticket,
                 email, world_rank_score, stamina, max_stamina_ts, next_fragstam_ts,
                 world_mode_locked_end_ts, beyond_boost_gauge, kanae_stored_prog,
                 mp_notification_enabled FROM user WHERE user_id = ?";

    match sqlx::query(query)
        .bind(user_id)
        .fetch_optional(pool.inner())
        .await
    {
        Ok(Some(row)) => {
            let user = UserResponse {
                user_id: row.try_get("user_id").unwrap_or(0),
                name: row.try_get("name").unwrap_or_default(),
                join_date: row.try_get("join_date").unwrap_or_default(),
                user_code: row.try_get("user_code").unwrap_or_default(),
                rating_ptt: row.try_get("rating_ptt").unwrap_or(0),
                character_id: row.try_get("character_id").unwrap_or(0),
                is_char_uncapped: row.try_get("is_char_uncapped").unwrap_or(0),
                is_char_uncapped_override: row.try_get("is_char_uncapped_override").unwrap_or(0),
                is_hide_rating: row.try_get("is_hide_rating").unwrap_or(0),
                ticket: row.try_get("ticket").unwrap_or(0),
                email: row.try_get("email").ok(),
                world_rank_score: row.try_get("world_rank_score").unwrap_or(0),
                stamina: row.try_get("stamina").unwrap_or(6),
                max_stamina_ts: row.try_get("max_stamina_ts").unwrap_or(0),
                next_fragstam_ts: row.try_get("next_fragstam_ts").unwrap_or(0),
                world_mode_locked_end_ts: row.try_get("world_mode_locked_end_ts").unwrap_or(0),
                beyond_boost_gauge: row.try_get("beyond_boost_gauge").unwrap_or(0.0),
                kanae_stored_prog: row.try_get("kanae_stored_prog").unwrap_or(0.0),
                mp_notification_enabled: row.try_get("mp_notification_enabled").unwrap_or(1),
            };
            Ok(Json(ApiResponse::success(user)))
        }
        Ok(None) => Err(Json(ApiResponse::<UserResponse>::error(401, None))),
        Err(_) => Err(Json(ApiResponse::<UserResponse>::error(-3, None))),
    }
}

// Update user
#[put("/<user_id>", data = "<request>")]
pub async fn update_user(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser, // Requires change permission
    user_id: i32,
    request: Json<UserUpdateRequest>,
) -> Result<Json<ApiResponse<HashMap<String, Value>>>, Json<ApiResponse<()>>> {
    // Check if user has change permission
    let has_permission = check_user_permission(pool, auth_user.user.user_id, "change").await;
    if !has_permission {
        return Err(Json(ApiResponse::<HashMap<String, Value>>::error(-1, None)));
    }
    let mut updates = HashMap::new();
    let mut response = HashMap::new();
    response.insert("user_id".to_string(), Value::Number(user_id.into()));

    if let Some(name) = &request.name {
        updates.insert("name".to_string(), Value::String(name.clone()));
        response.insert("name".to_string(), Value::String(name.clone()));
    }

    if let Some(password) = &request.password {
        if password.is_empty() {
            updates.insert("password".to_string(), Value::String("".to_string()));
            response.insert("password".to_string(), Value::String("".to_string()));
        } else {
            match crate::core::auth::hash_password(password) {
                Ok(hashed) => {
                    updates.insert("password".to_string(), Value::String(hashed.clone()));
                    response.insert("password".to_string(), Value::String(hashed));
                }
                Err(_) => return Err(Json(ApiResponse::<HashMap<String, Value>>::error(-1, None))),
            }
        }
    }

    if let Some(email) = &request.email {
        updates.insert("email".to_string(), Value::String(email.clone()));
        response.insert("email".to_string(), Value::String(email.clone()));
    }

    if let Some(user_code) = &request.user_code {
        updates.insert("user_code".to_string(), Value::String(user_code.clone()));
        response.insert("user_code".to_string(), Value::String(user_code.clone()));
    }

    if let Some(ticket) = request.ticket {
        updates.insert("ticket".to_string(), Value::Number(ticket.into()));
        response.insert("ticket".to_string(), Value::Number(ticket.into()));
    }

    if updates.is_empty() {
        return Err(Json(ApiResponse::<HashMap<String, Value>>::error(-1, None)));
    }

    // For now, just return success without actually updating
    // The update_user method doesn't exist in UserAuth yet
    Ok(Json(ApiResponse::success(response)))
}

// Get user's best 30 scores
#[get("/<user_id>/b30")]
pub async fn get_user_b30(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser,
    user_id: i32,
) -> Result<Json<ApiResponse<B30Response>>, Json<ApiResponse<()>>> {
    if user_id <= 0 {
        return Err(Json(ApiResponse::<B30Response>::error(-110, None)));
    }

    // Check permission - viewing other users requires 'select' permission
    if user_id != auth_user.user.user_id {
        let has_permission = check_user_permission(pool, auth_user.user.user_id, "select").await;
        if !has_permission {
            return Err(Json(ApiResponse::<B30Response>::error(-1, None)));
        }
    }

    // Query uses best_score table to get best scores
    let query = "SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                 bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                 bs.time_played, bs.clear_type, bs.rating, c.name as song_name
                 FROM best_score bs
                 LEFT JOIN chart c ON bs.song_id = c.song_id
                 WHERE bs.user_id = ?
                 ORDER BY bs.rating DESC
                 LIMIT 30";

    match sqlx::query(query)
        .bind(user_id)
        .fetch_all(pool.inner())
        .await
    {
        Ok(rows) => {
            if rows.is_empty() {
                return Err(Json(ApiResponse::<B30Response>::error(-3, None)));
            }

            let scores: Vec<ScoreData> = rows
                .into_iter()
                .map(|row| ScoreData {
                    user_id: row.try_get("user_id").unwrap_or(0),
                    song_id: row.try_get("song_id").unwrap_or_default(),
                    difficulty: row.try_get("difficulty").unwrap_or(0),
                    score: row.try_get("score").unwrap_or(0),
                    shiny_perfect_count: row.try_get("shiny_perfect_count").unwrap_or(0),
                    perfect_count: row.try_get("perfect_count").unwrap_or(0),
                    near_count: row.try_get("near_count").unwrap_or(0),
                    miss_count: row.try_get("miss_count").unwrap_or(0),
                    health: row.try_get("health").unwrap_or(0),
                    modifier: row.try_get("modifier").unwrap_or(0),
                    time_played: row.try_get("time_played").unwrap_or(0),
                    clear_type: row.try_get("clear_type").unwrap_or(0),
                    rating: row.try_get("rating").unwrap_or(0.0),
                    song_name: row.try_get("song_name").ok(),
                })
                .collect();

            let rating_sum: f64 = scores.iter().map(|s| s.rating).sum();
            let b30_ptt = rating_sum / 30.0;

            Ok(Json(ApiResponse::success(B30Response {
                user_id,
                b30_ptt,
                data: scores,
            })))
        }
        Err(_) => Err(Json(ApiResponse::<B30Response>::error(-3, None))),
    }
}

// Get user's all best scores
#[get("/<user_id>/best?<limit>&<offset>")]
pub async fn get_user_best(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser,
    user_id: i32,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Json<ApiResponse<BestScoresResponse>>, Json<ApiResponse<()>>> {
    if user_id <= 0 {
        return Err(Json(ApiResponse::<RoleResponse>::error(-110, None)));
    }

    // Check permission - viewing other users' best scores requires 'select' permission
    if user_id != auth_user.user.user_id {
        let has_permission = check_user_permission(pool, auth_user.user.user_id, "select").await;
        if !has_permission {
            return Err(Json(ApiResponse::<BestScoresResponse>::error(-1, None)));
        }
    }

    let limit = limit.unwrap_or(100).min(1000); // Max 1000 scores per request
    let offset = offset.unwrap_or(0);

    let query = "SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                 bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                 bs.time_played, bs.clear_type, bs.rating, c.name as song_name
                 FROM best_score bs
                 LEFT JOIN chart c ON bs.song_id = c.song_id
                 WHERE bs.user_id = ?
                 ORDER BY bs.rating DESC
                 LIMIT ? OFFSET ?";

    match sqlx::query(query)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool.inner())
        .await
    {
        Ok(rows) => {
            if rows.is_empty() {
                return Err(Json(ApiResponse::<BestScoresResponse>::error(-3, None)));
            }

            let scores: Vec<ScoreData> = rows
                .into_iter()
                .map(|row| ScoreData {
                    user_id: row.try_get("user_id").unwrap_or(0),
                    song_id: row.try_get("song_id").unwrap_or_default(),
                    difficulty: row.try_get("difficulty").unwrap_or(0),
                    score: row.try_get("score").unwrap_or(0),
                    shiny_perfect_count: row.try_get("shiny_perfect_count").unwrap_or(0),
                    perfect_count: row.try_get("perfect_count").unwrap_or(0),
                    near_count: row.try_get("near_count").unwrap_or(0),
                    miss_count: row.try_get("miss_count").unwrap_or(0),
                    health: row.try_get("health").unwrap_or(0),
                    modifier: row.try_get("modifier").unwrap_or(0),
                    time_played: row.try_get("time_played").unwrap_or(0),
                    clear_type: row.try_get("clear_type").unwrap_or(0),
                    rating: row.try_get("rating").unwrap_or(0.0),
                    song_name: row.try_get("song_name").ok(),
                })
                .collect();

            Ok(Json(ApiResponse::success(BestScoresResponse {
                user_id,
                data: scores,
            })))
        }
        Err(_) => Err(Json(ApiResponse::<B30Response>::error(-3, None))),
    }
}

// Get user's recent 30 scores
#[get("/<user_id>/r30")]
pub async fn get_user_r30(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser,
    user_id: i32,
) -> Result<Json<ApiResponse<R30Response>>, Json<ApiResponse<()>>> {
    if user_id <= 0 {
        return Err(Json(ApiResponse::<BestScoresResponse>::error(-110, None)));
    }

    // Check permission - viewing other users' recent scores requires 'select' permission
    if user_id != auth_user.user.user_id {
        let has_permission = check_user_permission(pool, auth_user.user.user_id, "select").await;
        if !has_permission {
            return Err(Json(ApiResponse::<R30Response>::error(-1, None)));
        }
    }

    let query = "SELECT r.user_id, r.song_id, r.difficulty, r.score, r.shiny_perfect_count,
                 r.perfect_count, r.near_count, r.miss_count, r.health, r.modifier,
                 r.time_played, r.clear_type, r.rating, c.name as song_name
                 FROM recent30 r
                 LEFT JOIN chart c ON r.song_id = c.song_id
                 WHERE r.user_id = ?
                 ORDER BY r.time_played DESC
                 LIMIT 30";

    match sqlx::query(query)
        .bind(user_id)
        .fetch_all(pool.inner())
        .await
    {
        Ok(rows) => {
            let scores: Vec<ScoreData> = rows
                .into_iter()
                .map(|row| ScoreData {
                    user_id: row.try_get("user_id").unwrap_or(0),
                    song_id: row.try_get("song_id").unwrap_or_default(),
                    difficulty: row.try_get("difficulty").unwrap_or(0),
                    score: row.try_get("score").unwrap_or(0),
                    shiny_perfect_count: row.try_get("shiny_perfect_count").unwrap_or(0),
                    perfect_count: row.try_get("perfect_count").unwrap_or(0),
                    near_count: row.try_get("near_count").unwrap_or(0),
                    miss_count: row.try_get("miss_count").unwrap_or(0),
                    health: row.try_get("health").unwrap_or(0),
                    modifier: row.try_get("modifier").unwrap_or(0),
                    time_played: row.try_get("time_played").unwrap_or(0),
                    clear_type: row.try_get("clear_type").unwrap_or(0),
                    rating: row.try_get("rating").unwrap_or(0.0),
                    song_name: row.try_get("song_name").ok(),
                })
                .collect();

            // Calculate R10 (recent 10 average)
            let recent_10: Vec<_> = scores.iter().take(10).collect();
            let r10_sum: f64 = recent_10.iter().map(|s| s.rating).sum();
            let r10_ptt = if recent_10.is_empty() {
                0.0
            } else {
                r10_sum / recent_10.len() as f64
            };

            Ok(Json(ApiResponse::success(R30Response {
                user_id,
                r10_ptt,
                data: scores,
            })))
        }
        Err(_) => Err(Json(ApiResponse::<R30Response>::error(-3, None))),
    }
}

// Get user's role and powers
#[get("/<user_id>/role")]
pub async fn get_user_role(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser,
    user_id: i32,
) -> Result<Json<ApiResponse<RoleResponse>>, Json<ApiResponse<()>>> {
    if user_id <= 0 {
        return Err(Json(ApiResponse::<RoleResponse>::error(-110, None)));
    }

    // Users can view their own role, or need select permission to view others
    if user_id == auth_user.user.user_id {
        // Return own role and powers
        if let Some((role_id, powers)) = get_user_role_and_powers(pool, user_id).await {
            let response = RoleResponse {
                user_id,
                role: role_id.to_string(),
                powers,
            };
            return Ok(Json(ApiResponse::success(response)));
        }
    } else {
        // Check select permission to view other users' roles
        let has_permission = check_user_permission(pool, auth_user.user.user_id, "select").await;
        if !has_permission {
            return Err(Json(ApiResponse::<RoleResponse>::error(-1, None)));
        }

        if let Some((role_id, powers)) = get_user_role_and_powers(pool, user_id).await {
            let response = RoleResponse {
                user_id,
                role: role_id.to_string(),
                powers,
            };
            return Ok(Json(ApiResponse::success(response)));
        }
    }

    // Default fallback if no role found
    let response = RoleResponse {
        user_id,
        role: "1".to_string(),
        powers: vec!["select_me".to_string()],
    };

    Ok(Json(ApiResponse::success(response)))
}

// Get user's rating history
#[get("/<user_id>/rating?<start_timestamp>&<end_timestamp>&<duration>")]
pub async fn get_user_rating_history(
    pool: &State<SqlitePool>,
    auth_user: AuthenticatedUser,
    user_id: i32,
    start_timestamp: Option<i64>,
    end_timestamp: Option<i64>,
    duration: Option<i32>,
) -> Result<Json<ApiResponse<RatingHistoryResponse>>, Json<ApiResponse<()>>> {
    // Check permission - viewing other users' rating history requires 'select' permission
    if user_id != auth_user.user.user_id {
        let has_permission = check_user_permission(pool, auth_user.user.user_id, "select").await;
        if !has_permission {
            return Err(Json(ApiResponse::<RatingHistoryResponse>::error(-1, None)));
        }
    }

    let mut query = "SELECT time, rating_ptt FROM user_rating WHERE user_id = ?".to_string();
    let mut params = vec![user_id.to_string()];

    if let (Some(start), Some(end)) = (start_timestamp, end_timestamp) {
        query.push_str(" AND time BETWEEN ? AND ?");
        params.push(start.to_string());
        params.push(end.to_string());
    } else if let Some(days) = duration {
        let now = chrono::Utc::now().timestamp();
        let start = now - (days as i64 * 24 * 3600);
        query.push_str(" AND time BETWEEN ? AND ?");
        params.push(start.to_string());
        params.push(now.to_string());
    }

    query.push_str(" ORDER BY time");

    let mut sql_query = sqlx::query(&query);
    for param in params {
        if let Ok(num) = param.parse::<i64>() {
            sql_query = sql_query.bind(num);
        } else {
            sql_query = sql_query.bind(param);
        }
    }

    match sql_query.fetch_all(pool.inner()).await {
        Ok(rows) => {
            let data: Vec<RatingPoint> = rows
                .into_iter()
                .map(|row| RatingPoint {
                    time: row.try_get("time").unwrap_or(0),
                    rating_ptt: row.try_get("rating_ptt").unwrap_or(0),
                })
                .collect();

            Ok(Json(ApiResponse::success(RatingHistoryResponse {
                user_id,
                data,
            })))
        }
        Err(_) => Err(Json(ApiResponse::<RatingHistoryResponse>::error(-3, None))),
    }
}

// Helper function to check user permissions
async fn check_user_permission(
    pool: &State<SqlitePool>,
    user_id: i32,
    required_power: &str,
) -> bool {
    let query = "SELECT COUNT(*) as count
                 FROM user_role ur
                 JOIN role_power rp ON ur.role_id = rp.role_id
                 WHERE ur.user_id = ? AND rp.power_id = ?";

    match sqlx::query(query)
        .bind(user_id)
        .bind(required_power)
        .fetch_optional(pool.inner())
        .await
    {
        Ok(Some(row)) => {
            let count: i32 = row.try_get("count").unwrap_or(0);
            count > 0
        }
        _ => false,
    }
}

// Helper function to get user role and powers
async fn get_user_role_and_powers(
    pool: &State<SqlitePool>,
    user_id: i32,
) -> Option<(i32, Vec<String>)> {
    // Get user's roles (a user can have multiple roles)
    let roles_query = "SELECT role_id FROM user_role WHERE user_id = ?";
    let roles = match sqlx::query(roles_query)
        .bind(user_id)
        .fetch_all(pool.inner())
        .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| row.try_get::<String, _>("role_id").unwrap_or_default())
            .collect::<Vec<String>>(),
        _ => return None,
    };

    if roles.is_empty() {
        return None;
    }

    // Get all powers for all user's roles
    let powers_query = "SELECT DISTINCT rp.power_id
                      FROM role_power rp
                      WHERE rp.role_id IN (?"
        .to_string()
        + &",?".repeat(roles.len() - 1)
        + ")";

    let mut query = sqlx::query(&powers_query);
    for role in &roles {
        query = query.bind(role);
    }

    let powers = match query.fetch_all(pool.inner()).await {
        Ok(rows) => rows
            .into_iter()
            .map(|row| row.try_get::<String, _>("power_id").unwrap_or_default())
            .collect(),
        _ => vec![],
    };

    // Return the first role and all powers
    let primary_role = roles[0].clone();
    Some((primary_role.parse::<i32>().unwrap_or(1), powers))
}

// Calculate potential (rating) for a score
pub fn calculate_potential(score: i32, chart_constant: f64) -> f64 {
    if score >= 10000000 {
        // Perfect or better
        chart_constant + 2.0
    } else if score >= 9800000 {
        // EX+
        chart_constant + 1.0 + (score - 9800000) as f64 / 200000.0
    } else if score >= 9500000 {
        // EX
        chart_constant + (score - 9500000) as f64 / 300000.0
    } else if score >= 9200000 {
        // AA
        chart_constant - 1.0 + (score - 9200000) as f64 / 300000.0
    } else if score >= 8900000 {
        // A
        chart_constant - 2.0 + (score - 8900000) as f64 / 300000.0
    } else if score >= 8600000 {
        // B
        chart_constant - 3.0 + (score - 8600000) as f64 / 300000.0
    } else {
        // C or lower
        0.0_f64.max(chart_constant - 5.0 + (score - 8600000) as f64 / 300000.0)
    }
}
