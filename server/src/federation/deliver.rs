/// Deliver ActivityPub activities to remote inboxes via signed HTTP POST.
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    error::AppError,
    federation::http_sig::{http_date_now, sign_request},
};

/// POST an ActivityPub activity to a remote inbox.
pub async fn post_to_inbox(
    client: &Client,
    inbox_url: &str,
    activity: &Value,
    actor_key_id: &str,
    private_key_pem: &str,
) -> Result<(), AppError> {
    let body = serde_json::to_string(activity)
        .map_err(|e| AppError::Internal(format!("serialize activity: {e}")))?;

    let digest = {
        let hash = Sha256::digest(body.as_bytes());
        format!("SHA-256={}", B64.encode(hash))
    };

    let date = http_date_now();

    let parsed = url::Url::parse(inbox_url)
        .map_err(|e| AppError::Internal(format!("invalid inbox url: {e}")))?;
    let host = parsed.host_str().unwrap_or("").to_string();
    let port_str = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    let host_with_port = format!("{host}{port_str}");
    let path = parsed.path().to_string();

    let signature = sign_request(
        private_key_pem,
        actor_key_id,
        &host_with_port,
        &path,
        &digest,
        &date,
    )?;

    let resp = client
        .post(inbox_url)
        .header("Content-Type", "application/activity+json")
        .header("Accept", "application/activity+json")
        .header("Date", &date)
        .header("Digest", &digest)
        .header("Signature", &signature)
        .body(body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("http delivery error: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::warn!("inbox delivery failed {status}: {text}");
    }

    Ok(())
}
