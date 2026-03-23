use crate::vertex_test::progress;
use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::client::VertexClient;
use threatflux_vertex_rust_sdk::config::Config;
use threatflux_vertex_rust_sdk::models::ModelsApi;

pub async fn list() -> Result<()> {
    println!("{}", "Listing Available Locations...".bold().cyan());

    let spinner = progress::spinner("Fetching locations...");

    let config = Config::from_env()?;
    let current_region = config.region.clone();
    let client = VertexClient::new(config).await?;

    let models_api = ModelsApi::new(&client);
    let response = models_api.list_locations(None, None).await?;

    spinner.finish_and_clear();

    if response.locations.is_empty() {
        println!("No locations found.");
        return Ok(());
    }

    println!("{} {} locations found:", "✓".green(), response.locations.len());
    println!("{}", "─".repeat(80));

    for (i, location) in response.locations.iter().enumerate() {
        println!(
            "\n{}. {}",
            (i + 1).to_string().yellow().bold(),
            location.location_id.cyan().bold()
        );
        let display_name = &location.display_name;
        println!("   Display Name: {display_name}");
        println!("   Full Name: {}", location.name.dimmed());

        if location.supports_vertex_ai() {
            println!("   {}", "✓ Supports Vertex AI".green());
        }

        if let Some(labels) = &location.labels {
            if !labels.is_empty() {
                println!("   Labels: {labels:?}");
            }
        }
    }

    println!("\n{}", "Current Configuration:".bold());
    println!("Current Region: {}", current_region.yellow().bold());

    Ok(())
}
