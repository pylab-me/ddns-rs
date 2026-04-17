mod config;
mod error;
mod ip_source;
mod provider;
mod service;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use tracing_subscriber::EnvFilter;

use crate::config::AppConfig;
use crate::service::DdnsService;

#[derive(Debug, Parser)]
#[command(name = "ddns-rs")]
#[command(version)]
#[command(about = "A small DDNS daemon with Cloudflare support")]
struct Cli {
    #[arg(short, long, global = true, default_value = "config.yml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run one sync pass and exit.
    Once,
    /// Run forever, syncing on the configured interval.
    Run,
    /// Validate the configuration file and print a short summary.
    CheckConfig,
    /// Resolve the current public IP for all domains and print the result.
    PrintIp,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::from_file(&cli.config)?;

    init_tracing(config.globals.log_level.as_deref());

    let service = DdnsService::new(config);

    match cli.command.unwrap_or(Command::Run) {
        Command::Once => service.run_once().await?,
        Command::Run => service.run_forever().await?,
        Command::CheckConfig => service.check_config().await?,
        Command::PrintIp => service.print_ips().await?,
    }

    Ok(())
}

fn init_tracing(configured_level: Option<&str>) {
    let filter = configured_level.unwrap_or("info");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
}
