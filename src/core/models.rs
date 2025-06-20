use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::core::error::ArcResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub email: Option<String>,
    pub join_date: i64,
    pub user_code: String,
    pub rating_ptt: i32,
    pub character_id: i32,
    pub is_skill_sealed: i32,
    pub is_char_uncapped: i32,
    pub is_char_uncapped_override: i32,
    pub is_hide_rating: i32,
    pub ticket: i32,
    pub world_rank_score: i32,
    pub ban_flag: Option<String>,
    pub stamina: i32,
    pub max_stamina_ts: i64,
    pub next_fragstam_ts: i64,
    pub world_mode_locked_end_ts: i64,
    pub beyond_boost_gauge: f64,
    pub kanae_stored_prog: f64,
    pub mp_notification_enabled: i32,
    pub favorite_character: i32,
    pub max_stamina_notification_enabled: i32,
    pub insight_state: Option<i32>,
}

impl User {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(User {
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            email: row.try_get("email").ok(),
            join_date: row.try_get("join_date")?,
            user_code: row.try_get("user_code")?,
            rating_ptt: row.try_get("rating_ptt")?,
            character_id: row.try_get("character_id")?,
            is_skill_sealed: row.try_get("is_skill_sealed")?,
            is_char_uncapped: row.try_get("is_char_uncapped")?,
            is_char_uncapped_override: row.try_get("is_char_uncapped_override")?,
            is_hide_rating: row.try_get("is_hide_rating")?,
            ticket: row.try_get("ticket")?,
            world_rank_score: row.try_get("world_rank_score")?,
            ban_flag: row.try_get("ban_flag").ok(),
            stamina: row.try_get("stamina")?,
            max_stamina_ts: row.try_get("max_stamina_ts")?,
            next_fragstam_ts: row.try_get("next_fragstam_ts")?,
            world_mode_locked_end_ts: row.try_get("world_mode_locked_end_ts")?,
            beyond_boost_gauge: row.try_get("beyond_boost_gauge")?,
            kanae_stored_prog: row.try_get("kanae_stored_prog")?,
            mp_notification_enabled: row.try_get("mp_notification_enabled")?,
            favorite_character: row.try_get("favorite_character")?,
            max_stamina_notification_enabled: row.try_get("max_stamina_notification_enabled")?,
            insight_state: row.try_get("insight_state").ok(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub character_id: i32,
    pub name: String,
    pub max_level: i32,
    pub frag1: f64,
    pub prog1: f64,
    pub overdrive1: f64,
    pub frag20: f64,
    pub prog20: f64,
    pub overdrive20: f64,
    pub frag30: f64,
    pub prog30: f64,
    pub overdrive30: f64,
    pub skill_id: String,
    pub skill_unlock_level: i32,
    pub skill_requires_uncap: i32,
    pub skill_id_uncap: String,
    pub char_type: i32,
    pub is_uncapped: i32,
}

impl Character {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(Character {
            character_id: row.try_get("character_id")?,
            name: row.try_get("name")?,
            max_level: row.try_get("max_level")?,
            frag1: row.try_get("frag1")?,
            prog1: row.try_get("prog1")?,
            overdrive1: row.try_get("overdrive1")?,
            frag20: row.try_get("frag20")?,
            prog20: row.try_get("prog20")?,
            overdrive20: row.try_get("overdrive20")?,
            frag30: row.try_get("frag30")?,
            prog30: row.try_get("prog30")?,
            overdrive30: row.try_get("overdrive30")?,
            skill_id: row.try_get("skill_id")?,
            skill_unlock_level: row.try_get("skill_unlock_level")?,
            skill_requires_uncap: row.try_get("skill_requires_uncap")?,
            skill_id_uncap: row.try_get("skill_id_uncap")?,
            char_type: row.try_get("char_type")?,
            is_uncapped: row.try_get("is_uncapped")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCharacter {
    pub user_id: i32,
    pub character_id: i32,
    pub level: i32,
    pub exp: f64,
    pub is_uncapped: i32,
    pub is_uncapped_override: i32,
    pub skill_flag: i32,
}

impl UserCharacter {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(UserCharacter {
            user_id: row.try_get("user_id")?,
            character_id: row.try_get("character_id")?,
            level: row.try_get("level")?,
            exp: row.try_get("exp")?,
            is_uncapped: row.try_get("is_uncapped")?,
            is_uncapped_override: row.try_get("is_uncapped_override")?,
            skill_flag: row.try_get("skill_flag")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl BestScore {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(BestScore {
            user_id: row.try_get("user_id")?,
            song_id: row.try_get("song_id")?,
            difficulty: row.try_get("difficulty")?,
            score: row.try_get("score")?,
            shiny_perfect_count: row.try_get("shiny_perfect_count")?,
            perfect_count: row.try_get("perfect_count")?,
            near_count: row.try_get("near_count")?,
            miss_count: row.try_get("miss_count")?,
            health: row.try_get("health")?,
            modifier: row.try_get("modifier")?,
            time_played: row.try_get("time_played")?,
            best_clear_type: row.try_get("best_clear_type")?,
            clear_type: row.try_get("clear_type")?,
            rating: row.try_get("rating")?,
            score_v2: row.try_get("score_v2")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Recent30 {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(Recent30 {
            user_id: row.try_get("user_id")?,
            r_index: row.try_get("r_index")?,
            time_played: row.try_get("time_played")?,
            song_id: row.try_get("song_id")?,
            difficulty: row.try_get("difficulty")?,
            score: row.try_get("score")?,
            shiny_perfect_count: row.try_get("shiny_perfect_count")?,
            perfect_count: row.try_get("perfect_count")?,
            near_count: row.try_get("near_count")?,
            miss_count: row.try_get("miss_count")?,
            health: row.try_get("health")?,
            modifier: row.try_get("modifier")?,
            clear_type: row.try_get("clear_type")?,
            rating: row.try_get("rating")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserItem {
    pub user_id: i32,
    pub item_id: String,
    pub item_type: String,
    pub amount: i32,
}

impl UserItem {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(UserItem {
            user_id: row.try_get("user_id")?,
            item_id: row.try_get("item_id")?,
            item_type: row.try_get("type")?,
            amount: row.try_get("amount")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub song_id: String,
    pub name: String,
    pub rating_pst: i32,
    pub rating_prs: i32,
    pub rating_ftr: i32,
    pub rating_byn: i32,
    pub rating_etr: i32,
}

impl Chart {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(Chart {
            song_id: row.try_get("song_id")?,
            name: row.try_get("name")?,
            rating_pst: row.try_get("rating_pst")?,
            rating_prs: row.try_get("rating_prs")?,
            rating_ftr: row.try_get("rating_ftr")?,
            rating_byn: row.try_get("rating_byn")?,
            rating_etr: row.try_get("rating_etr")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub user_id_me: i32,
    pub user_id_other: i32,
}

impl Friend {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(Friend {
            user_id_me: row.try_get("user_id_me")?,
            user_id_other: row.try_get("user_id_other")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSave {
    pub user_id: i32,
    pub scores_data: String,
    pub clearlamps_data: String,
    pub clearedsongs_data: String,
    pub unlocklist_data: String,
    pub installid_data: String,
    pub devicemodelname_data: String,
    pub story_data: String,
    pub created_at: i64,
    pub finalestate_data: Option<String>,
}

impl UserSave {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(UserSave {
            user_id: row.try_get("user_id")?,
            scores_data: row.try_get("scores_data")?,
            clearlamps_data: row.try_get("clearlamps_data")?,
            clearedsongs_data: row.try_get("clearedsongs_data")?,
            unlocklist_data: row.try_get("unlocklist_data")?,
            installid_data: row.try_get("installid_data")?,
            devicemodelname_data: row.try_get("devicemodelname_data")?,
            story_data: row.try_get("story_data")?,
            created_at: row.try_get("createdAt")?,
            finalestate_data: row.try_get("finalestate_data").ok(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSession {
    pub access_token: String,
    pub user_id: i32,
    pub login_time: i64,
    pub login_ip: String,
    pub login_device: String,
}

impl LoginSession {
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> ArcResult<Self> {
        Ok(LoginSession {
            access_token: row.try_get("access_token")?,
            user_id: row.try_get("user_id")?,
            login_time: row.try_get("login_time")?,
            login_ip: row.try_get("login_ip")?,
            login_device: row.try_get("login_device")?,
        })
    }
}
