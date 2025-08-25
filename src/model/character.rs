use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;

use crate::config::LEVEL_STEPS;
use crate::error::{ArcError, ArcResult};

/// Character level and experience management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub min_level: i32,
    pub mid_level: i32,
    pub max_level: i32,
    pub level: i32,
    pub exp: f64,
}

impl Level {
    pub fn new() -> Self {
        Self {
            min_level: 1,
            mid_level: 20,
            max_level: 20,
            level: 1,
            exp: 0.0,
        }
    }

    /// Get experience required for current level
    pub fn level_exp(&self) -> i32 {
        LEVEL_STEPS.get(&self.level).copied().unwrap_or(0)
    }

    /// Add experience and calculate new level - matches Python version exactly
    pub fn add_exp(&mut self, exp_addition: f64) -> ArcResult<()> {
        let exp = self.exp + exp_addition;

        // Check if we've reached max level exp
        let max_level_exp = LEVEL_STEPS.get(&self.max_level).copied().unwrap_or(25000) as f64;
        if exp >= max_level_exp {
            self.exp = max_level_exp;
            self.level = self.max_level;
            return Ok(());
        }

        // Create arrays a (levels) and b (exp values) from LEVEL_STEPS - matches Python logic
        let mut a: Vec<i32> = Vec::new();
        let mut b: Vec<i32> = Vec::new();
        for (&k, &v) in LEVEL_STEPS.iter() {
            a.push(k);
            b.push(v);
        }

        // Sort by level to ensure proper order
        let mut pairs: Vec<(i32, i32)> = a.into_iter().zip(b).collect();
        pairs.sort_by_key(|&(level, _)| level);
        let a: Vec<i32> = pairs.iter().map(|&(level, _)| level).collect();
        let b: Vec<i32> = pairs.iter().map(|&(_, exp_val)| exp_val).collect();

        if exp < b[0] as f64 {
            // 向下溢出，是异常状态，不该被try捕获，不然数据库无法回滚
            return Err(ArcError::input("EXP value error."));
        }

        let mut i = a.len() - 1;
        while exp < b[i] as f64 {
            i -= 1;
        }

        self.exp = exp;
        self.level = a[i];
        Ok(())
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new()
    }
}

/// Character skill information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub skill_id: Option<String>,
    pub skill_id_uncap: Option<String>,
    pub skill_unlock_level: i32,
    pub skill_requires_uncap: bool,
}

impl Skill {
    pub fn new() -> Self {
        Self {
            skill_id: None,
            skill_id_uncap: None,
            skill_unlock_level: 1,
            skill_requires_uncap: false,
        }
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self::new()
    }
}

/// Character value calculations (frag, prog, overdrive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterValue {
    pub start: f64,    // Level 1 value
    pub mid: f64,      // Level 20 value
    pub end: f64,      // Level 30 value
    pub addition: f64, // Additional value (for special characters like Fatalis)
}

impl CharacterValue {
    pub fn new() -> Self {
        Self {
            start: 0.0,
            mid: 0.0,
            end: 0.0,
            addition: 0.0,
        }
    }

    pub fn set_parameter(&mut self, start: f64, mid: f64, end: f64) {
        self.start = start;
        self.mid = mid;
        self.end = end;
    }

    /// Calculate character value using the 20-level math formula
    /// This is the same formula used in the Python version
    fn calc_char_value_20_math(level: i32, value_1: f64, value_20: f64) -> f64 {
        let level_f = level as f64;
        let coefficient = 0.00058317539; // 4/6859

        if level <= 10 {
            coefficient * (level_f - 1.0).powi(3) * (value_20 - value_1) + value_1
        } else {
            -coefficient * (20.0 - level_f).powi(3) * (value_20 - value_1) + value_20
        }
    }

    /// Calculate character value for levels 21-30 (linear interpolation)
    fn calc_char_value_30(level: i32, stata: f64, statb: f64, lva: i32, lvb: i32) -> f64 {
        let level_f = level as f64;
        let lva_f = lva as f64;
        let lvb_f = lvb as f64;

        (level_f - lva_f) * (statb - stata) / (lvb_f - lva_f) + stata
    }

    /// Get the character value for a given level
    pub fn get_value(&self, level: &Level) -> f64 {
        let value = if level.min_level <= level.level && level.level <= level.mid_level {
            // Levels 1-20: Use the mathematical formula
            Self::calc_char_value_20_math(level.level, self.start, self.mid)
        } else if level.mid_level < level.level && level.level <= level.max_level {
            // Levels 21-30: Linear interpolation
            Self::calc_char_value_30(level.level, self.mid, self.end, 20, 30)
        } else {
            0.0
        };

        value + self.addition
    }
}

impl Default for CharacterValue {
    fn default() -> Self {
        Self::new()
    }
}

/// Base character information from the character table
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

impl Character {
    /// Check if this is a base character (character_id == 1)
    pub fn is_base_character(&self) -> bool {
        self.character_id == 1
    }

    /// Check if character is uncapped
    pub fn is_uncapped(&self) -> bool {
        self.is_uncapped.unwrap_or(0) != 0
    }

    /// Check if skill requires uncap
    pub fn skill_requires_uncap(&self) -> bool {
        self.skill_requires_uncap.unwrap_or(0) != 0
    }

    /// Convert to dictionary format for API responses
    pub fn to_dict(
        &self,
        has_cores: bool,
        uncap_cores: Option<Vec<CoreItem>>,
    ) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();

        result.insert(
            "character_id".to_string(),
            Value::Number(self.character_id.into()),
        );
        result.insert(
            "name".to_string(),
            Value::String(self.name.clone().unwrap_or_default()),
        );
        result.insert(
            "char_type".to_string(),
            Value::Number(self.char_type.unwrap_or(0).into()),
        );
        result.insert("is_uncapped".to_string(), Value::Bool(self.is_uncapped()));
        result.insert(
            "max_level".to_string(),
            Value::Number(self.max_level.unwrap_or(20).into()),
        );
        result.insert(
            "skill_id".to_string(),
            self.skill_id
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        result.insert(
            "skill_unlock_level".to_string(),
            Value::Number(self.skill_unlock_level.unwrap_or(1).into()),
        );
        result.insert(
            "skill_requires_uncap".to_string(),
            Value::Bool(self.skill_requires_uncap()),
        );
        result.insert(
            "skill_id_uncap".to_string(),
            self.skill_id_uncap
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        result.insert(
            "frag1".to_string(),
            Value::Number(serde_json::Number::from_f64(self.frag1.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "frag20".to_string(),
            Value::Number(serde_json::Number::from_f64(self.frag20.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "frag30".to_string(),
            Value::Number(serde_json::Number::from_f64(self.frag30.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "prog1".to_string(),
            Value::Number(serde_json::Number::from_f64(self.prog1.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "prog20".to_string(),
            Value::Number(serde_json::Number::from_f64(self.prog20.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "prog30".to_string(),
            Value::Number(serde_json::Number::from_f64(self.prog30.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "overdrive1".to_string(),
            Value::Number(serde_json::Number::from_f64(self.overdrive1.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "overdrive20".to_string(),
            Value::Number(serde_json::Number::from_f64(self.overdrive20.unwrap_or(0.0)).unwrap()),
        );
        result.insert(
            "overdrive30".to_string(),
            Value::Number(serde_json::Number::from_f64(self.overdrive30.unwrap_or(0.0)).unwrap()),
        );

        if has_cores {
            if let Some(cores) = uncap_cores {
                let cores_json: Vec<serde_json::Value> = cores
                    .into_iter()
                    .map(|core| core.to_dict_character_format())
                    .collect();
                result.insert("uncap_cores".to_string(), Value::Array(cores_json));
            }
        }

        result
    }
}

/// User character data from user_char table
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

/// User character data from user_char_full table
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

/// Character item data from char_item table (for uncap cores)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CharacterItem {
    pub character_id: i32,
    pub item_id: String,
    #[sqlx(rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

/// Core item representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreItem {
    pub item_id: String,
    pub amount: i32,
}

impl CoreItem {
    pub fn to_dict_character_format(&self) -> serde_json::Value {
        serde_json::json!({
            "item_id": self.item_id,
            "amount": self.amount,
            "type": "core"
        })
    }
}

/// Complete user character information combining base character and user data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCharacterInfo {
    // Basic info
    pub character_id: i32,
    pub name: String,
    pub char_type: i32,

    // Level and experience
    pub level: Level,

    // Skill info
    pub skill: Skill,

    // Character values
    pub frag: CharacterValue,
    pub prog: CharacterValue,
    pub overdrive: CharacterValue,

    // Uncap states
    pub is_uncapped: bool,
    pub is_uncapped_override: bool,

    // Skill states
    pub skill_flag: bool,

    // Additional data
    pub uncap_cores: Vec<CoreItem>,
    pub voice: Option<Vec<i32>>,
    pub fatalis_is_limited: bool,
}

impl UserCharacterInfo {
    /// Get frag value at current level
    pub fn frag_value(&self) -> f64 {
        self.frag.get_value(&self.level)
    }

    /// Get prog value at current level
    pub fn prog_value(&self) -> f64 {
        self.prog.get_value(&self.level)
    }

    /// Get overdrive value at current level
    pub fn overdrive_value(&self) -> f64 {
        self.overdrive.get_value(&self.level)
    }

    /// Get displayed skill ID based on level and uncap state
    pub fn skill_id_displayed(&self) -> Option<String> {
        // If uncapped and has uncap skill, use uncap skill
        if self.is_uncapped_displayed() && self.skill.skill_id_uncap.is_some() {
            return self.skill.skill_id_uncap.clone();
        }

        // If has regular skill and level is sufficient, use regular skill
        if self.skill.skill_id.is_some() && self.level.level >= self.skill.skill_unlock_level {
            return self.skill.skill_id.clone();
        }

        None
    }

    /// Get displayed uncap state (respects override)
    pub fn is_uncapped_displayed(&self) -> bool {
        if self.is_uncapped_override {
            false
        } else {
            self.is_uncapped
        }
    }

    /// Check if this is a base character
    pub fn is_base_character(&self) -> bool {
        self.character_id == 1
    }

    /// Get skill state for Maya character
    pub fn skill_state(&self) -> Option<String> {
        if let Some(skill_id) = &self.skill_id_displayed() {
            if skill_id == "skill_maya" {
                return Some(if self.skill_flag {
                    "add_random".to_string()
                } else {
                    "remove_random".to_string()
                });
            }
        }
        None
    }

    /// Convert to dictionary format for API responses - matches Python version exactly
    pub fn to_dict(&self) -> serde_json::Value {
        let mut r = serde_json::json!({
            "base_character": self.is_base_character(),
            "is_uncapped_override": self.is_uncapped_override,
            "is_uncapped": self.is_uncapped,
            "uncap_cores": self.uncap_cores.iter().map(|core| core.to_dict_character_format()).collect::<Vec<_>>(),
            "char_type": self.char_type,
            "skill_id_uncap": self.skill.skill_id_uncap.as_ref().unwrap_or(&String::new()).clone(),
            "skill_requires_uncap": self.skill.skill_requires_uncap,
            "skill_unlock_level": self.skill.skill_unlock_level,
            "skill_id": self.skill.skill_id.as_ref().unwrap_or(&String::new()).clone(),
            "overdrive": self.overdrive_value(),
            "prog": self.prog_value(),
            "frag": self.frag_value(),
            "level_exp": self.level.level_exp(),
            "exp": self.level.exp,
            "level": self.level.level,
            "name": self.name.clone(),
            "character_id": self.character_id
        });

        // Add voice data for specific characters
        if let Some(voice) = &self.voice {
            if let Value::Object(ref mut map) = r {
                map.insert("voice".to_string(), serde_json::json!(voice));
            }
        }

        // Add Fatalis specific data
        if self.character_id == 55 {
            if let Value::Object(ref mut map) = r {
                map.insert(
                    "fatalis_is_limited".to_string(),
                    serde_json::json!(self.fatalis_is_limited),
                );
            }
        }

        // Add base character ID for specific characters
        if [1, 6, 7, 17, 18, 24, 32, 35, 52].contains(&self.character_id) {
            if let Value::Object(ref mut map) = r {
                map.insert("base_character_id".to_string(), serde_json::json!(1));
            }
        }

        // Add skill state
        if let Some(skill_state) = self.skill_state() {
            if let Value::Object(ref mut map) = r {
                map.insert("skill_state".to_string(), serde_json::json!(skill_state));
            }
        }

        r
    }
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

impl From<(Character, UserCharacter)> for CharacterInfo {
    fn from((character, user_char): (Character, UserCharacter)) -> Self {
        // Calculate character values using the proper formula
        let mut level = Level::new();
        level.level = user_char.level;
        level.exp = user_char.exp;
        level.max_level = if user_char.is_uncapped() { 30 } else { 20 };

        let mut frag = CharacterValue::new();
        frag.set_parameter(
            character.frag1.unwrap_or(0.0),
            character.frag20.unwrap_or(0.0),
            character.frag30.unwrap_or(0.0),
        );

        let mut prog = CharacterValue::new();
        prog.set_parameter(
            character.prog1.unwrap_or(0.0),
            character.prog20.unwrap_or(0.0),
            character.prog30.unwrap_or(0.0),
        );

        let mut overdrive = CharacterValue::new();
        overdrive.set_parameter(
            character.overdrive1.unwrap_or(0.0),
            character.overdrive20.unwrap_or(0.0),
            character.overdrive30.unwrap_or(0.0),
        );

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
            frag: frag.get_value(&level),
            prog: prog.get_value(&level),
            overdrive: overdrive.get_value(&level),
        }
    }
}

impl From<(Character, UserCharacterFull)> for CharacterInfo {
    fn from((character, user_char): (Character, UserCharacterFull)) -> Self {
        // Calculate character values using the proper formula
        let mut level = Level::new();
        level.level = user_char.level;
        level.exp = user_char.exp;
        level.max_level = if user_char.is_uncapped() { 30 } else { 20 };

        let mut frag = CharacterValue::new();
        frag.set_parameter(
            character.frag1.unwrap_or(0.0),
            character.frag20.unwrap_or(0.0),
            character.frag30.unwrap_or(0.0),
        );

        let mut prog = CharacterValue::new();
        prog.set_parameter(
            character.prog1.unwrap_or(0.0),
            character.prog20.unwrap_or(0.0),
            character.prog30.unwrap_or(0.0),
        );

        let mut overdrive = CharacterValue::new();
        overdrive.set_parameter(
            character.overdrive1.unwrap_or(0.0),
            character.overdrive20.unwrap_or(0.0),
            character.overdrive30.unwrap_or(0.0),
        );

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
            frag: frag.get_value(&level),
            prog: prog.get_value(&level),
            overdrive: overdrive.get_value(&level),
        }
    }
}

/// For update full character table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UpdateCharacter {
    pub character_id: i32,
    pub max_level: Option<i32>,
    pub is_uncapped: Option<i8>,
}
