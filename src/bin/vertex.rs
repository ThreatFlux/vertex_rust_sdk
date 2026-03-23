use anyhow::Result;
use env_logger::Env;
use threatflux_vertex_rust_sdk::config::EnvConfig;

#[path = "vertex/cli.rs"]
mod cli;
#[path = "vertex/commands/mod.rs"]
mod commands;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    EnvConfig::load_dotenv();

    if cli.debug {
        env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    }

    if let Some(project) = &cli.project {
        std::env::set_var("VERTEX_PROJECT_ID", project);
    }
    if let Some(region) = &cli.region {
        std::env::set_var("VERTEX_REGION", region);
    }

    commands::run(cli).await
}
