use crate::config::CONFIG;
use crate::error::{ArcError, ArcResult};
use crate::model::item::{CharacterMapping, Item, ItemTypes, UserTicket};

use crate::service::UserService;
use sqlx::{MySql, Pool};
use std::collections::HashMap;

/// Item service for handling item operations
pub struct ItemService {
    pool: Pool<MySql>,
}

impl ItemService {
    /// Create a new item service instance
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }

    /// Check if item exists in database
    pub async fn select_exists(&self, item_id: &str, item_type: &str) -> ArcResult<bool> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM item WHERE item_id = ? AND type = ?) as `exists`",
            item_id,
            item_type
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.exists != 0)
    }

    /// Insert new item into database
    pub async fn insert_item(
        &self,
        item_id: &str,
        item_type: &str,
        is_available: bool,
        ignore: bool,
    ) -> ArcResult<()> {
        if ignore {
            sqlx::query!(
                "INSERT IGNORE INTO item VALUES (?, ?, ?)",
                item_id,
                item_type,
                is_available
            )
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query!(
                "INSERT INTO item VALUES (?, ?, ?)",
                item_id,
                item_type,
                is_available
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Delete item from database
    pub async fn delete_item(&self, item_id: &str, item_type: &str) -> ArcResult<()> {
        sqlx::query!(
            "DELETE FROM item WHERE item_id = ? AND type = ?",
            item_id,
            item_type
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update item availability
    pub async fn update_item(
        &self,
        item_id: &str,
        item_type: &str,
        is_available: bool,
    ) -> ArcResult<()> {
        sqlx::query!(
            "UPDATE item SET is_available = ? WHERE item_id = ? AND type = ?",
            is_available,
            item_id,
            item_type
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Select item from database
    pub async fn select_item(&self, item_id: &str, item_type: &str) -> ArcResult<bool> {
        let result = sqlx::query!(
            "SELECT is_available FROM item WHERE item_id = ? AND type = ?",
            item_id,
            item_type
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(row.is_available.unwrap_or(0) != 0),
            None => Err(ArcError::no_data(format!("No such item `{}`: `{}`", item_type, item_id), 108)),
        }
    }

    /// Select user item amount
    pub async fn select_user_item(
        &self,
        user_id: i32,
        item_id: &str,
        item_type: &str,
    ) -> ArcResult<i32> {
        let result = sqlx::query!(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?",
            user_id,
            item_id,
            item_type
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(row.amount.unwrap_or(1)),
            None => Ok(0),
        }
    }

    /// Claim normal item for user (binary ownership)
    pub async fn claim_normal_item(
        &self,
        user_id: i32,
        item_id: &str,
        item_type: &str,
    ) -> ArcResult<()> {
        // Check if item is available
        let _is_available = match self.select_item(item_id, item_type).await {
            Ok(available) => {
                if !available {
                    return Err(ArcError::ItemUnavailable {
                        message: "The item is unavailable.".to_string(),
                        error_code: 108,
                        api_error_code: -122,
                        extra_data: None,
                        status: 200,
                    });
                }
                true
            }
            Err(_) => {
                return Err(ArcError::no_data("No item data.".to_string(), 108));
            }
        };

        // Check if user already has the item
        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?) as `exists`",
            user_id,
            item_id,
            item_type
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists == 0 {
            sqlx::query!(
                "INSERT INTO user_item VALUES (?, ?, ?, 1)",
                user_id,
                item_id,
                item_type
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Claim positive item for user (with quantity)
    pub async fn claim_positive_item(
        &self,
        user_id: i32,
        item_id: &str,
        item_type: &str,
        amount: i32,
    ) -> ArcResult<()> {
        let current_amount = self.select_user_item(user_id, item_id, item_type).await?;

        if current_amount > 0 {
            if current_amount + amount < 0 {
                return Err(ArcError::ItemNotEnough {
                    message: format!("The user does not have enough `{}`.", item_id),
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
            .execute(&self.pool)
            .await?;
        } else {
            if amount < 0 {
                return Err(ArcError::input(format!(
                    "The amount of `{}` is wrong.",
                    item_id
                )));
            }
            sqlx::query!(
                "INSERT INTO user_item VALUES (?, ?, ?, ?)",
                user_id,
                item_id,
                item_type,
                amount
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Claim core item with reverse option
    pub async fn claim_core_item(
        &self,
        user_id: i32,
        core_type: &str,
        amount: i32,
        reverse: bool,
    ) -> ArcResult<()> {
        let actual_amount = if reverse { -amount } else { amount };
        self.claim_positive_item(user_id, core_type, ItemTypes::CORE, actual_amount)
            .await
    }

    /// Convert character name to ID
    pub async fn resolve_character_id(&self, character_id: &str) -> ArcResult<i32> {
        if character_id.chars().all(|c| c.is_ascii_digit()) {
            return Ok(character_id.parse::<i32>().unwrap_or(0));
        }

        let result = sqlx::query_as!(
            CharacterMapping,
            "SELECT character_id FROM `character` WHERE name = ?",
            character_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(mapping) => Ok(mapping.character_id),
            None => Err(ArcError::no_data(format!("No character `{}`.", character_id), 108)),
        }
    }

    /// Claim character for user
    pub async fn claim_character_item(&self, user_id: i32, character_id: &str) -> ArcResult<()> {
        let char_id = self.resolve_character_id(character_id).await?;

        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM user_char WHERE user_id = ? AND character_id = ?) as `exists`",
            user_id,
            char_id
        )
        .fetch_one(&self.pool)
        .await?;

        if exists.exists == 0 {
            sqlx::query!(
                "INSERT INTO user_char VALUES (?, ?, 1, 0, 0, 0, 0)",
                user_id,
                char_id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Claim memory item (add to user ticket)
    pub async fn claim_memory_item(&self, user_id: i32, amount: i32) -> ArcResult<()> {
        let current_ticket = sqlx::query_as!(
            UserTicket,
            "SELECT ticket FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match current_ticket {
            Some(ticket_info) => {
                let current = ticket_info.ticket.unwrap_or(0);
                sqlx::query!(
                    "UPDATE user SET ticket = ? WHERE user_id = ?",
                    current + amount,
                    user_id
                )
                .execute(&self.pool)
                .await?;
            }
            None => {
                return Err(ArcError::no_data("The ticket of the user is null.".to_string(), 108));
            }
        }

        Ok(())
    }

    /// Claim prog boost item
    pub async fn claim_prog_boost_item(&self, user_id: i32) -> ArcResult<()> {
        let user_service = UserService::new(self.pool.clone());
        user_service
            .update_user_one_column(user_id, "prog_boost", &300)
            .await
    }

    /// Claim stamina6 item
    pub async fn claim_stamina6_item(&self, user_id: i32) -> ArcResult<()> {
        let user_service = UserService::new(self.pool.clone());

        // Add 6 stamina
        user_service.add_stamina(user_id, 6).await?;

        // Clear world mode locked state
        user_service
            .update_user_one_column(user_id, "world_mode_locked_end_ts", &(-1i64))
            .await
    }

    /// Claim stamina item
    pub async fn claim_stamina_item(&self, user_id: i32, amount: i32) -> ArcResult<()> {
        let user_service = UserService::new(self.pool.clone());
        user_service.add_stamina(user_id, amount).await
    }

    /// Generic item claiming dispatcher
    pub async fn claim_item(
        &self,
        user_id: i32,
        item_id: &str,
        item_type: &str,
        amount: i32,
    ) -> ArcResult<()> {
        match item_type {
            ItemTypes::CORE => self.claim_core_item(user_id, item_id, amount, false).await,
            ItemTypes::CHARACTER => self.claim_character_item(user_id, item_id).await,
            ItemTypes::MEMORY => self.claim_memory_item(user_id, amount).await,
            ItemTypes::FRAGMENT => Ok(()), // Fragment does nothing in Python
            ItemTypes::ANNI5TIX | ItemTypes::PICK_TICKET => {
                self.claim_positive_item(user_id, item_id, item_type, amount)
                    .await
            }
            ItemTypes::WORLD_SONG
            | ItemTypes::WORLD_UNLOCK
            | ItemTypes::COURSE_BANNER
            | ItemTypes::SINGLE
            | ItemTypes::PACK => self.claim_normal_item(user_id, item_id, item_type).await,
            ItemTypes::PROG_BOOST_300 => self.claim_prog_boost_item(user_id).await,
            ItemTypes::STAMINA6 => self.claim_stamina6_item(user_id).await,
            ItemTypes::STAMINA => self.claim_stamina_item(user_id, amount).await,
            _ => Err(ArcError::input(format!(
                "The item type `{}` is invalid.",
                item_type
            ))),
        }
    }

    /// Create item from dictionary
    pub fn create_item_from_dict(
        &self,
        data: &HashMap<&str, serde_json::Value>,
    ) -> ArcResult<Item> {
        let item_type = data
            .get("item_type")
            .or_else(|| data.get("type"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArcError::input("The dict of item is wrong."))?;

        let item_id = data
            .get("item_id")
            .or_else(|| data.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(item_type.to_string()));

        let amount = data
            .get("amount")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .or(Some(1));

        let is_available = data
            .get("is_available")
            .and_then(|v| v.as_bool())
            .or(Some(true));

        Ok(Item::new(
            item_id,
            item_type.to_string(),
            amount,
            is_available,
        ))
    }

    /// Create item from string format
    pub fn create_item_from_string(&self, s: &str) -> ArcResult<Item> {
        if s.starts_with("fragment") {
            let amount = s[8..].parse::<i32>().unwrap_or(0);
            Ok(Item::new(
                Some("fragment".to_string()),
                ItemTypes::FRAGMENT.to_string(),
                Some(amount),
                Some(true),
            ))
        } else if s.starts_with("core") {
            let parts: Vec<&str> = s.split('_').collect();
            if parts.len() >= 3 {
                let item_id = format!("{}_{}", parts[0], parts[1]);
                let amount = parts.last().unwrap().parse::<i32>().unwrap_or(0);
                Ok(Item::new(
                    Some(item_id),
                    ItemTypes::CORE.to_string(),
                    Some(amount),
                    Some(true),
                ))
            } else {
                Err(ArcError::input("The string of item is wrong."))
            }
        } else if s.starts_with("course_banner") {
            Ok(Item::new(
                Some(s.to_string()),
                ItemTypes::COURSE_BANNER.to_string(),
                Some(1),
                Some(true),
            ))
        } else {
            Err(ArcError::input("The string of item is wrong."))
        }
    }

    /// Get user items by type
    pub async fn get_user_items_by_type(
        &self,
        user_id: i32,
        item_type: &str,
    ) -> ArcResult<Vec<Item>> {
        let mut items = Vec::new();

        // Check for full unlock configs
        if (CONFIG.world_song_full_unlock && item_type == ItemTypes::WORLD_SONG)
            || (CONFIG.world_scenery_full_unlock && item_type == ItemTypes::WORLD_UNLOCK)
        {
            let rows = sqlx::query!("SELECT item_id FROM item WHERE type = ?", item_type)
                .fetch_all(&self.pool)
                .await?;

            for row in rows {
                items.push(Item::new(
                    Some(row.item_id),
                    item_type.to_string(),
                    Some(1),
                    Some(true),
                ));
            }
        } else {
            let rows = sqlx::query!(
                "SELECT item_id, amount FROM user_item WHERE type = ? AND user_id = ?",
                item_type,
                user_id
            )
            .fetch_all(&self.pool)
            .await?;

            for row in rows {
                let amount = row.amount.unwrap_or(1);
                items.push(Item::new(
                    Some(row.item_id),
                    item_type.to_string(),
                    Some(amount),
                    Some(true),
                ));
            }
        }

        Ok(items)
    }

    /// Add items to collection
    pub async fn add_items_to_collection(
        &self,
        collection_id: &str,
        items: &[Item],
        table_name: &str,
        _table_primary_key: &str,
    ) -> ArcResult<()> {
        for item in items {
            let item_id = item
                .item_id
                .as_ref()
                .ok_or_else(|| ArcError::input("Item ID is required for collection operations"))?;
            let _amount = item.amount.unwrap_or(1);

            if !self.select_exists(item_id, &item.item_type).await? {
                return Err(ArcError::no_data(format!("No such item `{}`: `{}`", item.item_type, item_id), 108));
            }
        }

        for item in items {
            let item_id = item.item_id.as_ref().unwrap();
            let amount = item.amount.unwrap_or(1);

            sqlx::query(&format!("INSERT INTO {} VALUES (?, ?, ?, ?)", table_name))
                .bind(collection_id)
                .bind(item_id)
                .bind(&item.item_type)
                .bind(amount)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Remove items from collection
    pub async fn remove_items_from_collection(
        &self,
        collection_id: &str,
        items: &[Item],
        table_name: &str,
        table_primary_key: &str,
    ) -> ArcResult<()> {
        for item in items {
            let item_id = item
                .item_id
                .as_ref()
                .ok_or_else(|| ArcError::input("Item ID is required for collection operations"))?;

            sqlx::query(&format!(
                "DELETE FROM {} WHERE {} = ? AND item_id = ? AND type = ?",
                table_name, table_primary_key
            ))
            .bind(collection_id)
            .bind(item_id)
            .bind(&item.item_type)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Update items in collection
    pub async fn update_items_in_collection(
        &self,
        collection_id: &str,
        items: &[Item],
        table_name: &str,
        table_primary_key: &str,
    ) -> ArcResult<()> {
        for item in items {
            let item_id = item
                .item_id
                .as_ref()
                .ok_or_else(|| ArcError::input("Item ID is required for collection operations"))?;
            let amount = item.amount.unwrap_or(1);

            sqlx::query(&format!(
                "UPDATE {} SET amount = ? WHERE {} = ? AND item_id = ? AND type = ?",
                table_name, table_primary_key
            ))
            .bind(amount)
            .bind(collection_id)
            .bind(item_id)
            .bind(&item.item_type)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get user positive item amount
    pub async fn get_user_positive_item_amount(
        &self,
        user_id: i32,
        item_id: &str,
        item_type: &str,
    ) -> ArcResult<i32> {
        let result = sqlx::query!(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?",
            user_id,
            item_id,
            item_type
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|row| row.amount).unwrap_or(0))
    }
}

/// Item factory for creating items of different types
pub struct ItemFactory {
    service: ItemService,
}

impl ItemFactory {
    /// Create new item factory
    pub fn new(pool: Pool<MySql>) -> Self {
        Self {
            service: ItemService::new(pool),
        }
    }

    /// Create item by type
    pub fn create_item(&self, item_type: &str) -> ArcResult<Item> {
        let (default_amount, is_available) = match item_type {
            ItemTypes::CORE => (0, true),
            ItemTypes::CHARACTER => (1, true),
            ItemTypes::MEMORY => (1, true),
            ItemTypes::FRAGMENT => (0, true),
            ItemTypes::ANNI5TIX => (1, true),
            ItemTypes::PICK_TICKET => (1, true),
            ItemTypes::WORLD_SONG => (1, true),
            ItemTypes::WORLD_UNLOCK => (1, true),
            ItemTypes::SINGLE => (1, true),
            ItemTypes::PACK => (1, true),
            ItemTypes::PROG_BOOST_300 => (1, true),
            ItemTypes::STAMINA6 => (1, true),
            ItemTypes::STAMINA => (1, true),
            ItemTypes::COURSE_BANNER => (1, true),
            _ => {
                return Err(ArcError::input(format!(
                    "The item type `{}` is invalid.",
                    item_type
                )));
            }
        };

        Ok(Item::new(
            Some(item_type.to_string()),
            item_type.to_string(),
            Some(default_amount),
            Some(is_available),
        ))
    }

    /// Create item from dictionary
    pub fn from_dict(&self, data: &HashMap<&str, serde_json::Value>) -> ArcResult<Item> {
        self.service.create_item_from_dict(data)
    }

    /// Create item from string
    pub fn from_string(&self, s: &str) -> ArcResult<Item> {
        self.service.create_item_from_string(s)
    }
}

/// User item list for managing user's items
pub struct UserItemList {
    pool: Pool<MySql>,
    user_id: Option<i32>,
    items: Vec<Item>,
}

impl UserItemList {
    /// Create new user item list
    pub fn new(pool: Pool<MySql>, user_id: Option<i32>) -> Self {
        Self {
            pool,
            user_id,
            items: Vec::new(),
        }
    }

    /// Select items from specific type
    pub async fn select_from_type(&mut self, item_type: &str) -> ArcResult<()> {
        let user_id = self
            .user_id
            .ok_or_else(|| ArcError::input("User ID is required for selecting user items"))?;

        let service = ItemService::new(self.pool.clone());
        self.items = service.get_user_items_by_type(user_id, item_type).await?;
        Ok(())
    }

    /// Get the items list
    pub fn get_items(&self) -> &Vec<Item> {
        &self.items
    }
}
