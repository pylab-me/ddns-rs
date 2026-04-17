use std::net::IpAddr;

use anyhow::Context;
use anyhow::Result;
use reqwest::Client;
use tracing::debug;
use tracing::warn;

use crate::error::IpLookupError;

pub async fn resolve_public_ip(client: &Client, urls: &[String]) -> Result<IpAddr> {
    for url in urls {
        match try_resolve_public_ip(client, url).await {
            Ok(ip) => return Ok(ip),
            Err(error) => {
                warn!(source_url = %url, error = %error, "public IP source failed");
            }
        }
    }

    Err(IpLookupError::AllSourcesFailed.into())
}

async fn try_resolve_public_ip(client: &Client, url: &str) -> Result<IpAddr> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to query public IP source: {url}"))?
        .error_for_status()
        .with_context(|| format!("public IP source returned an HTTP error: {url}"))?;

    let body = response
        .text()
        .await
        .with_context(|| format!("failed to read response body from public IP source: {url}"))?;

    let trimmed = body.trim();
    let ip: IpAddr = trimmed
        .parse()
        .map_err(|_| IpLookupError::InvalidIpResponse {
            url: url.to_string(),
            body: trimmed.to_string(),
        })?;

    debug!(source_url = %url, ip = %ip, "resolved public IP");
    Ok(ip)
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;

    #[test]
    fn parse_ip_addr() {
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
    }
}
