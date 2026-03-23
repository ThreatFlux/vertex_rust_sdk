use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    client::VertexClient, config::Config, models::CountTokensRequest,
};

pub async fn count_tokens(text: &str, model: &str) -> Result<()> {
    println!("{}", "Counting Tokens...".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("Model: {}", model.yellow());
    println!("Text: {}", text.italic());
    println!();

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let request = CountTokensRequest::new(text);
    let response = client.count_tokens(model, &request).await?;

    println!("{} Token count: {}", "📊".blue(), response.total_tokens.to_string().green().bold());

    Ok(())
}
