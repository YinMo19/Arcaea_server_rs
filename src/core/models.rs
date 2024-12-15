use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use rocket::serde::{Deserialize, Serialize};

// types
type Id = u8;

// models
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SingleCall<'r> {
    pub endpoint: Cow<'r, str>,
    pub id: Option<Id>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ArcError<'r> {
    pub message: Cow<'r, str>,
    pub error_code: i32,
    pub api_error_code: i32,
    pub extra_data: Option<HashMap<String, String>>,
    pub status: u16,
}

impl<'r> ArcError<'r> {
    pub fn new(
        message: &'r str,
        error_code: i32,
        api_error_code: i32,
        extra_data: Option<HashMap<String, String>>,
        status: u16,
    ) -> Self {
        ArcError {
            message: Cow::Borrowed(message),
            error_code,
            api_error_code,
            extra_data,
            status,
        }
    }
}

pub const DEFAULT_ERR: ArcError<'static> = ArcError {
    message: Cow::Borrowed("An unknown error occurred"),
    error_code: -1,
    api_error_code: -1,
    extra_data: None,
    status: 500,
};

impl<'r> fmt::Display for ArcError<'r> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl<'r> std::error::Error for ArcError<'r> {}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GameInfo {
    pub max_stamina: i32,
    pub stamina_recover_tick: i32,
    pub core_exp: i32,
    pub curr_ts: i64,
    pub level_steps: Vec<LevelStep>,
    pub world_ranking_enabled: bool,
    pub is_byd_chapter_unlocked: bool,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LevelStep {
    pub level: i32,
    pub level_exp: i32,
}
