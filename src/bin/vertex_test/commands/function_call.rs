use crate::vertex_test::commands::generate;
use anyhow::Result;
use colored::Colorize;

pub async fn run(prompt: String, model: String) -> Result<()> {
    println!("{}", "Testing Function/Tool Calling...".bold().cyan());
    println!("Model: {}", model.yellow());
    println!("Prompt: {}", prompt.italic());
    println!();

    println!(
        "{}",
        "⚠ Function calling not fully implemented yet, falling back to regular generation".yellow()
    );
    generate::run(Some(prompt), Vec::new(), model).await
}
