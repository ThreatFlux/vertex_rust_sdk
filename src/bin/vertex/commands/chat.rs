use std::io::{self, Write};

use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    client::VertexClient, config::Config, models::GenerateContentRequest, types::Content,
};

pub async fn chat(model: &str, system_instruction: Option<&str>) -> Result<()> {
    println!("{}", "Interactive Chat Session".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("Model: {}", model.yellow());
    if let Some(instruction) = system_instruction {
        println!("System: {}", instruction.italic());
    }
    println!("Type 'exit' to quit, 'clear' to clear history");
    println!();

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;

    let mut conversation = Vec::new();

    if let Some(instruction) = system_instruction {
        conversation.push(Content::system_text(instruction));
    }

    loop {
        print!("{} ", "You:".bold().blue());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" {
            println!("Goodbye! 👋");
            break;
        }

        if input == "clear" {
            conversation.clear();
            if let Some(instruction) = system_instruction {
                conversation.push(Content::system_text(instruction));
            }
            println!("Conversation history cleared.");
            continue;
        }

        if input.is_empty() {
            continue;
        }

        conversation.push(Content::user_text(input));

        let request = GenerateContentRequest::with_contents(conversation.clone());

        print!("{} ", "AI:".bold().green());
        io::stdout().flush().unwrap();

        match client.generate_content(model, &request).await {
            Ok(response) => {
                if let Some(text) = response.text() {
                    println!("{text}");
                    conversation.push(Content::model_text(text));
                } else {
                    println!("{} No response received", "⚠️".yellow());
                }
            }
            Err(e) => {
                eprintln!("{} Error: {}", "❌".red(), e);
                conversation.pop();
            }
        }

        println!();
    }

    Ok(())
}
