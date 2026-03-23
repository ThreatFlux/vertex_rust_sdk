use std::io::{self, Write};

use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    client::VertexClient,
    config::Config,
    models::GenerateContentRequest,
    types::{GenerationConfig, Part, Tool},
};
use tokio_stream::StreamExt;

pub async fn code_exec(
    prompt: &str,
    model: &str,
    temperature: f32,
    max_output_tokens: i32,
    system_instruction: Option<&str>,
) -> Result<()> {
    println!("{}", "Code Execution Demo".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("Model: {}", model.yellow());
    if let Some(instruction) = system_instruction {
        println!("System: {}", instruction.italic().blue());
    }
    println!("Prompt: {}\n", prompt.italic());

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let generation_config = GenerationConfig {
        temperature: Some(temperature),
        max_output_tokens: Some(max_output_tokens),
        ..Default::default()
    };

    let code_execution_tool = Tool::code_execution();

    let mut request = GenerateContentRequest::new(prompt)
        .with_generation_config(generation_config)
        .with_tools(vec![code_execution_tool]);

    if let Some(instruction) = system_instruction {
        request = request.with_system_text(instruction);
    }

    println!("{}", "Sending request with code execution tool...".blue());

    let response = client.generate_content(model, &request).await?;

    if let Some(text) = response.text() {
        println!("\n{}", "Model Response:".bold().green());
        println!("{text}");
    }

    let executable_codes = response.executable_code();
    if !executable_codes.is_empty() {
        println!("\n{}", "Executable Code Found:".bold().magenta());
        for (i, code) in executable_codes.iter().enumerate() {
            println!("\n{} Code Block {} ({:?}):", "🐍".blue(), i + 1, code.language);
            println!("{}", "─".repeat(40).blue());
            println!("{}", code.code.bright_white());
        }
    }

    let execution_results = response.code_execution_results();
    if !execution_results.is_empty() {
        println!("\n{}", "Code Execution Results:".bold().green());
        for (i, result) in execution_results.iter().enumerate() {
            println!("\n{} Result {} ({:?}):", "⚡".green(), i + 1, result.outcome);
            println!("{}", "─".repeat(40).green());
            println!("{}", result.output.bright_white());
        }
    }

    if let Some(candidate) = response.candidates.first() {
        println!("\n{}", "Response Parts Breakdown:".bold().yellow());
        for (i, part) in candidate.content.parts.iter().enumerate() {
            match part {
                Part::Text { text } => {
                    println!("  {}: Text - {} chars", i + 1, text.len());
                }
                Part::ExecutableCode { executable_code } => {
                    println!("  {}: ExecutableCode - {:?}", i + 1, executable_code.language);
                }
                Part::CodeExecutionResult { code_execution_result } => {
                    println!(
                        "  {}: CodeExecutionResult - {:?}",
                        i + 1,
                        code_execution_result.outcome
                    );
                }
                _ => {
                    println!("  {}: Other part type", i + 1);
                }
            }
        }
    }

    if let Some(usage) = &response.usage_metadata {
        println!("\n{}", "Token Usage:".bold().blue());
        println!("  Prompt tokens: {}", usage.prompt_token_count);
        if let Some(candidates) = usage.candidates_token_count {
            println!("  Response tokens: {candidates}");
        }
        println!("  Total tokens: {}", usage.total_token_count);
    }

    println!("\n{} Code execution demo completed successfully!", "✅".green());

    Ok(())
}

pub async fn code_exec_stream(
    prompt: &str,
    model: &str,
    temperature: f32,
    max_output_tokens: i32,
    system_instruction: Option<&str>,
) -> Result<()> {
    println!("{}", "Streaming Code Execution Demo".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("Model: {}", model.yellow());
    if let Some(instruction) = system_instruction {
        println!("System: {}", instruction.italic().blue());
    }
    println!("Prompt: {}\n", prompt.italic());

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let generation_config = GenerationConfig {
        temperature: Some(temperature),
        max_output_tokens: Some(max_output_tokens),
        ..Default::default()
    };

    let code_execution_tool = Tool::code_execution();

    let mut request = GenerateContentRequest::new(prompt)
        .with_generation_config(generation_config)
        .with_tools(vec![code_execution_tool]);

    if let Some(instruction) = system_instruction {
        request = request.with_system_text(instruction);
    }

    println!("{}", "Streaming response with code execution...".blue());
    println!("{}", "Response:".bold().green());

    let mut stream = client.stream_generate_content(model, &request).await?;
    let mut final_usage = None;
    let mut all_executable_codes = Vec::new();
    let mut all_execution_results = Vec::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(response) => {
                if let Some(text) = response.text() {
                    print!("{text}");
                    io::stdout().flush().unwrap();
                }

                let codes = response.executable_code();
                all_executable_codes.extend(codes);

                let results = response.code_execution_results();
                all_execution_results.extend(results);

                if response.is_final() {
                    final_usage = response.usage_metadata;
                }
            }
            Err(e) => {
                eprintln!("\n{} Error: {}", "❌".red(), e);
                break;
            }
        }
    }

    println!();

    if !all_executable_codes.is_empty() {
        println!("\n{}", "Executable Code Found:".bold().magenta());
        for (i, code) in all_executable_codes.iter().enumerate() {
            println!("\n{} Code Block {} ({:?}):", "🐍".blue(), i + 1, code.language);
            println!("{}", "─".repeat(40).blue());
            println!("{}", code.code.bright_white());
        }
    }

    if !all_execution_results.is_empty() {
        println!("\n{}", "Code Execution Results:".bold().green());
        for (i, result) in all_execution_results.iter().enumerate() {
            println!("\n{} Result {} ({:?}):", "⚡".green(), i + 1, result.outcome);
            println!("{}", "─".repeat(40).green());
            println!("{}", result.output.bright_white());
        }
    }

    if let Some(usage) = final_usage {
        println!("\n{}", "Token Usage:".bold().blue());
        println!("  Prompt tokens: {}", usage.prompt_token_count);
        if let Some(candidates) = usage.candidates_token_count {
            println!("  Response tokens: {candidates}");
        }
        println!("  Total tokens: {}", usage.total_token_count);
    }

    println!("\n{} Streaming code execution demo completed successfully!", "✅".green());

    Ok(())
}
