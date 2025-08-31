use crate::error::{ArcError, ArcResult};
use serde_json::Value;

use crate::model::download::{
    BestScore, CourseTokenRequest, CourseTokenResponse, ScoreSubmission, SongplayToken,
    WorldTokenRequest, WorldTokenResponse,
};
use crate::model::score::{Potential, Recent30Tuple, Score, UserPlay, UserScore};
use crate::model::user::User;
use base64::{engine::general_purpose, Engine as _};
use md5;
use rand::Rng;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Constants for score calculations
const BEST30_WEIGHT: f64 = 0.75;
const RECENT10_WEIGHT: f64 = 0.25;
const COURSE_STAMINA_COST: i32 = 2;
const INVASION_START_WEIGHT: f64 = 0.1;
const INVASION_HARD_WEIGHT: f64 = 0.05;

/// Score service for handling score submission, validation, and calculations
pub struct ScoreService {
    pool: MySqlPool,
}

impl ScoreService {
    /// Create a new score service instance
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Generate a simple score token (hardcoded for bypass)
    pub async fn get_score_token(&self) -> ArcResult<String> {
        Ok("1145141919810".to_string())
    }

    /// Generate world mode score token with stamina and skill validation
    pub async fn get_world_score_token(
        &self,
        user_id: i32,
        request: WorldTokenRequest,
    ) -> ArcResult<WorldTokenResponse> {
        let user = self.get_user_info(user_id).await?;

        let stamina_multiply = request.stamina_multiply.unwrap_or(1);
        let fragment_multiply = request.fragment_multiply.unwrap_or(100);
        let prog_boost_multiply = request.prog_boost_multiply.unwrap_or(0);
        let beyond_boost_gauge_use = request.beyond_boost_gauge_use.unwrap_or(0);

        // Handle special skills
        let mut skill_cytusii_flag: Option<String> = None;
        let mut skill_chinatsu_flag: Option<String> = None;
        let mut invasion_flag = 0;

        if let Some(skill_id) = &request.skill_id {
            if (skill_id == "skill_ilith_ivy" || skill_id == "skill_hikari_vanessa")
                && request.is_skill_sealed.as_deref() == Some("false")
            {
                skill_cytusii_flag = Some(generate_random_skill_flag(5));
            }

            if skill_id == "skill_chinatsu" && request.is_skill_sealed.as_deref() == Some("false") {
                skill_chinatsu_flag = Some(generate_random_skill_flag(7));
            }
        }

        // Check for invasion (random chance)
        if user.insight_state.unwrap_or(0) == 4 {
            let rand_val: f64 = rand::thread_rng().gen();
            if rand_val < INVASION_HARD_WEIGHT {
                invasion_flag = 2;
            } else if rand_val < INVASION_START_WEIGHT + INVASION_HARD_WEIGHT {
                invasion_flag = 1;
            }
        }

        // Validate stamina
        let stamina_cost = self.get_world_map_stamina_cost(user_id).await.unwrap_or(1);
        let required_stamina = stamina_cost * stamina_multiply;
        if user.stamina.unwrap_or(0) < required_stamina {
            return Err(ArcError::Base {
                message: format!(
                    "Stamina is not enough. Required: {}, Current: {}",
                    required_stamina,
                    user.stamina.unwrap_or(0)
                ),
                error_code: 108,
                api_error_code: -901,
                extra_data: None,
                status: 200,
            });
        }

        // Generate token
        let token = generate_song_token();

        // Clear existing tokens for user
        self.clear_user_songplay_tokens(user_id).await?;

        // Insert new token
        sqlx::query!(
            "INSERT INTO songplay_token VALUES (?, ?, ?, ?, '', -1, 0, 0, ?, ?, ?, ?, ?, ?, ?)",
            token,
            user_id,
            request.song_id,
            request.difficulty,
            stamina_multiply,
            fragment_multiply,
            prog_boost_multiply,
            beyond_boost_gauge_use,
            skill_cytusii_flag,
            skill_chinatsu_flag,
            invasion_flag
        )
        .execute(&self.pool)
        .await?;

        // Update user stamina
        sqlx::query!(
            "UPDATE user SET stamina = stamina - ? WHERE user_id = ?",
            required_stamina,
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Build play parameters
        let mut play_parameters = HashMap::new();

        if let Some(skill_flag) = skill_cytusii_flag.or(skill_chinatsu_flag) {
            if let Some(skill_id) = request.skill_id {
                let values: Vec<String> = skill_flag
                    .chars()
                    .map(|c| get_world_value_name(c.to_digit(10).unwrap_or(0) as i32))
                    .collect();
                play_parameters.insert(
                    skill_id,
                    Value::Array(values.into_iter().map(Value::String).collect()),
                );
            }
        }

        if invasion_flag == 1 {
            play_parameters.insert("invasion_start".to_string(), Value::Bool(true));
        } else if invasion_flag == 2 {
            play_parameters.insert("invasion_hard".to_string(), Value::Bool(true));
        }

        let updated_user = self.get_user_info(user_id).await?;

        Ok(WorldTokenResponse {
            stamina: updated_user.stamina.unwrap_or(0),
            max_stamina_ts: updated_user.max_stamina_ts.unwrap_or(0),
            token,
            play_parameters,
        })
    }

    /// Generate course mode score token
    pub async fn get_course_score_token(
        &self,
        user_id: i32,
        request: CourseTokenRequest,
    ) -> ArcResult<CourseTokenResponse> {
        let _user = self.get_user_info(user_id).await?;
        let use_course_skip_purchase = request.use_course_skip_purchase.unwrap_or(false);

        let mut status = "created".to_string();
        let token;

        if let Some(previous_token) = request.previous_token {
            // Check existing token
            let existing_token = sqlx::query!(
                "SELECT course_state FROM songplay_token WHERE token = ? AND user_id = ?",
                previous_token,
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;

            match existing_token {
                Some(row) => {
                    let course_state = row.course_state;
                    if let Some(state) = course_state {
                        if (0..=3).contains(&state) {
                            // Update token
                            token = generate_course_token();
                            sqlx::query!(
                                "UPDATE songplay_token SET token = ? WHERE token = ?",
                                token,
                                previous_token
                            )
                            .execute(&self.pool)
                            .await?;
                        } else {
                            // Course finished
                            self.clear_user_songplay_tokens(user_id).await?;
                            status = if state == 4 { "cleared" } else { "failed" }.to_string();
                            token = String::new();
                        }
                    } else {
                        return Err(ArcError::no_data("Invalid course state".to_string(), 108));
                    }
                }
                None => {
                    // Create new course session
                    if let Some(course_id) = request.course_id {
                        token = generate_course_token();
                        self.create_course_session(
                            user_id,
                            &course_id,
                            &token,
                            use_course_skip_purchase,
                        )
                        .await?;
                    } else {
                        return Err(ArcError::input("Course ID is required for new session"));
                    }
                }
            }
        } else {
            // Create new course session
            if let Some(course_id) = request.course_id {
                token = generate_course_token();
                self.create_course_session(user_id, &course_id, &token, use_course_skip_purchase)
                    .await?;
            } else {
                return Err(ArcError::input("Course ID is required for new session"));
            }
        }

        let updated_user = self.get_user_info(user_id).await?;

        Ok(CourseTokenResponse {
            stamina: updated_user.stamina.unwrap_or(0),
            max_stamina_ts: updated_user.max_stamina_ts.unwrap_or(0),
            token,
            status,
        })
    }

    /// Submit and validate a score
    pub async fn submit_score(
        &self,
        user_id: i32,
        submission: ScoreSubmission,
    ) -> ArcResult<HashMap<String, serde_json::Value>> {
        // Get user info
        let user = self.get_user_info(user_id).await?;

        // Validate token and get play state
        let play_state = self.get_play_state(&submission.song_token, user_id).await?;

        // Create UserPlay instance
        let mut user_play = UserPlay {
            user_score: UserScore {
                score: Score::new(),
                user_id,
                name: user.name.unwrap_or_default(),
                best_clear_type: 0,
                character: user.character_id.unwrap_or(0),
                is_char_uncapped: user.is_char_uncapped.unwrap_or(0),
                is_skill_sealed: user.is_skill_sealed.unwrap_or(0),
                rank: None,
            },
            song_token: submission.song_token.clone(),
            song_hash: submission.song_hash.clone(),
            submission_hash: submission.submission_hash.clone(),
            beyond_gauge: submission.beyond_gauge,
            unrank_flag: false,
            new_best_protect_flag: false,
            is_world_mode: Some(play_state.course_id.is_none()),
            stamina_multiply: play_state.stamina_multiply,
            fragment_multiply: play_state.fragment_multiply,
            prog_boost_multiply: play_state.prog_boost_multiply,
            beyond_boost_gauge_usage: play_state.beyond_boost_gauge_usage,
            course_play_state: play_state.course_state,
            combo_interval_bonus: submission.combo_interval_bonus,
            hp_interval_bonus: submission.hp_interval_bonus,
            fever_bonus: submission.fever_bonus,
            skill_cytusii_flag: play_state.skill_cytusii_flag,
            skill_chinatsu_flag: play_state.skill_chinatsu_flag,
            highest_health: submission.highest_health,
            lowest_health: submission.lowest_health,
            invasion_flag: play_state.invasion_flag,
            ptt: None,
        };

        // Set score data
        user_play.user_score.score.song_id = submission.song_id.clone();
        user_play.user_score.score.difficulty = submission.difficulty;
        user_play.user_score.score.set_score(
            Some(submission.score),
            Some(submission.shiny_perfect_count),
            Some(submission.perfect_count),
            Some(submission.near_count),
            Some(submission.miss_count),
            Some(submission.health),
            Some(submission.modifier),
            Some(current_timestamp()),
            Some(submission.clear_type),
        );

        // Get chart constant for rating calculation
        let chart_const = self
            .get_chart_constant(&submission.song_id, submission.difficulty)
            .await?;
        user_play.user_score.score.get_rating_by_calc(chart_const);

        // Validate score
        let expected_hash = self
            .get_song_file_hash(&submission.song_id, submission.difficulty)
            .await;
        if !user_play.is_valid(expected_hash.as_deref()) {
            return Err(ArcError::input("Invalid score"));
        }

        // Handle unranked scores
        if user_play.user_score.score.rating < 0.0 {
            user_play.unrank_flag = true;
            user_play.user_score.score.rating = 0.0;
        }

        // Set timestamp
        user_play.user_score.score.time_played = current_timestamp();

        // Upload score (update recent, best, and potential)
        self.upload_score(&mut user_play).await?;

        // Handle world mode
        if user_play.is_world_mode == Some(true) {
            self.handle_world_mode(&mut user_play).await?;
        }

        // Handle course mode
        if user_play.course_play_state >= 0 {
            self.handle_course_mode(&mut user_play).await?;
        }

        // Create potential instance for response
        user_play.ptt = Some(self.calculate_user_potential(user_id).await?);

        Ok(user_play.to_dict())
    }

    /// Get top 20 scores for a song
    pub async fn get_song_top_scores(
        &self,
        song_id: &str,
        difficulty: i32,
    ) -> ArcResult<Vec<HashMap<String, serde_json::Value>>> {
        let scores = sqlx::query!(
            "SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
             bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
             bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
             u.name, u.character_id, u.is_char_uncapped, u.is_skill_sealed
             FROM best_score bs
             JOIN user u ON bs.user_id = u.user_id
             WHERE bs.song_id = ? AND bs.difficulty = ?
             ORDER BY bs.score DESC
             LIMIT 20",
            song_id,
            difficulty
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for (rank, row) in scores.iter().enumerate() {
            let best_score = BestScore {
                user_id: row.user_id,
                song_id: row.song_id.clone(),
                difficulty: row.difficulty,
                score: row.score.unwrap_or(0),
                shiny_perfect_count: row.shiny_perfect_count.unwrap_or(0),
                perfect_count: row.perfect_count.unwrap_or(0),
                near_count: row.near_count.unwrap_or(0),
                miss_count: row.miss_count.unwrap_or(0),
                health: row.health.unwrap_or(0),
                modifier: row.modifier.unwrap_or(0),
                time_played: row.time_played.unwrap_or(0),
                best_clear_type: row.best_clear_type.unwrap_or(0),
                clear_type: row.clear_type.unwrap_or(0),
                rating: row.rating.unwrap_or(0.0),
                score_v2: row.score_v2.unwrap_or(0.0),
            };
            let user_info = (
                row.user_id,
                row.name.clone().unwrap_or_default(),
                row.character_id.unwrap_or(0),
                row.is_char_uncapped.unwrap_or(0),
                row.is_skill_sealed.unwrap_or(0),
            );
            let mut user_score = UserScore::from_best_score_row(&best_score, user_info);
            user_score.rank = Some((rank + 1) as i32);
            result.push(user_score.to_dict(true));
        }

        Ok(result)
    }

    /// Get user's rank for a song
    pub async fn get_user_song_rank(
        &self,
        user_id: i32,
        song_id: &str,
        difficulty: i32,
    ) -> ArcResult<Vec<HashMap<String, serde_json::Value>>> {
        // Get user's score
        let user_score = sqlx::query!(
            "SELECT * FROM best_score WHERE user_id = ? AND song_id = ? AND difficulty = ?",
            user_id,
            song_id,
            difficulty
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = user_score {
            // Get user's rank
            let rank = sqlx::query!(
                "SELECT COUNT(*) as rank FROM best_score
                 WHERE song_id = ? AND difficulty = ? AND score > ?",
                song_id,
                difficulty,
                row.score
            )
            .fetch_one(&self.pool)
            .await?;

            let user_info = self.get_user_info(user_id).await?;
            let user_score_info = (
                user_id,
                user_info.name.unwrap_or_default(),
                user_info.character_id.unwrap_or(0),
                user_info.is_char_uncapped.unwrap_or(0),
                user_info.is_skill_sealed.unwrap_or(0),
            );

            let best_score = BestScore {
                user_id: row.user_id,
                song_id: row.song_id,
                difficulty: row.difficulty,
                score: row.score.unwrap_or(0),
                shiny_perfect_count: row.shiny_perfect_count.unwrap_or(0),
                perfect_count: row.perfect_count.unwrap_or(0),
                near_count: row.near_count.unwrap_or(0),
                miss_count: row.miss_count.unwrap_or(0),
                health: row.health.unwrap_or(0),
                modifier: row.modifier.unwrap_or(0),
                time_played: row.time_played.unwrap_or(0),
                best_clear_type: row.best_clear_type.unwrap_or(0),
                clear_type: row.clear_type.unwrap_or(0),
                rating: row.rating.unwrap_or(0.0),
                score_v2: row.score_v2.unwrap_or(0.0),
            };
            let mut user_score_obj = UserScore::from_best_score_row(&best_score, user_score_info);
            user_score_obj.rank = Some((rank.rank + 1) as i32);

            Ok(vec![user_score_obj.to_dict(true)])
        } else {
            Ok(vec![])
        }
    }

    /// Get friend rankings for a song
    pub async fn get_friend_song_ranks(
        &self,
        user_id: i32,
        song_id: &str,
        difficulty: i32,
    ) -> ArcResult<Vec<HashMap<String, serde_json::Value>>> {
        let scores = sqlx::query!(
            "SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
             bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
             bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
             u.name, u.character_id, u.is_char_uncapped, u.is_skill_sealed
             FROM best_score bs
             JOIN user u ON bs.user_id = u.user_id
             JOIN friend f ON (f.user_id_me = ? AND f.user_id_other = bs.user_id)
             WHERE bs.song_id = ? AND bs.difficulty = ?
             ORDER BY bs.score DESC
             LIMIT 50",
            user_id,
            song_id,
            difficulty
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for (rank, row) in scores.iter().enumerate() {
            let best_score = BestScore {
                user_id: row.user_id,
                song_id: row.song_id.clone(),
                difficulty: row.difficulty,
                score: row.score.unwrap_or(0),
                shiny_perfect_count: row.shiny_perfect_count.unwrap_or(0),
                perfect_count: row.perfect_count.unwrap_or(0),
                near_count: row.near_count.unwrap_or(0),
                miss_count: row.miss_count.unwrap_or(0),
                health: row.health.unwrap_or(0),
                modifier: row.modifier.unwrap_or(0),
                time_played: row.time_played.unwrap_or(0),
                best_clear_type: row.best_clear_type.unwrap_or(0),
                clear_type: row.clear_type.unwrap_or(0),
                rating: row.rating.unwrap_or(0.0),
                score_v2: row.score_v2.unwrap_or(0.0),
            };
            let user_info = (
                row.user_id,
                row.name.clone().unwrap_or_default(),
                row.character_id.unwrap_or(0),
                row.is_char_uncapped.unwrap_or(0),
                row.is_skill_sealed.unwrap_or(0),
            );
            let mut user_score = UserScore::from_best_score_row(&best_score, user_info);
            user_score.rank = Some((rank + 1) as i32);
            result.push(user_score.to_dict(true));
        }

        Ok(result)
    }

    // Helper methods

    async fn get_user_info(&self, user_id: i32) -> ArcResult<User> {
        sqlx::query_as!(User, "SELECT * FROM user WHERE user_id = ?", user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ArcError::no_data(format!("User not found: {}", e), 108))
    }

    async fn get_play_state(&self, token: &str, user_id: i32) -> ArcResult<SongplayToken> {
        let result = sqlx::query!(
            "SELECT token, user_id, song_id, difficulty, course_id, course_state, course_score, course_clear_type, stamina_multiply, fragment_multiply, prog_boost_multiply, beyond_boost_gauge_usage, skill_cytusii_flag, skill_chinatsu_flag, invasion_flag FROM songplay_token WHERE token = ? AND user_id = ?",
            token,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ArcError::no_data("Invalid token".to_string(), 108))?;

        Ok(SongplayToken {
            token: result.token,
            user_id: result.user_id.unwrap_or(0),
            song_id: result.song_id.unwrap_or_default(),
            difficulty: result.difficulty.unwrap_or(0),
            course_id: result.course_id,
            course_state: result.course_state.unwrap_or(-1),
            course_score: result.course_score.unwrap_or(0),
            course_clear_type: result.course_clear_type.unwrap_or(0),
            stamina_multiply: result.stamina_multiply.unwrap_or(1),
            fragment_multiply: result.fragment_multiply.unwrap_or(100),
            prog_boost_multiply: result.prog_boost_multiply.unwrap_or(0),
            beyond_boost_gauge_usage: result.beyond_boost_gauge_usage.unwrap_or(0),
            skill_cytusii_flag: result.skill_cytusii_flag,
            skill_chinatsu_flag: result.skill_chinatsu_flag,
            invasion_flag: result.invasion_flag.unwrap_or(0),
        })
    }

    async fn get_chart_constant(&self, song_id: &str, difficulty: i32) -> ArcResult<f64> {
        let chart = sqlx::query!(
            "SELECT rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr FROM chart WHERE song_id = ?",
            song_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(chart) = chart {
            let rating = match difficulty {
                0 => chart.rating_pst,
                1 => chart.rating_prs,
                2 => chart.rating_ftr,
                3 => chart.rating_byn,
                4 => chart.rating_etr,
                _ => None,
            };

            Ok(rating.unwrap_or(-1) as f64 / 10.0)
        } else {
            Ok(-1.0)
        }
    }

    async fn get_song_file_hash(&self, _song_id: &str, _difficulty: i32) -> Option<String> {
        // TODO: Implement actual file hash calculation
        None
    }

    async fn get_world_map_stamina_cost(&self, user_id: i32) -> ArcResult<i32> {
        // Get user's current map
        let user = sqlx::query!("SELECT current_map FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(user_row) = user {
            if let Some(current_map) = user_row.current_map {
                // TODO: Load map data from JSON files and get stamina cost
                // For now, return default stamina cost based on map
                if current_map.contains("beyond") {
                    Ok(2)
                } else {
                    Ok(1)
                }
            } else {
                Ok(1) // Default stamina cost if no current map
            }
        } else {
            Err(ArcError::no_data("User not found".to_string(), 108))
        }
    }

    async fn clear_user_songplay_tokens(&self, user_id: i32) -> ArcResult<()> {
        sqlx::query!("DELETE FROM songplay_token WHERE user_id = ?", user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_course_session(
        &self,
        user_id: i32,
        course_id: &str,
        token: &str,
        use_skip_purchase: bool,
    ) -> ArcResult<()> {
        if use_skip_purchase {
            // TODO: Handle course skip purchase
        } else {
            // Check stamina
            let user = self.get_user_info(user_id).await?;
            if user.stamina.unwrap_or(0) < COURSE_STAMINA_COST {
                return Err(ArcError::Base {
                    message: "Stamina is not enough".to_string(),
                    error_code: 108,
                    api_error_code: -901,
                    extra_data: None,
                    status: 200,
                });
            }

            // Deduct stamina
            sqlx::query!(
                "UPDATE user SET stamina = stamina - ? WHERE user_id = ?",
                COURSE_STAMINA_COST,
                user_id
            )
            .execute(&self.pool)
            .await?;
        }

        // Insert course token
        sqlx::query!(
            "INSERT INTO songplay_token VALUES (?, ?, '', 0, ?, 0, 0, 3, 1, 100, 0, 0, '', '', 0)",
            token,
            user_id,
            course_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn upload_score(&self, user_play: &mut UserPlay) -> ArcResult<()> {
        let user_id = user_play.user_score.user_id;
        let score = &user_play.user_score.score;

        // Record score to log database (placeholder for now)
        self.record_score(user_play).await?;

        // Update user recent score
        sqlx::query!(
            "UPDATE user SET song_id = ?, difficulty = ?, score = ?, shiny_perfect_count = ?,
             perfect_count = ?, near_count = ?, miss_count = ?, health = ?, modifier = ?,
             clear_type = ?, rating = ?, time_played = ? WHERE user_id = ?",
            score.song_id,
            score.difficulty,
            score.score,
            score.shiny_perfect_count,
            score.perfect_count,
            score.near_count,
            score.miss_count,
            score.health,
            score.modifier,
            score.clear_type,
            score.rating,
            score.time_played * 1000, // Convert to milliseconds
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Handle best score update
        self.update_best_score(user_play).await?;

        // Update recent 30 if not unranked
        if !user_play.unrank_flag {
            self.update_recent_30(user_play).await?;
        }

        // Update user rating
        self.update_user_rating(user_id).await?;

        Ok(())
    }

    async fn update_best_score(&self, user_play: &mut UserPlay) -> ArcResult<()> {
        let user_id = user_play.user_score.user_id;
        let score = &user_play.user_score.score;

        let existing = sqlx::query!(
            "SELECT score, best_clear_type FROM best_score WHERE user_id = ? AND song_id = ? AND difficulty = ?",
            user_id,
            score.song_id,
            score.difficulty
        )
        .fetch_optional(&self.pool)
        .await?;

        match existing {
            None => {
                // New score
                user_play.new_best_protect_flag = true;
                sqlx::query!(
                    "INSERT INTO best_score VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    user_id,
                    score.song_id,
                    score.difficulty,
                    score.score,
                    score.shiny_perfect_count,
                    score.perfect_count,
                    score.near_count,
                    score.miss_count,
                    score.health,
                    score.modifier,
                    score.time_played,
                    score.clear_type,
                    score.clear_type,
                    score.rating,
                    score.score_v2
                )
                .execute(&self.pool)
                .await?;
            }
            Some(existing_score) => {
                // Update best clear type if better
                if score.song_state()
                    > Score::get_song_state(existing_score.best_clear_type.unwrap_or(0))
                {
                    sqlx::query!(
                        "UPDATE best_score SET best_clear_type = ? WHERE user_id = ? AND song_id = ? AND difficulty = ?",
                        score.clear_type,
                        user_id,
                        score.song_id,
                        score.difficulty
                    )
                    .execute(&self.pool)
                    .await?;
                }

                // Update score if better
                if score.score >= existing_score.score.unwrap_or(0) {
                    user_play.new_best_protect_flag = true;
                    sqlx::query!(
                        "UPDATE best_score SET score = ?, shiny_perfect_count = ?, perfect_count = ?,
                         near_count = ?, miss_count = ?, health = ?, modifier = ?, clear_type = ?,
                         rating = ?, time_played = ?, score_v2 = ?
                         WHERE user_id = ? AND song_id = ? AND difficulty = ?",
                        score.score,
                        score.shiny_perfect_count,
                        score.perfect_count,
                        score.near_count,
                        score.miss_count,
                        score.health,
                        score.modifier,
                        score.clear_type,
                        score.rating,
                        score.time_played,
                        score.score_v2,
                        user_id,
                        score.song_id,
                        score.difficulty
                    )
                    .execute(&self.pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    async fn update_recent_30(&self, user_play: &UserPlay) -> ArcResult<()> {
        let user_id = user_play.user_score.user_id;
        let score = &user_play.user_score.score;

        // Get current recent30 tuples
        let current_tuples = self.get_recent30_tuples(user_id).await?;

        // Handle recent30 based on Python logic
        if current_tuples.len() < 30 {
            // Simple case: add new entry
            self.insert_recent30_entry(user_id, current_tuples.len() as i32, score)
                .await?;
        } else {
            // Complex case: apply replacement logic
            self.apply_recent30_replacement_logic(user_id, user_play, &current_tuples)
                .await?;
        }

        Ok(())
    }

    async fn get_recent30_tuples(&self, user_id: i32) -> ArcResult<Vec<Recent30Tuple>> {
        let rows = sqlx::query!(
            "SELECT r_index, song_id, difficulty, rating FROM recent30
             WHERE user_id = ? AND song_id != '' ORDER BY time_played DESC",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                Recent30Tuple::new(
                    row.r_index,
                    row.song_id.unwrap_or_else(|| "".to_string()),
                    row.difficulty.unwrap_or(0),
                    row.rating.unwrap_or(0.0),
                )
            })
            .collect())
    }

    async fn insert_recent30_entry(
        &self,
        user_id: i32,
        r_index: i32,
        score: &Score,
    ) -> ArcResult<()> {
        sqlx::query!(
            "INSERT INTO recent30 (user_id, r_index, time_played, song_id, difficulty, score,
             shiny_perfect_count, perfect_count, near_count, miss_count, health, modifier,
             clear_type, rating) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            user_id,
            r_index,
            score.time_played,
            score.song_id,
            score.difficulty,
            score.score,
            score.shiny_perfect_count,
            score.perfect_count,
            score.near_count,
            score.miss_count,
            score.health,
            score.modifier,
            score.clear_type,
            score.rating
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn apply_recent30_replacement_logic(
        &self,
        user_id: i32,
        user_play: &UserPlay,
        current_tuples: &[Recent30Tuple],
    ) -> ArcResult<()> {
        let score = &user_play.user_score.score;
        let song_key = (score.song_id.clone(), score.difficulty);

        // Build unique songs map
        let mut unique_songs: std::collections::HashMap<(String, i32), Vec<(usize, i32, f64)>> =
            std::collections::HashMap::new();

        for (i, tuple) in current_tuples.iter().enumerate() {
            let key = (tuple.song_id.clone(), tuple.difficulty);
            unique_songs
                .entry(key)
                .or_insert_with(Vec::new)
                .push((i, tuple.r_index, tuple.rating));
        }

        let new_song = song_key.clone();
        let len_unique = unique_songs.len();

        if len_unique >= 11 || (len_unique == 10 && !unique_songs.contains_key(&new_song)) {
            // Case 1: >=11 unique songs or exactly 10 and new song
            if user_play.is_protected() {
                // Protected: find lowest rating to replace
                let lowest = current_tuples
                    .iter()
                    .enumerate()
                    .filter(|(_, tuple)| tuple.rating <= score.rating)
                    .min_by(|(_, a), (_, b)| a.rating.partial_cmp(&b.rating).unwrap())
                    .map(|(idx, _)| idx);

                if let Some(idx) = lowest {
                    self.update_one_r30(user_id, current_tuples[idx].r_index, score)
                        .await?;
                }
            } else {
                // Not protected: replace oldest (last in current order)
                if let Some(oldest) = current_tuples.last() {
                    self.update_one_r30(user_id, oldest.r_index, score).await?;
                }
            }
        } else {
            // Case 2: Need to find duplicate songs for replacement
            let mut filtered_songs = unique_songs.clone();

            filtered_songs.retain(|_, v| v.len() > 1);

            // If new song has unique entry, add it to filtered
            if unique_songs.contains_key(&new_song) && !filtered_songs.contains_key(&new_song) {
                if let Some(entries) = unique_songs.get(&new_song) {
                    filtered_songs.insert(new_song.clone(), entries.clone());
                }
            }

            if user_play.is_protected() {
                // Protected: find lowest in filtered songs
                let mut candidates = Vec::new();
                for (_, entries) in &filtered_songs {
                    for &(idx, r_index, rating) in entries {
                        if rating <= score.rating {
                            candidates.push((idx, r_index, rating));
                        }
                    }
                }

                if let Some((_, r_index, _)) = candidates
                    .iter()
                    .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
                {
                    self.update_one_r30(user_id, *r_index, score).await?;
                }
            } else {
                // Not protected: find oldest in filtered songs
                let mut oldest_idx = 0;
                let mut oldest_r_index = 0;

                for (_, entries) in &filtered_songs {
                    for &(idx, r_index, _) in entries {
                        if idx > oldest_idx {
                            oldest_idx = idx;
                            oldest_r_index = r_index;
                        }
                    }
                }

                if oldest_r_index != 0 {
                    self.update_one_r30(user_id, oldest_r_index, score).await?;
                }
            }
        }

        Ok(())
    }

    async fn update_one_r30(&self, user_id: i32, r_index: i32, score: &Score) -> ArcResult<()> {
        sqlx::query!(
            "REPLACE INTO recent30 (user_id, r_index, time_played, song_id, difficulty, score,
             shiny_perfect_count, perfect_count, near_count, miss_count, health, modifier,
             clear_type, rating) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            user_id,
            r_index,
            score.time_played,
            score.song_id,
            score.difficulty,
            score.score,
            score.shiny_perfect_count,
            score.perfect_count,
            score.near_count,
            score.miss_count,
            score.health,
            score.modifier,
            score.clear_type,
            score.rating
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_user_rating(&self, user_id: i32) -> ArcResult<()> {
        let potential = self.calculate_user_potential(user_id).await?;
        let rating_ptt = (potential.calculate_value(BEST30_WEIGHT, RECENT10_WEIGHT) * 100.0) as i32;

        sqlx::query!(
            "UPDATE user SET rating_ptt = ? WHERE user_id = ?",
            rating_ptt,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn calculate_user_potential(&self, user_id: i32) -> ArcResult<Potential> {
        // Calculate best 30
        let best_30 = sqlx::query!(
            "SELECT rating FROM best_score WHERE user_id = ? ORDER BY rating DESC LIMIT 30",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let best_30_sum: f64 = best_30.iter().map(|r| r.rating.unwrap_or(0.0)).sum();

        // Calculate recent 10 (simplified)
        let recent_scores = sqlx::query!(
            "SELECT song_id, difficulty, rating FROM recent30 WHERE user_id = ? AND song_id != '' ORDER BY time_played DESC",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut max_ratings: HashMap<(String, i32), f64> = HashMap::new();
        for score in recent_scores {
            if let (Some(song_id), Some(difficulty), Some(rating)) =
                (score.song_id, score.difficulty, score.rating)
            {
                let key = (song_id, difficulty);
                let current_max = max_ratings.get(&key).copied().unwrap_or(0.0);
                if rating > current_max {
                    max_ratings.insert(key, rating);
                }
            }
        }

        let mut recent_ratings: Vec<f64> = max_ratings.values().copied().collect();
        recent_ratings.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let recent_10_sum: f64 = recent_ratings.iter().take(10).sum();

        Ok(Potential {
            user_id,
            best_30_sum,
            recent_10_sum,
            r30_tuples: None,
            r30: None,
            b30: None,
        })
    }

    async fn get_user_rating_ptt(&self, user_id: i32) -> ArcResult<i32> {
        let user = sqlx::query!("SELECT rating_ptt FROM user WHERE user_id = ?", user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(user.rating_ptt.unwrap_or(0))
    }

    /// Record score to log database
    async fn record_score(&self, _user_play: &UserPlay) -> ArcResult<()> {
        // This would record to a separate log database
        // For now, this is a placeholder implementation
        Ok(())
    }

    /// Record user rating PTT changes to log database
    async fn record_rating_ptt(&self, _user_id: i32, _user_rating_ptt: f64) -> ArcResult<()> {
        // This would record to a separate log database
        // For now, this is a placeholder implementation
        Ok(())
    }

    /// Handle world mode calculations
    async fn handle_world_mode(&self, user_play: &mut UserPlay) -> ArcResult<()> {
        // Get user's current world map info
        let _user_current_map = self
            .get_user_current_map(user_play.user_score.user_id)
            .await?;

        // This would implement the complex world mode logic from Python:
        // - Check map type (normal, beyond, breached)
        // - Apply character skills
        // - Calculate progress
        // - Update map position
        // - Handle rewards

        // For now, this is a placeholder implementation
        Ok(())
    }

    /// Handle course mode calculations
    async fn handle_course_mode(&self, _user_play: &mut UserPlay) -> ArcResult<()> {
        // This would implement course mode logic:
        // - Update course progress
        // - Check course completion
        // - Handle course rewards

        // For now, this is a placeholder implementation
        Ok(())
    }

    /// Get user's current world map
    async fn get_user_current_map(&self, user_id: i32) -> ArcResult<String> {
        let user = sqlx::query!("SELECT current_map FROM user WHERE user_id = ?", user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(user.current_map.unwrap_or_default())
    }

    /// Update user's global rank
    async fn update_user_global_rank(&self, user_id: i32) -> ArcResult<()> {
        // This would calculate and update the user's global ranking
        // based on their score_v2 values

        // Calculate total score_v2
        let total_score = sqlx::query!(
            "SELECT SUM(score_v2) as total FROM best_score WHERE user_id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        let world_rank_score = total_score.total.unwrap_or(0.0) as i32;

        sqlx::query!(
            "UPDATE user SET world_rank_score = ? WHERE user_id = ?",
            world_rank_score,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Generate a random song token
fn generate_song_token() -> String {
    let mut random_bytes = [0u8; 64];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut random_bytes);
    general_purpose::STANDARD.encode(random_bytes)
}

/// Generate a random course token
fn generate_course_token() -> String {
    let mut random_bytes = [0u8; 64];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut random_bytes);
    format!("c_{}", general_purpose::STANDARD.encode(random_bytes))
}

/// Generate a random skill flag with specified length
fn generate_random_skill_flag(length: usize) -> String {
    (0..length)
        .map(|_| rand::thread_rng().gen_range(0..3).to_string())
        .collect()
}

/// Get world value name from index
fn get_world_value_name(index: i32) -> String {
    match index {
        0 => "fragment".to_string(),
        1 => "progress".to_string(),
        2 => "overdrive".to_string(),
        _ => "fragment".to_string(),
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Calculate MD5 hash of a string
pub fn md5_hash(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}
