use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use reqwest::Client;
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::config::ProviderConfig;

#[derive(Debug, Clone)]
pub enum SyncOutcome {
    NoChange {
        record_id: String,
        content: String,
    },
    Updated {
        record_id: String,
        old_content: String,
        new_content: String,
    },
    Created {
        record_id: String,
        new_content: String,
    },
}

pub async fn sync_record(
    client: &Client,
    provider: &ProviderConfig,
    domain: &str,
    record_type: &str,
    ip_address: &str,
) -> Result<SyncOutcome> {
    let records = list_dns_records(client, provider, domain, record_type).await?;

    if let Some(record) = records.into_iter().next() {
        if record.content == ip_address {
            return Ok(SyncOutcome::NoChange {
                record_id: record.id,
                content: record.content,
            });
        }

        let record_id = record.id.clone();
        let old_content = record.content.clone();
        update_dns_record(
            client,
            provider,
            &record_id,
            domain,
            record_type,
            ip_address,
        )
        .await?;
        return Ok(SyncOutcome::Updated {
            record_id,
            old_content,
            new_content: ip_address.to_string(),
        });
    }

    let created = create_dns_record(client, provider, domain, record_type, ip_address).await?;
    Ok(SyncOutcome::Created {
        record_id: created.id,
        new_content: created.content,
    })
}

async fn list_dns_records(
    client: &Client,
    provider: &ProviderConfig,
    domain: &str,
    record_type: &str,
) -> Result<Vec<CloudflareRecord>> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
        provider.zone_id
    );

    let response: ApiEnvelope<Vec<CloudflareRecord>> = client
        .get(url)
        .bearer_auth(&provider.api_token)
        .query(&[("name", domain), ("type", record_type), ("per_page", "100")])
        .send()
        .await
        .context("failed to query Cloudflare DNS records")?
        .error_for_status()
        .context("Cloudflare DNS record query returned an HTTP error")?
        .json()
        .await
        .context("failed to deserialize Cloudflare DNS record query response")?;

    if !response.success {
        return Err(anyhow!(
            "Cloudflare DNS record query failed: {:?}",
            response.errors
        ));
    }

    Ok(response.result)
}

async fn update_dns_record(
    client: &Client,
    provider: &ProviderConfig,
    record_id: &str,
    domain: &str,
    record_type: &str,
    ip_address: &str,
) -> Result<CloudflareRecord> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
        provider.zone_id, record_id
    );

    let payload = CloudflareRecordWrite {
        content: ip_address.to_string(),
        name: domain.to_string(),
        record_type: record_type.to_string(),
        proxied: provider.proxied,
        ttl: provider.ttl,
    };

    let response: ApiEnvelope<CloudflareRecord> = client
        .put(url)
        .bearer_auth(&provider.api_token)
        .json(&payload)
        .send()
        .await
        .context("failed to update Cloudflare DNS record")?
        .error_for_status()
        .context("Cloudflare DNS record update returned an HTTP error")?
        .json()
        .await
        .context("failed to deserialize Cloudflare DNS record update response")?;

    if !response.success {
        return Err(anyhow!(
            "Cloudflare DNS record update failed: {:?}",
            response.errors
        ));
    }

    info!(record_id = %record_id, domain = %domain, record_type = %record_type, new_ip = %ip_address, "updated DNS record");
    Ok(response.result)
}

async fn create_dns_record(
    client: &Client,
    provider: &ProviderConfig,
    domain: &str,
    record_type: &str,
    ip_address: &str,
) -> Result<CloudflareRecord> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
        provider.zone_id
    );

    let payload = CloudflareRecordWrite {
        content: ip_address.to_string(),
        name: domain.to_string(),
        record_type: record_type.to_string(),
        proxied: provider.proxied,
        ttl: provider.ttl,
    };

    let response: ApiEnvelope<CloudflareRecord> = client
        .post(url)
        .bearer_auth(&provider.api_token)
        .json(&payload)
        .send()
        .await
        .context("failed to create Cloudflare DNS record")?
        .error_for_status()
        .context("Cloudflare DNS record create returned an HTTP error")?
        .json()
        .await
        .context("failed to deserialize Cloudflare DNS record create response")?;

    if !response.success {
        return Err(anyhow!(
            "Cloudflare DNS record create failed: {:?}",
            response.errors
        ));
    }

    info!(record_id = %response.result.id, domain = %domain, record_type = %record_type, new_ip = %ip_address, "created DNS record");
    Ok(response.result)
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    result: T,
    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CloudflareRecord {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    proxied: Option<bool>,
    ttl: Option<u32>,
}

#[derive(Debug, Serialize)]
struct CloudflareRecordWrite {
    content: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    proxied: bool,
    ttl: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_cloudflare_record() {
        let raw = r#"{
            "id": "abc",
            "name": "a.example.com",
            "type": "A",
            "content": "1.2.3.4",
            "proxied": true,
            "ttl": 1
        }"#;

        let record: CloudflareRecord = serde_json::from_str(raw).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.content, "1.2.3.4");
    }
}
