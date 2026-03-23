use crate::vertex_test::commands::{auth, check, function_call, generate, stream};
use anyhow::Result;
use colored::Colorize;
use tokio::time::{sleep, Duration};

pub async fn run(model: String) -> Result<()> {
    println!("{}", "Running All Tests...".bold().magenta());
    println!("{}", "=".repeat(50).magenta());

    println!("\n{}", "[1/5] Environment Check".bold());
    check::run()?;
    sleep(Duration::from_secs(1)).await;

    println!("\n{}", "[2/5] Authentication Test".bold());
    auth::run().await?;
    sleep(Duration::from_secs(1)).await;

    println!("\n{}", "[3/5] Non-Streaming Generation Test".bold());
    generate::run(
        Some("What is 2+2? Give a very short answer.".to_string()),
        Vec::new(),
        model.clone(),
    )
    .await?;
    sleep(Duration::from_secs(1)).await;

    println!("\n{}", "[4/5] Streaming Generation Test".bold());
    stream::run("Count from 1 to 10 slowly".to_string(), model.clone(), Vec::new()).await?;
    sleep(Duration::from_secs(1)).await;

    println!("\n{}", "[5/5] Function Calling Test".bold());
    function_call::run("What's the weather in Tokyo?".to_string(), model.replace("flash", "pro"))
        .await?;

    println!("\n{}", "=".repeat(50).green());
    println!("{}", "All tests completed successfully!".bold().green());

    Ok(())
}
