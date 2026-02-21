/// Authentication routes: register and login.
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::{hash_password, issue_jwt, verify_password},
    config::Config,
    error::AppError,
    federation::http_sig::generate_key_pair,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub actor_url: String,
}

pub async fn register(
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    // reject empty credentials
    if body.username.trim().is_empty() || body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "username required and password must be at least 8 characters".into(),
        ));
    }
    // reject usernames with invalid chars
    if !body.username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest(
            "username may only contain letters, digits, underscores, and hyphens".into(),
        ));
    }

    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&body.username)
        .fetch_optional(pool.get_ref())
        .await?;
    if existing.is_some() {
        return Err(AppError::BadRequest("username already taken".into()));
    }

    let hash = hash_password(&body.password)?;
    let (priv_pem, pub_pem) = generate_key_pair()?;
    let actor_url = format!("{}/users/{}", cfg.instance_url, body.username);

    let id = Uuid::new_v4();
    // first registered user is automatically admin
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool.get_ref())
        .await?;
    let is_admin = count == 0;

    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, public_key, private_key, actor_url, is_admin)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(&body.username)
    .bind(&hash)
    .bind(&body.display_name)
    .bind(&pub_pem)
    .bind(&priv_pem)
    .bind(&actor_url)
    .bind(is_admin)
    .execute(pool.get_ref())
    .await?;

    let token = issue_jwt(id, &cfg.jwt_secret)?;
    Ok(HttpResponse::Created().json(AuthResponse {
        token,
        user_id: id.to_string(),
        username: body.username.clone(),
        actor_url,
    }))
}

pub async fn login(
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let row = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, username, password_hash FROM users WHERE username = $1",
    )
    .bind(&body.username)
    .fetch_optional(pool.get_ref())
    .await?;

    let (id, username, hash) = row.ok_or(AppError::Unauthorized)?;
    if !verify_password(&body.password, &hash)? {
        return Err(AppError::Unauthorized);
    }

    let actor_url = format!("{}/users/{}", cfg.instance_url, username);
    let token = issue_jwt(id, &cfg.jwt_secret)?;
    Ok(HttpResponse::Ok().json(AuthResponse {
        token,
        user_id: id.to_string(),
        username,
        actor_url,
    }))
}
