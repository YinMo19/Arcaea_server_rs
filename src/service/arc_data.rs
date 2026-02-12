use crate::error::{ArcError, ArcResult};
use crate::service::runtime_assets::asset_path;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct ArcData {
    pub characters: Vec<ArcDataCharacter>,
    #[serde(default, alias = "char_cores")]
    pub character_cores: Vec<ArcDataCore>,
    pub cores: Vec<String>,
    pub world_songs: Vec<String>,
    pub world_unlocks: Vec<String>,
    #[serde(default)]
    pub course_banners: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArcDataCharacter {
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
    pub skill_requires_uncap: i8,
    pub skill_id_uncap: String,
    pub char_type: i32,
    pub is_uncapped: i8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArcDataCore {
    pub character_id: i32,
    pub item_id: String,
    #[serde(default = "default_core_type", rename = "type")]
    pub item_type: String,
    pub amount: i32,
}

fn default_core_type() -> String {
    "core".to_string()
}

pub fn arc_data_file_path_from_env() -> String {
    env::var("ARC_DATA_FILE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| asset_path("arc_data.json").to_string_lossy().into_owned())
}

pub fn load_arc_data_from_file(file_path: &str) -> ArcResult<ArcData> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| ArcError::input(format!("Failed to read arc_data file `{file_path}`: {e}")))?;

    let mut data: ArcData = serde_json::from_str(&content).map_err(|e| {
        ArcError::input(format!("Failed to parse arc_data file `{file_path}`: {e}"))
    })?;

    if data.course_banners.is_empty() {
        data.course_banners = (1..=11).map(|i| format!("course_banner_{i}")).collect();
    }

    validate_arc_data(&data)?;
    Ok(data)
}

fn validate_arc_data(data: &ArcData) -> ArcResult<()> {
    if data.characters.is_empty() {
        return Err(ArcError::input("arc_data has no `characters` entries"));
    }

    ensure_unique_non_empty(&data.cores, "cores")?;
    ensure_unique_non_empty(&data.world_songs, "world_songs")?;
    ensure_unique_non_empty(&data.world_unlocks, "world_unlocks")?;
    ensure_unique_non_empty(&data.course_banners, "course_banners")?;

    let mut character_ids = HashSet::new();
    for character in &data.characters {
        if !character_ids.insert(character.character_id) {
            return Err(ArcError::input(format!(
                "duplicate character_id {} in `characters`",
                character.character_id
            )));
        }
        if character.name.trim().is_empty() {
            return Err(ArcError::input(format!(
                "character_id {} has an empty name",
                character.character_id
            )));
        }
    }

    let core_set: HashSet<&str> = data.cores.iter().map(String::as_str).collect();
    let mut char_core_keys = HashSet::new();
    for core in &data.character_cores {
        if !character_ids.contains(&core.character_id) {
            return Err(ArcError::input(format!(
                "character_cores contains unknown character_id {}",
                core.character_id
            )));
        }
        if core.item_type != "core" {
            return Err(ArcError::input(format!(
                "character_cores item_type must be `core`, got `{}` for character_id {}",
                core.item_type, core.character_id
            )));
        }
        if core.item_id.trim().is_empty() {
            return Err(ArcError::input(format!(
                "character_cores contains empty item_id for character_id {}",
                core.character_id
            )));
        }
        if !core_set.contains(core.item_id.as_str()) {
            return Err(ArcError::input(format!(
                "character_cores item `{}` is not present in `cores`",
                core.item_id
            )));
        }

        let key = (
            core.character_id,
            core.item_id.as_str(),
            core.item_type.as_str(),
        );
        if !char_core_keys.insert(key) {
            return Err(ArcError::input(format!(
                "duplicate character_cores entry ({}, {}, {})",
                core.character_id, core.item_id, core.item_type
            )));
        }
    }

    Ok(())
}

fn ensure_unique_non_empty(values: &[String], name: &str) -> ArcResult<()> {
    if values.is_empty() {
        return Err(ArcError::input(format!("arc_data has no `{name}` entries")));
    }

    let mut seen = HashSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(ArcError::input(format!("`{name}` contains an empty value")));
        }
        if !seen.insert(value.as_str()) {
            return Err(ArcError::input(format!(
                "`{name}` contains duplicate value `{value}`"
            )));
        }
    }
    Ok(())
}
