use crate::error::{ArcError, ArcResult};
use crate::model::{Character, UserCharacter};
use sqlx::MySqlPool;

/// Character service for managing character items and user character data
pub struct CharacterService {
    pool: MySqlPool,
}

impl CharacterService {
    /// Create a new character service
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Grant a character to a user by character ID
    pub async fn grant_character_by_id(&self, user_id: i32, character_id: i32) -> ArcResult<()> {
        // Check if user already has this character
        let exists = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM user_char WHERE user_id = ? AND character_id = ?",
            user_id,
            character_id
        )
        .fetch_one(&self.pool)
        .await?;

        if exists == 0 {
            // Grant the character with default values
            sqlx::query!(
                "INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
                 VALUES (?, ?, 1, 0, 0, 0, 0)",
                user_id,
                character_id
            )
            .execute(&self.pool)
            .await?;
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
            ArcError::no_data(
                format!("No character with name: {}", character_name),
                404,
                -121,
            )
        })?;

        self.grant_character_by_id(user_id, character_id).await
    }

    /// Check if a user has a specific character
    pub async fn user_has_character(&self, user_id: i32, character_id: i32) -> ArcResult<bool> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM user_char WHERE user_id = ? AND character_id = ?",
            user_id,
            character_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get user's character data
    pub async fn get_user_character(
        &self,
        user_id: i32,
        character_id: i32,
    ) -> ArcResult<Option<UserCharacter>> {
        let user_char = sqlx::query!(
            "SELECT user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
             FROM user_char WHERE user_id = ? AND character_id = ?",
            user_id,
            character_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let user_char = user_char.map(|row| UserCharacter {
            user_id: row.user_id,
            character_id: row.character_id,
            level: row.level.unwrap_or(1),
            exp: row.exp.unwrap_or(0.0),
            is_uncapped: row.is_uncapped.unwrap_or(0),
            is_uncapped_override: row.is_uncapped_override.unwrap_or(0),
            skill_flag: row.skill_flag.unwrap_or(0),
        });

        Ok(user_char)
    }

    /// Get all characters owned by a user
    pub async fn get_user_characters(&self, user_id: i32) -> ArcResult<Vec<UserCharacter>> {
        let user_char_rows = sqlx::query!(
            "SELECT user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
             FROM user_char WHERE user_id = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let user_chars = user_char_rows
            .into_iter()
            .map(|row| UserCharacter {
                user_id: row.user_id,
                character_id: row.character_id,
                level: row.level.unwrap_or(1),
                exp: row.exp.unwrap_or(0.0),
                is_uncapped: row.is_uncapped.unwrap_or(0),
                is_uncapped_override: row.is_uncapped_override.unwrap_or(0),
                skill_flag: row.skill_flag.unwrap_or(0),
            })
            .collect();

        Ok(user_chars)
    }

    /// Update character level
    pub async fn update_character_level(
        &self,
        user_id: i32,
        character_id: i32,
        level: i32,
    ) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE user_char SET level = ? WHERE user_id = ? AND character_id = ?",
            level,
            user_id,
            character_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update character experience
    pub async fn update_character_exp(
        &self,
        user_id: i32,
        character_id: i32,
        exp: f64,
    ) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE user_char SET exp = ? WHERE user_id = ? AND character_id = ?",
            exp,
            user_id,
            character_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update character uncap status
    pub async fn update_character_uncap(
        &self,
        user_id: i32,
        character_id: i32,
        is_uncapped: bool,
        is_uncapped_override: Option<bool>,
    ) -> ArcResult<()> {
        let is_uncapped_val = if is_uncapped { 1 } else { 0 };
        let is_uncapped_override_val = is_uncapped_override.map(|b| if b { 1 } else { 0 });

        if let Some(override_val) = is_uncapped_override_val {
            sqlx::query!(
                "UPDATE user_char SET is_uncapped = ?, is_uncapped_override = ? WHERE user_id = ? AND character_id = ?",
                is_uncapped_val,
                override_val,
                user_id,
                character_id
            )
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE user_char SET is_uncapped = ? WHERE user_id = ? AND character_id = ?",
                is_uncapped_val,
                user_id,
                character_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Update character skill flag
    pub async fn update_character_skill_flag(
        &self,
        user_id: i32,
        character_id: i32,
        skill_flag: i32,
    ) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE user_char SET skill_flag = ? WHERE user_id = ? AND character_id = ?",
            skill_flag,
            user_id,
            character_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get character base information
    pub async fn get_character_info(&self, character_id: i32) -> ArcResult<Option<Character>> {
        let character = sqlx::query_as!(
            Character,
            "SELECT character_id, name, max_level, frag1, prog1, overdrive1, frag20, prog20, overdrive20,
             frag30, prog30, overdrive30, skill_id, skill_unlock_level, skill_requires_uncap,
             skill_id_uncap, char_type, is_uncapped
             FROM `character` WHERE character_id = ?",
            character_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(character)
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
        sqlx::query!(
            "DELETE FROM user_char WHERE user_id = ? AND character_id = ?",
            user_id,
            character_id
        )
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
        let count =
            sqlx::query_scalar!("SELECT COUNT(*) FROM user_char WHERE user_id = ?", user_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }
}
