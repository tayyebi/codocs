/// ActivityPub Outbox: GET /users/:username/outbox
/// Returns recent Create activities for the user's notes.
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, error::AppError};

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    actor_url: String,
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: Uuid,
    ap_id: Option<String>,
    page_url: String,
    selector: Option<String>,
    body: String,
    created_at: DateTime<Utc>,
}

pub async fn get_outbox(
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let username = path.into_inner();

    let user: UserRow = sqlx::query_as(
        "SELECT id, actor_url FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or(AppError::NotFound)?;

    let notes: Vec<NoteRow> = sqlx::query_as(
        "SELECT id, ap_id, page_url, selector, body, created_at
         FROM notes WHERE author_id = $1 ORDER BY created_at DESC LIMIT 20",
    )
    .bind(user.id)
    .fetch_all(pool.get_ref())
    .await?;

    let items: Vec<serde_json::Value> = notes
        .into_iter()
        .map(|n| {
            let note_ap_id = n
                .ap_id
                .unwrap_or_else(|| format!("{}/notes/{}", cfg.instance_url, n.id));
            let mut note_obj = json!({
                "@context": "https://www.w3.org/ns/activitystreams",
                "id": note_ap_id,
                "type": "Note",
                "attributedTo": user.actor_url,
                "content": n.body,
                "url": n.page_url,
                "published": n.created_at.to_rfc3339(),
            });
            if let Some(sel) = n.selector {
                note_obj["tag"] = json!([{"type": "Tag", "name": "selector", "href": sel}]);
            }
            json!({
                "type": "Create",
                "id": format!("{note_ap_id}/activity"),
                "actor": user.actor_url,
                "object": note_obj,
            })
        })
        .collect();

    let outbox_url = format!("{}/users/{}/outbox", cfg.instance_url, username);
    let collection = json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": outbox_url,
        "type": "OrderedCollection",
        "totalItems": items.len(),
        "orderedItems": items,
    });

    Ok(HttpResponse::Ok()
        .content_type("application/activity+json")
        .json(collection))
}
