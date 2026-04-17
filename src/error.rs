use std::net::IpAddr;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config must contain at least one domain")]
    NoDomains,
    #[error("domain name cannot be empty")]
    EmptyDomainName,
    #[error("domain {domain} must have at least one ip_urls entry")]
    NoIpSources { domain: String },
    #[error("domain {domain} must have at least one provider entry")]
    NoProviders { domain: String },
    #[error("domain {domain} has unsupported ip_type {ip_type}; only 4 and 6 are supported")]
    UnsupportedIpType { domain: String, ip_type: u8 },
    #[error(
        "domain {domain} has unsupported record_type {record_type}; only A and AAAA are supported"
    )]
    UnsupportedRecordType { domain: String, record_type: String },
    #[error("domain {domain} has incompatible ip_type={ip_type} and record_type={record_type}")]
    IpTypeRecordTypeMismatch {
        domain: String,
        ip_type: u8,
        record_type: String,
    },
    #[error("domain {domain} resolved IP {resolved_ip} does not match record type {record_type}")]
    ResolvedIpRecordTypeMismatch {
        domain: String,
        resolved_ip: IpAddr,
        record_type: String,
    },
    #[error("domain {domain} references unsupported provider {provider}")]
    UnsupportedProvider { domain: String, provider: String },
    #[error("domain {domain} is missing a Cloudflare API token")]
    MissingApiToken { domain: String },
    #[error("domain {domain} is missing a Cloudflare zone_id")]
    MissingZoneId { domain: String },
    #[error("domain {domain} has invalid ttl {ttl}; use 1 for auto or 60..86400")]
    InvalidTtl { domain: String, ttl: u32 },
}

#[derive(Debug, Error)]
pub enum IpLookupError {
    #[error("all public IP sources failed")]
    AllSourcesFailed,
    #[error("received invalid IP text from {url}: {body}")]
    InvalidIpResponse { url: String, body: String },
}
