use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::{auth::from_env, config::Config};

pub fn show_config() -> Result<()> {
    println!("{}", "Current Configuration".bold().cyan());
    println!("{}", "═".repeat(40).cyan());

    let config = Config::from_env()?;

    println!("Project ID: {}", config.project_id.green());
    println!("Region: {}", config.region.green());
    println!("Base URL: {}", config.base_url().blue());
    println!("Timeout: {} seconds", config.timeout_secs.to_string().yellow());

    // Check environment variables
    println!("\n{}", "Environment Variables:".bold().cyan());

    let env_vars = [
        ("VERTEX_PROJECT_ID", std::env::var("VERTEX_PROJECT_ID")),
        ("VERTEX_REGION", std::env::var("VERTEX_REGION")),
        ("GOOGLE_APPLICATION_CREDENTIALS", std::env::var("GOOGLE_APPLICATION_CREDENTIALS")),
    ];

    for (key, value) in env_vars {
        match value {
            Ok(val) => println!("  {}: {}", key.bold(), val.green()),
            Err(_) => println!("  {}: {}", key.bold(), "Not set".red()),
        }
    }

    Ok(())
}

pub async fn check_config() -> Result<()> {
    println!("{}", "Checking Configuration...".bold().cyan());
    println!("{}", "═".repeat(40).cyan());

    let config = Config::from_env()?;

    let project_ok = if config.project_id.is_empty() {
        println!("{} Project ID not set", "❌".red());
        println!("   Set VERTEX_PROJECT_ID environment variable");
        false
    } else {
        println!("{} Project ID: {}", "✅".green(), config.project_id.green());
        true
    };

    let region_ok = if config.region.is_empty() {
        println!("{} Region not set", "❌".red());
        println!("   Set VERTEX_REGION environment variable");
        false
    } else {
        println!("{} Region: {}", "✅".green(), config.region.green());
        true
    };

    let mut all_ok = project_ok && region_ok;

    // Check authentication
    match from_env().await {
        Ok(_) => {
            println!("{} Authentication configured", "✅".green());
        }
        Err(e) => {
            println!("{} Authentication error: {}", "❌".red(), e);
            println!("   Set GOOGLE_APPLICATION_CREDENTIALS environment variable");
            println!("   or run: gcloud auth application-default login");
            all_ok = false;
        }
    }

    if all_ok {
        println!("\n{} All configuration checks passed!", "🎉".green());
    } else {
        println!("\n{} Some configuration issues found. Please fix them.", "⚠️".yellow());
    }

    Ok(())
}

pub fn init_config() -> Result<()> {
    use std::{fs, path::Path};

    println!("{}", "Initializing Configuration...".bold().cyan());
    println!("{}", "═".repeat(40).cyan());

    // Check if .env file exists
    if Path::new(".env").exists() {
        println!("{} .env file already exists", "ℹ️".blue());
    } else {
        // Create .env file with template
        let env_content = r"# Vertex AI Configuration
# Get your project ID from: https://console.cloud.google.com/
VERTEX_PROJECT_ID=your-project-id

# Region where you want to run Vertex AI
# Common regions: us-central1, us-east1, europe-west1
VERTEX_REGION=us-central1

# Authentication (choose one):
# Option 1: Service account key file
# GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json

# Option 2: Use gcloud CLI
# Run: gcloud auth application-default login
";

        fs::write(".env", env_content)?;
        println!("{} Created .env file", "✅".green());
        println!("   Please edit .env and set your project ID and region");
    }

    // Show next steps
    println!("\n{}", "Next steps:".bold().yellow());
    println!("1. Edit .env file with your project ID and region");
    println!("2. Set up authentication:");
    println!("   - Service account: Set GOOGLE_APPLICATION_CREDENTIALS");
    println!("   - gcloud CLI: Run 'gcloud auth application-default login'");
    println!("3. Test with: {} config check", "vertex".cyan());

    Ok(())
}
