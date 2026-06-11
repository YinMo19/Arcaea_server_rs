use crate::error::ArcError;
use crate::model::item::ItemTypes;
use crate::model::{Present, PresentItem};
use crate::service::world::StaminaImpl;
use sqlx::{MySql, MySqlPool, Transaction};

/// Present service for handling user present/gift system
pub struct PresentService {
    pool: MySqlPool,
}

impl PresentService {
    /// Create a new present service instance
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get all non-expired presents for a user
    ///
    /// Returns a list of presents that are available to the user and not yet expired.
    /// Expired presents are automatically filtered out.
    pub async fn get_user_presents(&self, user_id: i32) -> Result<Vec<Present>, ArcError> {
        let current_ts = chrono::Utc::now().timestamp_millis();

        // Get all presents for the user that haven't expired
        let present_records = sqlx::query!(
            r#"
            SELECT p.*
            FROM present p
            INNER JOIN user_present up ON p.present_id = up.present_id
            WHERE up.user_id = ? AND (p.expire_ts > ? OR p.expire_ts IS NULL)
            "#,
            user_id,
            current_ts
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user presents: {e}"),
        })?;

        // Get items for each present
        let mut result_presents = Vec::new();
        for record in present_records {
            let items = self.get_present_items(&record.present_id).await?;
            let present = Present {
                present_id: record.present_id,
                expire_ts: record.expire_ts,
                description: record.description,
                items: Some(items),
            };
            result_presents.push(present);
        }

        Ok(result_presents)
    }

    /// Get items for a specific present
    async fn get_present_items(&self, present_id: &str) -> Result<Vec<PresentItem>, ArcError> {
        let item_records = sqlx::query!(
            r#"
            SELECT *
            FROM present_item
            WHERE present_id = ?
            "#,
            present_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get present items: {e}"),
        })?;

        let items = item_records
            .into_iter()
            .map(|record| PresentItem {
                present_id: record.present_id,
                item_id: record.item_id,
                item_type: record.r#type,
                amount: record.amount.unwrap_or(1),
            })
            .collect();

        Ok(items)
    }

    /// Claim a present for a user
    ///
    /// This will:
    /// 1. Verify the present exists and belongs to the user
    /// 2. Check if the present has not expired
    /// 3. Remove the present from user_present table
    /// 4. Grant all items in the present to the user
    pub async fn claim_present(&self, user_id: i32, present_id: &str) -> Result<(), ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {e}"),
        })?;

        // Check if user has this present
        let user_present_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM user_present WHERE user_id = ? AND present_id = ?) as `exists!: i64`",
            user_id,
            present_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to check user present existence: {e}"),
        })?;

        if user_present_exists == 0 {
            return Err(ArcError::no_data(
                format!("Present '{present_id}' not found for user {user_id}"),
                108,
            ));
        }

        // Get present info to check expiry
        let present_record = sqlx::query!("SELECT * FROM present WHERE present_id = ?", present_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to get present info: {e}"),
            })?;

        let present_record = present_record.ok_or_else(|| {
            ArcError::no_data(format!("Present '{present_id}' does not exist"), 108)
        })?;

        // Check if present has expired
        if let Some(expire_ts) = present_record.expire_ts {
            let current_ts = chrono::Utc::now().timestamp_millis();
            if expire_ts < current_ts {
                return Err(ArcError::input(format!(
                    "Present '{present_id}' has expired"
                )));
            }
        }

        // Get present items
        let item_records = sqlx::query!(
            r#"
            SELECT *
            FROM present_item
            WHERE present_id = ?
            "#,
            present_id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get present items: {e}"),
        })?;

        let items: Vec<PresentItem> = item_records
            .into_iter()
            .map(|record| PresentItem {
                present_id: record.present_id,
                item_id: record.item_id,
                item_type: record.r#type,
                amount: record.amount.unwrap_or(1),
            })
            .collect();

        // Remove present from user
        sqlx::query!(
            "DELETE FROM user_present WHERE user_id = ? AND present_id = ?",
            user_id,
            present_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to remove user present: {e}"),
        })?;

        // Grant items to user
        for item in items {
            self.grant_item_to_user(
                &mut tx,
                user_id,
                &item.item_id,
                &item.item_type,
                item.amount,
            )
            .await?;
        }

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {e}"),
        })?;

        Ok(())
    }

    /// Grant an item to a user
    async fn grant_item_to_user(
        &self,
        tx: &mut Transaction<'_, MySql>,
        user_id: i32,
        item_id: &str,
        item_type: &str,
        amount: i32,
    ) -> Result<(), ArcError> {
        match item_type {
            ItemTypes::CORE => {
                self.grant_positive_item_to_user(tx, user_id, item_id, ItemTypes::CORE, amount)
                    .await
            }
            ItemTypes::CHARACTER => {
                self.grant_character_item_to_user(tx, user_id, item_id)
                    .await
            }
            ItemTypes::MEMORY => self.grant_memory_item_to_user(tx, user_id, amount).await,
            ItemTypes::FRAGMENT => Ok(()),
            ItemTypes::ANNI5TIX | ItemTypes::PICK_TICKET => {
                self.grant_positive_item_to_user(tx, user_id, item_id, item_type, amount)
                    .await
            }
            ItemTypes::WORLD_SONG
            | ItemTypes::WORLD_UNLOCK
            | ItemTypes::COURSE_BANNER
            | ItemTypes::ONLINE_BANNER
            | ItemTypes::SINGLE
            | ItemTypes::PACK => {
                self.grant_normal_item_to_user(tx, user_id, item_id, item_type)
                    .await
            }
            ItemTypes::PROG_BOOST_300 => {
                sqlx::query!(
                    "UPDATE user SET prog_boost = ? WHERE user_id = ?",
                    300,
                    user_id
                )
                .execute(&mut **tx)
                .await?;
                Ok(())
            }
            ItemTypes::STAMINA6 => {
                self.grant_stamina_item_to_user(tx, user_id, 6).await?;
                sqlx::query!(
                    "UPDATE user SET world_mode_locked_end_ts = ? WHERE user_id = ?",
                    -1i64,
                    user_id
                )
                .execute(&mut **tx)
                .await?;
                Ok(())
            }
            ItemTypes::STAMINA => self.grant_stamina_item_to_user(tx, user_id, amount).await,
            _ => Err(ArcError::input(format!(
                "The item type `{item_type}` is invalid."
            ))),
        }
    }

    async fn grant_normal_item_to_user(
        &self,
        tx: &mut Transaction<'_, MySql>,
        user_id: i32,
        item_id: &str,
        item_type: &str,
    ) -> Result<(), ArcError> {
        sqlx::query!(
            "INSERT IGNORE INTO user_item (user_id, item_id, type, amount) VALUES (?, ?, ?, 1)",
            user_id,
            item_id,
            item_type
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn grant_positive_item_to_user(
        &self,
        tx: &mut Transaction<'_, MySql>,
        user_id: i32,
        item_id: &str,
        item_type: &str,
        amount: i32,
    ) -> Result<(), ArcError> {
        let current_amount = sqlx::query!(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?",
            user_id,
            item_id,
            item_type
        )
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(row) = current_amount {
            let current_amount = row.amount.unwrap_or(0);
            if current_amount + amount < 0 {
                return Err(ArcError::ItemNotEnough {
                    message: format!("The user does not have enough `{item_id}`."),
                    error_code: 108,
                    api_error_code: -122,
                    extra_data: None,
                    status: 200,
                });
            }

            sqlx::query!(
                "UPDATE user_item SET amount = ? WHERE user_id = ? AND item_id = ? AND type = ?",
                current_amount + amount,
                user_id,
                item_id,
                item_type
            )
            .execute(&mut **tx)
            .await?;
        } else {
            if amount < 0 {
                return Err(ArcError::input(format!(
                    "The amount of `{item_id}` is wrong."
                )));
            }

            sqlx::query!(
                "INSERT INTO user_item (user_id, item_id, type, amount) VALUES (?, ?, ?, ?)",
                user_id,
                item_id,
                item_type,
                amount
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn grant_character_item_to_user(
        &self,
        tx: &mut Transaction<'_, MySql>,
        user_id: i32,
        character_id: &str,
    ) -> Result<(), ArcError> {
        let character_id = if character_id.chars().all(|c| c.is_ascii_digit()) {
            character_id.parse::<i32>().unwrap_or(0)
        } else {
            sqlx::query_scalar!(
                "SELECT character_id FROM `character` WHERE name = ?",
                character_id
            )
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| ArcError::no_data(format!("No character `{character_id}`."), 108))?
        };

        sqlx::query!(
            "INSERT IGNORE INTO user_char
             (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
             VALUES (?, ?, 1, 0, 0, 0, 0)",
            user_id,
            character_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn grant_memory_item_to_user(
        &self,
        tx: &mut Transaction<'_, MySql>,
        user_id: i32,
        amount: i32,
    ) -> Result<(), ArcError> {
        let current_ticket = sqlx::query!("SELECT ticket FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| ArcError::no_data("The ticket of the user is null.".to_string(), 108))?
            .ticket
            .unwrap_or(0);

        sqlx::query!(
            "UPDATE user SET ticket = ? WHERE user_id = ?",
            current_ticket + amount,
            user_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn grant_stamina_item_to_user(
        &self,
        tx: &mut Transaction<'_, MySql>,
        user_id: i32,
        amount: i32,
    ) -> Result<(), ArcError> {
        let Some(row) = sqlx::query!(
            "SELECT max_stamina_ts, stamina FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&mut **tx)
        .await?
        else {
            return Err(ArcError::no_data(
                "User not found for stamina update".to_string(),
                108,
            ));
        };

        let mut stamina =
            StaminaImpl::new(row.stamina.unwrap_or(0), row.max_stamina_ts.unwrap_or(0));
        stamina.set_stamina(stamina.get_current_stamina() + amount);

        sqlx::query!(
            "UPDATE user SET stamina = ?, max_stamina_ts = ? WHERE user_id = ?",
            stamina.get_current_stamina(),
            stamina.max_stamina_ts(),
            user_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Check if a present exists
    pub async fn present_exists(&self, present_id: &str) -> Result<bool, ArcError> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM present WHERE present_id = ?) as `exists!: i64`",
            present_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to check present existence: {e}"),
        })?;

        Ok(exists == 1)
    }

    /// Add a present to a user
    ///
    /// This adds an existing present to a user's present list.
    /// The present must already exist in the present table.
    pub async fn add_present_to_user(
        &self,
        user_id: i32,
        present_id: &str,
    ) -> Result<(), ArcError> {
        // Check if present exists
        if !self.present_exists(present_id).await? {
            return Err(ArcError::no_data(
                format!("Present '{present_id}' does not exist"),
                404,
            ));
        }

        // Add present to user
        sqlx::query!(
            "INSERT IGNORE INTO user_present (user_id, present_id) VALUES (?, ?)",
            user_id,
            present_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to add present to user: {e}"),
        })?;

        Ok(())
    }

    /// Create a new present
    ///
    /// Creates a new present with the given items and adds it to the specified user.
    pub async fn create_present(
        &self,
        present_id: &str,
        expire_ts: Option<i64>,
        description: &str,
        items: Vec<PresentItem>,
        user_id: i32,
    ) -> Result<(), ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {e}"),
        })?;

        // Insert present
        sqlx::query!(
            "INSERT INTO present (present_id, expire_ts, description) VALUES (?, ?, ?)",
            present_id,
            expire_ts,
            description
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to create present: {e}"),
        })?;

        // Insert present items
        for item in &items {
            // Ensure item exists in item table
            sqlx::query!(
                "INSERT IGNORE INTO item (item_id, type, is_available) VALUES (?, ?, 1)",
                item.item_id,
                item.item_type
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to insert item: {e}"),
            })?;

            // Insert present item
            sqlx::query!(
                "INSERT INTO present_item (present_id, item_id, type, amount) VALUES (?, ?, ?, ?)",
                present_id,
                item.item_id,
                item.item_type,
                item.amount
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to insert present item: {e}"),
            })?;
        }

        // Add present to user
        sqlx::query!(
            "INSERT INTO user_present (user_id, present_id) VALUES (?, ?)",
            user_id,
            present_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to add present to user: {e}"),
        })?;

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {e}"),
        })?;

        Ok(())
    }
}
