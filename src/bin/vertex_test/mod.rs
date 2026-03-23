pub mod attachments;
pub mod cli;
pub mod commands;
pub mod config;
pub mod output;
pub mod progress;

use crate::vertex_test::cli::Cli;
use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use std::env;
use threatflux_vertex_rust_sdk::config::EnvConfig;

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    EnvConfig::load_dotenv();

    if let Some(max_retries) = cli.max_retries {
        env::set_var("VERTEX_MAX_RETRIES", max_retries.to_string());
    }

    configure_logging(cli.debug);

    commands::run(cli).await
}

fn configure_logging(debug: bool) {
    if debug {
        env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    }
}
