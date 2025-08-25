use serde::{Deserialize, Serialize};

use std::collections::HashMap;

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

    /// Set score data from individual components
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

        let acc_rating = (shiny_ratio - 0.9).min(0.095).max(0.0) / 9.5 * 25.0;
        let score_rating = (score_ratio - 0.99).min(0.01).max(0.0) * 75.0;

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
        result.insert("rating".to_string(), serde_json::Value::from(self.rating));
        result.insert(
            "modifier".to_string(),
            serde_json::Value::from(self.modifier),
        );
        result.insert(
            "time_played".to_string(),
            serde_json::Value::from(self.time_played),
        );
        result.insert("health".to_string(), serde_json::Value::from(self.health));
        result.insert(
            "clear_type".to_string(),
            serde_json::Value::from(self.clear_type),
        );
        result.insert(
            "miss_count".to_string(),
            serde_json::Value::from(self.miss_count),
        );
        result.insert(
            "near_count".to_string(),
            serde_json::Value::from(self.near_count),
        );
        result.insert(
            "perfect_count".to_string(),
            serde_json::Value::from(self.perfect_count),
        );
        result.insert(
            "shiny_perfect_count".to_string(),
            serde_json::Value::from(self.shiny_perfect_count),
        );
        result.insert("score".to_string(), serde_json::Value::from(self.score));
        result.insert(
            "difficulty".to_string(),
            serde_json::Value::from(self.difficulty),
        );
        result.insert(
            "song_id".to_string(),
            serde_json::Value::from(self.song_id.clone()),
        );

        if let Some(ref song_name) = self.song_name {
            result.insert(
                "song_name".to_string(),
                serde_json::Value::from(song_name.clone()),
            );
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

    /// Convert to dictionary with user info
    pub fn to_dict(&self, has_user_info: bool) -> HashMap<String, serde_json::Value> {
        let mut result = self.score.to_dict();
        result.insert(
            "best_clear_type".to_string(),
            serde_json::Value::from(self.best_clear_type),
        );

        if has_user_info {
            result.insert("user_id".to_string(), serde_json::Value::from(self.user_id));
            result.insert(
                "name".to_string(),
                serde_json::Value::from(self.name.clone()),
            );
            result.insert(
                "is_skill_sealed".to_string(),
                serde_json::Value::from(self.is_skill_sealed),
            );
            result.insert(
                "is_char_uncapped".to_string(),
                serde_json::Value::from(self.is_char_uncapped),
            );
            result.insert(
                "character".to_string(),
                serde_json::Value::from(self.character),
            );
        }

        if let Some(rank) = self.rank {
            result.insert("rank".to_string(), serde_json::Value::from(rank));
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
    pub is_world_mode: bool,
    pub stamina_multiply: i32,
    pub fragment_multiply: i32,
    pub prog_boost_multiply: i32,
    pub beyond_boost_gauge_usage: i32,
    pub course_play_state: i32,
    pub combo_interval_bonus: Option<i32>,
    pub hp_interval_bonus: Option<i32>,
    pub skill_cytusii_flag: Option<String>,
    pub skill_chinatsu_flag: Option<String>,
    pub highest_health: Option<i32>,
    pub lowest_health: Option<i32>,
    pub invasion_flag: i32,
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

        // Validate combo interval bonus
        if let Some(combo_bonus) = self.combo_interval_bonus {
            if combo_bonus < 0 || combo_bonus > self.user_score.score.all_note_count() / 150 {
                return false;
            }
        }

        // Validate hp interval bonus
        if let Some(hp_bonus) = self.hp_interval_bonus {
            if hp_bonus < 0 {
                return false;
            }
        }

        // Validate submission hash
        let hash_input = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}",
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
            self.user_score.score.clear_type,
            self.combo_interval_bonus
                .map(|b| b.to_string())
                .unwrap_or_default()
        );

        let user_hash_input = format!("{}{}", self.user_score.user_id, self.song_hash);
        let expected_hash = crate::service::score::md5_hash(&format!(
            "{}{}",
            hash_input,
            crate::service::score::md5_hash(&user_hash_input)
        ));

        expected_hash == self.submission_hash
    }

    /// Convert to response dictionary
    pub fn to_dict(
        &self,
        user_rating_ptt: i32,
        finale_play_value: f64,
    ) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();
        result.insert(
            "user_rating".to_string(),
            serde_json::Value::from(user_rating_ptt),
        );
        result.insert(
            "finale_challenge_higher".to_string(),
            serde_json::Value::from(self.user_score.score.rating > self.get_potential_value()),
        );
        result.insert("global_rank".to_string(), serde_json::Value::Null); // TODO: implement global rank
        result.insert(
            "finale_play_value".to_string(),
            serde_json::Value::from(finale_play_value),
        );
        result
    }

    /// Get potential value placeholder
    fn get_potential_value(&self) -> f64 {
        // This should be calculated from user's current potential
        // For now, return a placeholder
        0.0
    }
}

/// Potential calculation for users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Potential {
    pub user_id: i32,
    pub best_30_sum: f64,
    pub recent_10_sum: f64,
}

impl Potential {
    /// Calculate user's potential value
    pub fn calculate_value(&self, best30_weight: f64, recent10_weight: f64) -> f64 {
        self.best_30_sum * best30_weight + self.recent_10_sum * recent10_weight
    }

    /// Calculate finale play value
    pub fn calculate_finale_play_value(rating: f64) -> f64 {
        9.065 * rating.sqrt()
    }
}

/// Recent 30 tuple for internal calculations
#[derive(Debug, Clone)]
pub struct Recent30Tuple {
    pub r_index: i32,
    pub song_id: String,
    pub difficulty: i32,
    pub rating: f64,
}
