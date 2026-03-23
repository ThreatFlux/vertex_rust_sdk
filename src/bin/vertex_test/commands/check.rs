use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::config::EnvConfig;

pub fn run() -> Result<()> {
    println!("{}", "Checking Environment Configuration...".bold().cyan());
    println!("{}", "=".repeat(50).cyan());

    EnvConfig::print_config();

    println!("\n{}", "Checking Required Variables...".bold());
    match EnvConfig::check_required() {
        Ok(()) => {
            println!("{} All required environment variables are set!", "✓".green());
        }
        Err(e) => {
            println!("{} {}", "✗".red(), e);
            println!("\n{}", "Setup Instructions:".bold());
            println!("1. Copy .env.example to .env");
            println!("2. Fill in your GCP credentials");
            println!("3. Run this command again");
            return Err(e);
        }
    }

    Ok(())
}
