use crate::error::{ArcError, ArcResult};
// use crate::service::UserService;
use serde_json::Value;

use crate::config::{Constants, CONFIG};
use crate::model::download::{
    CourseTokenRequest, CourseTokenResponse, ScoreSubmission, SongplayToken, WorldTokenRequest,
    WorldTokenResponse,
};
use crate::model::score::{
    Potential, RankingScoreRow, RankingScoreRowComplete, Recent30Tuple, Score, UserPlay, UserScore,
};
use crate::model::user::User;
use crate::model::world::WorldStep;
use crate::service::character::CharacterService;
use crate::service::item::ItemService;
use crate::service::user::UserService;
use crate::service::world::{get_map_parser, StaminaImpl};
use base64::{engine::general_purpose, Engine as _};
use md5;
use rand::Rng;
use serde_json::json;
use sqlx::{MySqlPool, Row};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Constants for score calculations
const BEST30_WEIGHT: f64 = 1.0 / 40.0;
const RECENT10_WEIGHT: f64 = 1.0 / 40.0;
type SongKey = (String, i32);
type SongEntry = (usize, i32, f64);
type SongEntryMap = HashMap<SongKey, Vec<SongEntry>>;
type JsonMap = HashMap<String, Value>;

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
        self.sync_world_token_user_state(user_id, &request).await?;

        let user = self.get_user_info(user_id).await?;

        let stamina_multiply = request.stamina_multiply.unwrap_or(1);
        let fragment_multiply = request.fragment_multiply.unwrap_or(100);
        let mut prog_boost_multiply = request.prog_boost_multiply.unwrap_or(0);
        let mut beyond_boost_gauge_use = request.beyond_boost_gauge_use.unwrap_or(0);

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

        // Validate prog_boost and beyond_boost_gauge like Python version
        if prog_boost_multiply != 0 || beyond_boost_gauge_use != 0 {
            let boost_data = sqlx::query!(
                "SELECT prog_boost, beyond_boost_gauge FROM user WHERE user_id = ?",
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;

            if let Some(data) = boost_data {
                prog_boost_multiply = if data.prog_boost.unwrap_or(0) == 300 {
                    300
                } else {
                    0
                };
                if data.beyond_boost_gauge.unwrap_or(0.0) < beyond_boost_gauge_use as f64
                    || !matches!(beyond_boost_gauge_use, 100 | 200)
                {
                    beyond_boost_gauge_use = 0;
                }
            } else {
                prog_boost_multiply = 0;
                beyond_boost_gauge_use = 0;
            }
        }

        // Get user map and character info for stamina and skill processing
        let stamina_cost = self.get_user_current_map(user_id).await?;
        let raw_stamina = user.stamina.unwrap_or(0);
        let raw_max_stamina_ts = user.max_stamina_ts.unwrap_or(0);
        let mut stamina = StaminaImpl::new(raw_stamina, raw_max_stamina_ts);

        // Auto-repair legacy stamina state:
        // if overcap stamina exists while max_stamina_ts is still in the future, normalize it
        // to Python's stamina setter semantics before validation.
        if raw_stamina > Constants::MAX_STAMINA && raw_max_stamina_ts > current_timestamp() {
            stamina.set_stamina(raw_stamina);
            sqlx::query!(
                "UPDATE user SET stamina = ?, max_stamina_ts = ? WHERE user_id = ?",
                stamina.get_current_stamina(),
                stamina.max_stamina_ts(),
                user_id
            )
            .execute(&self.pool)
            .await?;
        }
        let current_stamina = stamina.get_current_stamina();

        if current_stamina < stamina_cost * stamina_multiply {
            return Err(ArcError::StaminaNotEnough {
                message: "Stamina is not enough.".to_string(),
                error_code: 107,
                api_error_code: -999,
                extra_data: None,
                status: 200,
            });
        }

        // Check character skill and invasion
        let mut fatalis_stamina_multiply = 1;
        if user.is_skill_sealed.unwrap_or(1) == 0 {
            // Get character info for skill processing
            let character_info = sqlx::query!(
                "SELECT c.skill_id FROM user u
                 JOIN `character` c ON u.character_id = c.character_id
                 WHERE u.user_id = ?",
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;

            // Invasion logic - only if insight is enabled (insight_state == 3 or 5)
            let insight_state = user.insight_state.unwrap_or(4);
            if insight_state == 3 || insight_state == 5 {
                // Use weighted choice like Python's choices([0, 1, 2], [weights])
                let no_invasion_weight =
                    (1.0 - CONFIG.invasion_start_weight - CONFIG.invasion_hard_weight).max(0.0f64);
                let weights = [
                    no_invasion_weight,
                    CONFIG.invasion_start_weight,
                    CONFIG.invasion_hard_weight,
                ];
                let mut cumulative = 0.0;
                let rand_val: f64 = rand::thread_rng().gen();

                for (i, &weight) in weights.iter().enumerate() {
                    cumulative += weight;
                    if rand_val < cumulative {
                        let flag = i as i32;
                        if flag != 0 {
                            invasion_flag = flag;
                        }
                        break;
                    }
                }
            }

            // Python baseline: Fatalis double stamina triggers only when invasion didn't trigger.
            if invasion_flag == 0 {
                if let Some(char_data) = character_info {
                    if char_data.skill_id.as_deref() == Some("skill_fatalis") {
                        fatalis_stamina_multiply = 2;
                    }
                }
            }
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

        // Update user stamina (matches Python's Stamina setter semantics)
        stamina.set_stamina(
            current_stamina - stamina_cost * stamina_multiply * fatalis_stamina_multiply,
        );
        sqlx::query!(
            "UPDATE user SET stamina = ?, max_stamina_ts = ? WHERE user_id = ?",
            stamina.get_current_stamina(),
            stamina.max_stamina_ts(),
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Build play parameters
        let mut play_parameters = HashMap::new();

        if let Some(skill_flag) = skill_cytusii_flag.clone().or(skill_chinatsu_flag.clone()) {
            if let Some(skill_id) = request.skill_id.clone() {
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

        Ok(WorldTokenResponse {
            stamina: stamina.get_current_stamina(),
            max_stamina_ts: stamina.max_stamina_ts(),
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
        let use_course_skip_purchase = request.use_course_skip_purchase;

        let mut status = "created".to_string();
        let token;

        // Get play state from previous token if provided
        let course_play_state = if let Some(previous_token) = &request.previous_token {
            let existing_token = sqlx::query!(
                "SELECT course_state FROM songplay_token WHERE token = ? AND user_id = ?",
                previous_token,
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;

            existing_token
                .map(|t| t.course_state.unwrap_or(-1))
                .unwrap_or(-1)
        } else {
            -1
        };

        if course_play_state == -1 {
            // No token, course mode just started
            if let Some(course_id) = request.course_id {
                token = self
                    .create_course_session(user_id, &course_id, use_course_skip_purchase)
                    .await?;
            } else {
                return Err(ArcError::input(
                    "course_id is required for new course session",
                ));
            }
        } else if (0..=3).contains(&course_play_state) {
            // Validate token and continue course
            if let Some(previous_token) = request.previous_token {
                token = self.update_course_token(&previous_token, user_id).await?;
            } else {
                return Err(ArcError::input(
                    "previous_token is required for continuing course",
                ));
            }
        } else {
            // Course mode has ended
            self.clear_user_songplay_tokens(user_id).await?;
            status = if course_play_state == 4 {
                "cleared".to_string()
            } else {
                "failed".to_string()
            };
            token = request.previous_token.unwrap_or_default();
        }

        let (stamina, max_stamina_ts) = self.get_user_stamina_info(user_id).await?;

        Ok(CourseTokenResponse {
            stamina,
            max_stamina_ts,
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
            is_world_mode: None,
            stamina_multiply: 1,
            fragment_multiply: 100,
            prog_boost_multiply: 0,
            beyond_boost_gauge_usage: 0,
            course_id: None,
            course_play_state: -1,
            course_score: 0,
            course_clear_type: 3,
            combo_interval_bonus: submission.combo_interval_bonus,
            hp_interval_bonus: submission.hp_interval_bonus,
            fever_bonus: submission.fever_bonus,
            rank_bonus: submission.rank_bonus,
            maya_gauge: submission.maya_gauge,
            nextstage_bonus: submission.nextstage_bonus,
            skill_cytusii_flag: None,
            skill_chinatsu_flag: None,
            highest_health: submission.highest_health,
            lowest_health: submission.lowest_health,
            room_code: submission.room_code.clone(),
            room_total_score: submission.room_total_score,
            room_total_players: submission.room_total_players,
            invasion_flag: 0,
            ptt: None,
        };

        // Set chart info
        user_play
            .user_score
            .score
            .set_chart(submission.song_id.clone(), submission.difficulty);

        // Set score data
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

        // Validate score first (before any processing)
        let expected_hash = self
            .get_song_file_hash(&submission.song_id, submission.difficulty)
            .await;
        if !user_play.is_valid(expected_hash.as_deref()) {
            return Err(ArcError::Input {
                message: "Invalid score.".to_string(),
                error_code: 107,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        // Upload score (which handles rating calculation internally)
        let mut result = self.upload_score(&mut user_play).await?;

        // Python baseline response: (world/course payload) + common fields
        let potential = self.calculate_user_potential(user_id).await?;
        let ptt_value = potential.calculate_value(BEST30_WEIGHT, RECENT10_WEIGHT);
        user_play.ptt = Some(potential);

        let user_rating = self.get_user_rating_ptt(user_id).await?;
        let global_rank = self.get_user_global_rank(user_id).await?;

        result.insert("user_rating".to_string(), Value::from(user_rating));
        result.insert(
            "finale_challenge_higher".to_string(),
            Value::from(user_play.user_score.score.rating > ptt_value),
        );
        result.insert("global_rank".to_string(), Value::from(global_rank));
        result.insert(
            "finale_play_value".to_string(),
            Value::from(9.065 * user_play.user_score.score.rating.sqrt()),
        );

        Ok(result)
    }

    /// Get top 20 scores for a song
    pub async fn get_song_top_scores(
        &self,
        song_id: &str,
        difficulty: i32,
    ) -> ArcResult<Vec<HashMap<String, serde_json::Value>>> {
        let scores = if CONFIG.character_full_unlock {
            sqlx::query_as!(
                RankingScoreRow,
                r#"SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                    bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                    bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
                    u.name, u.character_id, u.is_char_uncapped, u.is_char_uncapped_override,
                    u.favorite_character, u.is_skill_sealed,
                    uc.is_uncapped as favorite_is_uncapped,
                    uc.is_uncapped_override as favorite_is_uncapped_override
                 FROM best_score bs
                 JOIN user u ON bs.user_id = u.user_id
                 LEFT JOIN user_char_full uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                 WHERE bs.song_id = ? AND bs.difficulty = ?
                 ORDER BY bs.score DESC, bs.time_played DESC
                 LIMIT 20"#,
                song_id,
                difficulty
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                RankingScoreRow,
                r#"SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                    bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                    bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
                    u.name, u.character_id, u.is_char_uncapped, u.is_char_uncapped_override,
                    u.favorite_character, u.is_skill_sealed,
                    uc.is_uncapped as favorite_is_uncapped,
                    uc.is_uncapped_override as favorite_is_uncapped_override
                 FROM best_score bs
                 JOIN user u ON bs.user_id = u.user_id
                 LEFT JOIN user_char uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                 WHERE bs.song_id = ? AND bs.difficulty = ?
                 ORDER BY bs.score DESC, bs.time_played DESC
                 LIMIT 20"#,
                song_id,
                difficulty
            )
            .fetch_all(&self.pool)
            .await?
        };

        let result = scores
            .into_iter()
            .enumerate()
            .map(|(rank, row)| {
                row.to_user_score_with_rank(Some((rank + 1) as i32))
                    .to_dict(true)
            })
            .collect();

        Ok(result)
    }

    /// Get user's rank for a song
    pub async fn get_user_song_rank(
        &self,
        user_id: i32,
        song_id: &str,
        difficulty: i32,
    ) -> ArcResult<Vec<HashMap<String, serde_json::Value>>> {
        // Get user's score and time_played
        let user_score = sqlx::query!(
            "SELECT score, time_played FROM best_score WHERE user_id = ? AND song_id = ? AND difficulty = ?",
            user_id,
            song_id,
            difficulty
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(user_row) = user_score {
            // Calculate user's rank (considering both score and time_played for tie-breaking)
            let rank_result = sqlx::query!(
                "SELECT COUNT(*) as rank FROM best_score
                 WHERE song_id = ? AND difficulty = ? AND
                 (score > ? OR (score = ? AND time_played > ?))",
                song_id,
                difficulty,
                user_row.score,
                user_row.score,
                user_row.time_played
            )
            .fetch_one(&self.pool)
            .await?;
            let my_rank = (rank_result.rank + 1) as i32;

            // Get total count
            let total_result = sqlx::query!(
                "SELECT COUNT(*) as total FROM best_score WHERE song_id = ? AND difficulty = ?",
                song_id,
                difficulty
            )
            .fetch_one(&self.pool)
            .await?;
            let total_count = total_result.total as i32;

            // Calculate ranking display parameters using Python logic
            const MAX_LOCAL_POSITION: i32 = 5;
            const MAX_GLOBAL_POSITION: i32 = 9999;
            const LIMIT: i32 = 20;

            let (sql_limit, sql_offset, need_myself) = self.get_my_rank_parameters(
                my_rank,
                total_count,
                LIMIT,
                MAX_LOCAL_POSITION,
                MAX_GLOBAL_POSITION,
            );

            // Get scores with calculated offset and limit
            let scores = if CONFIG.character_full_unlock {
                sqlx::query_as!(
                    RankingScoreRow,
                    r#"SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                        bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                        bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
                        u.name, u.character_id, u.is_char_uncapped, u.is_char_uncapped_override,
                        u.favorite_character, u.is_skill_sealed,
                        uc.is_uncapped as favorite_is_uncapped,
                        uc.is_uncapped_override as favorite_is_uncapped_override
                     FROM best_score bs
                     JOIN user u ON bs.user_id = u.user_id
                     LEFT JOIN user_char_full uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                     WHERE bs.song_id = ? AND bs.difficulty = ?
                     ORDER BY bs.score DESC, bs.time_played DESC
                     LIMIT ? OFFSET ?"#,
                    song_id,
                    difficulty,
                    sql_limit,
                    sql_offset
                )
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as!(
                    RankingScoreRow,
                    r#"SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                        bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                        bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
                        u.name, u.character_id, u.is_char_uncapped, u.is_char_uncapped_override,
                        u.favorite_character, u.is_skill_sealed,
                        uc.is_uncapped as favorite_is_uncapped,
                        uc.is_uncapped_override as favorite_is_uncapped_override
                     FROM best_score bs
                     JOIN user u ON bs.user_id = u.user_id
                     LEFT JOIN user_char uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                     WHERE bs.song_id = ? AND bs.difficulty = ?
                     ORDER BY bs.score DESC, bs.time_played DESC
                     LIMIT ? OFFSET ?"#,
                    song_id,
                    difficulty,
                    sql_limit,
                    sql_offset
                )
                .fetch_all(&self.pool)
                .await?
            };

            let mut result = Vec::new();

            for (i, row) in scores.iter().enumerate() {
                let rank = if sql_offset > 0 {
                    sql_offset + (i as i32) + 1
                } else {
                    (i as i32) + 1
                };

                result.push(row.to_user_score_with_rank(Some(rank)).to_dict(true));
            }

            // Add user's own score at the end if needed
            if need_myself {
                let user_own_score = if CONFIG.character_full_unlock {
                    sqlx::query_as!(
                        RankingScoreRow,
                        r#"SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                            bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                            bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
                            u.name, u.character_id, u.is_char_uncapped, u.is_char_uncapped_override,
                            u.favorite_character, u.is_skill_sealed,
                            uc.is_uncapped as favorite_is_uncapped,
                            uc.is_uncapped_override as favorite_is_uncapped_override
                         FROM best_score bs
                         JOIN user u ON bs.user_id = u.user_id
                         LEFT JOIN user_char_full uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                         WHERE bs.user_id = ? AND bs.song_id = ? AND bs.difficulty = ?"#,
                        user_id,
                        song_id,
                        difficulty
                    )
                    .fetch_one(&self.pool)
                    .await?
                } else {
                    sqlx::query_as!(
                        RankingScoreRow,
                        r#"SELECT bs.user_id, bs.song_id, bs.difficulty, bs.score, bs.shiny_perfect_count,
                            bs.perfect_count, bs.near_count, bs.miss_count, bs.health, bs.modifier,
                            bs.time_played, bs.best_clear_type, bs.clear_type, bs.rating, bs.score_v2,
                            u.name, u.character_id, u.is_char_uncapped, u.is_char_uncapped_override,
                            u.favorite_character, u.is_skill_sealed,
                            uc.is_uncapped as favorite_is_uncapped,
                            uc.is_uncapped_override as favorite_is_uncapped_override
                         FROM best_score bs
                         JOIN user u ON bs.user_id = u.user_id
                         LEFT JOIN user_char uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                         WHERE bs.user_id = ? AND bs.song_id = ? AND bs.difficulty = ?"#,
                        user_id,
                        song_id,
                        difficulty
                    )
                    .fetch_one(&self.pool)
                    .await?
                };

                result.push(
                    user_own_score
                        .to_user_score_with_rank(Some(-1))
                        .to_dict(true),
                );
            }

            Ok(result)
        } else {
            Ok(vec![])
        }
    }

    /// Get friend rankings for a song
    /// Calculate ranking display parameters for user's personal ranking
    /// This implements the Python get_my_rank_parameter logic
    fn get_my_rank_parameters(
        &self,
        my_rank: i32,
        total_count: i32,
        limit: i32,
        max_local_position: i32,
        max_global_position: i32,
    ) -> (i32, i32, bool) {
        let mut sql_limit = limit;
        let mut sql_offset = 0;
        let mut need_myself = false;

        if my_rank <= max_local_position {
            // Rank is at the front, not enough people ahead
        } else if my_rank > max_global_position {
            // Rank is too far back, don't show ranking
            sql_limit -= 1;
            sql_offset = max_global_position - limit + 1;
            need_myself = true;
        } else if total_count - my_rank < limit - max_local_position {
            // Not enough people behind, show ranking
            sql_offset = total_count - limit;
        } else if max_local_position <= my_rank
            && my_rank < max_global_position - limit + max_local_position
        {
            // Enough people ahead, show ranking
            sql_offset = my_rank - max_local_position;
        } else {
            // Default case
            sql_offset = max_global_position - limit;
        }

        if sql_offset < 0 {
            sql_offset = 0;
        }

        (sql_limit, sql_offset, need_myself)
    }

    pub async fn get_friend_song_ranks(
        &self,
        user_id: i32,
        song_id: &str,
        difficulty: i32,
    ) -> ArcResult<Vec<HashMap<String, serde_json::Value>>> {
        // First get all friend scores using a JOIN instead of IN clause
        let scores = if CONFIG.character_full_unlock {
            sqlx::query_as!(
                RankingScoreRowComplete,
                r#"SELECT bs.*, u.name, u.character_id, u.is_char_uncapped,
                    u.is_char_uncapped_override, u.favorite_character, u.is_skill_sealed,
                    uc.is_uncapped as favorite_is_uncapped,
                    uc.is_uncapped_override as favorite_is_uncapped_override,
                    c.name as song_name
                 FROM best_score bs
                 JOIN user u ON bs.user_id = u.user_id
                 LEFT JOIN user_char_full uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                 LEFT JOIN chart c ON bs.song_id = c.song_id
                 WHERE bs.song_id = ? AND bs.difficulty = ?
                 AND (bs.user_id = ? OR EXISTS(
                     SELECT 1 FROM friend f
                     WHERE f.user_id_me = ? AND f.user_id_other = bs.user_id
                 ))
                 ORDER BY bs.score DESC, bs.time_played DESC
                 LIMIT 50"#,
                song_id,
                difficulty,
                user_id,
                user_id
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                RankingScoreRowComplete,
                r#"SELECT bs.*, u.name, u.character_id, u.is_char_uncapped,
                    u.is_char_uncapped_override, u.favorite_character, u.is_skill_sealed,
                    uc.is_uncapped as favorite_is_uncapped,
                    uc.is_uncapped_override as favorite_is_uncapped_override,
                    c.name as song_name
                 FROM best_score bs
                 JOIN user u ON bs.user_id = u.user_id
                 LEFT JOIN user_char uc ON uc.user_id = u.user_id AND uc.character_id = u.favorite_character
                 LEFT JOIN chart c ON bs.song_id = c.song_id
                 WHERE bs.song_id = ? AND bs.difficulty = ?
                 AND (bs.user_id = ? OR EXISTS(
                     SELECT 1 FROM friend f
                     WHERE f.user_id_me = ? AND f.user_id_other = bs.user_id
                 ))
                 ORDER BY bs.score DESC, bs.time_played DESC
                 LIMIT 50"#,
                song_id,
                difficulty,
                user_id,
                user_id
            )
            .fetch_all(&self.pool)
            .await?
        };

        let result = scores
            .into_iter()
            .enumerate()
            .map(|(rank, row)| {
                row.to_user_score_with_rank_and_display(Some((rank + 1) as i32))
                    .to_dict(true)
            })
            .collect();

        Ok(result)
    }

    // Helper methods

    async fn sync_world_token_user_state(
        &self,
        user_id: i32,
        request: &WorldTokenRequest,
    ) -> ArcResult<()> {
        let has_character_selection = request.character_id.is_some();
        let has_skill_sealed = request.is_skill_sealed.is_some();
        let has_uncap_override = request.is_char_uncapped_override.is_some();

        if !has_character_selection && !has_skill_sealed && !has_uncap_override {
            return Ok(());
        }

        let user = self.get_user_info(user_id).await?;
        let mut target_character_id = user.character_id.unwrap_or(0);
        let mut target_is_skill_sealed = user.is_skill_sealed.unwrap_or(0);
        let mut target_is_uncapped = user.is_char_uncapped.unwrap_or(0);
        let mut target_is_uncapped_override = user.is_char_uncapped_override.unwrap_or(0);

        if let Some(character_id) = request.character_id {
            target_character_id = character_id;

            if CONFIG.character_full_unlock {
                let row = sqlx::query!(
                    "SELECT is_uncapped, is_uncapped_override FROM user_char_full WHERE user_id = ? AND character_id = ?",
                    user_id,
                    character_id
                )
                .fetch_optional(&self.pool)
                .await?;

                if let Some(row) = row {
                    target_is_uncapped = row.is_uncapped.unwrap_or(0);
                    target_is_uncapped_override = row.is_uncapped_override.unwrap_or(0);
                } else {
                    // Python baseline (`change_character`) uses fallback false/false when character row is absent.
                    target_is_uncapped = 0;
                    target_is_uncapped_override = 0;
                }
            } else {
                let row = sqlx::query!(
                    "SELECT is_uncapped, is_uncapped_override FROM user_char WHERE user_id = ? AND character_id = ?",
                    user_id,
                    character_id
                )
                .fetch_optional(&self.pool)
                .await?;

                if let Some(row) = row {
                    target_is_uncapped = row.is_uncapped.unwrap_or(0);
                    target_is_uncapped_override = row.is_uncapped_override.unwrap_or(0);
                } else {
                    // Python baseline (`change_character`) uses fallback false/false when character row is absent.
                    target_is_uncapped = 0;
                    target_is_uncapped_override = 0;
                }
            }
        }

        if let Some(skill_sealed) = request
            .is_skill_sealed
            .as_deref()
            .and_then(parse_bool_string)
        {
            target_is_skill_sealed = if skill_sealed { 1 } else { 0 };
        }

        if let Some(uncap_override) = request
            .is_char_uncapped_override
            .as_deref()
            .and_then(parse_bool_string)
        {
            target_is_uncapped_override = if uncap_override { 1 } else { 0 };
        }

        if target_is_uncapped == 0 {
            target_is_uncapped_override = 0;
        }

        if target_character_id != user.character_id.unwrap_or(0)
            || target_is_skill_sealed != user.is_skill_sealed.unwrap_or(0)
            || target_is_uncapped != user.is_char_uncapped.unwrap_or(0)
            || target_is_uncapped_override != user.is_char_uncapped_override.unwrap_or(0)
        {
            sqlx::query!(
                "UPDATE user SET character_id = ?, is_skill_sealed = ?, is_char_uncapped = ?, is_char_uncapped_override = ? WHERE user_id = ?",
                target_character_id,
                target_is_skill_sealed,
                target_is_uncapped,
                target_is_uncapped_override,
                user_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn get_user_info(&self, user_id: i32) -> ArcResult<User> {
        sqlx::query_as!(User, "SELECT * FROM user WHERE user_id = ?", user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ArcError::no_data(format!("User not found: {}", e), 108))
    }

    async fn get_user_stamina_info(&self, user_id: i32) -> ArcResult<(i32, i64)> {
        let row = sqlx::query!(
            "SELECT max_stamina_ts, stamina FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        let stamina = StaminaImpl::new(row.stamina.unwrap_or(0), row.max_stamina_ts.unwrap_or(0));
        Ok((stamina.get_current_stamina(), stamina.max_stamina_ts()))
    }

    async fn get_play_state(&self, token: &str, user_id: i32) -> ArcResult<Option<SongplayToken>> {
        let result = sqlx::query!(
            "SELECT token, user_id, song_id, difficulty, course_id, course_state, course_score, course_clear_type, stamina_multiply, fragment_multiply, prog_boost_multiply, beyond_boost_gauge_usage, skill_cytusii_flag, skill_chinatsu_flag, invasion_flag FROM songplay_token WHERE token = ? AND user_id = ?",
            token,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(result) = result else {
            return Ok(None);
        };

        Ok(Some(SongplayToken {
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
        }))
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

    async fn get_song_file_hash(&self, song_id: &str, difficulty: i32) -> Option<String> {
        let file_name = format!("{difficulty}.aff");

        // Python baseline: check chart MD5 if the server has the file; otherwise skip.
        // In this Rust repo we usually keep songs under `./songs/<song_id>/<difficulty>.aff`,
        // but also try `CONFIG.song_file_folder_path` for compatibility.
        let candidates = [
            Path::new("songs").join(song_id).join(&file_name),
            Path::new(&CONFIG.song_file_folder_path)
                .join(song_id)
                .join(&file_name),
        ];

        for path in candidates {
            if path.is_file() {
                if let Ok(contents) = std::fs::read(&path) {
                    return Some(format!("{:x}", md5::compute(&contents)));
                }
            }
        }

        None
    }

    #[allow(dead_code)]
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
        use_skip_purchase: bool,
    ) -> ArcResult<String> {
        let token = generate_course_token();

        // Python baseline: insert token first, then deduct stamina / consume skip item.
        sqlx::query!(
            "INSERT INTO songplay_token VALUES (?, ?, '', 0, ?, 0, 0, 3, 1, 100, 0, 0, '', '', 0)",
            token,
            user_id,
            course_id
        )
        .execute(&self.pool)
        .await?;

        if use_skip_purchase {
            // TODO: consume core_course_skip_purchase (matches Python ItemCore usage)
        } else {
            let stamina_row = sqlx::query!(
                "SELECT max_stamina_ts, stamina FROM user WHERE user_id = ?",
                user_id
            )
            .fetch_one(&self.pool)
            .await?;

            let mut stamina = StaminaImpl::new(
                stamina_row.stamina.unwrap_or(0),
                stamina_row.max_stamina_ts.unwrap_or(0),
            );
            let current_stamina = stamina.get_current_stamina();

            if current_stamina < Constants::COURSE_STAMINA_COST {
                return Err(ArcError::StaminaNotEnough {
                    message: "Stamina is not enough.".to_string(),
                    error_code: 107,
                    api_error_code: -999,
                    extra_data: None,
                    status: 200,
                });
            }

            stamina.set_stamina(current_stamina - Constants::COURSE_STAMINA_COST);
            sqlx::query!(
                "UPDATE user SET stamina = ?, max_stamina_ts = ? WHERE user_id = ?",
                stamina.get_current_stamina(),
                stamina.max_stamina_ts(),
                user_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(token)
    }

    async fn update_course_token(&self, previous_token: &str, user_id: i32) -> ArcResult<String> {
        let new_token = generate_course_token();
        sqlx::query!(
            "UPDATE songplay_token SET token = ? WHERE token = ? AND user_id = ?",
            new_token,
            previous_token,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(new_token)
    }

    async fn upload_score(&self, user_play: &mut UserPlay) -> ArcResult<JsonMap> {
        let user_id = user_play.user_score.user_id;

        // Get play state first (Python baseline: token may be missing; only used to detect world/course mode).
        if user_play.song_token == "1145141919810" {
            // Hardcoded bypass token
            user_play.is_world_mode = Some(false);
            user_play.course_id = None;
            user_play.course_play_state = -1;
            user_play.course_score = 0;
            user_play.course_clear_type = 3;
        } else if let Some(play_state) = self.get_play_state(&user_play.song_token, user_id).await?
        {
            let course_id = play_state.course_id.clone().unwrap_or_default();
            if course_id.is_empty() {
                // World mode: course_id is an empty string in Python
                user_play.is_world_mode = Some(true);
                user_play.course_id = None;
                user_play.course_play_state = -1;
                user_play.course_score = 0;
                user_play.course_clear_type = 3;
            } else {
                // Course mode
                user_play.is_world_mode = Some(false);
                user_play.course_id = Some(course_id);
                user_play.course_play_state = play_state.course_state;
                user_play.course_score = play_state.course_score;
                user_play.course_clear_type = play_state.course_clear_type;
            }

            user_play.stamina_multiply = play_state.stamina_multiply;
            user_play.fragment_multiply = play_state.fragment_multiply;
            user_play.prog_boost_multiply = play_state.prog_boost_multiply;
            user_play.beyond_boost_gauge_usage = play_state.beyond_boost_gauge_usage;
            user_play.skill_cytusii_flag = play_state.skill_cytusii_flag;
            user_play.skill_chinatsu_flag = play_state.skill_chinatsu_flag;
            user_play.invasion_flag = play_state.invasion_flag;
        } else {
            // Missing token: treat as non-world/non-course (same as Python)
            user_play.is_world_mode = Some(false);
            user_play.course_id = None;
            user_play.course_play_state = -1;
            user_play.course_score = 0;
            user_play.course_clear_type = 3;
        }

        // Get rating by calc (like Python version)
        let chart_const = self
            .get_chart_constant(
                &user_play.user_score.score.song_id,
                user_play.user_score.score.difficulty,
            )
            .await?;
        user_play.user_score.score.get_rating_by_calc(chart_const);

        // Handle unranked scores
        if user_play.user_score.score.rating < 0.0 {
            user_play.unrank_flag = true;
            user_play.user_score.score.rating = 0.0;
        } else {
            user_play.unrank_flag = false;
        }

        // Set timestamp (Python baseline: best_score / recent30 uses seconds)
        user_play.user_score.score.time_played = current_timestamp_seconds();

        // Record score to log database
        self.record_score(user_play).await?;

        // Update user recent score (like Python version)
        sqlx::query!(
            "UPDATE user SET song_id = ?, difficulty = ?, score = ?, shiny_perfect_count = ?,
             perfect_count = ?, near_count = ?, miss_count = ?, health = ?, modifier = ?,
             clear_type = ?, rating = ?, time_played = ? WHERE user_id = ?",
            user_play.user_score.score.song_id,
            user_play.user_score.score.difficulty,
            user_play.user_score.score.score,
            user_play.user_score.score.shiny_perfect_count,
            user_play.user_score.score.perfect_count,
            user_play.user_score.score.near_count,
            user_play.user_score.score.miss_count,
            user_play.user_score.score.health,
            user_play.user_score.score.modifier,
            user_play.user_score.score.clear_type,
            user_play.user_score.score.rating,
            user_play.user_score.score.time_played * 1000,
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

        // Handle world mode if applicable
        let mut mode_payload = HashMap::new();
        if user_play.is_world_mode == Some(true) {
            mode_payload = self.handle_world_mode(user_play).await?;
        } else if user_play.course_play_state >= 0 {
            mode_payload = self.handle_course_mode(user_play).await?;
        }

        Ok(mode_payload)
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
                // first try's protect.
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

                // update global rank.
                self.update_user_global_rank(user_id).await?;
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

                    self.update_user_global_rank(user_id).await?;
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
            "INSERT INTO recent30 (
                user_id, r_index, time_played, song_id, difficulty, score,
                shiny_perfect_count, perfect_count, near_count, miss_count, health, modifier,
                clear_type, rating
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                r_index = VALUES(r_index),
                time_played = VALUES(time_played),
                song_id = VALUES(song_id),
                difficulty = VALUES(difficulty),
                score = VALUES(score),
                shiny_perfect_count = VALUES(shiny_perfect_count),
                perfect_count = VALUES(perfect_count),
                near_count = VALUES(near_count),
                miss_count = VALUES(miss_count),
                health = VALUES(health),
                modifier = VALUES(modifier),
                clear_type = VALUES(clear_type),
                rating = VALUES(rating);",
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
        let new_song_tuple = (score.song_id.clone(), score.difficulty);

        // If protected, find the lowest rating, and if tied, the oldest one (most efficient approach)
        let lowest_eligible_tuple = if user_play.is_protected() {
            current_tuples
                .iter()
                .enumerate()
                .filter(|(_, tuple)| tuple.rating <= score.rating)
                .min_by(|(_, a), (_, b)| {
                    a.rating
                        .partial_cmp(&b.rating)
                        .unwrap()
                        .then(a.r_index.cmp(&b.r_index)) // Lower r_index = older
                })
        } else {
            None
        };

        // Build unique_songs map exactly like Python
        let mut unique_songs: SongEntryMap = HashMap::new();
        for (i, tuple) in current_tuples.iter().enumerate() {
            let key = (tuple.song_id.clone(), tuple.difficulty);
            unique_songs
                .entry(key)
                .or_default()
                .push((i, tuple.r_index, tuple.rating));
        }

        // Check if we have too many unique songs
        if unique_songs.len() >= 11
            || (unique_songs.len() == 10 && !unique_songs.contains_key(&new_song_tuple))
        {
            if user_play.is_protected() {
                // Replace lowest and oldest score
                if let Some((_, tuple)) = lowest_eligible_tuple {
                    self.update_one_r30(user_id, tuple.r_index, score).await?;
                }
            } else {
                // Replace the last (oldest) score
                if let Some(last_tuple) = current_tuples.last() {
                    self.update_one_r30(user_id, last_tuple.r_index, score)
                        .await?;
                }
            }
            return Ok(());
        }

        // Filter songs with multiple entries
        let mut filtered_songs: SongEntryMap = unique_songs
            .iter()
            .filter(|(_, entries)| entries.len() > 1)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // If new song has unique entry in r30, it should be replaceable too
        if unique_songs.contains_key(&new_song_tuple)
            && !filtered_songs.contains_key(&new_song_tuple)
        {
            if let Some(entries) = unique_songs.get(&new_song_tuple) {
                filtered_songs.insert(new_song_tuple, entries.clone());
            }
        }

        if user_play.is_protected() {
            // Protected: find lowest score in filtered songs (efficient iterator approach)
            if let Some(target_tuple) = current_tuples
                .iter()
                .filter(|tuple| {
                    tuple.rating <= score.rating
                        && filtered_songs.contains_key(&(tuple.song_id.clone(), tuple.difficulty))
                })
                .min_by(|a, b| {
                    a.rating
                        .partial_cmp(&b.rating)
                        .unwrap()
                        .then(a.r_index.cmp(&b.r_index))
                })
            {
                self.update_one_r30(user_id, target_tuple.r_index, score)
                    .await?;
                return Ok(());
            }
        } else {
            // Not protected: find oldest score in filtered songs (efficient iterator approach)
            if let Some(oldest_r_index) = filtered_songs
                .values()
                .flat_map(|entries| entries.iter())
                .max_by_key(|(idx, _, _)| *idx)
                .map(|(_, r_index, _)| *r_index)
            {
                self.update_one_r30(user_id, oldest_r_index, score).await?;
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

    async fn get_user_global_rank(&self, user_id: i32) -> ArcResult<i32> {
        let user_score = sqlx::query!(
            "SELECT world_rank_score FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(score_row) = user_score else {
            return Ok(0);
        };

        let world_rank_score = score_row.world_rank_score.unwrap_or(0);
        if world_rank_score == 0 {
            return Ok(0);
        }

        let rank_result = sqlx::query!(
            "SELECT COUNT(*) as count FROM user WHERE world_rank_score > ?",
            world_rank_score
        )
        .fetch_one(&self.pool)
        .await?;

        let rank = rank_result.count as i32 + 1;
        if rank <= CONFIG.world_rank_max {
            Ok(rank)
        } else {
            Ok(0)
        }
    }

    /// Record score to log database
    async fn record_score(&self, _user_play: &UserPlay) -> ArcResult<()> {
        // This would record to a separate log database
        // For now, this is a placeholder implementation
        Ok(())
    }

    /// Record user rating PTT changes to log database
    #[allow(dead_code)]
    async fn record_rating_ptt(&self, _user_id: i32, _user_rating_ptt: f64) -> ArcResult<()> {
        // This would record to a separate log database
        // For now, this is a placeholder implementation
        Ok(())
    }

    /// Handle world mode calculations and build Python-compatible payload.
    async fn handle_world_mode(&self, user_play: &mut UserPlay) -> ArcResult<JsonMap> {
        let user_id = user_play.user_score.user_id;

        let user_row = sqlx::query(
            "SELECT character_id, is_skill_sealed, current_map, world_mode_locked_end_ts, beyond_boost_gauge
             FROM user WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let mut character_id = user_row
            .try_get::<Option<i32>, _>("character_id")?
            .unwrap_or(0);
        let is_skill_sealed = user_row
            .try_get::<Option<i8>, _>("is_skill_sealed")?
            .unwrap_or(0)
            != 0;
        let mut current_map = user_row
            .try_get::<Option<String>, _>("current_map")?
            .unwrap_or_default();
        if current_map.is_empty() {
            current_map = "tutorial".to_string();
        }
        let mut world_mode_locked_end_ts = user_row
            .try_get::<Option<i64>, _>("world_mode_locked_end_ts")?
            .unwrap_or(-1);
        let mut beyond_boost_gauge = user_row
            .try_get::<Option<f64>, _>("beyond_boost_gauge")?
            .unwrap_or(0.0);

        let parser = get_map_parser();
        let map = parser.load_world_map(&current_map)?;

        let user_world = sqlx::query(
            "SELECT curr_position, curr_capture, is_locked FROM user_world WHERE user_id = ? AND map_id = ?",
        )
        .bind(user_id)
        .bind(&current_map)
        .fetch_optional(&self.pool)
        .await?;

        let (mut curr_position, mut curr_capture, is_locked) = if let Some(row) = user_world {
            (
                row.try_get::<Option<i32>, _>("curr_position")?.unwrap_or(0),
                row.try_get::<Option<f64>, _>("curr_capture")?
                    .unwrap_or(0.0),
                row.try_get::<Option<i8>, _>("is_locked")?.unwrap_or(1) != 0,
            )
        } else {
            sqlx::query(
                "INSERT INTO user_world (user_id, map_id, curr_position, curr_capture, is_locked) VALUES (?, ?, 0, 0, 1)",
            )
            .bind(user_id)
            .bind(&current_map)
            .execute(&self.pool)
            .await?;
            (0, 0.0, true)
        };

        if is_locked {
            return Err(ArcError::MapLocked {
                message: "The map is locked.".to_string(),
                error_code: 108,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        let prev_position = curr_position;
        let prev_capture = curr_capture;

        if !is_skill_sealed
            && (user_play.invasion_flag == 1
                || (user_play.invasion_flag == 2 && user_play.user_score.score.health <= 0))
        {
            character_id = 72;
        }

        let character_service = CharacterService::new(self.pool.clone());
        let mut character = character_service
            .get_user_character_info(user_id, character_id)
            .await?;
        let mut skill_id_displayed = character.skill_id_displayed();

        if is_skill_sealed {
            character.skill.skill_id = None;
            character.skill.skill_id_uncap = None;
            character.skill.skill_unlock_level = i32::MAX;
            character.frag.set_parameter(50.0, 50.0, 50.0);
            character.prog.set_parameter(50.0, 50.0, 50.0);
            character.overdrive.set_parameter(50.0, 50.0, 50.0);
            character.frag.addition = 0.0;
            character.prog.addition = 0.0;
            character.overdrive.addition = 0.0;
            skill_id_displayed = None;
        }

        if user_play.prog_boost_multiply != 0 {
            sqlx::query("UPDATE user SET prog_boost = 0 WHERE user_id = ?")
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        self.clear_user_songplay_tokens(user_id).await?;

        let rating = user_play.user_score.score.rating;
        let stamina_multiply = user_play.stamina_multiply as f64;
        let fragment_multiply = user_play.fragment_multiply as f64;
        let prog_boost_multiply = user_play.prog_boost_multiply as f64;
        let beyond_boost_usage = user_play.beyond_boost_gauge_usage as f64;

        let frag_value = character.frag_value();
        let mut prog_value = character.prog_value();
        let overdrive_value = character.overdrive_value();

        let (
            base_progress,
            progress_normalized,
            final_progress,
            mut partner_multiply,
            _step_times,
            affinity_multiply,
            new_law_multiply,
        ) = if map.is_beyond {
            let base_progress = rating.sqrt() * 0.43
                + if user_play.user_score.score.clear_type == 0 {
                    25.0 / 28.0
                } else {
                    75.0 / 28.0
                };
            let step_times = stamina_multiply * fragment_multiply / 100.0
                * (1.0 + prog_boost_multiply / 100.0 + beyond_boost_usage / 100.0);

            let partner_multiply = overdrive_value / 50.0;
            let mut affinity_multiply = 1.0;
            let mut new_law_multiply = 1.0;

            let progress_normalized = if map.is_breached {
                if let Some(new_law) = &map.new_law {
                    let new_law_prog = match new_law.as_str() {
                        "over100_step50" => Some(overdrive_value + prog_value / 2.0),
                        "frag50" => Some(frag_value),
                        "lowlevel" => {
                            Some(50.0 * f64::max(1.0, 2.0 - 0.1 * character.level.level as f64))
                        }
                        "antiheroism" => {
                            let x = (overdrive_value - frag_value).abs();
                            let y = (overdrive_value - prog_value).abs();
                            Some(overdrive_value - (x - y).abs())
                        }
                        _ => None,
                    };
                    if let Some(v) = new_law_prog {
                        new_law_multiply = v / 50.0;
                    }
                }
                if map.disable_over.unwrap_or(false) {
                    base_progress * new_law_multiply
                } else {
                    base_progress * partner_multiply * new_law_multiply
                }
            } else {
                if let Some(idx) = map
                    .character_affinity
                    .iter()
                    .position(|&id| id == character.character_id)
                {
                    if let Some(multiplier) = map.affinity_multiplier.get(idx) {
                        affinity_multiply = *multiplier;
                    }
                }
                base_progress * partner_multiply * affinity_multiply
            };

            let final_progress = progress_normalized * step_times;
            (
                base_progress,
                progress_normalized,
                final_progress,
                partner_multiply,
                step_times,
                affinity_multiply,
                new_law_multiply,
            )
        } else {
            let base_progress = 2.5 + 2.45 * rating.sqrt();
            let partner_multiply = prog_value / 50.0;
            let progress_normalized = base_progress * partner_multiply;
            let step_times =
                stamina_multiply * fragment_multiply / 100.0 * (prog_boost_multiply / 100.0 + 1.0);
            let final_progress = progress_normalized * step_times;
            (
                base_progress,
                progress_normalized,
                final_progress,
                partner_multiply,
                step_times,
                1.0,
                1.0,
            )
        };

        let (next_position, next_capture) = climb_user_map(
            &map.steps,
            map.is_beyond,
            map.beyond_health.unwrap_or(0) as f64,
            prev_position,
            prev_capture,
            final_progress,
        );
        curr_position = next_position;
        curr_capture = next_capture;

        let item_service = ItemService::new(self.pool.clone());
        let mut rewards = Vec::new();
        if curr_position > prev_position {
            for i in (prev_position + 1)..=curr_position {
                if let Some(step) = map.steps.get(i as usize) {
                    if !step.items.is_empty() {
                        rewards.push(json!({
                            "position": step.position,
                            "items": step.items.iter().map(step_item_to_value).collect::<Vec<_>>()
                        }));
                    }

                    for item in &step.items {
                        item_service
                            .claim_item(user_id, &item.item_id, &item.item_type, item.amount)
                            .await?;
                    }
                }
            }
        }

        let steps_for_climbing_pre_reset =
            steps_for_climbing(&map.steps, prev_position, curr_position);
        if let Some(last_step) = steps_for_climbing_pre_reset.last() {
            if last_step.step_type.iter().any(|x| x == "plusstamina") {
                if let Some(plus_stamina) = last_step.plus_stamina_value {
                    let stamina_row =
                        sqlx::query("SELECT max_stamina_ts, stamina FROM user WHERE user_id = ?")
                            .bind(user_id)
                            .fetch_one(&self.pool)
                            .await?;
                    let mut stamina = StaminaImpl::new(
                        stamina_row
                            .try_get::<Option<i32>, _>("stamina")?
                            .unwrap_or(0),
                        stamina_row
                            .try_get::<Option<i64>, _>("max_stamina_ts")?
                            .unwrap_or(0),
                    );
                    let current_stamina = stamina.get_current_stamina();
                    stamina.set_stamina(current_stamina + plus_stamina);
                    sqlx::query(
                        "UPDATE user SET stamina = ?, max_stamina_ts = ? WHERE user_id = ?",
                    )
                    .bind(stamina.get_current_stamina())
                    .bind(stamina.max_stamina_ts())
                    .bind(user_id)
                    .execute(&self.pool)
                    .await?;
                }
            }
        }

        if !CONFIG.character_full_unlock && !is_skill_sealed {
            let exp_addition =
                stamina_multiply * (prog_boost_multiply / 100.0 + 1.0) * rating * 6.0;
            if exp_addition != 0.0 {
                character = character_service
                    .upgrade_character(user_id, character.character_id, exp_addition)
                    .await?;
                prog_value = character.prog_value();
                if !map.is_beyond {
                    partner_multiply = prog_value / 50.0;
                }
            }
        }

        if !is_skill_sealed {
            if let Some(skill_id) = skill_id_displayed.as_deref() {
                if skill_id == "skill_fatalis" {
                    world_mode_locked_end_ts =
                        current_timestamp() + Constants::SKILL_FATALIS_WORLD_LOCKED_TIME;
                    sqlx::query("UPDATE user SET world_mode_locked_end_ts = ? WHERE user_id = ?")
                        .bind(world_mode_locked_end_ts)
                        .bind(user_id)
                        .execute(&self.pool)
                        .await?;
                } else if skill_id == "skill_maya" {
                    character_service
                        .change_character_skill_state(user_id, character.character_id)
                        .await?;
                    character.skill_flag = !character.skill_flag;
                }
            }
        }

        if map.is_beyond {
            if user_play.beyond_boost_gauge_usage > 0
                && user_play.beyond_boost_gauge_usage as f64 <= beyond_boost_gauge
            {
                beyond_boost_gauge -= user_play.beyond_boost_gauge_usage as f64;
                if beyond_boost_gauge.abs() <= 1e-5 {
                    beyond_boost_gauge = 0.0;
                }
                sqlx::query("UPDATE user SET beyond_boost_gauge = ? WHERE user_id = ?")
                    .bind(beyond_boost_gauge)
                    .bind(user_id)
                    .execute(&self.pool)
                    .await?;
            }
        } else {
            beyond_boost_gauge += 2.45 * rating.sqrt() + 27.0;
            if beyond_boost_gauge > 200.0 {
                beyond_boost_gauge = 200.0;
            }
            sqlx::query("UPDATE user SET beyond_boost_gauge = ? WHERE user_id = ?")
                .bind(beyond_boost_gauge)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        if curr_position == map.step_count - 1 && map.is_repeatable {
            curr_position = 0;
        }

        sqlx::query(
            "UPDATE user_world SET curr_position = ?, curr_capture = ?, is_locked = 0 WHERE user_id = ? AND map_id = ?",
        )
        .bind(curr_position)
        .bind(curr_capture)
        .bind(user_id)
        .bind(&current_map)
        .execute(&self.pool)
        .await?;

        let user_service = UserService::new(self.pool.clone());
        user_service
            .update_user_world_complete_info(user_id)
            .await?;

        let (current_stamina, max_stamina_ts) = self.get_user_stamina_info(user_id).await?;
        let steps_for_response = steps_for_climbing(&map.steps, prev_position, curr_position);

        let mut user_map = json!({
            "user_id": user_id,
            "curr_position": curr_position,
            "curr_capture": curr_capture,
            "is_locked": false,
            "map_id": current_map,
            "prev_capture": prev_capture,
            "prev_position": prev_position,
            "beyond_health": map.beyond_health
        });

        let mut char_stats = json!({
            "character_id": character.character_id,
            "frag": character.frag_value(),
            "prog": character.prog_value(),
            "overdrive": character.overdrive_value()
        });

        if let Some(skill_state) = character.skill_state() {
            if let Value::Object(ref mut map_obj) = char_stats {
                map_obj.insert("skill_state".to_string(), Value::String(skill_state));
            }
        }

        let mut result = HashMap::new();
        result.insert("rewards".to_string(), Value::Array(rewards));
        result.insert("exp".to_string(), json!(character.level.exp));
        result.insert("level".to_string(), json!(character.level.level));
        result.insert("base_progress".to_string(), json!(base_progress));
        result.insert("progress".to_string(), json!(final_progress));
        result.insert("user_map".to_string(), user_map.clone());
        result.insert("char_stats".to_string(), char_stats);
        result.insert("current_stamina".to_string(), json!(current_stamina));
        result.insert("max_stamina_ts".to_string(), json!(max_stamina_ts));
        result.insert(
            "world_mode_locked_end_ts".to_string(),
            json!(world_mode_locked_end_ts),
        );
        result.insert("beyond_boost_gauge".to_string(), json!(beyond_boost_gauge));
        result.insert(
            "progress_before_sub_boost".to_string(),
            json!(final_progress),
        );
        result.insert("progress_sub_boost_amount".to_string(), json!(0));
        result.insert("partner_multiply".to_string(), json!(partner_multiply));

        if user_play.stamina_multiply != 1 {
            result.insert(
                "stamina_multiply".to_string(),
                json!(user_play.stamina_multiply),
            );
        }
        if user_play.fragment_multiply != 100 {
            result.insert(
                "fragment_multiply".to_string(),
                json!(user_play.fragment_multiply),
            );
        }
        if user_play.prog_boost_multiply != 0 {
            result.insert(
                "prog_boost_multiply".to_string(),
                json!(user_play.prog_boost_multiply),
            );
        }

        if map.is_beyond {
            result.insert(
                "pre_boost_progress".to_string(),
                json!(progress_normalized * fragment_multiply / 100.0),
            );
            if let Value::Object(ref mut map_obj) = user_map {
                map_obj.insert("steps".to_string(), json!(steps_for_response.len() as i32));
            }
            result.insert("user_map".to_string(), user_map);
            result.insert("affinity_multiply".to_string(), json!(affinity_multiply));
            if user_play.beyond_boost_gauge_usage != 0 {
                result.insert(
                    "beyond_boost_gauge_usage".to_string(),
                    json!(user_play.beyond_boost_gauge_usage),
                );
            }
            if map.is_breached {
                result.insert("new_law_multiply".to_string(), json!(new_law_multiply));
            }
        } else {
            result.insert(
                "progress_partial_after_stat".to_string(),
                json!(progress_normalized),
            );
            result.insert("partner_adjusted_prog".to_string(), json!(prog_value));
            if let Value::Object(ref mut map_obj) = user_map {
                map_obj.insert(
                    "steps".to_string(),
                    Value::Array(
                        steps_for_response
                            .iter()
                            .map(|step| step.to_dict())
                            .collect::<Vec<_>>(),
                    ),
                );
            }
            result.insert("user_map".to_string(), user_map);
        }

        Ok(result)
    }

    /// Handle course mode calculations and build Python-compatible payload.
    async fn handle_course_mode(&self, user_play: &mut UserPlay) -> ArcResult<JsonMap> {
        let user_id = user_play.user_score.user_id;
        let Some(course_id) = user_play.course_id.clone() else {
            return Ok(HashMap::new());
        };

        let mut course_score = user_play.course_score + user_play.user_score.score.score;
        let mut course_clear_type = user_play.course_clear_type;

        let user_course = sqlx::query(
            "SELECT high_score, best_clear_type FROM user_course WHERE user_id = ? AND course_id = ?",
        )
        .bind(user_id)
        .bind(&course_id)
        .fetch_optional(&self.pool)
        .await?;

        let (mut high_score, mut best_clear_type) = if let Some(row) = user_course {
            (
                row.try_get::<Option<i32>, _>("high_score")?.unwrap_or(0),
                row.try_get::<Option<i32>, _>("best_clear_type")?
                    .unwrap_or(0),
            )
        } else {
            (0, 0)
        };

        let mut need_upsert = false;
        if course_score > high_score {
            high_score = course_score;
            need_upsert = true;
        }

        if user_play.user_score.score.health < 0 {
            user_play.course_play_state = 5;
            course_score = 0;
            course_clear_type = 0;

            sqlx::query(
                "UPDATE songplay_token SET course_state = ?, course_score = ?, course_clear_type = ? WHERE token = ?",
            )
            .bind(user_play.course_play_state)
            .bind(course_score)
            .bind(course_clear_type)
            .bind(&user_play.song_token)
            .execute(&self.pool)
            .await?;

            if need_upsert {
                sqlx::query(
                    "INSERT INTO user_course (user_id, course_id, high_score, best_clear_type)
                     VALUES (?, ?, ?, ?)
                     ON DUPLICATE KEY UPDATE high_score = VALUES(high_score), best_clear_type = VALUES(best_clear_type)",
                )
                .bind(user_id)
                .bind(&course_id)
                .bind(high_score)
                .bind(best_clear_type)
                .execute(&self.pool)
                .await?;
            }

            return Ok(HashMap::new());
        }

        user_play.course_play_state += 1;
        if Score::get_song_state(course_clear_type)
            > Score::get_song_state(user_play.user_score.score.clear_type)
        {
            course_clear_type = user_play.user_score.score.clear_type;
        }

        sqlx::query(
            "UPDATE songplay_token SET course_state = ?, course_score = ?, course_clear_type = ? WHERE token = ?",
        )
        .bind(user_play.course_play_state)
        .bind(course_score)
        .bind(course_clear_type)
        .bind(&user_play.song_token)
        .execute(&self.pool)
        .await?;

        let mut rewards = Vec::new();
        if user_play.course_play_state == 4 {
            if best_clear_type == 0 {
                let course_items = sqlx::query(
                    "SELECT item_id, type, amount FROM course_item WHERE course_id = ?",
                )
                .bind(&course_id)
                .fetch_all(&self.pool)
                .await?;

                let item_service = ItemService::new(self.pool.clone());
                for row in course_items {
                    let item_id = row.try_get::<String, _>("item_id")?;
                    let item_type = row.try_get::<String, _>("type")?;
                    let amount = row.try_get::<Option<i32>, _>("amount")?.unwrap_or(1);
                    item_service
                        .claim_item(user_id, &item_id, &item_type, amount)
                        .await?;
                    rewards.push(json!({
                        "id": item_id,
                        "type": item_type,
                        "amount": amount
                    }));
                }
            }

            if Score::get_song_state(course_clear_type) > Score::get_song_state(best_clear_type) {
                best_clear_type = course_clear_type;
                need_upsert = true;
            }
        }

        if need_upsert {
            sqlx::query(
                "INSERT INTO user_course (user_id, course_id, high_score, best_clear_type)
                 VALUES (?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE high_score = VALUES(high_score), best_clear_type = VALUES(best_clear_type)",
            )
            .bind(user_id)
            .bind(&course_id)
            .bind(high_score)
            .bind(best_clear_type)
            .execute(&self.pool)
            .await?;
        }

        if user_play.course_play_state == 4 {
            let (stamina, max_stamina_ts) = self.get_user_stamina_info(user_id).await?;
            let mut result = HashMap::new();
            result.insert("rewards".to_string(), Value::Array(rewards));
            result.insert("current_stamina".to_string(), json!(stamina));
            result.insert("max_stamina_ts".to_string(), json!(max_stamina_ts));
            result.insert(
                "user_course_banners".to_string(),
                Value::Array(self.get_user_course_banners(user_id).await?),
            );
            return Ok(result);
        }

        Ok(HashMap::new())
    }

    async fn get_user_course_banners(&self, user_id: i32) -> ArcResult<Vec<Value>> {
        let rows = sqlx::query(
            "SELECT item_id FROM user_item WHERE user_id = ? AND type = 'course_banner'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut banners = Vec::with_capacity(rows.len());
        for row in rows {
            banners.push(Value::String(row.try_get::<String, _>("item_id")?));
        }
        Ok(banners)
    }

    /// Get user's current world map stamina cost
    async fn get_user_current_map(&self, user_id: i32) -> ArcResult<i32> {
        let user = sqlx::query!("SELECT current_map FROM user WHERE user_id = ?", user_id)
            .fetch_one(&self.pool)
            .await?;

        let current_map = user.current_map.unwrap_or_default();
        let current_map = if current_map.is_empty() {
            "tutorial".to_string()
        } else {
            current_map
        };

        let parser = get_map_parser();
        let map = parser.load_world_map(&current_map)?;

        Ok(map.stamina_cost.unwrap_or(1))
    }

    /// Update user's global rank
    async fn update_user_global_rank(&self, user_id: i32) -> ArcResult<()> {
        // This would calculate and update the user's global ranking
        // based on their score_v2 values

        // Calculate total score_v2
        let total_score = sqlx::query!(
            r#"WITH user_scores AS (
                SELECT song_id, difficulty, score_v2
                FROM best_score
                WHERE user_id = ?
                AND difficulty IN (2, 3, 4)
            )
            SELECT SUM(cal_score) AS total FROM (
                SELECT SUM(score_v2) AS cal_score
                FROM user_scores
                WHERE difficulty = 2
                AND song_id IN (SELECT song_id FROM chart WHERE rating_ftr > 0)

                UNION ALL

                SELECT SUM(score_v2) AS cal_score
                FROM user_scores
                WHERE difficulty = 3
                AND song_id IN (SELECT song_id FROM chart WHERE rating_byn > 0)

                UNION ALL

                SELECT SUM(score_v2) AS cal_score
                FROM user_scores
                WHERE difficulty = 4
                AND song_id IN (SELECT song_id FROM chart WHERE rating_etr > 0)
            ) AS subquery"#,
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

fn step_item_to_value(item: &crate::model::world::StepItem) -> Value {
    json!({
        "id": item.item_id,
        "type": item.item_type,
        "amount": item.amount
    })
}

fn steps_for_climbing(
    steps: &[WorldStep],
    prev_position: i32,
    curr_position: i32,
) -> Vec<WorldStep> {
    if curr_position < prev_position || steps.is_empty() {
        return Vec::new();
    }

    let start = prev_position.max(0) as usize;
    let mut end = (curr_position + 1).max(0) as usize;
    end = end.min(steps.len());
    if start >= end {
        return Vec::new();
    }

    steps[start..end].to_vec()
}

fn climb_user_map(
    steps: &[WorldStep],
    is_beyond: bool,
    beyond_health: f64,
    prev_position: i32,
    prev_capture: f64,
    step_value: f64,
) -> (i32, f64) {
    if step_value < 0.0 || steps.is_empty() {
        return (prev_position.max(0), prev_capture.max(0.0));
    }

    if is_beyond {
        let mut curr_capture = prev_capture + step_value;
        if curr_capture > beyond_health {
            curr_capture = beyond_health;
        }

        let mut i = 0usize;
        let mut t = prev_capture + step_value;
        while i < steps.len() && t > 0.0 {
            let dt = steps[i].capture as f64;
            if dt > t {
                t = 0.0;
            } else {
                t -= dt;
                i += 1;
            }
        }

        let curr_position = if i >= steps.len() {
            steps.len() as i32 - 1
        } else {
            i as i32
        };
        return (curr_position, curr_capture);
    }

    let mut i = prev_position.max(0) as usize;
    let mut j = prev_capture;
    let mut t = step_value;
    while t > 0.0 && i < steps.len() {
        let dt = steps[i].capture as f64 - j;
        if dt > t {
            j += t;
            t = 0.0;
        } else {
            t -= dt;
            j = 0.0;
            i += 1;
        }
    }

    if i >= steps.len() {
        (steps.len() as i32 - 1, 0.0)
    } else {
        (i as i32, j)
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
        0 => "frag".to_string(),
        1 => "prog".to_string(),
        2 => "over".to_string(),
        _ => "frag".to_string(),
    }
}

fn parse_bool_string(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn current_timestamp_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Calculate MD5 hash of a string
pub fn md5_hash(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}
