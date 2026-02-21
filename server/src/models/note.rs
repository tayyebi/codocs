use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: Uuid,
    pub ap_id: Option<String>,
    pub author_id: Option<Uuid>,
    pub remote_author_url: Option<String>,
    pub page_url: String,
    pub selector: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// Enriched note view returned to extension clients
#[derive(Debug, Serialize)]
pub struct NoteView {
    pub id: Uuid,
    pub author: String,
    pub author_url: String,
    pub page_url: String,
    pub selector: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub is_local: bool,
}
