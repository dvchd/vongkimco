#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub is_admin: i64,
    #[serde(default)]
    pub is_member: i64,
    pub google_sub: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn is_admin_bool(&self) -> bool {
        self.is_admin != 0
    }

    /// Admins are implicitly members for permission purposes.
    pub fn is_member_bool(&self) -> bool {
        self.is_admin != 0 || self.is_member != 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AllowedMember {
    pub email: String,
    pub note: Option<String>,
    pub added_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeedbackPost {
    pub id: String,
    pub user_id: String,
    pub category: String,
    pub title: Option<String>,
    pub body: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeedbackReply {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub body: String,
    pub is_admin_reply: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MembershipRequest {
    pub id: String,
    pub user_id: String,
    pub note: Option<String>,
    pub status: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeviceToken {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub device_name: Option<String>,
    pub platform: Option<String>,
    pub created_at: String,
    pub last_seen: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub device_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub note: Option<String>,
    pub client_session_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivitySample {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub sampled_at: String,
    pub state: String,
    pub idle_seconds: i64,
    pub keyboard_events: i64,
    pub mouse_events: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppSnapshot {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub sampled_at: String,
    pub foreground_app: Option<String>,
    pub foreground_title: Option<String>,
    pub apps_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Screenshot {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub captured_at: String,
    pub file_path: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub bytes: Option<i64>,
    pub mime: String,
}
