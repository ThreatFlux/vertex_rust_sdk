use anyhow::Result;
use threatflux_vertex_rust_sdk::{client::VertexClient, config::Config};

mod cases;
mod client;
mod config;
mod reporter;
mod runner;

use client::VertexContentGenerator;
use reporter::StdoutReporter;
use runner::{run_system_suite, TokioSleeper};

pub use config::SystemTestConfig;

pub async fn system_test(model: &str) -> Result<()> {
    system_test_with_config(model, SystemTestConfig::from_env()).await
}

pub async fn system_test_with_config(model: &str, test_config: SystemTestConfig) -> Result<()> {
    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let generator = VertexContentGenerator::new(client);
    let mut reporter = StdoutReporter::new();
    let sleeper = TokioSleeper;

    run_system_suite(model, &test_config, &generator, &mut reporter, &sleeper).await
}

#[cfg(test)]
mod tests;
