use crate::config::{Constants, CONFIG};
use crate::error::{ArcError, ArcResult};
use crate::model::{
    Character, CharacterValue, CoreItem, Level, Skill, UpdateCharacter, UserCharacter,
    UserCharacterInfo,
};
use crate::service::arc_data::load_arc_data_from_file;
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};

/// Character service for managing character items and user character data
pub struct CharacterService {
    pool: MySqlPool,
}

impl CharacterService {
    /// Create a new character service
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Synchronize `character` and `char_item` data from arc_data JSON without clearing DB.
    /// This is intended for startup-time "compare-and-add/update" workflows.
    pub async fn sync_arc_data_from_file(&self, file_path: &str) -> ArcResult<(usize, usize)> {
        let config = load_arc_data_from_file(file_path)?;

        let mut tx = self.pool.begin().await?;

        for character in &config.characters {
            sqlx::query!(
                r#"
                INSERT INTO `character` (
                    character_id, name, max_level, frag1, prog1, overdrive1,
                    frag20, prog20, overdrive20, frag30, prog30, overdrive30,
                    skill_id, skill_unlock_level, skill_requires_uncap,
                    skill_id_uncap, char_type, is_uncapped
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    name = VALUES(name),
                    max_level = VALUES(max_level),
                    frag1 = VALUES(frag1),
                    prog1 = VALUES(prog1),
                    overdrive1 = VALUES(overdrive1),
                    frag20 = VALUES(frag20),
                    prog20 = VALUES(prog20),
                    overdrive20 = VALUES(overdrive20),
                    frag30 = VALUES(frag30),
                    prog30 = VALUES(prog30),
                    overdrive30 = VALUES(overdrive30),
                    skill_id = VALUES(skill_id),
                    skill_unlock_level = VALUES(skill_unlock_level),
                    skill_requires_uncap = VALUES(skill_requires_uncap),
                    skill_id_uncap = VALUES(skill_id_uncap),
                    char_type = VALUES(char_type),
                    is_uncapped = VALUES(is_uncapped)
                "#,
                character.character_id,
                character.name,
                character.max_level,
                character.frag1,
                character.prog1,
                character.overdrive1,
                character.frag20,
                character.prog20,
                character.overdrive20,
                character.frag30,
                character.prog30,
                character.overdrive30,
                character.skill_id,
                character.skill_unlock_level,
                character.skill_requires_uncap,
                character.skill_id_uncap,
                character.char_type,
                character.is_uncapped
            )
            .execute(&mut *tx)
            .await?;
        }

        for core in &config.character_cores {
            sqlx::query!(
                r#"
                INSERT INTO char_item (character_id, item_id, type, amount)
                VALUES (?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE amount = VALUES(amount)
                "#,
                core.character_id,
                core.item_id,
                core.item_type,
                core.amount
            )
            .execute(&mut *tx)
            .await?;
        }

        for item_id in &config.cores {
            sqlx::query!(
                r#"
                INSERT INTO item (item_id, type, is_available)
                VALUES (?, 'core', 1)
                ON DUPLICATE KEY UPDATE is_available = VALUES(is_available)
                "#,
                item_id
            )
            .execute(&mut *tx)
            .await?;
        }

        for item_id in &config.world_songs {
            sqlx::query!(
                r#"
                INSERT INTO item (item_id, type, is_available)
                VALUES (?, 'world_song', 1)
                ON DUPLICATE KEY UPDATE is_available = VALUES(is_available)
                "#,
                item_id
            )
            .execute(&mut *tx)
            .await?;
        }

        for item_id in &config.world_unlocks {
            sqlx::query!(
                r#"
                INSERT INTO item (item_id, type, is_available)
                VALUES (?, 'world_unlock', 1)
                ON DUPLICATE KEY UPDATE is_available = VALUES(is_available)
                "#,
                item_id
            )
            .execute(&mut *tx)
            .await?;
        }

        for item_id in &config.course_banners {
            sqlx::query!(
                r#"
                INSERT INTO item (item_id, type, is_available)
                VALUES (?, 'course_banner', 1)
                ON DUPLICATE KEY UPDATE is_available = VALUES(is_available)
                "#,
                item_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok((config.characters.len(), config.character_cores.len()))
    }

    /// Get the appropriate table name based on configuration
    fn get_user_char_table(&self) -> &'static str {
        if CONFIG.character_full_unlock {
            "user_char_full"
        } else {
            "user_char"
        }
    }

    /// Grant a character to a user by character ID
    pub async fn grant_character_by_id(&self, user_id: i32, character_id: i32) -> ArcResult<()> {
        // Check if user already has this character
        let exists = if CONFIG.character_full_unlock {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM user_char_full WHERE user_id = ? AND character_id = ?",
                user_id,
                character_id
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM user_char WHERE user_id = ? AND character_id = ?",
                user_id,
                character_id
            )
            .fetch_one(&self.pool)
            .await?
        };

        if exists == 0 {
            // Grant the character with default values
            if CONFIG.character_full_unlock {
                sqlx::query!(
                    "INSERT INTO user_char_full (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
                     VALUES (?, ?, 1, 0, 0, 0, 0)",
                    user_id,
                    character_id
                )
                .execute(&self.pool)
                .await?;
            } else {
                sqlx::query!(
                    "INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
                     VALUES (?, ?, 1, 0, 0, 0, 0)",
                    user_id,
                    character_id
                )
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Grant a character to a user by character name
    pub async fn grant_character_by_name(
        &self,
        user_id: i32,
        character_name: &str,
    ) -> ArcResult<()> {
        // Look up character ID by name
        let character_id = sqlx::query_scalar!(
            "SELECT character_id FROM `character` WHERE name = ?",
            character_name
        )
        .fetch_optional(&self.pool)
        .await?;

        let character_id = character_id.ok_or_else(|| {
            ArcError::no_data(format!("No character with name: {character_name}"), 404)
        })?;

        self.grant_character_by_id(user_id, character_id).await
    }

    /// Get user character statistics as JSON values for API response
    /// This matches Python's UserCharacter.to_dict() method exactly
    pub async fn get_user_character_stats(
        &self,
        user_id: i32,
    ) -> ArcResult<Vec<serde_json::Value>> {
        let table_name = self.get_user_char_table();

        let query = format!(
            r#"
            SELECT uc.character_id, uc.level, uc.exp, uc.is_uncapped, uc.is_uncapped_override, uc.skill_flag,
                   c.name, c.max_level, c.char_type, c.skill_id, c.skill_id_uncap, c.skill_unlock_level,
                   c.skill_requires_uncap, c.frag1, c.frag20, c.frag30, c.prog1, c.prog20, c.prog30,
                   c.overdrive1, c.overdrive20, c.overdrive30
            FROM {table_name} uc
            JOIN `character` c ON uc.character_id = c.character_id
            WHERE uc.user_id = ?
            ORDER BY uc.character_id
            "#
        );

        let characters = sqlx::query(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        let mut character_stats = Vec::new();

        for char_record in characters {
            let character_id: i32 = char_record.get("character_id");
            let level: i32 = char_record.get::<Option<i32>, _>("level").unwrap_or(1);
            let exp: f64 = char_record.get::<Option<f64>, _>("exp").unwrap_or(0.0);
            let is_uncapped: bool =
                char_record.get::<Option<i8>, _>("is_uncapped").unwrap_or(0) != 0;
            let is_uncapped_override: bool = char_record
                .get::<Option<i8>, _>("is_uncapped_override")
                .unwrap_or(0)
                != 0;
            let skill_flag: bool =
                char_record.get::<Option<i32>, _>("skill_flag").unwrap_or(0) != 0;

            // Create level info
            let mut level_info = Level::new();
            level_info.level = level;
            level_info.exp = exp;
            level_info.max_level = char_record.get::<Option<i32>, _>("max_level").unwrap_or(20);

            // Create character values
            let mut frag = CharacterValue::new();
            frag.set_parameter(
                char_record.get::<Option<f64>, _>("frag1").unwrap_or(0.0),
                char_record.get::<Option<f64>, _>("frag20").unwrap_or(0.0),
                char_record.get::<Option<f64>, _>("frag30").unwrap_or(0.0),
            );

            let mut prog = CharacterValue::new();
            prog.set_parameter(
                char_record.get::<Option<f64>, _>("prog1").unwrap_or(0.0),
                char_record.get::<Option<f64>, _>("prog20").unwrap_or(0.0),
                char_record.get::<Option<f64>, _>("prog30").unwrap_or(0.0),
            );

            let mut overdrive = CharacterValue::new();
            overdrive.set_parameter(
                char_record
                    .get::<Option<f64>, _>("overdrive1")
                    .unwrap_or(0.0),
                char_record
                    .get::<Option<f64>, _>("overdrive20")
                    .unwrap_or(0.0),
                char_record
                    .get::<Option<f64>, _>("overdrive30")
                    .unwrap_or(0.0),
            );

            // Handle Fatalis special calculations (character 55)
            let mut fatalis_is_limited = false;
            if character_id == 55 {
                let addition = if CONFIG.character_full_unlock {
                    fatalis_is_limited = true;
                    Constants::FATALIS_MAX_VALUE as f64
                } else {
                    // Get world step count from user_kvdata
                    let steps = sqlx::query_scalar!(
                        r#"SELECT value FROM user_kvdata WHERE user_id = ? AND class = 'world' AND `key` = 'total_step_count' AND idx = 0"#,
                        user_id
                    )
                    .fetch_optional(&self.pool)
                    .await?
                    .and_then(|v| v.and_then(|s| s.parse::<i32>().ok()))
                    .unwrap_or(0) as f64;

                    let addition = steps / 30.0;
                    if addition >= Constants::FATALIS_MAX_VALUE as f64 {
                        fatalis_is_limited = true;
                        Constants::FATALIS_MAX_VALUE as f64
                    } else {
                        addition
                    }
                };
                prog.addition = addition;
                overdrive.addition = addition;
            }

            // Get uncap cores for this character
            let uncap_cores = self.get_character_uncap_cores(character_id).await?;

            // Create skill info
            let skill_id = char_record.get::<Option<String>, _>("skill_id");
            let skill_id_uncap = char_record.get::<Option<String>, _>("skill_id_uncap");
            let skill_unlock_level: i32 = char_record
                .get::<Option<i32>, _>("skill_unlock_level")
                .unwrap_or(1);
            let skill_requires_uncap: bool = char_record
                .get::<Option<i8>, _>("skill_requires_uncap")
                .unwrap_or(0)
                != 0;

            // Calculate skill_id_displayed - matches Python logic exactly
            let is_uncapped_displayed = if is_uncapped_override {
                false
            } else {
                is_uncapped
            };
            let skill_id_displayed = if is_uncapped_displayed
                && skill_id_uncap.is_some()
                && !skill_id_uncap.as_ref().unwrap().is_empty()
            {
                skill_id_uncap.clone()
            } else if skill_id.is_some()
                && !skill_id.as_ref().unwrap().is_empty()
                && level >= skill_unlock_level
            {
                skill_id.clone()
            } else {
                None
            };

            // Calculate skill_state - matches Python logic exactly
            let skill_state = if skill_id_displayed.as_ref() == Some(&"skill_maya".to_string()) {
                if skill_flag {
                    Some("add_random".to_string())
                } else {
                    Some("remove_random".to_string())
                }
            } else {
                None
            };

            // Build character stat matching Python's to_dict exactly
            let mut character_stat = serde_json::json!({
                "base_character": character_id == 1,
                "is_uncapped_override": is_uncapped_override,
                "is_uncapped": is_uncapped,
                "uncap_cores": uncap_cores,
                "char_type": char_record.get::<Option<i32>, _>("char_type").unwrap_or(0),
                "skill_id_uncap": skill_id_uncap.unwrap_or_default(),
                "skill_requires_uncap": skill_requires_uncap,
                "skill_unlock_level": skill_unlock_level,
                "skill_id": skill_id.unwrap_or_default(),
                "overdrive": overdrive.get_value(&level_info),
                "prog": prog.get_value(&level_info),
                "frag": frag.get_value(&level_info),
                "level_exp": level_info.level_exp(),
                "exp": exp,
                "level": level,
                "name": char_record.get::<Option<String>, _>("name").unwrap_or_default(),
                "character_id": character_id
            });

            // Add voice for specific characters (21, 46)
            if [21, 46].contains(&character_id) {
                if let Value::Object(ref mut map) = character_stat {
                    map.insert("voice".to_string(), json!([0, 1, 2, 3, 100, 1000, 1001]));
                }
            }

            // Add fatalis_is_limited for character 55
            if character_id == 55 {
                if let Value::Object(ref mut map) = character_stat {
                    map.insert("fatalis_is_limited".to_string(), json!(fatalis_is_limited));
                }
            }

            // Add base_character_id for specific characters [1, 6, 7, 17, 18, 24, 32, 35, 52]
            if [1, 6, 7, 17, 18, 24, 32, 35, 52].contains(&character_id) {
                if let Value::Object(ref mut map) = character_stat {
                    map.insert("base_character_id".to_string(), json!(1));
                }
            }

            // Add skill_state if present
            if let Some(state) = skill_state {
                if let Value::Object(ref mut map) = character_stat {
                    map.insert("skill_state".to_string(), json!(state));
                }
            }

            character_stats.push(character_stat);
        }

        Ok(character_stats)
    }

    /// Get list of character IDs that the user owns
    pub async fn get_user_character_ids(&self, user_id: i32) -> ArcResult<Vec<i32>> {
        let character_ids = if CONFIG.character_full_unlock {
            sqlx::query_scalar!(
                "SELECT character_id FROM user_char_full WHERE user_id = ? ORDER BY character_id",
                user_id
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT character_id FROM user_char WHERE user_id = ? ORDER BY character_id",
                user_id
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(character_ids)
    }

    /// Get uncap cores required for a character
    async fn get_character_uncap_cores(
        &self,
        character_id: i32,
    ) -> ArcResult<Vec<serde_json::Value>> {
        let cores = sqlx::query!(
            r#"
            SELECT item_id as core_type, amount
            FROM char_item
            WHERE character_id = ? AND type = 'core'
            ORDER BY item_id
            "#,
            character_id
        )
        .fetch_all(&self.pool)
        .await?;

        let uncap_cores: Vec<serde_json::Value> = cores
            .into_iter()
            .map(|core| {
                serde_json::json!({
                    "core_type": core.core_type,
                    "amount": core.amount
                })
            })
            .collect();

        Ok(uncap_cores)
    }

    /// Get character uncap cores as CoreItem structs
    async fn get_character_uncap_cores_as_items(
        &self,
        character_id: i32,
    ) -> ArcResult<Vec<CoreItem>> {
        let core_items = sqlx::query!(
            "SELECT character_id, item_id, type, amount FROM char_item WHERE character_id = ? AND type = 'core'",
            character_id
        )
        .fetch_all(&self.pool)
        .await?;

        let cores = core_items
            .into_iter()
            .map(|item| CoreItem {
                item_id: item.item_id,
                amount: item.amount.unwrap_or(0),
            })
            .collect();

        Ok(cores)
    }

    /// Check if a user has a specific character
    pub async fn user_has_character(&self, user_id: i32, character_id: i32) -> ArcResult<bool> {
        let _table_name = self.get_user_char_table();

        let count = if CONFIG.character_full_unlock {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM user_char_full WHERE user_id = ? AND character_id = ?",
                user_id,
                character_id
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM user_char WHERE user_id = ? AND character_id = ?",
                user_id,
                character_id
            )
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count > 0)
    }

    /// Get character base information by ID
    pub async fn get_character_info(&self, character_id: i32) -> ArcResult<Character> {
        let character = sqlx::query_as!(
            Character,
            "SELECT character_id, name, max_level, frag1, prog1, overdrive1, frag20, prog20, overdrive20,
             frag30, prog30, overdrive30, skill_id, skill_unlock_level, skill_requires_uncap,
             skill_id_uncap, char_type, is_uncapped
             FROM `character` WHERE character_id = ?",
            character_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ArcError::no_data(format!("No such character: {character_id}"), 404)
        })?;

        Ok(character)
    }

    /// Get user character uncap condition (is_uncapped, is_uncapped_override)
    pub async fn get_user_character_uncap_condition(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<(bool, bool)> {
        let _table_name = self.get_user_char_table();

        let result = if CONFIG.character_full_unlock {
            let row = sqlx::query!(
                "SELECT is_uncapped, is_uncapped_override FROM user_char_full WHERE user_id = ? AND character_id = ?",
                user_id,
                character_id
            )
            .fetch_optional(&self.pool)
            .await?;

            row.map(|r| (r.is_uncapped, r.is_uncapped_override))
        } else {
            let row = sqlx::query!(
                "SELECT is_uncapped, is_uncapped_override FROM user_char WHERE user_id = ? AND character_id = ?",
                user_id,
                character_id
            )
            .fetch_optional(&self.pool)
            .await?;

            row.map(|r| (r.is_uncapped, r.is_uncapped_override))
        };

        if let Some((is_uncapped, is_uncapped_override)) = result {
            Ok((
                is_uncapped.unwrap_or(0) != 0,
                is_uncapped_override.unwrap_or(0) != 0,
            ))
        } else {
            Ok((false, false))
        }
    }

    /// Get complete user character information
    pub async fn get_user_character_info(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<UserCharacterInfo> {
        let _table_name = self.get_user_char_table();

        // Get character base info first
        let character = self.get_character_info(character_id).await?;

        // Get user character data
        let (level, exp, is_uncapped, is_uncapped_override, skill_flag) = if CONFIG
            .character_full_unlock
        {
            let row = sqlx::query!(
                "SELECT level, exp, is_uncapped, is_uncapped_override, skill_flag FROM user_char_full WHERE user_id = ? AND character_id = ?",
                user_id, character_id
            )
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ArcError::no_data("The character of the user does not exist.", 404))?;

            (
                row.level.unwrap_or(1),
                row.exp.unwrap_or(0.0),
                row.is_uncapped.unwrap_or(0) != 0,
                row.is_uncapped_override.unwrap_or(0) != 0,
                row.skill_flag.unwrap_or(0) != 0,
            )
        } else {
            let row = sqlx::query!(
                "SELECT level, exp, is_uncapped, is_uncapped_override, skill_flag FROM user_char WHERE user_id = ? AND character_id = ?",
                user_id, character_id
            )
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ArcError::no_data("The character of the user does not exist.", 404))?;

            (
                row.level.unwrap_or(1),
                row.exp.unwrap_or(0.0),
                row.is_uncapped.unwrap_or(0) != 0,
                row.is_uncapped_override.unwrap_or(0) != 0,
                row.skill_flag.unwrap_or(0) != 0,
            )
        };

        // Build level information
        let mut level_info = Level::new();
        level_info.level = level;
        level_info.exp = exp;
        level_info.max_level = character.max_level.unwrap_or(20);

        // Build skill information
        let mut skill = Skill::new();
        skill.skill_id = character.skill_id.clone();
        skill.skill_id_uncap = character.skill_id_uncap.clone();
        skill.skill_unlock_level = character.skill_unlock_level.unwrap_or(1);
        skill.skill_requires_uncap = character.skill_requires_uncap();

        // Build character values
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

        // Get uncap cores
        let uncap_cores = self
            .get_character_uncap_cores_as_items(character_id)
            .await?;

        // Set voice data for specific characters
        let voice = if [21, 46].contains(&character_id) {
            Some(vec![0, 1, 2, 3, 100, 1000, 1001])
        } else {
            None
        };

        // Handle Fatalis special calculations (character 55)
        let mut fatalis_is_limited = false;
        if character_id == 55 {
            let addition = if CONFIG.character_full_unlock {
                fatalis_is_limited = true;
                Constants::FATALIS_MAX_VALUE as f64
            } else {
                // Get world step count from user_kvdata
                let steps = sqlx::query_scalar!(
                    r#"SELECT value FROM user_kvdata WHERE user_id = ? AND class = 'world' AND `key` = 'total_step_count' AND idx = 0"#,
                    user_id
                )
                .fetch_optional(&self.pool)
                .await?
                .and_then(|v| v.and_then(|s| s.parse::<i32>().ok()))
                .unwrap_or(0) as f64;

                let addition = steps / 30.0;
                if addition >= Constants::FATALIS_MAX_VALUE as f64 {
                    fatalis_is_limited = true;
                    Constants::FATALIS_MAX_VALUE as f64
                } else {
                    addition
                }
            };
            prog.addition = addition;
            overdrive.addition = addition;
        }

        let user_char_info = UserCharacterInfo {
            character_id,
            name: character.name.unwrap_or_default(),
            char_type: character.char_type.unwrap_or(0),
            level: level_info,
            skill,
            frag,
            prog,
            overdrive,
            is_uncapped,
            is_uncapped_override,
            skill_flag,
            uncap_cores,
            voice,
            fatalis_is_limited,
        };

        Ok(user_char_info)
    }

    /// Toggle uncap override state for a character - matches Python change_uncap_override
    pub async fn toggle_character_uncap_override(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<UserCharacterInfo> {
        let table_name = self.get_user_char_table();

        // Get current uncap condition
        let (is_uncapped, is_uncapped_override) = self
            .get_user_character_uncap_condition(user_id, character_id)
            .await?;

        // Can only toggle if character is actually uncapped
        if !is_uncapped {
            return Err(ArcError::Base {
                message: "Unknown Error".to_string(),
                error_code: 108,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        let new_override = !is_uncapped_override;

        // Update user table
        sqlx::query!(
            "UPDATE user SET is_char_uncapped_override = ? WHERE user_id = ?",
            if new_override { 1 } else { 0 },
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Update character table
        let query = format!(
            "UPDATE {table_name} SET is_uncapped_override = ? WHERE user_id = ? AND character_id = ?"
        );
        sqlx::query(&query)
            .bind(if new_override { 1 } else { 0 })
            .bind(user_id)
            .bind(character_id)
            .execute(&self.pool)
            .await?;

        // Return updated character info
        self.get_user_character_info(user_id, character_id).await
    }

    /// Perform character uncap (first time) - matches Python character_uncap
    pub async fn character_uncap(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<UserCharacterInfo> {
        if CONFIG.character_full_unlock {
            return Err(ArcError::Base {
                message: "All characters are available.".to_string(),
                error_code: 108,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        // Get current uncap state
        let (is_uncapped, _) = self
            .get_user_character_uncap_condition(user_id, character_id)
            .await?;

        if is_uncapped {
            return Err(ArcError::Base {
                message: "The character has been uncapped.".to_string(),
                error_code: 108,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        // Get required cores
        let uncap_cores = self
            .get_character_uncap_cores_as_items(character_id)
            .await?;

        // Check if user has enough cores
        for core in &uncap_cores {
            if core.amount > 0 {
                let user_amount = sqlx::query_scalar!(
                    "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = 'core'",
                    user_id,
                    core.item_id
                )
                .fetch_optional(&self.pool)
                .await?
                .flatten()
                .unwrap_or(0);

                if core.amount > user_amount {
                    return Err(ArcError::ItemNotEnough {
                        message: "The cores are not enough.".to_string(),
                        error_code: 108,
                        api_error_code: -100,
                        extra_data: None,
                        status: 200,
                    });
                }
            }
        }

        // Consume cores
        for core in &uncap_cores {
            if core.amount > 0 {
                sqlx::query!(
                    "UPDATE user_item SET amount = amount - ? WHERE user_id = ? AND item_id = ? AND type = 'core'",
                    core.amount,
                    user_id,
                    core.item_id
                )
                .execute(&self.pool)
                .await?;

                // Remove item if amount becomes 0
                sqlx::query!(
                    "DELETE FROM user_item WHERE user_id = ? AND item_id = ? AND type = 'core' AND amount <= 0",
                    user_id,
                    core.item_id
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Update character uncap state - use the correct table based on config
        let table_name = self.get_user_char_table();
        let query = format!(
            "UPDATE {table_name} SET is_uncapped = 1, is_uncapped_override = 0 WHERE user_id = ? AND character_id = ?"
        );
        sqlx::query(&query)
            .bind(user_id)
            .bind(character_id)
            .execute(&self.pool)
            .await?;

        // Return updated character info
        self.get_user_character_info(user_id, character_id).await
    }

    /// Upgrade character with experience - matches Python upgrade
    pub async fn upgrade_character(
        &self,
        user_id: i32,
        character_id: i32,
        exp_addition: f64,
    ) -> ArcResult<UserCharacterInfo> {
        if exp_addition == 0.0 {
            return self.get_user_character_info(user_id, character_id).await;
        }

        if CONFIG.character_full_unlock {
            return Err(ArcError::Base {
                message: "All characters are available.".to_string(),
                error_code: 108,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        // Get current character info
        let mut char_info = self.get_user_character_info(user_id, character_id).await?;

        // Set max level based on uncap state
        char_info.level.max_level = if char_info.is_uncapped { 30 } else { 20 };

        // Add experience and calculate new level
        char_info.level.add_exp(exp_addition)?;

        // Update database
        let table_name = self.get_user_char_table();
        let query = format!(
            "UPDATE {table_name} SET level = ?, exp = ? WHERE user_id = ? AND character_id = ?"
        );
        sqlx::query(&query)
            .bind(char_info.level.level)
            .bind(char_info.level.exp)
            .bind(user_id)
            .bind(character_id)
            .execute(&self.pool)
            .await?;

        Ok(char_info)
    }

    /// Upgrade character using core items (ether drops) - matches Python upgrade_by_core
    pub async fn upgrade_character_by_core(
        &self,
        user_id: i32,
        character_id: i32,
        core_amount: i32,
    ) -> ArcResult<UserCharacterInfo> {
        if core_amount >= 0 {
            return Err(ArcError::input(
                "The amount of `core_generic` should be negative.",
            ));
        }

        // Check if user has enough core_generic
        let user_amount = sqlx::query_scalar!(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = 'core_generic' AND type = 'core'",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .flatten()
        .unwrap_or(0);

        if (-core_amount) > user_amount {
            return Err(ArcError::ItemNotEnough {
                message: "Not enough core_generic.".to_string(),
                error_code: 108,
                api_error_code: -100,
                extra_data: None,
                status: 200,
            });
        }

        // Consume cores
        sqlx::query!(
            "UPDATE user_item SET amount = amount + ? WHERE user_id = ? AND item_id = 'core_generic' AND type = 'core'",
            core_amount,
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Remove item if amount becomes 0
        sqlx::query!(
            "DELETE FROM user_item WHERE user_id = ? AND item_id = 'core_generic' AND type = 'core' AND amount <= 0",
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Calculate exp to add
        let exp_addition = (-core_amount) as f64 * Constants::CORE_EXP as f64;

        // Upgrade character
        self.upgrade_character(user_id, character_id, exp_addition)
            .await
    }

    /// Change character skill state (for Maya) - matches Python change_skill_state
    pub async fn change_character_skill_state(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<()> {
        let table_name = self.get_user_char_table();

        // Toggle skill flag
        let query = format!(
            "UPDATE {table_name} SET skill_flag = 1 - skill_flag WHERE user_id = ? AND character_id = ?"
        );
        sqlx::query(&query)
            .bind(user_id)
            .bind(character_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get all characters owned by a user
    pub async fn get_user_characters(&self, user_id: i32) -> ArcResult<Vec<UserCharacter>> {
        let user_chars = if CONFIG.character_full_unlock {
            let rows = sqlx::query!(
                "SELECT user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag FROM user_char_full WHERE user_id = ?",
                user_id
            )
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter()
                .map(|row| UserCharacter {
                    user_id: row.user_id,
                    character_id: row.character_id,
                    level: row.level.unwrap_or(1),
                    exp: row.exp.unwrap_or(0.0),
                    is_uncapped: row.is_uncapped.unwrap_or(0),
                    is_uncapped_override: row.is_uncapped_override.unwrap_or(0),
                    skill_flag: row.skill_flag.unwrap_or(0),
                })
                .collect()
        } else {
            let rows = sqlx::query!(
                "SELECT user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag FROM user_char WHERE user_id = ?",
                user_id
            )
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter()
                .map(|row| UserCharacter {
                    user_id: row.user_id,
                    character_id: row.character_id,
                    level: row.level.unwrap_or(1),
                    exp: row.exp.unwrap_or(0.0),
                    is_uncapped: row.is_uncapped.unwrap_or(0),
                    is_uncapped_override: row.is_uncapped_override.unwrap_or(0),
                    skill_flag: row.skill_flag.unwrap_or(0),
                })
                .collect()
        };

        Ok(user_chars)
    }

    /// Get all available characters
    pub async fn get_all_characters(&self) -> ArcResult<Vec<Character>> {
        let characters = sqlx::query_as!(
            Character,
            "SELECT character_id, name, max_level, frag1, prog1, overdrive1, frag20, prog20, overdrive20,
             frag30, prog30, overdrive30, skill_id, skill_unlock_level, skill_requires_uncap,
             skill_id_uncap, char_type, is_uncapped
             FROM `character` ORDER BY character_id"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(characters)
    }

    /// Grant Hikari (Fatalis) character - character ID 55
    /// Used in finale_start endpoint
    pub async fn grant_hikari_fatalis(&self, user_id: i32) -> ArcResult<()> {
        self.grant_character_by_id(user_id, 55).await
    }

    /// Grant Hikari & Tairitsu (Reunion) character - character ID 5
    /// Used in finale_end endpoint
    pub async fn grant_hikari_tairitsu_reunion(&self, user_id: i32) -> ArcResult<()> {
        self.grant_character_by_id(user_id, 5).await
    }

    /// Grant Insight (Ascendant - 8th Seeker) character - character ID 72
    /// Used in insight_complete eden_append_1 endpoint
    pub async fn grant_insight_ascendant(&self, user_id: i32) -> ArcResult<()> {
        self.grant_character_by_id(user_id, 72).await
    }

    /// Grant multiple characters to user
    pub async fn grant_characters(&self, user_id: i32, character_ids: &[i32]) -> ArcResult<()> {
        for &character_id in character_ids {
            self.grant_character_by_id(user_id, character_id).await?;
        }
        Ok(())
    }

    /// Remove a character from user (if needed for debugging or admin purposes)
    pub async fn remove_character(&self, user_id: i32, character_id: i32) -> ArcResult<()> {
        let table_name = self.get_user_char_table();

        let query = format!("DELETE FROM {table_name} WHERE user_id = ? AND character_id = ?");
        sqlx::query(&query)
            .bind(user_id)
            .bind(character_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Copy character data to user_char_full table
    pub async fn copy_to_full_table(&self, user_id: i32) -> ArcResult<()> {
        // First delete existing data for the user
        sqlx::query!("DELETE FROM user_char_full WHERE user_id = ?", user_id)
            .execute(&self.pool)
            .await?;

        // Copy all user_char data to user_char_full
        sqlx::query!(
            "INSERT INTO user_char_full (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
             SELECT user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
             FROM user_char WHERE user_id = ?",
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get character count for user
    pub async fn get_user_character_count(&self, user_id: i32) -> ArcResult<i64> {
        let count = if CONFIG.character_full_unlock {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM user_char_full WHERE user_id = ?",
                user_id
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!("SELECT COUNT(*) FROM user_char WHERE user_id = ?", user_id)
                .fetch_one(&self.pool)
                .await?
        };

        Ok(count)
    }

    /// Update `user_char_full` from `character` without clearing existing rows.
    /// Missing rows are inserted; existing rows are upgraded only when target values are higher.
    pub async fn update_user_char_full(&self) -> ArcResult<()> {
        let characters = sqlx::query_as!(
            UpdateCharacter,
            "SELECT character_id, max_level, is_uncapped FROM `character`"
        )
        .fetch_all(&self.pool)
        .await?;
        let user_ids = sqlx::query_scalar!("SELECT user_id FROM user")
            .fetch_all(&self.pool)
            .await?;

        for user_id in user_ids {
            for character in &characters {
                let level = character.max_level.unwrap_or(20);
                let exp = if level == 30 { 25000.0 } else { 10000.0 };
                let is_uncapped = character.is_uncapped.unwrap_or(0);

                sqlx::query!(
                    r#"
                    INSERT INTO user_char_full (
                        user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
                    ) VALUES (?, ?, ?, ?, ?, 0, 0)
                    ON DUPLICATE KEY UPDATE
                        level = GREATEST(level, VALUES(level)),
                        exp = GREATEST(exp, VALUES(exp)),
                        is_uncapped = GREATEST(is_uncapped, VALUES(is_uncapped))
                    "#,
                    user_id,
                    character.character_id,
                    level,
                    exp,
                    is_uncapped
                )
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}
