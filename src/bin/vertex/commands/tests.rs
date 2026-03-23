use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::auth::from_env;

use super::{
    function_calls::test_functions_with_prompt,
    generation::{generate, stream_generate},
};

pub async fn test_auth() -> Result<()> {
    println!("{}", "Testing Authentication...".bold().cyan());

    match from_env().await {
        Ok(auth_provider) => match auth_provider.get_token().await {
            Ok(token) => {
                let token_preview = if token.len() > 20 {
                    format!("{}...{}", &token[..10], &token[token.len() - 10..])
                } else {
                    token
                };
                println!("{} Authentication successful!", "✅".green());
                println!("Token preview: {}", token_preview.italic());
            }
            Err(e) => {
                println!("{} Failed to get token: {}", "❌".red(), e);
                return Err(e);
            }
        },
        Err(e) => {
            println!("{} Authentication setup failed: {}", "❌".red(), e);
            return Err(e);
        }
    }

    Ok(())
}

pub async fn test_generate() -> Result<()> {
    println!("{}", "Testing Generation API...".bold().cyan());
    generate("Hello! Please introduce yourself.", "gemini-1.5-flash", 0.7, 100, None).await
}

pub async fn test_stream() -> Result<()> {
    println!("{}", "Testing Streaming API...".bold().cyan());
    stream_generate(
        "Count from 1 to 5, explaining each number.",
        "gemini-1.5-flash",
        0.7,
        200,
        None,
    )
    .await
}

pub async fn test_functions() -> Result<()> {
    println!("{}", "Testing Function Calling...".bold().cyan());
    test_functions_with_prompt(
        "What's the weather in Boston and New York? Also, what's 25 multiplied by 4?",
        "gemini-2.5-flash",
        None,
    )
    .await
}

pub async fn test_all() -> Result<()> {
    println!("{}", "Running All Tests...".bold().cyan());
    println!("{}", "═".repeat(60).cyan());

    println!("\n{}", "1. Authentication Test".bold().yellow());
    test_auth().await?;

    println!("\n{}", "2. Generation Test".bold().yellow());
    test_generate().await?;

    println!("\n{}", "3. Streaming Test".bold().yellow());
    test_stream().await?;

    println!("\n{}", "4. Function Calling Test".bold().yellow());
    test_functions().await?;

    println!("\n{} All tests completed!", "🎉".green());

    Ok(())
}
