use crate::vertex_test::attachments::{
    aggregate_stream_text, build_claude_user_message, load_inline_attachments, merge_claude_usage,
    InlineAttachment, InputFileArg,
};
use crate::vertex_test::config::{claude_model_supports_web_search, resolve_model_alias};
use crate::vertex_test::output::{display_citations, display_non_text_block, print_stream_delta};
use crate::vertex_test::progress;
use anyhow::{anyhow, Result};
use colored::Colorize;
use futures_util::StreamExt;
use threatflux_vertex_rust_sdk::claude::{
    Citation, ContentBlock, MessageRequest, StreamEvent, WebSearchTool,
};
use threatflux_vertex_rust_sdk::client::VertexClient;
use threatflux_vertex_rust_sdk::config::Config;
use threatflux_vertex_rust_sdk::models::GenerateContentRequest;
use threatflux_vertex_rust_sdk::types::{Content, Part};
use threatflux_vertex_rust_sdk::ModelDescriptor;

pub async fn run(prompt: String, model: String, input_files: Vec<InputFileArg>) -> Result<()> {
    println!("{}", "Testing Streaming Generation...".bold().cyan());
    println!("Model: {}", model.yellow());
    println!("Prompt: {}", prompt.italic());
    println!();

    if !input_files.is_empty() {
        println!("{}", "Attachments:".bold());
        for file in &input_files {
            println!("  - {} ({})", file.path.display(), file.mime_type);
        }
        println!();
    }

    let spinner = progress::spinner("Initializing client...");

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let resolved_model = resolve_model_alias(&model);
    let descriptor = ModelDescriptor::parse(&resolved_model)?;

    let attachments = if input_files.is_empty() {
        Vec::new()
    } else {
        spinner.set_message("Encoding attachments...");
        load_inline_attachments(&input_files).await?
    };

    if descriptor.publisher() == "anthropic" {
        run_claude_stream(spinner, &client, &resolved_model, &descriptor, &prompt, &attachments)
            .await
    } else {
        run_gemini_stream(spinner, &client, &resolved_model, &prompt, &attachments).await
    }
}

async fn run_claude_stream(
    spinner: indicatif::ProgressBar,
    client: &VertexClient,
    model: &str,
    descriptor: &ModelDescriptor,
    prompt: &str,
    attachments: &[InlineAttachment],
) -> Result<()> {
    spinner.set_message("Requesting Claude stream...");

    let mut request = MessageRequest::new().max_tokens(500).temperature(0.7);
    let message = build_claude_user_message(prompt, attachments);
    request = request.add_message(message);

    if claude_model_supports_web_search(descriptor.model()) {
        request = request.add_web_search_tool(WebSearchTool::new().with_max_uses(Some(5)));
    }

    let mut stream = client.claude_stream(model, &request).await?;

    spinner.finish_and_clear();

    println!("{}", "Streaming response:".bold().green());
    println!("{}", "─".repeat(50));

    let mut usage: Option<threatflux_vertex_rust_sdk::claude::Usage> = None;
    let mut stream_citations: Vec<Citation> = Vec::new();

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(StreamEvent::MessageStart { message }) => {
                if let Some(u) = &message.usage {
                    usage = Some(merge_claude_usage(usage, u));
                }
            }
            Ok(StreamEvent::ContentBlockStart { content_block, .. }) => {
                if let ContentBlock::Text { citations, .. } = &content_block {
                    if !citations.is_empty() {
                        stream_citations.extend(citations.clone());
                    }
                }
                display_non_text_block(&content_block);
            }
            Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                if let Some(text) = delta.text {
                    print_stream_delta(&text)?;
                }
            }
            Ok(StreamEvent::MessageDelta { usage: u, .. }) => {
                usage = Some(merge_claude_usage(usage, &u));
            }
            Ok(StreamEvent::MessageStop) => break,
            Ok(StreamEvent::Error { error }) => {
                let error_json =
                    serde_json::to_string_pretty(&error).unwrap_or_else(|_| format!("{error:?}"));
                println!("\n{} {}", "Claude stream error:".red().bold(), error_json);
                return Err(anyhow!("Claude streaming request failed"));
            }
            Ok(_) => {}
            Err(e) => {
                println!("\n{} {}", "Stream error:".red().bold(), e);
                return Err(e.into());
            }
        }
    }

    println!();

    if !stream_citations.is_empty() {
        display_citations(&stream_citations);
    }

    if let Some(usage) = usage {
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

async fn run_gemini_stream(
    spinner: indicatif::ProgressBar,
    client: &VertexClient,
    model: &str,
    prompt: &str,
    attachments: &[InlineAttachment],
) -> Result<()> {
    let request = if attachments.is_empty() {
        GenerateContentRequest::new(prompt)
    } else {
        spinner.set_message("Preparing inline files...");
        let mut parts: Vec<Part> = Vec::with_capacity(attachments.len() * 2 + 1);
        for attachment in attachments {
            parts.extend(attachment.gemini_parts());
        }
        parts.push(Part::text(prompt.to_string()));
        let user_content = Content { role: "user".to_string(), parts };
        GenerateContentRequest::with_contents(vec![user_content])
    };

    spinner.set_message("Requesting stream...");

    let request =
        request.with_generation_config(threatflux_vertex_rust_sdk::types::GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(500),
            ..Default::default()
        });

    let mut stream = client.stream_generate_content(model, &request).await?;

    spinner.finish_and_clear();

    println!("{}", "Streaming response:".bold().green());
    println!("{}", "─".repeat(50));

    let mut usage_metadata = None;
    let mut streamed_text = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(response) => {
                let aggregated_text = aggregate_stream_text(&response);
                if let Some(delta) = aggregated_text.strip_prefix(&streamed_text) {
                    if !delta.is_empty() {
                        print_stream_delta(delta)?;
                    }
                } else if !aggregated_text.is_empty() {
                    print_stream_delta(&aggregated_text)?;
                }
                streamed_text = aggregated_text;

                if response.is_final() {
                    usage_metadata.clone_from(&response.usage_metadata);
                }
            }
            Err(e) => {
                println!("\n{} {}", "Stream error:".red().bold(), e);
                return Err(e.into());
            }
        }
    }

    println!();

    if let Some(usage) = usage_metadata {
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
