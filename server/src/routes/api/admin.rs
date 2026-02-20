use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::verify_jwt,
    config::Config,
    error::AppError,
};

async fn require_admin(req: &HttpRequest, pool: &PgPool, cfg: &Config) -> Result<Uuid, AppError> {
    let auth = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let token = auth.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    let user_id = verify_jwt(token, &cfg.jwt_secret)?;

    let is_admin: bool = sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    if !is_admin {
        return Err(AppError::Forbidden);
    }
    Ok(user_id)
}

#[derive(Serialize)]
pub struct UserSummary {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub actor_url: String,
    pub is_admin: bool,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    display_name: Option<String>,
    actor_url: String,
    is_admin: bool,
    created_at: DateTime<Utc>,
}

pub async fn list_users(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req, &pool, &cfg).await?;

    let users: Vec<UserRow> = sqlx::query_as(
        "SELECT id, username, display_name, actor_url, is_admin, created_at FROM users ORDER BY created_at",
    )
    .fetch_all(pool.get_ref())
    .await?;

    let out: Vec<UserSummary> = users
        .into_iter()
        .map(|u| UserSummary {
            id: u.id.to_string(),
            username: u.username,
            display_name: u.display_name,
            actor_url: u.actor_url,
            is_admin: u.is_admin,
            created_at: u.created_at.to_rfc3339(),
        })
        .collect();

    Ok(HttpResponse::Ok().json(out))
}

pub async fn delete_user(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let admin_id = require_admin(&req, &pool, &cfg).await?;
    let target_id = path.into_inner();

    if target_id == admin_id {
        return Err(AppError::BadRequest("cannot delete yourself".into()));
    }

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(target_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn promote_user(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req, &pool, &cfg).await?;
    let target_id = path.into_inner();

    sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1")
        .bind(target_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"ok": true})))
}
