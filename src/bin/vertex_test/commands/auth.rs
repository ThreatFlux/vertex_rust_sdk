use crate::vertex_test::progress;
use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::auth::from_env;

pub async fn run() -> Result<()> {
    println!("{}", "Testing Authentication...".bold().cyan());

    let spinner = progress::spinner("Creating auth provider...");

    let auth = from_env().await?;
    spinner.set_message("Getting access token...");

    let token = auth.get_token().await?;
    spinner.finish_with_message(format!("{} Authentication successful!", "✓".green()));

    println!("Token prefix: {}...", &token[..20]);
    println!("Token length: {} characters", token.len());

    Ok(())
}
