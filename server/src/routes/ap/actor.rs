/// ActivityPub Actor endpoint: GET /users/:username
use actix_web::{web, HttpResponse};
use serde_json::json;
use sqlx::PgPool;

use crate::{config::Config, error::AppError};

#[derive(sqlx::FromRow)]
struct UserRow {
    username: String,
    display_name: Option<String>,
    public_key: String,
    actor_url: String,
}

pub async fn get_actor(
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let username = path.into_inner();

    let user: UserRow = sqlx::query_as(
        "SELECT username, display_name, public_key, actor_url FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or(AppError::NotFound)?;

    let actor = json!({
        "@context": [
            "https://www.w3.org/ns/activitystreams",
            "https://w3id.org/security/v1"
        ],
        "id": user.actor_url,
        "type": "Person",
        "preferredUsername": user.username,
        "name": user.display_name.unwrap_or_else(|| user.username.clone()),
        "inbox": format!("{}/users/{}/inbox", cfg.instance_url, user.username),
        "outbox": format!("{}/users/{}/outbox", cfg.instance_url, user.username),
        "followers": format!("{}/users/{}/followers", cfg.instance_url, user.username),
        "following": format!("{}/users/{}/following", cfg.instance_url, user.username),
        "publicKey": {
            "id": format!("{}#main-key", user.actor_url),
            "owner": user.actor_url,
            "publicKeyPem": user.public_key,
        }
    });

    Ok(HttpResponse::Ok()
        .content_type("application/activity+json")
        .json(actor))
}
