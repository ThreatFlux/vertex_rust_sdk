use std::{io, time::Instant};

use anyhow::Result;
use threatflux_vertex_rust_sdk::{
    client::VertexClient,
    config::Config,
    types::{GroundingMetadata, UsageMetadata},
};
use tokio_stream::StreamExt;

use crate::commands::grounding::display_grounding_info;

use super::{
    builder::{
        build_generation_config, build_request_from_generation, build_stream_config,
        resolve_thinking_settings,
    },
    printer::{print_grounding, Printer},
    GenerationOptions, StreamOptions,
};

pub async fn generate(options: GenerationOptions) -> Result<()> {
    let thinking_settings = resolve_thinking_settings(
        &options.model,
        options.thinking_requested(),
        options.thinking_budget,
        options.thinking_level,
    )?;

    let mut printer = Printer::new(io::stdout());
    printer.banner("Generating Content...", &options.model)?;
    printer.system_prompt(options.system_instruction.as_deref())?;
    printer.cache(options.cache_id.as_deref())?;
    printer.thinking(options.thinking_requested(), &thinking_settings)?;
    printer.grounding(options.grounding)?;
    printer.structured_output(options.json, options.schema.is_some())?;
    printer.prompt(&options.prompt)?;

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let generation_config = build_generation_config(&options, &thinking_settings)?;
    let request = build_request_from_generation(
        &options.prompt,
        generation_config,
        options.system_instruction.as_deref(),
        options.cache_id.as_deref(),
        options.grounding,
    );

    let start_time = Instant::now();
    let response = client.generate_content(&options.model, &request).await?;
    let elapsed = start_time.elapsed();

    display_grounding_info(&response);
    printer.heading("Response:")?;
    if response.has_thinking() {
        let thoughts = response.thinking_content();
        printer.thinking_sections(&thoughts)?;
    }
    printer.print_response_text(&response, options.json)?;

    if let Some(usage) = &response.usage_metadata {
        printer.usage(usage)?;
    }

    printer.response_time(elapsed)?;
    Ok(())
}

pub async fn generate_with_options_cache_thinking_and_grounding(
    options: GenerationOptions,
) -> Result<()> {
    generate(options).await
}

pub async fn stream_generate(options: StreamOptions) -> Result<()> {
    let thinking_settings = resolve_thinking_settings(
        &options.model,
        options.thinking_requested(),
        options.thinking_budget,
        options.thinking_level,
    )?;

    let mut printer = Printer::new(io::stdout());
    printer.banner("Streaming Content Generation with Grounding...", &options.model)?;
    printer.system_prompt(options.system_instruction.as_deref())?;
    printer.cache(options.cache_id.as_deref())?;
    printer.thinking(options.thinking_requested(), &thinking_settings)?;
    printer.grounding(options.grounding)?;
    printer.prompt(&options.prompt)?;

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let generation_config = build_stream_config(&options, &thinking_settings);
    let request = build_request_from_generation(
        &options.prompt,
        generation_config,
        options.system_instruction.as_deref(),
        options.cache_id.as_deref(),
        options.grounding,
    );

    let start_time = Instant::now();
    let mut stream = client.stream_generate_content(&options.model, &request).await?;

    let mut final_grounding_metadata: Option<GroundingMetadata> = None;
    let mut final_usage: Option<UsageMetadata> = None;
    let mut thinking_sections = Vec::new();

    printer.stream_prefix()?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        if chunk.has_grounding() {
            final_grounding_metadata = chunk.grounding_metadata().cloned();
        }

        if chunk.has_thinking() {
            for thought in chunk.thinking_content() {
                thinking_sections.push(thought);
            }
        }

        if let Some(text) = chunk.text() {
            printer.inline_text(&text)?;
        }

        if chunk.is_final() {
            let elapsed = start_time.elapsed();
            printer.stream_complete(elapsed)?;

            if let Some(metadata) = &final_grounding_metadata {
                print_grounding(metadata);
            }

            if !thinking_sections.is_empty() {
                printer.thinking_sections(&thinking_sections)?;
            }

            if let Some(usage) = &chunk.usage_metadata {
                final_usage = Some(usage.clone());
            }

            break;
        }
    }

    if let Some(usage) = &final_usage {
        printer.usage(usage)?;
    }

    Ok(())
}

pub async fn stream_generate_with_cache_thinking_and_grounding(
    options: StreamOptions,
) -> Result<()> {
    stream_generate(options).await
}
