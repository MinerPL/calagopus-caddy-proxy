//! Cloudflare DNS records for subdomains of an allowed domain.

use crate::domain::is_under_domain;
use axum::http::StatusCode;
use serde_json::Value;
use shared::response::DisplayError;
use std::time::Duration;

const BASE: &str = "https://api.cloudflare.com/client/v4";

fn client() -> Result<reqwest::Client, anyhow::Error> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?)
}

fn gateway(message: impl Into<String>) -> anyhow::Error {
    DisplayError::new(message.into())
        .with_status(StatusCode::BAD_GATEWAY)
        .into()
}

async fn cloudflare_error(response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let body: Value = response.json().await.unwrap_or_default();
    let messages = body["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .filter_map(|error| error["message"].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    gateway(if messages.is_empty() {
        format!("cloudflare request failed with status {status}")
    } else {
        format!("cloudflare: {messages}")
    })
}

pub async fn create_record(
    zone_id: &str,
    token: &str,
    record_type: &str,
    name: &str,
    content: &str,
    proxied: bool,
) -> Result<String, anyhow::Error> {
    let response = client()?
        .post(format!("{BASE}/zones/{zone_id}/dns_records"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "type": record_type,
            "name": name,
            "content": content,
            "ttl": 1,
            "proxied": proxied,
        }))
        .send()
        .await
        .map_err(|err| gateway(format!("cloudflare: {err}")))?;

    if !response.status().is_success() {
        return Err(cloudflare_error(response).await);
    }

    let body: Value = response
        .json()
        .await
        .map_err(|err| gateway(format!("cloudflare: {err}")))?;
    body["result"]["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| gateway("cloudflare create response did not include a record id"))
}

/// A missing record is already gone, which is what deletion wanted.
pub async fn delete_record(
    zone_id: &str,
    token: &str,
    record_id: &str,
) -> Result<(), anyhow::Error> {
    let response = client()?
        .delete(format!("{BASE}/zones/{zone_id}/dns_records/{record_id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|err| gateway(format!("cloudflare: {err}")))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND || response.status().is_success() {
        return Ok(());
    }
    Err(cloudflare_error(response).await)
}

/// Confirms the token can read the zone and that the zone covers `domain`.
pub async fn verify_zone(
    zone_id: &str,
    token: &str,
    domain: &str,
) -> Result<String, anyhow::Error> {
    let response = client()?
        .get(format!("{BASE}/zones/{zone_id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|err| gateway(format!("cloudflare: {err}")))?;

    if !response.status().is_success() {
        return Err(cloudflare_error(response).await);
    }

    let body: Value = response
        .json()
        .await
        .map_err(|err| gateway(format!("cloudflare: {err}")))?;
    let zone_name = body["result"]["name"]
        .as_str()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if zone_name.is_empty() || !is_under_domain(domain, &zone_name) {
        return Err(DisplayError::new(format!(
            "cloudflare zone `{zone_name}` does not cover `{domain}`"
        ))
        .into());
    }

    Ok(zone_name)
}
