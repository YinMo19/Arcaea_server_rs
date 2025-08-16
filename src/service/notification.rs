use crate::error::ArcResult;
use crate::model::{NewNotification, Notification, NotificationResponse, RoomInviteNotification};
use sqlx::MySqlPool;

pub struct NotificationService {
    pool: MySqlPool,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get all notifications for a user and mark them as read (delete them)
    pub async fn get_user_notifications(
        &self,
        user_id: i32,
    ) -> ArcResult<Vec<NotificationResponse>> {
        // First check if user has notifications enabled
        let mp_notification_enabled = sqlx::query_scalar!(
            "SELECT mp_notification_enabled FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if mp_notification_enabled.unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }

        // Get all notifications for the user
        let notification_rows = sqlx::query!(
            "SELECT user_id, id, type, content, sender_user_id, sender_name, timestamp
             FROM notification WHERE user_id = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let notifications: Vec<Notification> = notification_rows
            .into_iter()
            .map(|row| Notification {
                user_id: row.user_id,
                id: row.id,
                notification_type: row.r#type.unwrap_or_default(),
                content: row.content.unwrap_or_default(),
                sender_user_id: row.sender_user_id.unwrap_or(0),
                sender_name: row.sender_name.unwrap_or_default(),
                timestamp: row.timestamp.unwrap_or(0),
            })
            .collect();

        let mut responses = Vec::new();
        let current_time = chrono::Utc::now().timestamp_millis();

        // Filter out expired notifications and convert to responses
        for notification in notifications {
            if !self.is_notification_expired(notification.timestamp, current_time) {
                responses.push(NotificationResponse::from(notification));
            }
        }

        // Delete all notifications for the user after retrieval
        sqlx::query!("DELETE FROM notification WHERE user_id = ?", user_id)
            .execute(&self.pool)
            .await?;

        Ok(responses)
    }

    /// Insert a new room invite notification
    pub async fn insert_room_invite_notification(
        &self,
        notification: &RoomInviteNotification,
    ) -> ArcResult<()> {
        // Check if receiver has notifications enabled
        let mp_notification_enabled = sqlx::query_scalar!(
            "SELECT mp_notification_enabled FROM user WHERE user_id = ?",
            notification.receiver_id
        )
        .fetch_one(&self.pool)
        .await?;

        if mp_notification_enabled.unwrap_or(0) == 0 {
            return Ok(());
        }

        // Get the next notification ID for this user
        let max_id = sqlx::query_scalar!(
            "SELECT MAX(id) FROM notification WHERE user_id = ?",
            notification.receiver_id
        )
        .fetch_one(&self.pool)
        .await?;

        let next_id = max_id.unwrap_or(-1) + 1;

        // Insert the notification
        sqlx::query!(
            "INSERT INTO notification (user_id, id, type, content, sender_user_id, sender_name, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            notification.receiver_id,
            next_id,
            "room_inv",
            notification.share_token,
            notification.sender_id,
            notification.sender_name,
            notification.timestamp
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a generic notification
    pub async fn insert_notification(&self, notification: &NewNotification) -> ArcResult<()> {
        // Check if receiver has notifications enabled
        let mp_notification_enabled = sqlx::query_scalar!(
            "SELECT mp_notification_enabled FROM user WHERE user_id = ?",
            notification.user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if mp_notification_enabled.unwrap_or(0) == 0 {
            return Ok(());
        }

        // Insert the notification
        sqlx::query!(
            "INSERT INTO notification (user_id, id, type, content, sender_user_id, sender_name, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            notification.user_id,
            notification.id,
            notification.notification_type,
            notification.content,
            notification.sender_user_id,
            notification.sender_name,
            notification.timestamp
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if a notification is expired
    /// Notifications expire after a certain time period (default: 7 days)
    fn is_notification_expired(&self, notification_timestamp: i64, current_time: i64) -> bool {
        const NOTIFICATION_EXPIRE_TIME: i64 = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds
        current_time - notification_timestamp > NOTIFICATION_EXPIRE_TIME
    }

    /// Clean up expired notifications from the database
    pub async fn cleanup_expired_notifications(&self) -> ArcResult<u64> {
        let current_time = chrono::Utc::now().timestamp_millis();
        const NOTIFICATION_EXPIRE_TIME: i64 = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds
        let expire_threshold = current_time - NOTIFICATION_EXPIRE_TIME;

        let result = sqlx::query!(
            "DELETE FROM notification WHERE timestamp < ?",
            expire_threshold
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get notification count for a user (excluding expired ones)
    pub async fn get_notification_count(&self, user_id: i32) -> ArcResult<i64> {
        let current_time = chrono::Utc::now().timestamp_millis();
        const NOTIFICATION_EXPIRE_TIME: i64 = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds
        let expire_threshold = current_time - NOTIFICATION_EXPIRE_TIME;

        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM notification WHERE user_id = ? AND timestamp >= ?",
            user_id,
            expire_threshold
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Create a room invite notification and insert it
    pub async fn create_room_invite(
        &self,
        sender_id: i32,
        sender_name: String,
        receiver_id: i32,
        share_token: String,
    ) -> ArcResult<()> {
        let notification =
            RoomInviteNotification::new(sender_id, sender_name, receiver_id, share_token);
        self.insert_room_invite_notification(&notification).await
    }
}
