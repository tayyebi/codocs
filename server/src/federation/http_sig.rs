/// HTTP Signatures (draft-cavage-http-signatures)
/// Signs and verifies HTTP requests for ActivityPub federation.
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    signature::{RandomizedSigner, SignatureEncoding},
    RsaPrivateKey, RsaPublicKey,
};
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use sha2::Sha256;
use rsa::signature::Verifier;

use crate::error::AppError;

/// Generate an RSA-2048 key pair, returning (private_pem, public_pem).
pub fn generate_key_pair() -> Result<(String, String), AppError> {
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|e| AppError::Internal(format!("rsa key gen: {e}")))?;
    let public_key = RsaPublicKey::from(&private_key);

    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
    let priv_pem = private_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("private key pem: {e}")))?
        .to_string();
    let pub_pem = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("public key pem: {e}")))?;

    Ok((priv_pem, pub_pem))
}

/// Build the Signature header value for a POST request.
pub fn sign_request(
    private_key_pem: &str,
    key_id: &str,
    host: &str,
    path: &str,
    body_digest: &str,
    date: &str,
) -> Result<String, AppError> {
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .map_err(|e| AppError::Internal(format!("parse private key: {e}")))?;
    let signing_key = SigningKey::<Sha256>::new_unprefixed(private_key);

    let signed_string = format!(
        "(request-target): post {path}\nhost: {host}\ndate: {date}\ndigest: {body_digest}"
    );

    let mut rng = rand::thread_rng();
    let signature = signing_key
        .sign_with_rng(&mut rng, signed_string.as_bytes());
    let sig_b64 = B64.encode(signature.to_bytes());

    Ok(format!(
        r#"keyId="{key_id}",algorithm="rsa-sha256",headers="(request-target) host date digest",signature="{sig_b64}""#
    ))
}

/// Verify an HTTP Signature on an incoming request.
#[allow(dead_code)]
pub fn verify_signature(
    public_key_pem: &str,
    signature_header: &str,
    method: &str,
    path: &str,
    host: &str,
    date: &str,
    body_digest: &str,
) -> Result<bool, AppError> {
    // parse signature header fields
    let sig_b64 = extract_param(signature_header, "signature")
        .ok_or_else(|| AppError::BadRequest("missing signature".into()))?;
    let headers_str = extract_param(signature_header, "headers")
        .unwrap_or_else(|| "date".to_string());

    let sig_bytes = B64
        .decode(&sig_b64)
        .map_err(|e| AppError::BadRequest(format!("bad signature base64: {e}")))?;

    let signed_string = build_signed_string(&headers_str, method, path, host, date, body_digest);

    let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
        .map_err(|e| AppError::Internal(format!("parse public key: {e}")))?;
    let verifying_key = VerifyingKey::<Sha256>::new_unprefixed(public_key);
    let signature = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| AppError::BadRequest(format!("invalid signature bytes: {e}")))?;

    Ok(verifying_key
        .verify(signed_string.as_bytes(), &signature)
        .is_ok())
}

#[allow(dead_code)]
fn build_signed_string(
    headers: &str,
    method: &str,
    path: &str,
    host: &str,
    date: &str,
    body_digest: &str,
) -> String {
    let mut parts = Vec::new();
    for h in headers.split_whitespace() {
        let line = match h {
            "(request-target)" => {
                format!("(request-target): {} {}", method.to_lowercase(), path)
            }
            "host" => format!("host: {host}"),
            "date" => format!("date: {date}"),
            "digest" => format!("digest: {body_digest}"),
            _ => continue,
        };
        parts.push(line);
    }
    parts.join("\n")
}

#[allow(dead_code)]
fn extract_param(header: &str, key: &str) -> Option<String> {
    for part in header.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(&format!("{key}=")) {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

pub fn http_date_now() -> String {
    Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}
