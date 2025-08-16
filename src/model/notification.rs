use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Base notification structure
#[derive(Debug, Clone, FromRow)]
pub struct Notification {
    pub user_id: i32,
    pub id: i32,
    pub notification_type: String,
    pub content: String,
    pub sender_user_id: i32,
    pub sender_name: String,
    pub timestamp: i64,
}

/// Response structure for notifications
#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub sender: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    #[serde(rename = "shareToken")]
    pub share_token: String,
    #[serde(rename = "sendTs")]
    pub send_ts: i64,
}

/// Room invite notification details
#[derive(Debug, Clone)]
pub struct RoomInviteNotification {
    pub sender_id: i32,
    pub sender_name: String,
    pub receiver_id: i32,
    pub share_token: String,
    pub timestamp: i64,
}

impl RoomInviteNotification {
    /// Create a new room invite notification
    pub fn new(sender_id: i32, sender_name: String, receiver_id: i32, share_token: String) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis();
        Self {
            sender_id,
            sender_name,
            receiver_id,
            share_token,
            timestamp,
        }
    }

    /// Convert to notification response format
    pub fn to_response(&self) -> NotificationResponse {
        NotificationResponse {
            sender: self.sender_name.clone(),
            notification_type: "room_inv".to_string(),
            share_token: self.share_token.clone(),
            send_ts: self.timestamp,
        }
    }
}

impl From<Notification> for NotificationResponse {
    fn from(notification: Notification) -> Self {
        match notification.notification_type.as_str() {
            "room_inv" => NotificationResponse {
                sender: notification.sender_name,
                notification_type: notification.notification_type,
                share_token: notification.content,
                send_ts: notification.timestamp,
            },
            _ => NotificationResponse {
                sender: notification.sender_name,
                notification_type: notification.notification_type,
                share_token: notification.content,
                send_ts: notification.timestamp,
            },
        }
    }
}

/// New notification for insertion
#[derive(Debug)]
pub struct NewNotification {
    pub user_id: i32,
    pub id: i32,
    pub notification_type: String,
    pub content: String,
    pub sender_user_id: i32,
    pub sender_name: String,
    pub timestamp: i64,
}

impl NewNotification {
    /// Create a new notification from room invite
    pub fn from_room_invite(notification: &RoomInviteNotification, id: i32) -> Self {
        Self {
            user_id: notification.receiver_id,
            id,
            notification_type: "room_inv".to_string(),
            content: notification.share_token.clone(),
            sender_user_id: notification.sender_id,
            sender_name: notification.sender_name.clone(),
            timestamp: notification.timestamp,
        }
    }
}
