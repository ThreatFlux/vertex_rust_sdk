use crate::vertex_test::config::{claude_model_supports_web_search, resolve_model_alias};
use crate::vertex_test::output::print_claude_blocks;
use crate::vertex_test::progress;
use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::claude::{MessageRequest, WebSearchTool};
use threatflux_vertex_rust_sdk::client::VertexClient;
use threatflux_vertex_rust_sdk::config::Config;
use threatflux_vertex_rust_sdk::ModelDescriptor;
use threatflux_vertex_rust_sdk::{api::generate::GenerateApi, models::GenerateContentRequest};

pub async fn run(prompt: Option<String>, prompt_words: Vec<String>, model: String) -> Result<()> {
    println!("{}", "Testing Non-Streaming Generation...".bold().cyan());
    println!("Model: {}", model.yellow());

    let final_prompt = prompt
        .or_else(|| (!prompt_words.is_empty()).then(|| prompt_words.join(" ")))
        .unwrap_or_else(|| "Tell me a short joke".to_string());
    println!("Prompt: {}", final_prompt.italic());
    println!();

    let spinner = progress::spinner("Initializing client...");

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let resolved_model = resolve_model_alias(&model);
    let descriptor = ModelDescriptor::parse(&resolved_model)?;

    if descriptor.publisher() == "anthropic" {
        run_claude(&client, &resolved_model, &final_prompt, &descriptor, spinner).await
    } else {
        run_gemini(&client, &resolved_model, &final_prompt, spinner).await
    }
}

async fn run_claude(
    client: &VertexClient,
    model: &str,
    prompt: &str,
    descriptor: &ModelDescriptor,
    spinner: indicatif::ProgressBar,
) -> Result<()> {
    spinner.set_message("Invoking Claude message endpoint...");

    let mut request =
        MessageRequest::new().max_tokens(500).temperature(0.7).add_user_message(prompt.to_string());

    if claude_model_supports_web_search(descriptor.model()) {
        request = request.add_web_search_tool(WebSearchTool::new().with_max_uses(Some(5)));
    }

    let response = client.claude_message(model, &request).await?;

    spinner.finish_and_clear();

    println!("{}", "Response:".bold().green());
    println!("{}", "─".repeat(50));
    print_claude_blocks(&response.content);

    if let Some(usage) = response.usage {
        println!("\n{}", "Token Usage:".bold());
        let input_tokens = usage.input_tokens;
        let output_tokens = usage.output_tokens;
        let total_tokens = usage.total();
        println!("  Input tokens: {input_tokens}");
        println!("  Output tokens: {output_tokens}");
        println!("  Total tokens: {total_tokens}");
    }

    Ok(())
}

async fn run_gemini(
    client: &VertexClient,
    model: &str,
    prompt: &str,
    spinner: indicatif::ProgressBar,
) -> Result<()> {
    spinner.set_message("Generating content...");

    let request = GenerateContentRequest::new(prompt).with_generation_config(
        threatflux_vertex_rust_sdk::types::GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(500),
            ..Default::default()
        },
    );

    let api = GenerateApi::new(client);
    let response = api.generate_content(model, request).await?;

    spinner.finish_and_clear();

    println!("{}", "Response:".bold().green());
    println!("{}", "─".repeat(50));

    if let Some(text) = response.text() {
        println!("{text}");
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
