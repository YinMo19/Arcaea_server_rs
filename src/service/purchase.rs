use crate::error::ArcError;
use crate::model::{Purchase, PurchaseItem, PurchaseList};
use sqlx::MySqlPool;

/// Purchase service for handling purchase system
pub struct PurchaseService {
    pool: MySqlPool,
}

impl PurchaseService {
    /// Create a new purchase service instance
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get pack purchase information for user
    ///
    /// Returns available pack purchases with pricing and discount information.
    pub async fn get_pack_purchases(
        &self,
        _user_id: i32,
    ) -> Result<Vec<serde_json::Value>, ArcError> {
        let purchases = self.get_purchases_by_type(_user_id, "pack").await?;
        Ok(purchases.iter().map(|p| p.to_dict(true, true)).collect())
    }

    /// Get single song purchase information for user
    ///
    /// Returns available single song purchases with pricing and discount information.
    pub async fn get_single_purchases(
        &self,
        _user_id: i32,
    ) -> Result<Vec<serde_json::Value>, ArcError> {
        let purchases = self.get_purchases_by_type(_user_id, "single").await?;
        Ok(purchases.iter().map(|p| p.to_dict(true, true)).collect())
    }

    /// Get bundle purchases (always returns empty as per Python implementation)
    pub async fn get_bundle_purchases(&self) -> Result<Vec<serde_json::Value>, ArcError> {
        Ok(vec![])
    }

    /// Get purchases by type
    async fn get_purchases_by_type(
        &self,
        user_id: i32,
        purchase_type: &str,
    ) -> Result<Vec<Purchase>, ArcError> {
        // Get all purchases that contain items of the specified type
        let purchase_records = sqlx::query!(
            r#"
            SELECT DISTINCT p.purchase_name, p.price, p.orig_price, p.discount_from, p.discount_to, p.discount_reason
            FROM purchase p
            INNER JOIN purchase_item pi ON p.purchase_name = pi.purchase_name
            WHERE pi.type = ?
            "#,
            purchase_type
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get purchases by type: {}", e),
        })?;

        let mut purchases = Vec::new();
        for record in purchase_records {
            let items = self.get_purchase_items(&record.purchase_name).await?;

            let purchase = Purchase {
                purchase_name: record.purchase_name,
                price: record.price.unwrap_or(0),
                orig_price: record.orig_price.unwrap_or(0),
                discount_from: record.discount_from,
                discount_to: record.discount_to,
                discount_reason: record.discount_reason,
                items,
            };

            purchases.push(purchase);
        }

        Ok(purchases)
    }

    /// Get items for a purchase
    async fn get_purchase_items(&self, purchase_name: &str) -> Result<Vec<PurchaseItem>, ArcError> {
        let item_records = sqlx::query!(
            r#"
            SELECT purchase_name, item_id, type, amount
            FROM purchase_item
            WHERE purchase_name = ?
            "#,
            purchase_name
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get purchase items: {}", e),
        })?;

        let items = item_records
            .into_iter()
            .map(|record| PurchaseItem {
                purchase_name: record.purchase_name,
                item_id: record.item_id,
                item_type: record.r#type,
                amount: record.amount.unwrap_or(1),
            })
            .collect();

        Ok(items)
    }

    /// Buy a pack or single
    ///
    /// Handles the purchase of packs or singles, checking user tickets and granting items.
    pub async fn buy_pack_or_single(
        &self,
        user_id: i32,
        purchase_name: &str,
    ) -> Result<serde_json::Value, ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Get purchase information
        let purchase_record = sqlx::query!(
            "SELECT purchase_name, price, orig_price, discount_from, discount_to, discount_reason FROM purchase WHERE purchase_name = ?",
            purchase_name
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get purchase: {}", e),
        })?;

        let purchase_record = purchase_record.ok_or_else(|| {
            ArcError::no_data(&format!("Purchase '{}' not found", purchase_name), 404, -2)
        })?;

        // Calculate actual price considering discounts
        let actual_price = self
            .calculate_actual_price(
                user_id,
                &mut tx,
                purchase_record.price.unwrap_or(0),
                purchase_record.orig_price.unwrap_or(0),
                purchase_record.discount_from,
                purchase_record.discount_to,
                purchase_record.discount_reason.as_deref(),
            )
            .await?;

        // Get user's current ticket count
        let user_ticket = sqlx::query_scalar!("SELECT ticket FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to get user ticket: {}", e),
            })?
            .flatten()
            .unwrap_or(0);

        // Check if user has enough tickets
        if user_ticket < actual_price {
            return Err(ArcError::input("Not enough tickets".to_string()));
        }

        // Deduct tickets
        sqlx::query!(
            "UPDATE user SET ticket = ticket - ? WHERE user_id = ?",
            actual_price,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to deduct tickets: {}", e),
        })?;

        // Get purchase items
        let item_records = sqlx::query!(
            r#"
            SELECT purchase_name, item_id, type, amount
            FROM purchase_item
            WHERE purchase_name = ?
            "#,
            purchase_name
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get purchase items: {}", e),
        })?;

        let items: Vec<PurchaseItem> = item_records
            .into_iter()
            .map(|record| PurchaseItem {
                purchase_name: record.purchase_name,
                item_id: record.item_id,
                item_type: record.r#type,
                amount: record.amount.unwrap_or(1),
            })
            .collect();

        // Grant items to user
        for item in &items {
            self.grant_item_to_user(
                &mut tx,
                user_id,
                &item.item_id,
                &item.item_type,
                item.amount,
            )
            .await?;
        }

        // Get updated user information
        let user_info = sqlx::query!("SELECT ticket FROM user WHERE user_id = ?", user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to get updated user info: {}", e),
            })?;

        // Get user's packs, singles, and characters
        let packs = self.get_user_items(&mut tx, user_id, "pack").await?;
        let singles = self.get_user_items(&mut tx, user_id, "single").await?;
        let characters = self.get_user_characters(&mut tx, user_id).await?;

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(serde_json::json!({
            "user_id": user_id,
            "ticket": user_info.ticket,
            "packs": packs,
            "singles": singles,
            "characters": characters
        }))
    }

    /// Calculate actual price considering discounts
    async fn calculate_actual_price(
        &self,
        user_id: i32,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        price: i32,
        orig_price: i32,
        discount_from: Option<i64>,
        discount_to: Option<i64>,
        discount_reason: Option<&str>,
    ) -> Result<i32, ArcError> {
        if let (Some(from), Some(to)) = (discount_from, discount_to) {
            let current_time = chrono::Utc::now().timestamp_millis();
            if from <= current_time && current_time <= to {
                if let Some(reason) = discount_reason {
                    match reason {
                        "anni5tix" => {
                            let amount = self
                                .get_user_item_amount(tx, user_id, "anni5tix", "anni5tix")
                                .await?;
                            if amount >= 1 {
                                return Ok(0);
                            }
                        }
                        "pick_ticket" => {
                            let amount = self
                                .get_user_item_amount(tx, user_id, "pick_ticket", "pick_ticket")
                                .await?;
                            if amount >= 1 {
                                return Ok(0);
                            }
                        }
                        _ => {}
                    }
                }
                return Ok(price);
            }
        }
        Ok(orig_price)
    }

    /// Get user item amount
    async fn get_user_item_amount(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        user_id: i32,
        item_id: &str,
        item_type: &str,
    ) -> Result<i32, ArcError> {
        let amount = sqlx::query_scalar!(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?",
            user_id,
            item_id,
            item_type
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user item amount: {}", e),
        })?
        .flatten()
        .unwrap_or(0);

        Ok(amount)
    }

    /// Grant an item to a user
    async fn grant_item_to_user(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        user_id: i32,
        item_id: &str,
        item_type: &str,
        amount: i32,
    ) -> Result<(), ArcError> {
        sqlx::query!(
            r#"
            INSERT INTO user_item (user_id, item_id, type, amount)
            VALUES (?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE amount = amount + VALUES(amount)
            "#,
            user_id,
            item_id,
            item_type,
            amount
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to grant item to user: {}", e),
        })?;

        Ok(())
    }

    /// Get user items of specific type
    async fn get_user_items(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        user_id: i32,
        item_type: &str,
    ) -> Result<Vec<String>, ArcError> {
        let items = sqlx::query_scalar!(
            "SELECT item_id FROM user_item WHERE user_id = ? AND type = ? AND amount > 0",
            user_id,
            item_type
        )
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user items: {}", e),
        })?;

        Ok(items)
    }

    /// Get user characters
    async fn get_user_characters(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        user_id: i32,
    ) -> Result<Vec<i32>, ArcError> {
        let characters = sqlx::query_scalar!(
            "SELECT character_id FROM user_char WHERE user_id = ?",
            user_id
        )
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user characters: {}", e),
        })?;

        Ok(characters)
    }

    /// Buy special items (stamina, boost, etc.)
    pub async fn buy_special_item(
        &self,
        user_id: i32,
        item_id: &str,
    ) -> Result<serde_json::Value, ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        let price = 50; // Fixed price for special items as per Python code

        // Get user's current ticket count
        let user_ticket = sqlx::query_scalar!("SELECT ticket FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArcError::Database {
                message: format!("Failed to get user ticket: {}", e),
            })?
            .flatten()
            .unwrap_or(0);

        // Check if user has enough tickets
        if user_ticket < price {
            return Err(ArcError::input("Not enough tickets".to_string()));
        }

        // Deduct tickets
        sqlx::query!(
            "UPDATE user SET ticket = ticket - ? WHERE user_id = ?",
            price,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to deduct tickets: {}", e),
        })?;

        // Grant the item
        self.grant_item_to_user(&mut tx, user_id, item_id, item_id, 1)
            .await?;

        // Get updated user info
        let user_info = sqlx::query!(
            "SELECT ticket, stamina, max_stamina_ts FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get updated user info: {}", e),
        })?;

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        let mut result = serde_json::json!({
            "user_id": user_id,
            "ticket": user_info.ticket
        });

        if item_id == "stamina6" {
            result["stamina"] = serde_json::json!(user_info.stamina);
            result["max_stamina_ts"] = serde_json::json!(user_info.max_stamina_ts);
            result["world_mode_locked_end_ts"] = serde_json::json!(-1);
        }

        Ok(result)
    }

    /// Purchase stamina using fragments
    pub async fn purchase_stamina_with_fragment(
        &self,
        user_id: i32,
    ) -> Result<serde_json::Value, ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Get user's next fragment stamina timestamp
        let user_info = sqlx::query!(
            "SELECT next_fragstam_ts, stamina, max_stamina_ts FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get user info: {}", e),
        })?;

        let user_info = user_info.ok_or_else(|| ArcError::no_data("User not found", 404, -2))?;

        let current_time = chrono::Utc::now().timestamp_millis();
        let next_fragstam_ts = user_info.next_fragstam_ts.unwrap_or(-1);

        if next_fragstam_ts > current_time {
            return Err(ArcError::ItemUnavailable {
                message: "Buying stamina by fragment is not available yet".to_string(),
                error_code: 905,
                api_error_code: 905,
                status: 400,
                extra_data: None,
            });
        }

        // Constants from config (would be loaded from config file)
        let fragstam_recover_tick = 86400000; // 24 hours in milliseconds

        // Update next fragment stamina timestamp
        let new_next_fragstam_ts = current_time + fragstam_recover_tick;
        sqlx::query!(
            "UPDATE user SET next_fragstam_ts = ? WHERE user_id = ?",
            new_next_fragstam_ts,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to update next fragstam ts: {}", e),
        })?;

        // Grant stamina6 item (this would trigger stamina recovery)
        self.grant_item_to_user(&mut tx, user_id, "stamina6", "stamina6", 1)
            .await?;

        // Get updated user info
        let updated_user_info = sqlx::query!(
            "SELECT stamina, max_stamina_ts, next_fragstam_ts FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get updated user info: {}", e),
        })?;

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(serde_json::json!({
            "user_id": user_id,
            "stamina": updated_user_info.stamina,
            "max_stamina_ts": updated_user_info.max_stamina_ts,
            "next_fragstam_ts": updated_user_info.next_fragstam_ts,
            "world_mode_locked_end_ts": -1
        }))
    }

    /// Redeem a code
    pub async fn redeem_code(
        &self,
        user_id: i32,
        code: &str,
    ) -> Result<serde_json::Value, ArcError> {
        let mut tx = self.pool.begin().await.map_err(|e| ArcError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Check if redeem code exists and is valid
        let redeem_exists =
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM redeem WHERE code = ?)", code)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ArcError::Database {
                    message: format!("Failed to check redeem code: {}", e),
                })?;

        if redeem_exists == 0 {
            return Err(ArcError::no_data("Invalid redeem code", 404, -2));
        }

        // Check if user has already used this code
        let already_used = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM user_redeem WHERE user_id = ? AND code = ?)",
            user_id,
            code
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to check if code already used: {}", e),
        })?;

        if already_used == 1 {
            return Err(ArcError::input("Code already used".to_string()));
        }

        // Mark code as used by user
        sqlx::query!(
            "INSERT INTO user_redeem (user_id, code) VALUES (?, ?)",
            user_id,
            code
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to mark code as used: {}", e),
        })?;

        // Get redeem items
        let redeem_items = sqlx::query!(
            "SELECT item_id, type, amount FROM redeem_item WHERE code = ?",
            code
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ArcError::Database {
            message: format!("Failed to get redeem items: {}", e),
        })?;

        let mut fragment_amount = 0;

        // Grant items to user
        for item in redeem_items {
            let item_amount = item.amount.unwrap_or(1);
            self.grant_item_to_user(&mut tx, user_id, &item.item_id, &item.r#type, item_amount)
                .await?;

            if item.r#type == "fragment" {
                fragment_amount += item_amount;
            }
        }

        tx.commit().await.map_err(|e| ArcError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(serde_json::json!({
            "coupon": if fragment_amount > 0 { format!("fragment{}", fragment_amount) } else { String::new() }
        }))
    }
}
