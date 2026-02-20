/// ActivityPub Inbox: POST /users/:username/inbox
/// Receives Follow, Create (Note), Undo, and Delete activities from remote servers.
use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::Config, error::AppError};

#[derive(sqlx::FromRow)]
struct LocalUser {
    #[allow(dead_code)]
    id: Uuid,
    actor_url: String,
    private_key: String,
}

pub async fn post_inbox(
    _req: HttpRequest,
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    http: web::Data<reqwest::Client>,
    path: web::Path<String>,
    body: web::Json<Value>,
) -> Result<HttpResponse, AppError> {
    let username = path.into_inner();

    // Verify target user exists
    let user: LocalUser = sqlx::query_as(
        "SELECT id, actor_url, private_key FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or(AppError::NotFound)?;

    let activity_id = body.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Deduplicate
    if !activity_id.is_empty() {
        let seen: Option<String> =
            sqlx::query_scalar("SELECT activity_id FROM processed_activities WHERE activity_id = $1")
                .bind(&activity_id)
                .fetch_optional(pool.get_ref())
                .await?;
        if seen.is_some() {
            return Ok(HttpResponse::Ok().finish());
        }
        sqlx::query("INSERT INTO processed_activities (activity_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(&activity_id)
            .execute(pool.get_ref())
            .await?;
    }

    let activity_type = body
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match activity_type.as_str() {
        "Follow" => handle_follow(&pool, &cfg, &http, &body, &user.actor_url, &user.private_key).await?,
        "Undo" => handle_undo(&pool, &body).await?,
        "Create" => handle_create(&pool, &cfg, &http, &body).await?,
        "Delete" => handle_delete(&pool, &body).await?,
        _ => {
            tracing::debug!("unhandled activity type: {activity_type}");
        }
    }

    Ok(HttpResponse::Accepted().finish())
}

async fn handle_follow(
    pool: &PgPool,
    _cfg: &Config,
    http: &reqwest::Client,
    body: &Value,
    followee_actor_url: &str,
    followee_private_key: &str,
) -> Result<(), AppError> {
    let follower_url = body
        .get("actor")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("missing actor".into()))?;

    // Fetch and cache remote actor
    let remote = fetch_remote_actor(http, follower_url, pool).await?;

    // Record follow
    sqlx::query(
        "INSERT INTO follows (id, follower_actor_url, followee_actor_url, accepted)
         VALUES ($1, $2, $3, TRUE)
         ON CONFLICT (follower_actor_url, followee_actor_url) DO UPDATE SET accepted = TRUE",
    )
    .bind(Uuid::new_v4())
    .bind(follower_url)
    .bind(followee_actor_url)
    .execute(pool)
    .await?;

    // Send Accept activity back
    let follow_id = body.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let accept = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{followee_actor_url}#accept/{}", Uuid::new_v4()),
        "type": "Accept",
        "actor": followee_actor_url,
        "object": follow_id,
    });

    let key_id = format!("{followee_actor_url}#main-key");
    crate::federation::deliver::post_to_inbox(
        http,
        &remote.inbox_url,
        &accept,
        &key_id,
        followee_private_key,
    )
    .await?;

    Ok(())
}

async fn handle_undo(pool: &PgPool, body: &Value) -> Result<(), AppError> {
    let object = body.get("object").unwrap_or(&Value::Null);
    let obj_type = object.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if obj_type == "Follow" {
        let follower = body.get("actor").and_then(|v| v.as_str()).unwrap_or("");
        let followee = object.get("object").and_then(|v| v.as_str()).unwrap_or("");
        sqlx::query(
            "DELETE FROM follows WHERE follower_actor_url = $1 AND followee_actor_url = $2",
        )
        .bind(follower)
        .bind(followee)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn handle_create(
    pool: &PgPool,
    _cfg: &Config,
    http: &reqwest::Client,
    body: &Value,
) -> Result<(), AppError> {
    let object = body.get("object").unwrap_or(&Value::Null);
    let obj_type = object.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if obj_type != "Note" {
        return Ok(());
    }

    let ap_id = object.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let actor_url = body.get("actor").and_then(|v| v.as_str()).unwrap_or("");
    let page_url = object.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let body_text = object.get("content").and_then(|v| v.as_str()).unwrap_or("");

    if page_url.is_empty() || body_text.is_empty() {
        return Ok(());
    }

    // Extract selector from tags
    let selector = object
        .get("tag")
        .and_then(|t| t.as_array())
        .and_then(|tags| {
            tags.iter().find(|t| {
                t.get("name").and_then(|n| n.as_str()) == Some("selector")
            })
        })
        .and_then(|t| t.get("href"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Cache remote actor
    fetch_remote_actor(http, actor_url, pool).await.ok();

    sqlx::query(
        "INSERT INTO notes (id, ap_id, remote_author_url, page_url, selector, body)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (ap_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(ap_id)
    .bind(actor_url)
    .bind(page_url)
    .bind(selector)
    .bind(body_text)
    .execute(pool)
    .await?;

    Ok(())
}

async fn handle_delete(pool: &PgPool, body: &Value) -> Result<(), AppError> {
    let object_id = body
        .get("object")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !object_id.is_empty() {
        sqlx::query("DELETE FROM notes WHERE ap_id = $1")
            .bind(object_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn fetch_remote_actor(
    http: &reqwest::Client,
    actor_url: &str,
    pool: &PgPool,
) -> Result<crate::models::user::RemoteActor, AppError> {
    // Return from cache if recent
    let cached: Option<crate::models::user::RemoteActor> = sqlx::query_as(
        "SELECT * FROM remote_actors WHERE actor_url = $1",
    )
    .bind(actor_url)
    .fetch_optional(pool)
    .await?;

    if let Some(ra) = cached {
        return Ok(ra);
    }

    let resp = http
        .get(actor_url)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("fetch actor: {e}")))?;

    let actor: Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("parse actor: {e}")))?;

    let inbox_url = actor
        .get("inbox")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let username = actor
        .get("preferredUsername")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let pub_key = actor
        .get("publicKey")
        .and_then(|k| k.get("publicKeyPem"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let host = url::Url::parse(actor_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();

    sqlx::query(
        "INSERT INTO remote_actors (actor_url, username, host, public_key, inbox_url)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (actor_url) DO UPDATE
         SET username = $2, host = $3, public_key = $4, inbox_url = $5, fetched_at = NOW()",
    )
    .bind(actor_url)
    .bind(&username)
    .bind(&host)
    .bind(&pub_key)
    .bind(&inbox_url)
    .execute(pool)
    .await?;

    Ok(crate::models::user::RemoteActor {
        actor_url: actor_url.to_string(),
        username,
        host,
        public_key: pub_key,
        inbox_url,
        fetched_at: chrono::Utc::now(),
    })
}
