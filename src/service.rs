use anyhow::Context;
use anyhow::Result;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::header::USER_AGENT;
use reqwest::Client;
use tokio::time::sleep;
use tracing::error;
use tracing::info;

use crate::config::AppConfig;
use crate::config::DomainConfig;
use crate::ip_source::resolve_public_ip;
use crate::provider::cloudflare::SyncOutcome;
use crate::provider::cloudflare::{self};

pub struct DdnsService {
    config: AppConfig,
    client: Client,
}

impl DdnsService {
    pub fn new(config: AppConfig) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&config.globals.user_agent)
                .unwrap_or_else(|_| HeaderValue::from_static("ddns-rs/0.1")),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(config.globals.timeout_duration())
            .build()
            .expect("failed to build HTTP client");

        Self { config, client }
    }

    pub async fn check_config(&self) -> Result<()> {
        self.config.validate()?;
        println!(
            "config OK: {} domain(s), interval={}s, timeout={}s",
            self.config.domains.len(),
            self.config.globals.interval,
            self.config.globals.timeout_secs
        );
        Ok(())
    }

    pub async fn print_ips(&self) -> Result<()> {
        for domain in &self.config.domains {
            let ip = resolve_public_ip(&self.client, &domain.ip_urls)
                .await
                .with_context(|| format!("failed to resolve current IP for {}", domain.name))?;
            domain.ensure_ip_matches_record_type(ip)?;
            println!("{} {}", domain.name, ip);
        }
        Ok(())
    }

    pub async fn run_once(&self) -> Result<()> {
        info!(
            domains = self.config.domains.len(),
            "starting DDNS sync pass"
        );
        for domain in &self.config.domains {
            self.sync_domain(domain).await?;
        }
        info!("finished DDNS sync pass");
        Ok(())
    }

    pub async fn run_forever(&self) -> Result<()> {
        let interval = self.config.globals.interval_duration();
        loop {
            if let Err(error) = self.run_once().await {
                error!(error = %error, "DDNS sync pass failed");
            }
            sleep(interval).await;
        }
    }

    async fn sync_domain(&self, domain: &DomainConfig) -> Result<()> {
        let ip = resolve_public_ip(&self.client, &domain.ip_urls)
            .await
            .with_context(|| format!("failed to resolve current IP for {}", domain.name))?;

        domain.ensure_ip_matches_record_type(ip)?;
        let record_type = domain.desired_record_type()?;
        let ip_text = ip.to_string();

        for provider in &domain.provider {
            match cloudflare::sync_record(
                &self.client,
                provider,
                &domain.name,
                &record_type,
                &ip_text,
            )
            .await?
            {
                SyncOutcome::NoChange { record_id, content } => {
                    info!(
                        domain = %domain.name,
                        record_type = %record_type,
                        provider = provider.provider_name(),
                        record_id = %record_id,
                        ip = %content,
                        "DNS record already up to date"
                    );
                }
                SyncOutcome::Updated {
                    record_id,
                    old_content,
                    new_content,
                } => {
                    info!(
                        domain = %domain.name,
                        record_type = %record_type,
                        provider = provider.provider_name(),
                        record_id = %record_id,
                        old_ip = %old_content,
                        new_ip = %new_content,
                        "DNS record updated"
                    );
                }
                SyncOutcome::Created {
                    record_id,
                    new_content,
                } => {
                    info!(
                        domain = %domain.name,
                        record_type = %record_type,
                        provider = provider.provider_name(),
                        record_id = %record_id,
                        new_ip = %new_content,
                        "DNS record created"
                    );
                }
            }
        }

        Ok(())
    }
}
