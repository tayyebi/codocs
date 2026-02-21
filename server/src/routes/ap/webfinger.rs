/// WebFinger endpoint: GET /.well-known/webfinger
/// Resolves acct:user@host to the ActivityPub actor URL.
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::{config::Config, error::AppError};

#[derive(Deserialize)]
pub struct WebFingerQuery {
    pub resource: String,
}

pub async fn webfinger(
    pool: web::Data<PgPool>,
    cfg: web::Data<Config>,
    query: web::Query<WebFingerQuery>,
) -> Result<HttpResponse, AppError> {
    // resource = acct:username@domain
    let resource = &query.resource;
    let acct = resource
        .strip_prefix("acct:")
        .ok_or_else(|| AppError::BadRequest("resource must start with acct:".into()))?;

    let (username, host) = acct
        .split_once('@')
        .ok_or_else(|| AppError::BadRequest("invalid acct format".into()))?;

    if host != cfg.instance_domain
        && host != format!("{}:{}", cfg.instance_domain, cfg.port)
    {
        return Err(AppError::NotFound);
    }

    let actor_url = sqlx::query_scalar::<_, String>(
        "SELECT actor_url FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or(AppError::NotFound)?;

    let response = json!({
        "subject": resource,
        "links": [
            {
                "rel": "self",
                "type": "application/activity+json",
                "href": actor_url,
            }
        ]
    });

    Ok(HttpResponse::Ok()
        .content_type("application/jrd+json")
        .json(response))
}
