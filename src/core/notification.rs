use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::error::ArcResult;

// Global in-memory database pool for notifications
lazy_static::lazy_static! {
    static ref NOTIFICATION_POOL: Arc<Mutex<Option<SqlitePool>>> = Arc::new(Mutex::new(None));
}

pub async fn init_notification_db() -> ArcResult<()> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;

    // Create notification table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notification (
            user_id INTEGER,
            id INTEGER,
            type TEXT,
            content TEXT,
            sender_user_id INTEGER,
            sender_name TEXT,
            timestamp INTEGER,
            PRIMARY KEY (user_id, id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    let mut notification_pool = NOTIFICATION_POOL.lock().await;
    *notification_pool = Some(pool);
    Ok(())
}

pub async fn get_notification_pool() -> ArcResult<SqlitePool> {
    let pool_guard = NOTIFICATION_POOL.lock().await;
    match &*pool_guard {
        Some(pool) => Ok(pool.clone()),
        None => {
            drop(pool_guard);
            init_notification_db().await?;
            let pool_guard = NOTIFICATION_POOL.lock().await;
            Ok(pool_guard.as_ref().unwrap().clone())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseNotification {
    pub receiver_id: i32,
    pub sender_id: i32,
    pub sender_name: String,
    pub timestamp: i64,
    pub content: String,
    pub notification_type: String,
}

impl BaseNotification {
    pub fn is_expired(&self, expire_time: i64) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        now - self.timestamp > expire_time
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInviteNotification {
    #[serde(flatten)]
    pub base: BaseNotification,
}

impl RoomInviteNotification {
    pub fn new(sender_id: i32, sender_name: String, receiver_id: i32, share_token: String) -> Self {
        Self {
            base: BaseNotification {
                receiver_id,
                sender_id,
                sender_name,
                timestamp: chrono::Utc::now().timestamp_millis(),
                content: share_token,
                notification_type: "room_inv".to_string(),
            },
        }
    }

    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "sender": self.base.sender_name,
            "type": self.base.notification_type,
            "shareToken": self.base.content,
            "sendTs": self.base.timestamp
        })
    }
}

pub struct NotificationFactory {
    pool: SqlitePool,
    user_id: i32,
}

impl NotificationFactory {
    pub async fn new(user_id: i32) -> ArcResult<Self> {
        let pool = get_notification_pool().await?;
        Ok(Self { pool, user_id })
    }

    pub async fn get_notifications(&self) -> ArcResult<Vec<serde_json::Value>> {
        let mut notifications = Vec::new();

        // Get notifications for the user
        let rows = sqlx::query(
            "SELECT type, content, sender_user_id, sender_name, timestamp FROM notification WHERE user_id = ?"
        )
        .bind(self.user_id)
        .fetch_all(&self.pool)
        .await?;

        let expire_time = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

        for row in rows {
            let notification_type: String = row.get("type");
            let content: String = row.get("content");
            let sender_id: i32 = row.get("sender_user_id");
            let sender_name: String = row.get("sender_name");
            let timestamp: i64 = row.get("timestamp");

            let base_notification = BaseNotification {
                receiver_id: self.user_id,
                sender_id,
                sender_name: sender_name.clone(),
                timestamp,
                content: content.clone(),
                notification_type: notification_type.clone(),
            };

            if !base_notification.is_expired(expire_time) {
                match notification_type.as_str() {
                    "room_inv" => {
                        let room_invite = RoomInviteNotification {
                            base: base_notification,
                        };
                        notifications.push(room_invite.to_dict());
                    }
                    _ => {
                        // Handle other notification types or ignore unknown types
                    }
                }
            }
        }

        // Clean up notifications after reading
        self.cleanup_notifications().await?;

        Ok(notifications)
    }

    pub async fn insert_room_invite_notification(
        &self,
        sender_id: i32,
        sender_name: String,
        share_token: String,
        mp_notification_enabled: bool,
    ) -> ArcResult<()> {
        if !mp_notification_enabled {
            return Ok(());
        }

        // Get the next ID for this user
        let next_id = self.get_next_notification_id().await?;

        let timestamp = chrono::Utc::now().timestamp_millis();

        sqlx::query(
            "INSERT INTO notification (user_id, id, type, content, sender_user_id, sender_name, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(self.user_id)
        .bind(next_id)
        .bind("room_inv")
        .bind(share_token)
        .bind(sender_id)
        .bind(sender_name)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_next_notification_id(&self) -> ArcResult<i32> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(id), -1) + 1 as next_id FROM notification WHERE user_id = ?",
        )
        .bind(self.user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("next_id"))
    }

    async fn cleanup_notifications(&self) -> ArcResult<()> {
        sqlx::query("DELETE FROM notification WHERE user_id = ?")
            .bind(self.user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub notifications: Vec<serde_json::Value>,
}

pub async fn check_user_mp_notification_enabled(
    pool: &SqlitePool,
    user_id: i32,
) -> ArcResult<bool> {
    let row = sqlx::query("SELECT mp_notification_enabled FROM user WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => {
            let enabled: i32 = row.get("mp_notification_enabled");
            Ok(enabled != 0)
        }
        None => Ok(false),
    }
}
