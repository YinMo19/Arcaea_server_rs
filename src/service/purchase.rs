use crate::config::Constants;
use crate::error::{ArcError, ArcResult};
use crate::service::{ItemService, UserService};
use serde_json::{json, Value};
use sqlx::{MySql, Pool};
use std::time::{SystemTime, UNIX_EPOCH};

/// Purchase service for handling purchase system operations
pub struct PurchaseService {
    pool: Pool<MySql>,
    item_service: ItemService,
    user_service: UserService,
}

impl PurchaseService {
    /// Create a new purchase service instance
    pub fn new(pool: Pool<MySql>) -> Self {
        let item_service = ItemService::new(pool.clone());
        let user_service = UserService::new(pool.clone());
        Self {
            pool,
            item_service,
            user_service,
        }
    }

    /// Get current timestamp in milliseconds
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Get pack purchase information for user
    ///
    /// Returns available pack purchases with pricing and discount information.
    pub async fn get_pack_purchases(&self, user_id: i32) -> ArcResult<Vec<Value>> {
        self.get_purchases_by_type(user_id, "pack").await
    }

    /// Get single song purchase information for user
    ///
    /// Returns available single song purchases with pricing and discount information.
    pub async fn get_single_purchases(&self, user_id: i32) -> ArcResult<Vec<Value>> {
        self.get_purchases_by_type(user_id, "single").await
    }

    /// Get bundle purchases (always returns empty as per Python implementation)
    pub async fn get_bundle_purchases(&self) -> ArcResult<Vec<Value>> {
        Ok(vec![])
    }

    /// Get purchases by type with discount calculation
    async fn get_purchases_by_type(&self, user_id: i32, item_type: &str) -> ArcResult<Vec<Value>> {
        let purchase_names = sqlx::query!(
            "SELECT purchase_name FROM purchase_item WHERE type = ?",
            item_type
        )
        .fetch_all(&self.pool)
        .await?;

        let mut purchases = Vec::new();

        for purchase_name_row in purchase_names {
            let purchase_name = purchase_name_row.purchase_name;
            match self
                .get_purchase_with_discounts(&purchase_name, user_id)
                .await
            {
                Ok(purchase_data) => purchases.push(purchase_data),
                Err(e) => {
                    log::warn!("Failed to get purchase {purchase_name}: {e}");
                    continue;
                }
            }
        }

        Ok(purchases)
    }

    /// Get individual purchase with discount calculation
    async fn get_purchase_with_discounts(
        &self,
        purchase_name: &str,
        user_id: i32,
    ) -> ArcResult<Value> {
        // Get purchase base information
        let purchase_info = sqlx::query!(
            "SELECT * FROM purchase WHERE purchase_name = ?",
            purchase_name
        )
        .fetch_optional(&self.pool)
        .await?;

        let purchase_info = purchase_info.ok_or_else(|| {
            ArcError::no_data(format!("Purchase `{purchase_name}` does not exist."), 501)
        })?;

        // Get purchase items
        let purchase_items = sqlx::query!(
            "SELECT item_id, type as item_type, amount FROM purchase_item WHERE purchase_name = ?",
            purchase_name
        )
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::new();
        let mut main_item: Option<Value> = None;

        for item in purchase_items {
            let item_json = json!({
                "id": item.item_id,
                "type": item.item_type,
                "amount": item.amount.unwrap_or(1),
                "is_available": true
            });

            // Main item is the one with same id as purchase name
            if item.item_id == purchase_name {
                main_item = Some(item_json.clone());
            } else {
                items.push(item_json);
            }
        }

        // Sort items with main item first
        if let Some(main) = main_item {
            items.insert(0, main);
        }

        // Calculate displayed price with discounts
        let displayed_price = self
            .calculate_displayed_price(
                purchase_info.price.unwrap_or(0),
                purchase_info.orig_price.unwrap_or(0),
                purchase_info.discount_from.unwrap_or(-1),
                purchase_info.discount_to.unwrap_or(-1),
                purchase_info.discount_reason.as_deref().unwrap_or(""),
                user_id,
            )
            .await?;

        let mut purchase_json = json!({
            "name": purchase_name,
            "price": displayed_price,
            "orig_price": purchase_info.orig_price.unwrap_or(0),
            "items": items
        });

        // Add discount information if applicable
        if purchase_info.discount_from.unwrap_or(-1) > 0
            && purchase_info.discount_to.unwrap_or(-1) > 0
        {
            purchase_json["discount_from"] = json!(purchase_info.discount_from);
            purchase_json["discount_to"] = json!(purchase_info.discount_to);

            let default_reason = String::new();
            let discount_reason = purchase_info
                .discount_reason
                .as_ref()
                .unwrap_or(&default_reason);
            if !discount_reason.is_empty()
                && (discount_reason == "anni5tix" || discount_reason == "pick_ticket")
                && displayed_price == 0
            {
                purchase_json["discount_reason"] = json!(discount_reason);
            }
        }

        Ok(purchase_json)
    }

    /// Calculate displayed price considering discounts
    async fn calculate_displayed_price(
        &self,
        price: i32,
        orig_price: i32,
        discount_from: i64,
        discount_to: i64,
        discount_reason: &str,
        user_id: i32,
    ) -> ArcResult<i32> {
        if discount_from > 0 && discount_to > 0 {
            let now = Self::current_timestamp();
            if discount_from <= now && now <= discount_to {
                match discount_reason {
                    "anni5tix" => {
                        let amount = self
                            .item_service
                            .get_user_positive_item_amount(user_id, "anni5tix", "anni5tix")
                            .await?;
                        if amount >= 1 {
                            return Ok(0);
                        }
                    }
                    "pick_ticket" => {
                        let amount = self
                            .item_service
                            .get_user_positive_item_amount(user_id, "pick_ticket", "pick_ticket")
                            .await?;
                        if amount >= 1 {
                            return Ok(0);
                        }
                    }
                    _ => {}
                }
                return Ok(price);
            }
        }
        Ok(orig_price)
    }

    /// Buy pack or single item
    ///
    /// Handles the purchase of packs or singles, checking user tickets and granting items.
    pub async fn buy_pack_or_single(&self, user_id: i32, purchase_name: &str) -> ArcResult<Value> {
        // Get purchase information
        let purchase_info = sqlx::query!(
            "SELECT * FROM purchase WHERE purchase_name = ?",
            purchase_name
        )
        .fetch_optional(&self.pool)
        .await?;

        let purchase_info = purchase_info.ok_or_else(|| {
            ArcError::no_data(format!("Purchase `{purchase_name}` does not exist."), 501)
        })?;

        // Get purchase items
        let purchase_items = sqlx::query!(
            "SELECT item_id, type as item_type, amount FROM purchase_item WHERE purchase_name = ?",
            purchase_name
        )
        .fetch_all(&self.pool)
        .await?;

        if purchase_items.is_empty() {
            return Err(ArcError::no_data(
                format!("The items of the purchase `{purchase_name}` do not exist."),
                501,
            ));
        }

        // Get user's current tickets
        let user_tickets = sqlx::query!("SELECT ticket FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&self.pool)
            .await?;

        let current_tickets = user_tickets
            .and_then(|row| row.ticket)
            .ok_or_else(|| ArcError::no_data("User not found.", 108))?;

        // Calculate actual price to pay
        let price_to_pay = self
            .calculate_displayed_price(
                purchase_info.price.unwrap_or(0),
                purchase_info.orig_price.unwrap_or(0),
                purchase_info.discount_from.unwrap_or(-1),
                purchase_info.discount_to.unwrap_or(-1),
                purchase_info.discount_reason.as_deref().unwrap_or(""),
                user_id,
            )
            .await?;

        // Check if user has enough tickets
        if current_tickets < price_to_pay {
            return Err(ArcError::ticket_not_enough(
                "The user does not have enough memories.",
                -6,
            ));
        }

        // Handle payment
        if !(purchase_info.orig_price.unwrap_or(0) == 0
            || (purchase_info.price.unwrap_or(0) == 0
                && purchase_info.discount_from.unwrap_or(-1) <= Self::current_timestamp()
                && Self::current_timestamp() <= purchase_info.discount_to.unwrap_or(-1)))
        {
            if price_to_pay == 0 {
                // Use special ticket
                let discount_reason = purchase_info.discount_reason.unwrap_or_default();
                if discount_reason == "anni5tix" || discount_reason == "pick_ticket" {
                    self.item_service
                        .claim_positive_item(user_id, &discount_reason, &discount_reason, -1)
                        .await?;
                }
            } else {
                // Deduct tickets
                sqlx::query!(
                    "UPDATE user SET ticket = ticket - ? WHERE user_id = ?",
                    price_to_pay,
                    user_id
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Grant all items to user
        for item in purchase_items {
            let item_id = item.item_id;
            let item_type = item.item_type;
            let amount = item.amount.unwrap_or(1);

            self.item_service
                .claim_item(user_id, &item_id, &item_type, amount)
                .await?;
        }

        // Get updated user info
        let user_info = self.user_service.get_user_info(user_id).await?;

        Ok(json!({
            "user_id": user_id,
            "ticket": user_info.ticket,
            "packs": user_info.packs,
            "singles": user_info.singles,
            "characters": user_info.characters
        }))
    }

    /// Buy special item (world mode boost and stamina)
    ///
    /// Special purchases for world mode boost and stamina items with fixed 50 ticket price.
    pub async fn buy_special_item(&self, user_id: i32, item_id: &str) -> ArcResult<Value> {
        let fixed_price = 50;

        // Get user's current tickets
        let user_tickets = sqlx::query!("SELECT ticket FROM user WHERE user_id = ?", user_id)
            .fetch_optional(&self.pool)
            .await?;

        let current_tickets = user_tickets
            .and_then(|row| row.ticket)
            .ok_or_else(|| ArcError::no_data("User not found.", 108))?;

        // Check if user has enough tickets
        if current_tickets < fixed_price {
            return Err(ArcError::ticket_not_enough(
                "The user does not have enough memories.",
                -6,
            ));
        }

        // Deduct tickets
        sqlx::query!(
            "UPDATE user SET ticket = ticket - ? WHERE user_id = ?",
            fixed_price,
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Claim the item
        self.item_service
            .claim_item(user_id, item_id, item_id, 1)
            .await?;

        // Prepare response
        let mut response = json!({
            "user_id": user_id,
            "ticket": current_tickets - fixed_price
        });

        // Add stamina info if it's stamina6
        if item_id == "stamina6" {
            let stamina_info = sqlx::query!(
                "SELECT stamina, max_stamina_ts FROM user WHERE user_id = ?",
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;

            if let Some(stamina) = stamina_info {
                response["stamina"] = json!(stamina.stamina);
                response["max_stamina_ts"] = json!(stamina.max_stamina_ts);
                response["world_mode_locked_end_ts"] = json!(-1);
            }
        }

        Ok(response)
    }

    /// Purchase stamina using fragments
    ///
    /// Allows users to purchase stamina using fragments once per day.
    pub async fn purchase_stamina_with_fragment(&self, user_id: i32) -> ArcResult<Value> {
        // Check fragment stamina cooldown
        let user_fragstam = sqlx::query!(
            "SELECT next_fragstam_ts FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let next_fragstam_ts = user_fragstam
            .and_then(|row| row.next_fragstam_ts)
            .unwrap_or(-1);

        let now = Self::current_timestamp();
        if next_fragstam_ts > now {
            return Err(ArcError::item_unavailable(
                "Buying stamina by fragment is not available yet.",
                905,
            ));
        }

        // Update next fragment stamina timestamp
        let next_ts = now + Constants::FRAGSTAM_RECOVER_TICK;
        sqlx::query!(
            "UPDATE user SET next_fragstam_ts = ? WHERE user_id = ?",
            next_ts,
            user_id
        )
        .execute(&self.pool)
        .await?;

        // Claim stamina6 item
        self.item_service
            .claim_item(user_id, "stamina6", "stamina6", 1)
            .await?;

        // Get updated user stamina info
        let stamina_info = sqlx::query!(
            "SELECT stamina, max_stamina_ts FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match stamina_info {
            Some(info) => Ok(json!({
                "user_id": user_id,
                "stamina": info.stamina,
                "max_stamina_ts": info.max_stamina_ts,
                "next_fragstam_ts": next_ts,
                "world_mode_locked_end_ts": -1
            })),
            None => Err(ArcError::no_data("User not found.", 108)),
        }
    }

    /// Redeem code
    ///
    /// Allows users to redeem codes for various rewards.
    pub async fn redeem_code(&self, user_id: i32, code: &str) -> ArcResult<Value> {
        // Check if code exists and is valid
        let redeem_info = sqlx::query!(
            "SELECT type as redeem_type FROM redeem WHERE code = ?",
            code
        )
        .fetch_optional(&self.pool)
        .await?;

        let _redeem_type = redeem_info
            .and_then(|row| row.redeem_type)
            .ok_or_else(|| ArcError::no_data("Invalid redeem code.", 502))?;

        // Check if user has already redeemed this code
        let already_redeemed = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM user_redeem WHERE user_id = ? AND code = ?) as `exists`",
            user_id,
            code
        )
        .fetch_one(&self.pool)
        .await?;

        if already_redeemed.exists != 0 {
            return Err(ArcError::data_exist("Code already redeemed.", 503, -1));
        }

        // Get redeem items
        let redeem_items = sqlx::query!(
            "SELECT item_id, type as item_type, amount FROM redeem_item WHERE code = ?",
            code
        )
        .fetch_all(&self.pool)
        .await?;

        let mut fragment_amount = 0;

        // Grant all redeem items
        for item in redeem_items {
            let item_id = item.item_id;
            let item_type = item.item_type;
            let amount = item.amount;

            // Track fragment amount for response
            if item_type == "fragment" {
                fragment_amount += amount.unwrap_or(0);
            }

            self.item_service
                .claim_item(user_id, &item_id, &item_type, amount.unwrap_or(0))
                .await?;
        }

        // Mark code as redeemed by user
        sqlx::query!(
            "INSERT INTO user_redeem (user_id, code) VALUES (?, ?)",
            user_id,
            code
        )
        .execute(&self.pool)
        .await?;

        // Return response with fragment info
        let coupon = if fragment_amount > 0 {
            format!("fragment{fragment_amount}")
        } else {
            String::new()
        };

        Ok(json!({
            "coupon": coupon
        }))
    }
}
