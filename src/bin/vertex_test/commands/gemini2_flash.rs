use crate::vertex_test::progress;
use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::api::generate::GenerateApi;
use threatflux_vertex_rust_sdk::builders::ContentRequestBuilder;
use threatflux_vertex_rust_sdk::client::VertexClient;
use threatflux_vertex_rust_sdk::config::Config;
use threatflux_vertex_rust_sdk::models::ModelsApi;
use threatflux_vertex_rust_sdk::types::Part;

pub async fn run(prompt: String) -> Result<()> {
    println!("{}", "Testing Gemini 2.0 Flash...".bold().cyan());
    println!("Prompt: {}", prompt.italic());
    println!();

    let spinner = progress::spinner("Initializing client...");

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let models_api = ModelsApi::new(&client);
    let model_name = "gemini-2.0-flash-001";

    spinner.set_message("Checking if gemini-2.0-flash is available...");

    match models_api.get_model(model_name).await {
        Ok(model_info) => {
            spinner.set_message("Model found! Generating content...");
            let display_name = crate::vertex_test::config::model_display_name(&model_info);
            println!("✓ Model available: {}", display_name.green());
            println!("  Methods: {}", model_info.supported_generation_methods.join(", "));

            if let Some(input_limit) = model_info.input_token_limit {
                println!("  Input limit: {input_limit} tokens");
            }
            if let Some(output_limit) = model_info.output_token_limit {
                println!("  Output limit: {output_limit} tokens");
            }
            println!();
        }
        Err(e) => {
            spinner.finish_and_clear();
            println!("⚠ Model not available: {e}");
            println!("Trying to list available Gemini models instead...");

            let gemini_models = models_api.get_gemini_models().await?;
            if gemini_models.is_empty() {
                println!("No Gemini models available.");
                return Ok(());
            }

            println!("\nAvailable Gemini models:");
            for model in &gemini_models {
                println!("  - {}", model.short_name());
            }

            if let Some(available_model) = gemini_models.first() {
                println!("\nUsing {} instead...", available_model.short_name().green());
                return super::generate::run(
                    Some(prompt),
                    Vec::new(),
                    available_model.short_name().to_string(),
                )
                .await;
            }

            return Ok(());
        }
    }

    let request = ContentRequestBuilder::new(&prompt).temperature(0.7).max_tokens(500).build();

    let api = GenerateApi::new(&client);
    let response = api.generate_content(model_name, request).await?;

    spinner.finish_and_clear();

    println!("{}", "Gemini 2.0 Flash Response:".bold().green());
    println!("{}", "─".repeat(50));

    if let Some(candidate) = response.candidates.first() {
        for part in &candidate.content.parts {
            if let Part::Text { text } = part {
                println!("{text}");
            }
        }
    }

    if let Some(usage) = &response.usage_metadata {
        println!("\n{}", "Token Usage:".bold());
        let prompt_tokens = usage.prompt_token_count;
        let response_tokens = usage.candidates_token_count.unwrap_or(0);
        let total_tokens = usage.total_token_count;
        println!("  Prompt tokens: {prompt_tokens}");
        println!("  Response tokens: {response_tokens}");
        println!("  Total tokens: {total_tokens}");
    }

    Ok(())
}
