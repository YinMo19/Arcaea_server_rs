use crate::model::download::{CourseTokenRequest, ScoreSubmission, WorldTokenRequest};
use crate::model::{CourseTokenResponse, WorldTokenResponse};
use crate::route::common::AuthGuard;
use crate::route::{success_return, RouteResult};
use crate::service::score::ScoreService;
use rocket::form::Form;
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
pub async fn score_token() -> RouteResult<Value> {
    Ok(success_return(
        serde_json::json!({"token": "1145141919810"}),
    ))
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
) -> RouteResult<WorldTokenResponse> {
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

    Ok(success_return(token_response))
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
) -> RouteResult<CourseTokenResponse> {
    let request = CourseTokenRequest {
        course_id,
        previous_token,
        use_course_skip_purchase: Some(use_course_skip_purchase.as_deref() == Some("true")),
    };

    let token_response = score_service
        .get_course_score_token(user_auth.user_id, request)
        .await?;

    Ok(success_return(token_response))
}

/// Submit a score
///
/// This endpoint handles score submission for both world mode and course mode.
/// It validates the score data, updates user records, calculates ratings,
/// and manages recent30/best score records.
#[post("/score/song", data = "<submission>")]
pub async fn song_score_post(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    submission: Form<ScoreSubmission>,
) -> RouteResult<HashMap<String, Value>> {
    let result = score_service
        .submit_score(user_auth.user_id, submission.into_inner())
        .await?;

    Ok(success_return(result))
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
) -> RouteResult<Vec<HashMap<String, Value>>> {
    let scores = score_service
        .get_song_top_scores(&song_id, difficulty)
        .await?;

    Ok(success_return(scores))
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
) -> RouteResult<Vec<HashMap<String, Value>>> {
    let scores = score_service
        .get_user_song_rank(user_auth.user_id, &song_id, difficulty)
        .await?;

    Ok(success_return(scores))
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
) -> RouteResult<Vec<HashMap<String, Value>>> {
    let scores = score_service
        .get_friend_song_ranks(user_auth.user_id, &song_id, difficulty)
        .await?;

    Ok(success_return(scores))
}
