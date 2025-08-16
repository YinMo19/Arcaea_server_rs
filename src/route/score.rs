use crate::error::ArcResult;
use crate::model::download::{CourseTokenRequest, ScoreSubmission, WorldTokenRequest};
use crate::route::common::AuthGuard;
use crate::service::score::ScoreService;
use rocket::form::{Form, FromForm};
use rocket::serde::json::Json;
use rocket::{get, post, routes, Route, State};

use serde_json::Value;
use std::collections::HashMap;

/// Score routes
pub fn routes() -> Vec<Route> {
    routes![
        score_token,
        score_token_world,
        score_token_course,
        song_score_post,
        song_score_top,
        song_score_me,
        song_score_friend
    ]
}

/// Get basic score token (hardcoded bypass)
///
/// This endpoint returns a hardcoded token that bypasses normal validation.
/// Used for development and testing purposes.
#[get("/score/token")]
pub async fn score_token(
    score_service: &State<ScoreService>,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let token = score_service.get_score_token().await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));

    let mut value = HashMap::new();
    value.insert("token".to_string(), Value::String(token));

    response.insert("value".to_string(), serde_json::to_value(value)?);

    Ok(Json(response))
}

/// Get world mode score token
///
/// This endpoint generates a token for world mode play, handling stamina costs,
/// skill effects, and invasion mechanics. It validates the user's stamina and
/// current world map state before allowing play.
#[get("/score/token/world?<song_id>&<difficulty>&<stamina_multiply>&<fragment_multiply>&<prog_boost_multiply>&<beyond_boost_gauge_use>&<skill_id>&<is_skill_sealed>")]
pub async fn score_token_world(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    song_id: String,
    difficulty: i32,
    stamina_multiply: Option<i32>,
    fragment_multiply: Option<i32>,
    prog_boost_multiply: Option<i32>,
    beyond_boost_gauge_use: Option<i32>,
    skill_id: Option<String>,
    is_skill_sealed: Option<String>,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let request = WorldTokenRequest {
        song_id,
        difficulty,
        stamina_multiply,
        fragment_multiply,
        prog_boost_multiply,
        beyond_boost_gauge_use,
        skill_id,
        is_skill_sealed,
    };

    let token_response = score_service
        .get_world_score_token(user_auth.user_id, request)
        .await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));
    response.insert("value".to_string(), serde_json::to_value(token_response)?);

    Ok(Json(response))
}

/// Get course mode score token
///
/// This endpoint manages course mode sessions, including creating new sessions,
/// continuing existing ones, and handling course completion. It manages stamina
/// costs and course skip purchases.
#[get("/score/token/course?<course_id>&<previous_token>&<use_course_skip_purchase>")]
pub async fn score_token_course(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    course_id: Option<String>,
    previous_token: Option<String>,
    use_course_skip_purchase: Option<String>,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let request = CourseTokenRequest {
        course_id,
        previous_token,
        use_course_skip_purchase: Some(use_course_skip_purchase.as_deref() == Some("true")),
    };

    let token_response = score_service
        .get_course_score_token(user_auth.user_id, request)
        .await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));
    response.insert("value".to_string(), serde_json::to_value(token_response)?);

    Ok(Json(response))
}

#[derive(FromForm)]
pub struct ScoreSubmissionForm {
    pub song_token: String,
    pub song_hash: String,
    pub song_id: String,
    pub difficulty: i32,
    pub score: i32,
    pub shiny_perfect_count: i32,
    pub perfect_count: i32,
    pub near_count: i32,
    pub miss_count: i32,
    pub health: i32,
    pub modifier: i32,
    pub clear_type: i32,
    pub beyond_gauge: i32,
    pub submission_hash: String,
    pub combo_interval_bonus: Option<i32>,
    pub hp_interval_bonus: Option<i32>,
    pub highest_health: Option<i32>,
    pub lowest_health: Option<i32>,
}

/// Submit a score
///
/// This endpoint handles score submission for both world mode and course mode.
/// It validates the score data, updates user records, calculates ratings,
/// and manages recent30/best score records.
#[post("/score/song", data = "<form>")]
pub async fn song_score_post(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    form: Form<ScoreSubmissionForm>,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let submission = ScoreSubmission {
        song_token: form.song_token.clone(),
        song_hash: form.song_hash.clone(),
        song_id: form.song_id.clone(),
        difficulty: form.difficulty,
        score: form.score,
        shiny_perfect_count: form.shiny_perfect_count,
        perfect_count: form.perfect_count,
        near_count: form.near_count,
        miss_count: form.miss_count,
        health: form.health,
        modifier: form.modifier,
        clear_type: form.clear_type,
        beyond_gauge: form.beyond_gauge,
        submission_hash: form.submission_hash.clone(),
        combo_interval_bonus: form.combo_interval_bonus,
        hp_interval_bonus: form.hp_interval_bonus,
        highest_health: form.highest_health,
        lowest_health: form.lowest_health,
    };

    let result = score_service
        .submit_score(user_auth.user_id, submission)
        .await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));
    response.insert("value".to_string(), serde_json::to_value(result)?);

    Ok(Json(response))
}

/// Get top 20 scores for a song
///
/// This endpoint returns the highest 20 scores for a specific song and difficulty,
/// including user information and rankings.
#[get("/score/song?<song_id>&<difficulty>")]
pub async fn song_score_top(
    _user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    song_id: String,
    difficulty: i32,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let scores = score_service
        .get_song_top_scores(&song_id, difficulty)
        .await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));
    response.insert("value".to_string(), serde_json::to_value(scores)?);

    Ok(Json(response))
}

/// Get user's ranking for a song
///
/// This endpoint returns the authenticated user's score and ranking
/// for a specific song and difficulty.
#[get("/score/song/me?<song_id>&<difficulty>")]
pub async fn song_score_me(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    song_id: String,
    difficulty: i32,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let scores = score_service
        .get_user_song_rank(user_auth.user_id, &song_id, difficulty)
        .await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));
    response.insert("value".to_string(), serde_json::to_value(scores)?);

    Ok(Json(response))
}

/// Get friend rankings for a song
///
/// This endpoint returns scores from the user's friends for a specific
/// song and difficulty, limited to 50 entries.
#[get("/score/song/friend?<song_id>&<difficulty>")]
pub async fn song_score_friend(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    song_id: String,
    difficulty: i32,
) -> ArcResult<Json<HashMap<String, Value>>> {
    let scores = score_service
        .get_friend_song_ranks(user_auth.user_id, &song_id, difficulty)
        .await?;

    let mut response = HashMap::new();
    response.insert("success".to_string(), Value::Bool(true));
    response.insert("value".to_string(), serde_json::to_value(scores)?);

    Ok(Json(response))
}
