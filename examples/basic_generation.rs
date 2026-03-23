//! Basic content generation example

use threatflux_vertex_rust_sdk::{GenerateContentRequest, GenerationConfig, VertexClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Get project and location from environment or use defaults
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .expect("Set GOOGLE_CLOUD_PROJECT environment variable");
    let location =
        std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "us-central1".to_string());

    // Create client
    let client = VertexClient::new_legacy(&project_id, &location).await?;

    // Create generation config
    let config = GenerationConfig {
        temperature: Some(0.7),
        max_output_tokens: Some(1024),
        ..GenerationConfig::default()
    };

    // Create request
    let request = GenerateContentRequest::new("Explain quantum computing in simple terms")
        .with_generation_config(config);

    println!("Generating content...");

    // Generate content
    let response = client.generate_content("gemini-2.0-flash-001", &request).await?;

    // Print response
    if let Some(text) = response.text() {
        println!("Response:\n{text}");
    } else {
        println!("No text content in response");
    }

    // Print usage metadata
    if let Some(usage) = response.usage_metadata {
        println!("\nUsage Statistics:");
        println!("  Prompt tokens: {}", usage.prompt_token_count);
        if let Some(candidates) = usage.candidates_token_count {
            println!("  Response tokens: {candidates}");
        }
        println!("  Total tokens: {}", usage.total_token_count);
    }

    Ok(())
}
