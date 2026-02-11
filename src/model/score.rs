use crate::config::CONFIG;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

type SongKey = (String, i32);
type SongEntry = (usize, i32, f64);
type SongEntryMap = HashMap<SongKey, Vec<SongEntry>>;

/// Basic score data structure for score calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub song_id: String,
    pub difficulty: i32,
    pub score: i32,
    pub shiny_perfect_count: i32,
    pub perfect_count: i32,
    pub near_count: i32,
    pub miss_count: i32,
    pub health: i32,
    pub modifier: i32,
    pub time_played: i64,
    pub clear_type: i32,
    pub rating: f64,
    pub score_v2: f64,
    pub chart_const: Option<f64>,
    pub song_name: Option<String>,
}

impl Default for Score {
    fn default() -> Self {
        Self::new()
    }
}

impl Score {
    /// Create a new Score instance
    pub fn new() -> Self {
        Self {
            song_id: String::new(),
            difficulty: 0,
            score: 0,
            shiny_perfect_count: 0,
            perfect_count: 0,
            near_count: 0,
            miss_count: 0,
            health: 0,
            modifier: 0,
            time_played: 0,
            clear_type: 0,
            rating: 0.0,
            score_v2: 0.0,
            chart_const: None,
            song_name: None,
        }
    }

    /// Set chart information
    pub fn set_chart(&mut self, song_id: String, difficulty: i32) {
        self.song_id = song_id;
        self.difficulty = difficulty;
    }

    /// Set score data from individual components
    #[allow(clippy::too_many_arguments)]
    pub fn set_score(
        &mut self,
        score: Option<i32>,
        shiny_perfect_count: Option<i32>,
        perfect_count: Option<i32>,
        near_count: Option<i32>,
        miss_count: Option<i32>,
        health: Option<i32>,
        modifier: Option<i32>,
        time_played: Option<i64>,
        clear_type: Option<i32>,
    ) {
        self.score = score.unwrap_or(0);
        self.shiny_perfect_count = shiny_perfect_count.unwrap_or(0);
        self.perfect_count = perfect_count.unwrap_or(0);
        self.near_count = near_count.unwrap_or(0);
        self.miss_count = miss_count.unwrap_or(0);
        self.health = health.unwrap_or(0);
        self.modifier = modifier.unwrap_or(0);
        self.time_played = time_played.unwrap_or(0);
        self.clear_type = clear_type.unwrap_or(0);
    }

    /// Convert score to grade (0-6, where 6 is EX+)
    pub fn get_song_grade(score: i32) -> i32 {
        match score {
            s if s >= 9900000 => 6, // EX+
            s if s >= 9800000 => 5, // EX
            s if s >= 9500000 => 4, // AA
            s if s >= 9200000 => 3, // A
            s if s >= 8900000 => 2, // B
            s if s >= 8600000 => 1, // C
            _ => 0,                 // D
        }
    }

    /// Get song grade for this score
    pub fn song_grade(&self) -> i32 {
        Self::get_song_grade(self.score)
    }

    /// Convert clear_type to song state for comparison
    pub fn get_song_state(clear_type: i32) -> i32 {
        match clear_type {
            3 => 5, // PM (Perfect Memory)
            2 => 4, // FC (Full Combo)
            5 => 3, // Hard Clear
            1 => 2, // Clear
            4 => 1, // Easy Clear
            _ => 0, // Track Lost
        }
    }

    /// Get song state for this score
    pub fn song_state(&self) -> i32 {
        Self::get_song_state(self.clear_type)
    }

    /// Get total note count
    pub fn all_note_count(&self) -> i32 {
        self.perfect_count + self.near_count + self.miss_count
    }

    /// Create tuple representation for comparisons
    pub fn to_tuple(&self) -> (String, i32) {
        (self.song_id.clone(), self.difficulty)
    }

    /// Validate score data
    pub fn is_valid(&self) -> bool {
        // Check for negative values
        if self.shiny_perfect_count < 0
            || self.perfect_count < 0
            || self.near_count < 0
            || self.miss_count < 0
            || self.score < 0
            || self.time_played <= 0
        {
            return false;
        }

        // Check difficulty range
        if !(0..=4).contains(&self.difficulty) {
            return false;
        }

        let all_note = self.all_note_count();
        if all_note == 0 {
            return false;
        }

        // Validate calculated score
        let calc_score = 10000000.0 / all_note as f64
            * (self.perfect_count as f64 + self.near_count as f64 / 2.0)
            + self.shiny_perfect_count as f64;

        if (calc_score - self.score as f64).abs() >= 5.0 {
            return false;
        }

        true
    }

    /// Calculate rating based on chart constant and score
    pub fn calculate_rating(defnum: f64, score: i32) -> f64 {
        if defnum <= 0.0 {
            return -1.0; // Unranked
        }

        if score >= 10000000 {
            defnum + 2.0
        } else if score < 9800000 {
            let ptt = defnum + (score - 9500000) as f64 / 300000.0;
            ptt.max(0.0)
        } else {
            defnum + 1.0 + (score - 9800000) as f64 / 200000.0
        }
    }

    /// Calculate score_v2 for global ranking
    pub fn calculate_score_v2(
        defnum: f64,
        shiny_perfect_count: i32,
        perfect_count: i32,
        near_count: i32,
        miss_count: i32,
    ) -> f64 {
        if defnum <= 0.0 {
            return 0.0; // Unranked
        }

        let all_note = perfect_count + near_count + miss_count;
        if all_note == 0 {
            return 0.0;
        }

        let shiny_ratio = shiny_perfect_count as f64 / all_note as f64;
        let score_ratio = (perfect_count as f64 + near_count as f64 / 2.0) / all_note as f64
            + shiny_perfect_count as f64 / 10000000.0;

        let acc_rating = (shiny_ratio - 0.9).clamp(0.0, 0.095) / 9.5 * 25.0;
        let score_rating = (score_ratio - 0.99).clamp(0.0, 0.01) * 75.0;

        defnum * (acc_rating + score_rating)
    }

    /// Calculate and set rating for this score
    pub fn get_rating_by_calc(&mut self, chart_const: f64) -> f64 {
        self.chart_const = Some(chart_const);
        self.rating = Self::calculate_rating(chart_const, self.score);
        self.score_v2 = Self::calculate_score_v2(
            chart_const,
            self.shiny_perfect_count,
            self.perfect_count,
            self.near_count,
            self.miss_count,
        );
        self.rating
    }

    /// Convert to dictionary representation
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();
        result.insert("rating".to_string(), Value::from(self.rating));
        result.insert("modifier".to_string(), Value::from(self.modifier));
        result.insert("time_played".to_string(), Value::from(self.time_played));
        result.insert("health".to_string(), Value::from(self.health));
        result.insert("clear_type".to_string(), Value::from(self.clear_type));
        result.insert("miss_count".to_string(), Value::from(self.miss_count));
        result.insert("near_count".to_string(), Value::from(self.near_count));
        result.insert("perfect_count".to_string(), Value::from(self.perfect_count));
        result.insert(
            "shiny_perfect_count".to_string(),
            Value::from(self.shiny_perfect_count),
        );
        result.insert("score".to_string(), Value::from(self.score));
        result.insert("difficulty".to_string(), Value::from(self.difficulty));
        result.insert("song_id".to_string(), Value::from(self.song_id.clone()));

        if let Some(ref song_name) = self.song_name {
            result.insert("song_name".to_string(), Value::from(song_name.clone()));
        }

        result
    }
}

/// User score with user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserScore {
    #[serde(flatten)]
    pub score: Score,
    pub user_id: i32,
    pub name: String,
    pub best_clear_type: i32,
    pub character: i32,
    pub is_char_uncapped: i8,
    pub is_skill_sealed: i8,
    pub rank: Option<i32>,
}

impl UserScore {
    /// Create new UserScore from database row
    pub fn from_best_score_row(
        row: &crate::model::download::BestScore,
        user_info: (i32, String, i32, i8, i8),
    ) -> Self {
        let mut score = Score::new();
        score.song_id = row.song_id.clone();
        score.difficulty = row.difficulty;
        score.score = row.score;
        score.shiny_perfect_count = row.shiny_perfect_count;
        score.perfect_count = row.perfect_count;
        score.near_count = row.near_count;
        score.miss_count = row.miss_count;
        score.health = row.health;
        score.modifier = row.modifier;
        score.time_played = row.time_played;
        score.clear_type = row.clear_type;
        score.rating = row.rating;
        score.score_v2 = row.score_v2;

        Self {
            score,
            user_id: user_info.0,
            name: user_info.1,
            best_clear_type: row.best_clear_type,
            character: user_info.2,
            is_char_uncapped: user_info.3,
            is_skill_sealed: user_info.4,
            rank: None,
        }
    }

    /// Create from list data (from SQL result) - simplified version
    pub fn from_list(&mut self, data: Vec<i32>) {
        // Simplified implementation - would need proper database row parsing
        // This is a placeholder for the actual implementation
        if data.len() >= 3 {
            self.user_id = data[0];
            self.score.score = data.get(3).copied().unwrap_or(0);
            self.score.rating = data.get(13).copied().unwrap_or(0) as f64;
        }
    }

    /// Convert to dictionary with user info
    pub fn to_dict(&self, has_user_info: bool) -> HashMap<String, serde_json::Value> {
        let mut result = self.score.to_dict();
        result.insert(
            "best_clear_type".to_string(),
            Value::from(self.best_clear_type),
        );

        if has_user_info {
            result.insert("user_id".to_string(), Value::from(self.user_id));
            result.insert("name".to_string(), Value::from(self.name.clone()));
            result.insert(
                "is_skill_sealed".to_string(),
                Value::from(self.is_skill_sealed != 0),
            );
            result.insert(
                "is_char_uncapped".to_string(),
                Value::from(self.is_char_uncapped != 0),
            );
            result.insert("character".to_string(), Value::from(self.character));
        }

        if let Some(rank) = self.rank {
            result.insert("rank".to_string(), Value::from(rank));
        }

        result
    }
}

/// User play session for score submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPlay {
    #[serde(flatten)]
    pub user_score: UserScore,
    pub song_token: String,
    pub song_hash: String,
    pub submission_hash: String,
    pub beyond_gauge: i32,
    pub unrank_flag: bool,
    pub new_best_protect_flag: bool,

    // World mode fields
    pub is_world_mode: Option<bool>,
    pub stamina_multiply: i32,
    pub fragment_multiply: i32,
    pub prog_boost_multiply: i32,
    pub beyond_boost_gauge_usage: i32,

    // Course mode fields
    pub course_play_state: i32,

    // Special skill fields
    pub combo_interval_bonus: Option<i32>,
    pub hp_interval_bonus: Option<i32>,
    pub fever_bonus: Option<i32>,
    pub skill_cytusii_flag: Option<String>,
    pub skill_chinatsu_flag: Option<String>,
    pub highest_health: Option<i32>,
    pub lowest_health: Option<i32>,
    pub invasion_flag: i32,

    // World mode calculation fields - simplified for now
    pub ptt: Option<Potential>,
}

impl UserPlay {
    /// Check if score is protected (health -1, score >= 9800000, or new best)
    pub fn is_protected(&self) -> bool {
        self.user_score.score.health == -1
            || self.user_score.score.score >= 9800000
            || self.new_best_protect_flag
    }

    /// Validate score with hash checking
    pub fn is_valid(&self, expected_song_hash: Option<&str>) -> bool {
        if !self.user_score.score.is_valid() {
            return false;
        }

        // Check song hash if provided
        if let Some(expected_hash) = expected_song_hash {
            if expected_hash != self.song_hash {
                return false;
            }
        }

        // Build hash input string exactly like Python version
        let mut hash_input = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}",
            self.song_token,
            self.song_hash,
            self.user_score.score.song_id,
            self.user_score.score.difficulty,
            self.user_score.score.score,
            self.user_score.score.shiny_perfect_count,
            self.user_score.score.perfect_count,
            self.user_score.score.near_count,
            self.user_score.score.miss_count,
            self.user_score.score.health,
            self.user_score.score.modifier,
            self.user_score.score.clear_type
        );

        // Validate combo interval bonus and add to hash if present
        if let Some(combo_bonus) = self.combo_interval_bonus {
            if combo_bonus < 0 || combo_bonus > self.user_score.score.all_note_count() / 150 {
                return false;
            }
            hash_input.push_str(&combo_bonus.to_string());
        }

        // Validate hp interval bonus (but don't add to hash)
        if let Some(hp_bonus) = self.hp_interval_bonus {
            if hp_bonus < 0 {
                return false;
            }
        }

        // Validate fever bonus (but don't add to hash)
        if let Some(fever_bonus) = self.fever_bonus {
            if fever_bonus < 0 || fever_bonus > self.user_score.score.perfect_count * 5 {
                return false;
            }
        }

        // Validate submission hash exactly like Python version
        let user_hash_input = format!("{}{}", self.user_score.user_id, self.song_hash);
        let expected_hash = crate::service::score::md5_hash(&format!(
            "{}{}",
            hash_input,
            crate::service::score::md5_hash(&user_hash_input)
        ));

        expected_hash == self.submission_hash
    }

    /// Convert to response dictionary
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        // Check if we have world mode or course mode data - matching Python logic
        if self.is_world_mode.is_none() || self.course_play_state == -1 {
            return HashMap::new();
        }

        let mut result = HashMap::new();

        if self.course_play_state == 4 {
            // Course mode completed - TODO: implement course_play.to_dict()
            // For now, return empty dict for course completion
        } else if self.is_world_mode == Some(true) {
            // World mode - TODO: implement world_play.to_dict()
            // For now, return empty dict for world mode
        }

        // Add common fields matching Python implementation
        // Get user's rating_ptt (should be from user data, not user_id)
        result.insert("user_rating".to_string(), Value::from(0)); // TODO: get actual user.rating_ptt

        // finale_challenge_higher: check if this score's rating > user's ptt value
        if let Some(ref ptt) = self.ptt {
            result.insert(
                "finale_challenge_higher".to_string(),
                Value::from(self.user_score.score.rating > ptt.value()),
            );
        } else {
            result.insert("finale_challenge_higher".to_string(), Value::from(false));
        }

        result.insert("global_rank".to_string(), Value::Null); // TODO: implement global rank

        // finale_play_value calculation: 9.065 * rating^0.5
        let finale_play_value = 9.065 * self.user_score.score.rating.sqrt();
        result.insert(
            "finale_play_value".to_string(),
            Value::from(finale_play_value),
        );

        result
    }
}

/// Potential calculation for users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Potential {
    pub user_id: i32,
    pub best_30_sum: f64,
    pub recent_10_sum: f64,

    // Cache for recent30 data
    pub r30_tuples: Option<Vec<Recent30Tuple>>,
    pub r30: Option<Vec<Score>>,
    pub b30: Option<Vec<f64>>,
}

impl Potential {
    /// Create new potential instance
    pub fn new(user_id: i32) -> Self {
        Self {
            user_id,
            best_30_sum: 0.0,
            recent_10_sum: 0.0,
            r30_tuples: None,
            r30: None,
            b30: None,
        }
    }

    /// Calculate user's potential value
    pub fn value(&self) -> f64 {
        self.calculate_value(CONFIG.best30_weight, CONFIG.recent10_weight)
    }

    /// Calculate user's potential value with custom weights
    pub fn calculate_value(&self, best30_weight: f64, recent10_weight: f64) -> f64 {
        self.best_30_sum * best30_weight + self.recent_10_sum * recent10_weight
    }

    /// Calculate finale play value
    pub fn calculate_finale_play_value(rating: f64) -> f64 {
        9.065 * rating.sqrt()
    }

    /// Get best 30 sum
    pub fn best_30(&self) -> f64 {
        self.best_30_sum
    }

    /// Get recent 10 sum
    pub fn recent_10(&self) -> f64 {
        self.recent_10_sum
    }

    /// Update one recent30 entry
    pub fn update_one_r30(&mut self, r_index: i32, user_score: &UserScore) {
        // This would update the database and internal state
        if let Some(ref mut tuples) = self.r30_tuples {
            let new_tuple = Recent30Tuple::new(
                r_index,
                user_score.score.song_id.clone(),
                user_score.score.difficulty,
                user_score.score.rating,
            );

            // Find existing entry with same r_index and replace it
            if let Some(existing) = tuples.iter_mut().find(|t| t.r_index == r_index) {
                *existing = new_tuple;
            } else if tuples.len() < 30 {
                tuples.push(new_tuple);
            }
        }
    }

    /// Push score to recent30 with complex logic
    pub fn r30_push_score(&mut self, user_score: &UserPlay) {
        // This implements the complex recent30 logic from Python
        if self.r30_tuples.is_none() {
            return; // Would need to load from database first
        }

        let tuples = self.r30_tuples.as_ref().unwrap();

        if tuples.len() < 30 {
            self.update_one_r30(tuples.len() as i32, &user_score.user_score);
            return;
        }

        if user_score.is_protected() {
            // Protected score logic - find lowest rating to replace
            let lowest_rating = tuples
                .iter()
                .filter(|t| t.rating <= user_score.user_score.score.rating)
                .min_by(|a, b| a.rating.partial_cmp(&b.rating).unwrap());

            if let Some(lowest) = lowest_rating {
                self.update_one_r30(lowest.r_index, &user_score.user_score);
            }
            return;
        }

        // Complex unique song logic (simplified)
        let mut unique_songs: SongEntryMap = HashMap::new();

        for (i, tuple) in tuples.iter().enumerate() {
            let key = (tuple.song_id.clone(), tuple.difficulty);
            unique_songs
                .entry(key)
                .or_default()
                .push((i, tuple.r_index, tuple.rating));
        }

        let new_song = user_score.user_score.score.to_tuple();

        if unique_songs.len() >= 11
            || (unique_songs.len() == 10 && !unique_songs.contains_key(&new_song))
        {
            // Replace oldest
            if let Some(oldest) = tuples.last() {
                self.update_one_r30(oldest.r_index, &user_score.user_score);
            }
        } else {
            // Find oldest in filtered songs (songs with multiple entries)
            let filtered_songs: std::collections::HashMap<_, _> = unique_songs
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .collect();

            if let Some((_, entries)) = filtered_songs.iter().max_by_key(|(_, entries)| {
                entries.iter().map(|(idx, _, _)| *idx).max().unwrap_or(0)
            }) {
                if let Some((_, r_index, _)) = entries.iter().max_by_key(|(idx, _, _)| *idx) {
                    self.update_one_r30(*r_index, &user_score.user_score);
                }
            }
        }
    }
}

/// Recent 30 tuple for internal calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recent30Tuple {
    pub r_index: i32,
    pub song_id: String,
    pub difficulty: i32,
    pub rating: f64,
}

impl Recent30Tuple {
    pub fn new(r_index: i32, song_id: String, difficulty: i32, rating: f64) -> Self {
        Self {
            r_index,
            song_id,
            difficulty,
            rating,
        }
    }
}

/// User score list for queries
#[derive(Debug, Clone)]
pub struct UserScoreList {
    pub scores: Option<Vec<UserScore>>,
    // Query parameters would go here
}

impl UserScoreList {
    /// Create new user score list
    pub fn new() -> Self {
        Self { scores: None }
    }

    /// Convert scores to dictionary list
    pub fn to_dict_list(&self) -> Vec<HashMap<String, serde_json::Value>> {
        self.scores
            .as_ref()
            .map(|scores| scores.iter().map(|s| s.to_dict(false)).collect())
            .unwrap_or_default()
    }

    /// Select song names for all scores
    pub fn select_song_name(&mut self) {
        // This would query the database for song names
        if let Some(ref mut scores) = self.scores {
            for score in scores {
                // Would lookup song name from chart table
                score.score.song_name = Some("Unknown Song".to_string());
            }
        }
    }
}

impl Default for UserScoreList {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete ranking score row that maps directly to final result
#[derive(Debug, sqlx::FromRow)]
pub struct RankingScoreRow {
    // Score fields
    pub user_id: i32,
    pub song_id: String,
    pub difficulty: i32,
    pub score: Option<i32>,
    pub shiny_perfect_count: Option<i32>,
    pub perfect_count: Option<i32>,
    pub near_count: Option<i32>,
    pub miss_count: Option<i32>,
    pub health: Option<i32>,
    pub modifier: Option<i32>,
    pub time_played: Option<i64>,
    pub best_clear_type: Option<i32>,
    pub clear_type: Option<i32>,
    pub rating: Option<f64>,
    pub score_v2: Option<f64>,
    // User fields
    pub name: Option<String>,
    pub character_id: Option<i32>,
    pub is_char_uncapped: Option<i8>,
    pub is_char_uncapped_override: Option<i8>,
    pub favorite_character: Option<i32>,
    pub is_skill_sealed: Option<i8>,
    pub favorite_is_uncapped: Option<i8>,
    pub favorite_is_uncapped_override: Option<i8>,
}

/// Complete ranking score row with additional fields for friend rankings
#[derive(Debug, sqlx::FromRow)]
pub struct RankingScoreRowComplete {
    // Score fields
    pub user_id: i32,
    pub song_id: String,
    pub difficulty: i32,
    pub score: Option<i32>,
    pub shiny_perfect_count: Option<i32>,
    pub perfect_count: Option<i32>,
    pub near_count: Option<i32>,
    pub miss_count: Option<i32>,
    pub health: Option<i32>,
    pub modifier: Option<i32>,
    pub time_played: Option<i64>,
    pub best_clear_type: Option<i32>,
    pub clear_type: Option<i32>,
    pub rating: Option<f64>,
    pub score_v2: Option<f64>,
    // User fields
    pub name: Option<String>,
    pub character_id: Option<i32>,
    pub is_char_uncapped: Option<i8>,
    pub is_char_uncapped_override: Option<i8>,
    pub favorite_character: Option<i32>,
    pub is_skill_sealed: Option<i8>,
    pub favorite_is_uncapped: Option<i8>,
    pub favorite_is_uncapped_override: Option<i8>,
    // Song fields
    pub song_name: Option<String>,
}

impl RankingScoreRow {
    /// Convert to UserScore with rank
    pub fn to_user_score_with_rank(&self, rank: Option<i32>) -> UserScore {
        let mut score = Score::new();
        score.song_id = self.song_id.clone();
        score.difficulty = self.difficulty;
        score.score = self.score.unwrap_or(0);
        score.shiny_perfect_count = self.shiny_perfect_count.unwrap_or(0);
        score.perfect_count = self.perfect_count.unwrap_or(0);
        score.near_count = self.near_count.unwrap_or(0);
        score.miss_count = self.miss_count.unwrap_or(0);
        score.health = self.health.unwrap_or(0);
        score.modifier = self.modifier.unwrap_or(0);
        score.time_played = self.time_played.unwrap_or(0);
        score.clear_type = self.clear_type.unwrap_or(0);
        score.rating = self.rating.unwrap_or(0.0);
        score.score_v2 = self.score_v2.unwrap_or(0.0);

        let favorite_character_id = self.favorite_character.unwrap_or(-1);
        let (displayed_character, is_uncapped, is_uncapped_override) =
            if favorite_character_id == -1 {
                (
                    self.character_id.unwrap_or(0),
                    self.is_char_uncapped.unwrap_or(0),
                    self.is_char_uncapped_override.unwrap_or(0),
                )
            } else {
                (
                    favorite_character_id,
                    self.favorite_is_uncapped.unwrap_or(0),
                    self.favorite_is_uncapped_override.unwrap_or(0),
                )
            };
        let is_char_uncapped_displayed = if is_uncapped_override != 0 {
            0
        } else {
            is_uncapped
        };

        UserScore {
            score,
            user_id: self.user_id,
            name: self.name.clone().unwrap_or_default(),
            best_clear_type: self.best_clear_type.unwrap_or(0),
            character: displayed_character,
            is_char_uncapped: is_char_uncapped_displayed,
            is_skill_sealed: self.is_skill_sealed.unwrap_or(0),
            rank,
        }
    }
}

impl RankingScoreRowComplete {
    /// Convert to UserScore with rank using character display logic
    pub fn to_user_score_with_rank_and_display(&self, rank: Option<i32>) -> UserScore {
        let mut score = Score::new();
        score.song_id = self.song_id.clone();
        score.difficulty = self.difficulty;
        score.score = self.score.unwrap_or(0);
        score.shiny_perfect_count = self.shiny_perfect_count.unwrap_or(0);
        score.perfect_count = self.perfect_count.unwrap_or(0);
        score.near_count = self.near_count.unwrap_or(0);
        score.miss_count = self.miss_count.unwrap_or(0);
        score.health = self.health.unwrap_or(0);
        score.modifier = self.modifier.unwrap_or(0);
        score.time_played = self.time_played.unwrap_or(0);
        score.clear_type = self.clear_type.unwrap_or(0);
        score.rating = self.rating.unwrap_or(0.0);
        score.score_v2 = self.score_v2.unwrap_or(0.0);

        let favorite_character_id = self.favorite_character.unwrap_or(-1);
        let (displayed_character, is_uncapped, is_uncapped_override) =
            if favorite_character_id == -1 {
                (
                    self.character_id.unwrap_or(0),
                    self.is_char_uncapped.unwrap_or(0),
                    self.is_char_uncapped_override.unwrap_or(0),
                )
            } else {
                (
                    favorite_character_id,
                    self.favorite_is_uncapped.unwrap_or(0),
                    self.favorite_is_uncapped_override.unwrap_or(0),
                )
            };
        let displayed_uncap = if is_uncapped_override != 0 {
            0
        } else {
            is_uncapped
        };

        UserScore {
            score,
            user_id: self.user_id,
            name: self.name.clone().unwrap_or_default(),
            best_clear_type: self.best_clear_type.unwrap_or(0),
            character: displayed_character,
            is_char_uncapped: displayed_uncap,
            is_skill_sealed: self.is_skill_sealed.unwrap_or(0),
            rank,
        }
    }
}
