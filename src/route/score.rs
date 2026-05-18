use crate::error::ArcError;
use crate::model::download::{CourseTokenRequest, ScoreSubmission, WorldTokenRequest};
use crate::model::{CourseTokenResponse, WorldTokenResponse};
use crate::route::common::AuthGuard;
use crate::route::{success_return, RouteResult};
use crate::service::score::ScoreService;
use rocket::form::Form;
use rocket::{get, post, routes, FromForm, Route, State};

use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, FromForm)]
pub struct ScoreSubmissionForm {
    pub song_token: Option<String>,
    pub song_hash: Option<String>,
    pub song_id: Option<String>,
    pub difficulty: Option<i32>,
    pub score: Option<i32>,
    pub shiny_perfect_count: Option<i32>,
    pub perfect_count: Option<i32>,
    pub near_count: Option<i32>,
    pub miss_count: Option<i32>,
    pub health: Option<i32>,
    pub modifier: Option<i32>,
    pub clear_type: Option<i32>,
    pub beyond_gauge: Option<i32>,
    pub submission_hash: Option<String>,
    pub combo_interval_bonus: Option<i32>,
    pub hp_interval_bonus: Option<i32>,
    pub fever_bonus: Option<i32>,
    pub rank_bonus: Option<i32>,
    pub maya_gauge: Option<i32>,
    pub nextstage_bonus: Option<i32>,
    pub highest_health: Option<i32>,
    pub lowest_health: Option<i32>,
    pub room_code: Option<String>,
    pub room_total_score: Option<i32>,
    pub room_total_players: Option<i32>,
}

impl TryFrom<ScoreSubmissionForm> for ScoreSubmission {
    type Error = ArcError;

    fn try_from(form: ScoreSubmissionForm) -> Result<Self, Self::Error> {
        Ok(Self {
            song_token: form.song_token.ok_or_else(score_submission_error)?,
            song_hash: form.song_hash.ok_or_else(score_submission_error)?,
            song_id: form.song_id.ok_or_else(score_submission_error)?,
            difficulty: form.difficulty.ok_or_else(score_submission_error)?,
            score: form.score.ok_or_else(score_submission_error)?,
            shiny_perfect_count: form
                .shiny_perfect_count
                .ok_or_else(score_submission_error)?,
            perfect_count: form.perfect_count.ok_or_else(score_submission_error)?,
            near_count: form.near_count.ok_or_else(score_submission_error)?,
            miss_count: form.miss_count.ok_or_else(score_submission_error)?,
            health: form.health.ok_or_else(score_submission_error)?,
            modifier: form.modifier.ok_or_else(score_submission_error)?,
            clear_type: form.clear_type.ok_or_else(score_submission_error)?,
            beyond_gauge: form.beyond_gauge.ok_or_else(score_submission_error)?,
            submission_hash: form.submission_hash.ok_or_else(score_submission_error)?,
            combo_interval_bonus: form.combo_interval_bonus,
            hp_interval_bonus: form.hp_interval_bonus,
            fever_bonus: form.fever_bonus,
            rank_bonus: form.rank_bonus,
            maya_gauge: form.maya_gauge,
            nextstage_bonus: form.nextstage_bonus,
            highest_health: form.highest_health,
            lowest_health: form.lowest_health,
            room_code: form.room_code,
            room_total_score: form.room_total_score,
            room_total_players: form.room_total_players,
        })
    }
}

fn score_submission_error() -> ArcError {
    ArcError::rocket_err("Missing score submission field")
}

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
#[get("/score/token/world?<song_id>&<difficulty>&<stamina_multiply>&<fragment_multiply>&<prog_boost_multiply>&<beyond_boost_gauge_use>&<character_id>&<is_char_uncapped_override>&<select_session>&<skill_id>&<is_skill_sealed>")]
#[allow(clippy::too_many_arguments)]
pub async fn score_token_world(
    user_auth: AuthGuard,
    score_service: &State<ScoreService>,
    song_id: String,
    difficulty: i32,
    stamina_multiply: Option<i32>,
    fragment_multiply: Option<i32>,
    prog_boost_multiply: Option<i32>,
    beyond_boost_gauge_use: Option<i32>,
    character_id: Option<i32>,
    is_char_uncapped_override: Option<String>,
    select_session: Option<String>,
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
        character_id,
        is_char_uncapped_override,
        select_session,
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
        use_course_skip_purchase: use_course_skip_purchase.as_deref() == Some("true"),
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
    submission: Form<ScoreSubmissionForm>,
) -> RouteResult<HashMap<String, Value>> {
    let submission = submission.into_inner().try_into()?;
    let result = score_service
        .submit_score(user_auth.user_id, submission)
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
