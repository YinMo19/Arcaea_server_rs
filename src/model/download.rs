use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Download token model for temporary download links
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DownloadToken {
    pub user_id: i32,
    pub song_id: String,
    pub file_name: String,
    pub token: String,
    pub time: i64,
}

/// Song play token model representing the songplay_token table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SongplayToken {
    pub token: String,
    pub user_id: i32,
    pub song_id: String,
    pub difficulty: i32,
    pub course_id: Option<String>,
    pub course_state: i32,
    pub course_score: i32,
    pub course_clear_type: i32,
    pub stamina_multiply: i32,
    pub fragment_multiply: i32,
    pub prog_boost_multiply: i32,
    pub beyond_boost_gauge_usage: i32,
    pub skill_cytusii_flag: Option<String>,
    pub skill_chinatsu_flag: Option<String>,
    pub invasion_flag: i32,
}

/// Best score model representing the best_score table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct BestScore {
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
    pub time_played: i64,
    pub best_clear_type: i32,
    pub clear_type: i32,
    pub rating: f64,
    pub score_v2: f64,
}

/// Recent 30 scores model representing the recent30 table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Recent30 {
    pub user_id: i32,
    pub r_index: i32,
    pub time_played: i64,
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
    pub rating: f64,
}

/// Chart information model representing the chart table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Chart {
    pub song_id: String,
    pub name: Option<String>,
    pub rating_pst: Option<i32>,
    pub rating_prs: Option<i32>,
    pub rating_ftr: Option<i32>,
    pub rating_byn: Option<i32>,
    pub rating_etr: Option<i32>,
}

/// Download file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

/// Download song information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSong {
    pub audio: Option<DownloadAudio>,
    pub chart: Option<std::collections::HashMap<String, DownloadFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_files: Option<Vec<DownloadFile>>,
}

/// Download audio information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadAudio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "3")]
    pub difficulty_3: Option<DownloadFile>,
}

/// Score submission data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSubmission {
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
    pub fever_bonus: Option<i32>,
    pub highest_health: Option<i32>,
    pub lowest_health: Option<i32>,
}

/// Score response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResponse {
    pub user_rating: i32,
    pub finale_challenge_higher: bool,
    pub global_rank: Option<i32>,
    pub finale_play_value: f64,
}

/// Token request for world mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldTokenRequest {
    pub song_id: String,
    pub difficulty: i32,
    pub stamina_multiply: Option<i32>,
    pub fragment_multiply: Option<i32>,
    pub prog_boost_multiply: Option<i32>,
    pub beyond_boost_gauge_use: Option<i32>,
    pub skill_id: Option<String>,
    pub is_skill_sealed: Option<String>,
}

/// Token response for world mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldTokenResponse {
    pub stamina: i32,
    pub max_stamina_ts: i64,
    pub token: String,
    pub play_parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// Token request for course mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTokenRequest {
    pub course_id: Option<String>,
    pub previous_token: Option<String>,
    pub use_course_skip_purchase: Option<bool>,
}

/// Token response for course mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseTokenResponse {
    pub stamina: i32,
    pub max_stamina_ts: i64,
    pub token: String,
    pub status: String,
}

/// Rank list entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankEntry {
    pub user_id: i32,
    pub name: String,
    pub score: i32,
    pub shiny_perfect_count: i32,
    pub perfect_count: i32,
    pub near_count: i32,
    pub miss_count: i32,
    pub health: i32,
    pub modifier: i32,
    pub time_played: i64,
    pub clear_type: i32,
    pub best_clear_type: i32,
    pub rating: f64,
    pub character: i32,
    pub is_char_uncapped: i8,
    pub is_skill_sealed: i8,
    pub rank: Option<i32>,
}
