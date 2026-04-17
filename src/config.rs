use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;

use crate::error::ConfigError;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub globals: GlobalConfig,
    pub domains: Vec<DomainConfig>,
}

impl AppConfig {
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Self = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.domains.is_empty() {
            return Err(ConfigError::NoDomains.into());
        }

        for domain in &self.domains {
            domain.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_interval_secs")]
    pub interval: u64,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    pub log_level: Option<String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            interval: default_interval_secs(),
            timeout_secs: default_timeout_secs(),
            user_agent: default_user_agent(),
            log_level: None,
        }
    }
}

impl GlobalConfig {
    pub fn interval_duration(&self) -> Duration {
        Duration::from_secs(self.interval)
    }

    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainConfig {
    pub name: String,
    pub ip_type: Option<u8>,
    pub record_type: Option<String>,
    #[serde(alias = "ip_urls")]
    pub ip_urls: Vec<String>,
    #[serde(alias = "providers", deserialize_with = "deserialize_provider_list")]
    pub provider: Vec<ProviderConfig>,
}

impl DomainConfig {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::EmptyDomainName.into());
        }

        if self.ip_urls.is_empty() {
            return Err(ConfigError::NoIpSources {
                domain: self.name.clone(),
            }
            .into());
        }

        if self.provider.is_empty() {
            return Err(ConfigError::NoProviders {
                domain: self.name.clone(),
            }
            .into());
        }

        let record_type = self.desired_record_type()?;
        for provider in &self.provider {
            provider.validate(&self.name)?;
            if let Some(expected) = self.ip_type {
                match (expected, record_type.as_str()) {
                    (4, "A") | (6, "AAAA") => {}
                    _ => {
                        return Err(ConfigError::IpTypeRecordTypeMismatch {
                            domain: self.name.clone(),
                            ip_type: expected,
                            record_type,
                        }
                        .into())
                    }
                }
            }
        }

        Ok(())
    }

    pub fn desired_record_type(&self) -> Result<String> {
        if let Some(record_type) = &self.record_type {
            let normalized = record_type.trim().to_ascii_uppercase();
            if normalized == "A" || normalized == "AAAA" {
                return Ok(normalized);
            }
            return Err(ConfigError::UnsupportedRecordType {
                domain: self.name.clone(),
                record_type: record_type.clone(),
            }
            .into());
        }

        match self.ip_type.unwrap_or(4) {
            4 => Ok("A".to_string()),
            6 => Ok("AAAA".to_string()),
            other => Err(ConfigError::UnsupportedIpType {
                domain: self.name.clone(),
                ip_type: other,
            }
            .into()),
        }
    }

    pub fn ensure_ip_matches_record_type(&self, ip: IpAddr) -> Result<()> {
        let record_type = self.desired_record_type()?;
        match (record_type.as_str(), ip) {
            ("A", IpAddr::V4(_)) | ("AAAA", IpAddr::V6(_)) => Ok(()),
            ("A", other) | ("AAAA", other) => Err(ConfigError::ResolvedIpRecordTypeMismatch {
                domain: self.name.clone(),
                resolved_ip: other,
                record_type,
            }
            .into()),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(alias = "kind")]
    pub kind: Option<String>,
    #[serde(alias = "api_key")]
    pub api_token: String,
    pub zone_id: String,
    #[serde(default = "default_proxied")]
    pub proxied: bool,
    #[serde(default = "default_ttl")]
    pub ttl: u32,
}

impl ProviderConfig {
    pub fn validate(&self, domain: &str) -> Result<()> {
        if self.provider_name().to_ascii_lowercase() != "cloudflare" {
            return Err(ConfigError::UnsupportedProvider {
                domain: domain.to_string(),
                provider: self.provider_name().to_string(),
            }
            .into());
        }

        if self.api_token.trim().is_empty() {
            return Err(ConfigError::MissingApiToken {
                domain: domain.to_string(),
            }
            .into());
        }

        if self.zone_id.trim().is_empty() {
            return Err(ConfigError::MissingZoneId {
                domain: domain.to_string(),
            }
            .into());
        }

        if self.ttl != 1 && !(60..=86400).contains(&self.ttl) {
            return Err(ConfigError::InvalidTtl {
                domain: domain.to_string(),
                ttl: self.ttl,
            }
            .into());
        }

        Ok(())
    }

    pub fn provider_name(&self) -> &str {
        self.kind.as_deref().unwrap_or(self.name.as_str())
    }
}

fn deserialize_provider_list<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<ProviderConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ProviderField {
        One(ProviderConfig),
        Many(Vec<ProviderConfig>),
    }

    let providers = ProviderField::deserialize(deserializer)?;
    Ok(match providers {
        ProviderField::One(one) => vec![one],
        ProviderField::Many(many) => many,
    })
}

fn default_interval_secs() -> u64 {
    300
}

fn default_timeout_secs() -> u64 {
    10
}

fn default_user_agent() -> String {
    "ddns-rs/0.1".to_string()
}

fn default_proxied() -> bool {
    true
}

fn default_ttl() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_record_type_from_ip_type() {
        let domain = DomainConfig {
            name: "a.example.com".to_string(),
            ip_type: Some(6),
            record_type: None,
            ip_urls: vec!["https://example.com".to_string()],
            provider: vec![ProviderConfig {
                name: "cloudflare".to_string(),
                kind: None,
                api_token: "token".to_string(),
                zone_id: "zone".to_string(),
                proxied: true,
                ttl: 1,
            }],
        };

        assert_eq!(domain.desired_record_type().unwrap(), "AAAA");
    }

    #[test]
    fn accept_single_provider_object() {
        let yaml = r#"
domains:
  - name: abc.example.com
    ip_type: 4
    ip_urls: ["https://api.ipify.org"]
    provider:
      name: cloudflare
      api_key: token
      zone_id: zone
"#;

        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.domains[0].provider.len(), 1);
    }
}
