mod client;
mod config;
mod conversation;
mod flow;
mod schemas;
mod simulator;

use client::{ClientResult, RealContentGenerator};
use config::ExampleConfig;
use flow::run_flow;

#[tokio::main]
async fn main() -> ClientResult<()> {
    env_logger::init();

    let config = ExampleConfig::from_env()?;
    println!("Using project {} in {}, model {}", config.project_id, config.location, config.model);

    let generator = RealContentGenerator::new(&config.project_id, &config.location).await?;

    println!("Sending initial request with function calling capabilities...\n");
    let result = run_flow(&generator, &config).await?;
    log::debug!("Initial request: {initial_request:?}", initial_request = result.initial_request);
    if let Some(final_request) = &result.final_request {
        log::debug!("Final request after tool calls: {final_request:?}");
    }

    if result.function_calls.is_empty() {
        if let Some(text) = result.final_text() {
            println!("Response: {text}");
        } else {
            println!("No text response received");
        }
        print_usage(result.final_usage());
        return Ok(());
    }

    println!("Model requested {} function call(s):", result.function_calls.len());

    for (call, response) in result.function_calls.iter().zip(result.function_responses.iter()) {
        println!("  Function: {}", call.name);
        println!("  Arguments: {:?}", call.args);
        println!("  Result: {}\n", response.response);
    }

    if let Some(text) = result.final_text() {
        println!("Final Response:\n{text}");
    } else {
        println!("No final text response received");
    }

    print_usage(result.final_usage());
    Ok(())
}

fn print_usage(usage: Option<&threatflux_vertex_rust_sdk::UsageMetadata>) {
    if let Some(usage) = usage {
        println!("\nFinal Usage Statistics:");
        println!("  Prompt tokens: {}", usage.prompt_token_count);
        if let Some(tokens) = usage.candidates_token_count {
            println!("  Response tokens: {tokens}");
        }
        println!("  Total tokens: {}", usage.total_token_count);
    }
}
