use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    client::VertexClient, config::Config, models::GenerateContentRequest,
};

pub async fn list_models(_gemini: bool, page_size: Option<i32>) -> Result<()> {
    println!("{}", "Listing Available Models...".bold().cyan());
    println!("{}", "═".repeat(60).cyan());

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let models_api = client.models();

    let response = models_api.list_models(page_size, None).await?;

    if response.models.is_empty() {
        println!("No models found.");
        return Ok(());
    }

    println!("Found {} models:\n", response.models.len());

    for model in response.models {
        println!("{}", model.name.bold().green());
        println!("  Display Name: {}", model.display_name.unwrap_or_else(|| "N/A".to_string()));
        println!("  Description: {}", model.description.unwrap_or_else(|| "N/A".to_string()));
        println!("  Version: {}", model.version.unwrap_or_else(|| "N/A".to_string()));

        if !model.supported_generation_methods.is_empty() {
            println!(
                "  Supported methods: {}",
                model.supported_generation_methods.join(", ").blue()
            );
        }

        if let Some(input_limit) = model.input_token_limit {
            println!("  Input token limit: {}", input_limit.to_string().yellow());
        }
        if let Some(output_limit) = model.output_token_limit {
            println!("  Output token limit: {}", output_limit.to_string().yellow());
        }

        if let Some(temp_range) = &model.temperature {
            println!("  Temperature range: {} - {}", temp_range.min, temp_range.max);
        }

        if let Some(top_p_range) = &model.top_p {
            println!("  Top-p range: {} - {}", top_p_range.min, top_p_range.max);
        }

        println!();
    }

    if let Some(next_page_token) = response.next_page_token {
        println!("Next page token: {}", next_page_token.italic());
    }

    Ok(())
}

pub async fn get_model(model_name: &str) -> Result<()> {
    println!("{}", "Getting Model Information...".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("Model: {}\n", model_name.yellow());

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let models_api = client.models();

    let model = models_api.get_model(model_name).await?;

    println!("{}", model.name.bold().green());
    println!("Display Name: {}", model.display_name.unwrap_or_else(|| "N/A".to_string()));
    println!("Description: {}", model.description.unwrap_or_else(|| "N/A".to_string()));
    println!("Version: {}", model.version.unwrap_or_else(|| "N/A".to_string()));

    if !model.supported_generation_methods.is_empty() {
        println!("Supported methods: {}", model.supported_generation_methods.join(", ").blue());
    }

    if let Some(input_limit) = model.input_token_limit {
        println!("Input token limit: {}", input_limit.to_string().yellow());
    }
    if let Some(output_limit) = model.output_token_limit {
        println!("Output token limit: {}", output_limit.to_string().yellow());
    }

    if let Some(temp_range) = &model.temperature {
        println!("Temperature range: {} - {}", temp_range.min, temp_range.max);
    }

    if let Some(top_p_range) = &model.top_p {
        println!("Top-p range: {} - {}", top_p_range.min, top_p_range.max);
    }

    Ok(())
}

pub async fn list_locations(page_size: Option<i32>) -> Result<()> {
    println!("{}", "Listing Available Locations...".bold().cyan());
    println!("{}", "═".repeat(60).cyan());

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let models_api = client.models();

    let response = models_api.list_locations(page_size, None).await?;

    if response.locations.is_empty() {
        println!("No locations found.");
        return Ok(());
    }

    println!("Found {} locations:\n", response.locations.len());

    for location in response.locations {
        println!("{}", location.name.bold().green());
        println!("  Location ID: {}", location.location_id.blue());
        println!("  Display Name: {}", location.display_name);
        println!();
    }

    if let Some(next_page_token) = response.next_page_token {
        println!("Next page token: {}", next_page_token.italic());
    }

    Ok(())
}

pub async fn test_model(model_name: &str, prompt: &str) -> Result<()> {
    println!("{}", "Testing Model...".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("Model: {}", model_name.yellow());
    println!("Prompt: {}\n", prompt.italic());

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let request = GenerateContentRequest::new(prompt);

    let start_time = std::time::Instant::now();
    let response = client.generate_content(model_name, &request).await?;
    let elapsed = start_time.elapsed();

    if let Some(text) = response.text() {
        println!("{}", "Response:".bold().green());
        println!("{text}");
        println!("\n{}", format!("Response time: {elapsed:?}").blue());
    } else {
        println!("{} No text in response", "⚠️".yellow());
    }

    if let Some(usage) = &response.usage_metadata {
        println!("\n{}", "Token Usage:".bold().blue());
        println!("  Prompt tokens: {}", usage.prompt_token_count);
        if let Some(candidates) = usage.candidates_token_count {
            println!("  Response tokens: {candidates}");
        }
        println!("  Total tokens: {}", usage.total_token_count);
    }

    Ok(())
}
