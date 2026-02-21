/// Notes API: create, list, and delete sticky notes.
/// Notes are tied to a page URL + optional CSS selector.
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::verify_jwt,
    config::Config,
    error::AppError,
    federation::deliver::post_to_inbox,
    models::note::{Note, NoteView},
};

fn extract_bearer(req: &HttpRequest, cfg: &Config) -> Result<Uuid, AppError> {
    let auth = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let token = auth.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    verify_jwt(token, &cfg.jwt_secret)
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub page_url: String,
    pub selector: Option<String>,
    pub body: String,
}

#[derive(Deserialize)]
pub struct ListNotesQuery {
    pub url: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    #[allow(dead_code)]
    username: String,
    actor_url: String,
    private_key: String,
}

pub async fn create_note(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    http: web::Data<reqwest::Client>,
    body: web::Json<CreateNoteRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = extract_bearer(&req, &cfg)?;

    if body.body.trim().is_empty() {
        return Err(AppError::BadRequest("note body must not be empty".into()));
    }

    let note_id = Uuid::new_v4();
    let ap_id = format!("{}/notes/{}", cfg.instance_url, note_id);

    sqlx::query(
        "INSERT INTO notes (id, ap_id, author_id, page_url, selector, body)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(note_id)
    .bind(&ap_id)
    .bind(user_id)
    .bind(&body.page_url)
    .bind(&body.selector)
    .bind(&body.body)
    .execute(pool.get_ref())
    .await?;

    // Federate: deliver Create activity to followers' inboxes
    let user: UserRow = sqlx::query_as(
        "SELECT username, actor_url, private_key FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let followers: Vec<(String,)> = sqlx::query_as(
        "SELECT ra.inbox_url FROM follows f
         JOIN remote_actors ra ON ra.actor_url = f.follower_actor_url
         WHERE f.followee_actor_url = $1 AND f.accepted = TRUE",
    )
    .bind(&user.actor_url)
    .fetch_all(pool.get_ref())
    .await?;

    if !followers.is_empty() {
        let activity = build_create_activity(
            &user.actor_url,
            &ap_id,
            &body.page_url,
            body.selector.as_deref(),
            &body.body,
        );
        let key_id = format!("{}#main-key", user.actor_url);
        for (inbox_url,) in &followers {
            if let Err(e) = post_to_inbox(&http, inbox_url, &activity, &key_id, &user.private_key).await {
                tracing::warn!("failed to deliver to {inbox_url}: {e}");
            }
        }
    }

    Ok(HttpResponse::Created().json(json!({"id": note_id, "ap_id": ap_id})))
}

pub async fn list_notes(
    pool: web::Data<PgPool>,
    query: web::Query<ListNotesQuery>,
) -> Result<HttpResponse, AppError> {
    let notes: Vec<Note> = sqlx::query_as(
        "SELECT * FROM notes WHERE page_url = $1 ORDER BY created_at DESC",
    )
    .bind(&query.url)
    .fetch_all(pool.get_ref())
    .await?;

    let mut views: Vec<NoteView> = Vec::with_capacity(notes.len());
    for note in notes {
        let (author, author_url, is_local) = if let Some(uid) = note.author_id {
            let u: Option<(String, String)> = sqlx::query_as(
                "SELECT username, actor_url FROM users WHERE id = $1",
            )
            .bind(uid)
            .fetch_optional(pool.get_ref())
            .await?;
            match u {
                Some((username, actor_url)) => (username, actor_url, true),
                None => ("unknown".into(), String::new(), true),
            }
        } else {
            let remote_url = note.remote_author_url.clone().unwrap_or_default();
            let remote: Option<crate::models::user::RemoteActor> = sqlx::query_as(
                "SELECT * FROM remote_actors WHERE actor_url = $1",
            )
            .bind(&remote_url)
            .fetch_optional(pool.get_ref())
            .await?;
            let author = remote
                .as_ref()
                .map(|r| format!("{}@{}", r.username, r.host))
                .unwrap_or_else(|| remote_url.clone());
            (author, remote_url, false)
        };
        views.push(NoteView {
            id: note.id,
            author,
            author_url,
            page_url: note.page_url,
            selector: note.selector,
            body: note.body,
            created_at: note.created_at,
            is_local,
        });
    }

    Ok(HttpResponse::Ok().json(views))
}

pub async fn delete_note(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = extract_bearer(&req, &cfg)?;
    let note_id = path.into_inner();

    let note: Option<Note> = sqlx::query_as("SELECT * FROM notes WHERE id = $1")
        .bind(note_id)
        .fetch_optional(pool.get_ref())
        .await?;

    let note = note.ok_or(AppError::NotFound)?;

    let is_admin: bool =
        sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool.get_ref())
            .await?;

    if note.author_id != Some(user_id) && !is_admin {
        return Err(AppError::Forbidden);
    }

    sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(note_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

fn build_create_activity(
    actor_url: &str,
    note_ap_id: &str,
    page_url: &str,
    selector: Option<&str>,
    body: &str,
) -> serde_json::Value {
    let mut note_obj = json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": note_ap_id,
        "type": "Note",
        "attributedTo": actor_url,
        "content": body,
        "url": page_url,
    });
    if let Some(sel) = selector {
        note_obj["tag"] = json!([{
            "type": "Tag",
            "name": "selector",
            "href": sel,
        }]);
    }
    json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{note_ap_id}/activity"),
        "type": "Create",
        "actor": actor_url,
        "object": note_obj,
        "to": ["https://www.w3.org/ns/activitystreams#Public"],
    })
}
