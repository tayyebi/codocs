use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Remote actor cached from ActivityPub federation.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RemoteActor {
    pub actor_url: String,
    pub username: String,
    pub host: String,
    pub public_key: String,
    pub inbox_url: String,
    pub fetched_at: DateTime<Utc>,
}

/// Local user account.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub public_key: String,
    #[serde(skip_serializing)]
    pub private_key: String,
    pub actor_url: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}
