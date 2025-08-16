use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Character database model representing the character table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Character {
    pub character_id: i32,
    pub name: Option<String>,
    pub max_level: Option<i32>,
    pub frag1: Option<f64>,
    pub prog1: Option<f64>,
    pub overdrive1: Option<f64>,
    pub frag20: Option<f64>,
    pub prog20: Option<f64>,
    pub overdrive20: Option<f64>,
    pub frag30: Option<f64>,
    pub prog30: Option<f64>,
    pub overdrive30: Option<f64>,
    pub skill_id: Option<String>,
    pub skill_unlock_level: Option<i32>,
    pub skill_requires_uncap: Option<i8>,
    pub skill_id_uncap: Option<String>,
    pub char_type: Option<i32>,
    pub is_uncapped: Option<i8>,
}

/// User character data representing the user_char table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserCharacter {
    pub user_id: i32,
    pub character_id: i32,
    pub level: i32,
    pub exp: f64,
    pub is_uncapped: i8,
    pub is_uncapped_override: i8,
    pub skill_flag: i32,
}

/// User character full data representing the user_char_full table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserCharacterFull {
    pub user_id: i32,
    pub character_id: i32,
    pub level: i32,
    pub exp: f64,
    pub is_uncapped: i8,
    pub is_uncapped_override: i8,
    pub skill_flag: i32,
}

/// Character item data representing the char_item table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CharacterItem {
    pub character_id: i32,
    pub item_id: String,
    #[sqlx(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

/// Character API response model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub character_id: i32,
    pub name: String,
    pub max_level: i32,
    pub level: i32,
    pub exp: f64,
    pub is_uncapped: bool,
    pub is_uncapped_override: bool,
    pub skill_flag: i32,
    pub skill_id: Option<String>,
    pub skill_unlock_level: Option<i32>,
    pub skill_requires_uncap: bool,
    pub skill_id_uncap: Option<String>,
    pub char_type: i32,
    pub frag: f64,
    pub prog: f64,
    pub overdrive: f64,
}

/// New user character for insertion
#[derive(Debug, Clone)]
pub struct NewUserCharacter {
    pub user_id: i32,
    pub character_id: i32,
    pub level: i32,
    pub exp: f64,
    pub is_uncapped: i8,
    pub is_uncapped_override: i8,
    pub skill_flag: i32,
}

impl Character {
    /// Check if character is uncapped
    pub fn is_uncapped(&self) -> bool {
        self.is_uncapped.unwrap_or(0) != 0
    }

    /// Check if skill requires uncap
    pub fn skill_requires_uncap(&self) -> bool {
        self.skill_requires_uncap.unwrap_or(0) != 0
    }

    /// Get character stats based on level
    pub fn get_stats(&self, level: i32, is_uncapped: bool) -> (f64, f64, f64) {
        let frag;
        let prog;
        let overdrive;

        if level == 1 {
            frag = self.frag1.unwrap_or(0.0);
            prog = self.prog1.unwrap_or(0.0);
            overdrive = self.overdrive1.unwrap_or(0.0);
        } else if level == 20 {
            frag = self.frag20.unwrap_or(0.0);
            prog = self.prog20.unwrap_or(0.0);
            overdrive = self.overdrive20.unwrap_or(0.0);
        } else if level == 30 && is_uncapped {
            frag = self.frag30.unwrap_or(0.0);
            prog = self.prog30.unwrap_or(0.0);
            overdrive = self.overdrive30.unwrap_or(0.0);
        } else {
            // Interpolate between known values
            let _max_level = if is_uncapped { 30 } else { 20 };
            if level <= 20 {
                // Interpolate between level 1 and 20
                let ratio = (level - 1) as f64 / 19.0;
                frag = self.frag1.unwrap_or(0.0)
                    + ratio * (self.frag20.unwrap_or(0.0) - self.frag1.unwrap_or(0.0));
                prog = self.prog1.unwrap_or(0.0)
                    + ratio * (self.prog20.unwrap_or(0.0) - self.prog1.unwrap_or(0.0));
                overdrive = self.overdrive1.unwrap_or(0.0)
                    + ratio * (self.overdrive20.unwrap_or(0.0) - self.overdrive1.unwrap_or(0.0));
            } else {
                // Interpolate between level 20 and 30
                let ratio = (level - 20) as f64 / 10.0;
                frag = self.frag20.unwrap_or(0.0)
                    + ratio * (self.frag30.unwrap_or(0.0) - self.frag20.unwrap_or(0.0));
                prog = self.prog20.unwrap_or(0.0)
                    + ratio * (self.prog30.unwrap_or(0.0) - self.prog20.unwrap_or(0.0));
                overdrive = self.overdrive20.unwrap_or(0.0)
                    + ratio * (self.overdrive30.unwrap_or(0.0) - self.overdrive20.unwrap_or(0.0));
            }
        }

        (frag, prog, overdrive)
    }
}

impl UserCharacter {
    /// Check if character is uncapped
    pub fn is_uncapped(&self) -> bool {
        self.is_uncapped != 0
    }

    /// Check if character uncap is overridden
    pub fn is_uncapped_override(&self) -> bool {
        self.is_uncapped_override != 0
    }
}

impl UserCharacterFull {
    /// Check if character is uncapped
    pub fn is_uncapped(&self) -> bool {
        self.is_uncapped != 0
    }

    /// Check if character uncap is overridden
    pub fn is_uncapped_override(&self) -> bool {
        self.is_uncapped_override != 0
    }
}

impl From<(Character, UserCharacter)> for CharacterInfo {
    fn from((character, user_char): (Character, UserCharacter)) -> Self {
        let (frag, prog, overdrive) = character.get_stats(user_char.level, user_char.is_uncapped());

        Self {
            character_id: character.character_id,
            name: character.name.clone().unwrap_or_default(),
            max_level: character.max_level.unwrap_or(20),
            level: user_char.level,
            exp: user_char.exp,
            is_uncapped: user_char.is_uncapped(),
            is_uncapped_override: user_char.is_uncapped_override(),
            skill_flag: user_char.skill_flag,
            skill_id: character.skill_id.clone(),
            skill_unlock_level: character.skill_unlock_level,
            skill_requires_uncap: character.skill_requires_uncap(),
            skill_id_uncap: character.skill_id_uncap.clone(),
            char_type: character.char_type.unwrap_or(0),
            frag,
            prog,
            overdrive,
        }
    }
}

impl From<(Character, UserCharacterFull)> for CharacterInfo {
    fn from((character, user_char): (Character, UserCharacterFull)) -> Self {
        let (frag, prog, overdrive) = character.get_stats(user_char.level, user_char.is_uncapped());

        Self {
            character_id: character.character_id,
            name: character.name.clone().unwrap_or_default(),
            max_level: character.max_level.unwrap_or(20),
            level: user_char.level,
            exp: user_char.exp,
            is_uncapped: user_char.is_uncapped(),
            is_uncapped_override: user_char.is_uncapped_override(),
            skill_flag: user_char.skill_flag,
            skill_id: character.skill_id.clone(),
            skill_unlock_level: character.skill_unlock_level,
            skill_requires_uncap: character.skill_requires_uncap(),
            skill_id_uncap: character.skill_id_uncap.clone(),
            char_type: character.char_type.unwrap_or(0),
            frag,
            prog,
            overdrive,
        }
    }
}
