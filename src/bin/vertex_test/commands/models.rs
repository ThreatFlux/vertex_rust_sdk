use crate::vertex_test::config::{model_description, model_display_name};
use crate::vertex_test::progress;
use anyhow::Result;
use colored::Colorize;
use threatflux_vertex_rust_sdk::client::VertexClient;
use threatflux_vertex_rust_sdk::config::Config;
use threatflux_vertex_rust_sdk::models::{Model, ModelsApi};

pub async fn list(gemini_only: bool, detailed: bool) -> Result<()> {
    println!("{}", "Listing Available Models...".bold().cyan());

    let spinner = progress::spinner("Fetching models...");

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let models_api = ModelsApi::new(&client);

    let models = if gemini_only {
        spinner.set_message("Fetching Gemini models...");
        models_api.get_gemini_models().await?
    } else {
        spinner.set_message("Fetching all models...");
        let response = models_api.list_models(None, None).await?;
        response.models
    };

    spinner.finish_and_clear();

    if models.is_empty() {
        println!("No models found.");
        return Ok(());
    }

    println!("{} {} models found:", "✓".green(), models.len());
    println!("{}", "─".repeat(80));

    for (i, model) in models.iter().enumerate() {
        print_model_entry(model, i + 1, detailed);
    }

    if !detailed {
        println!("\n{}", "Use --detailed flag for more information".dimmed());
    }

    Ok(())
}

fn print_model_entry(model: &Model, index: usize, detailed: bool) {
    println!("\n{}. {}", index.to_string().yellow().bold(), model.short_name().cyan().bold());

    let display_name = model_display_name(model);
    if detailed {
        println!("   Display Name: {display_name}");
        println!("   Description: {}", model_description(model));
        println!("   Full Name: {}", model.name.dimmed());

        if let Some(family) = model.family() {
            println!("   Family: {}", family.green());
        }

        println!("   Methods: {}", model.supported_generation_methods.join(", ").yellow());

        if let Some(input_limit) = model.input_token_limit {
            println!("   Input Token Limit: {}", input_limit.to_string().magenta());
        }

        if let Some(output_limit) = model.output_token_limit {
            println!("   Output Token Limit: {}", output_limit.to_string().magenta());
        }

        if let Some(temp_range) = &model.temperature {
            println!("   Temperature Range: {:.1} - {:.1}", temp_range.min, temp_range.max);
        }
    } else {
        println!("   {}", display_name.dimmed());
        if model.is_gemini() {
            println!("   {}", "🎯 Gemini Model".green());
        }
    }
}

pub async fn get(model_name: String) -> Result<()> {
    println!("{}", format!("Getting Model Details: {model_name}").bold().cyan());

    let spinner = progress::spinner("Fetching model details...");

    let config = Config::from_env()?;
    let client = VertexClient::new(config).await?;
    let models_api = ModelsApi::new(&client);
    let model = models_api.get_model(&model_name).await?;

    spinner.finish_and_clear();

    println!("{}", "Model Details".bold().green());
    println!("{}", "─".repeat(50));

    println!("Name: {}", model.short_name().cyan().bold());
    let display_name = model_display_name(&model);
    println!("Display Name: {display_name}");
    println!("Description: {}", model_description(&model));
    println!("Full Path: {}", model.name.dimmed());

    if let Some(version) = &model.version {
        println!("Version: {version}");
    }

    if let Some(family) = model.family() {
        println!("Family: {}", family.green());
    }

    println!("\n{}", "Capabilities:".bold());
    println!("Methods: {}", model.supported_generation_methods.join(", ").yellow());

    if model.supports_method("generateContent") {
        println!("  ✓ Content Generation");
    }
    if model.supports_method("streamGenerateContent") {
        println!("  ✓ Streaming Generation");
    }
    if model.supports_method("countTokens") {
        println!("  ✓ Token Counting");
    }

    if let Some(input_limit) = model.input_token_limit {
        println!("\nInput Token Limit: {}", input_limit.to_string().magenta().bold());
    }

    if let Some(output_limit) = model.output_token_limit {
        println!("Output Token Limit: {}", output_limit.to_string().magenta().bold());
    }

    if let Some(temp_range) = &model.temperature {
        println!("Temperature Range: {:.1} - {:.1}", temp_range.min, temp_range.max);
    }

    if let Some(top_p_range) = &model.top_p {
        println!("Top-P Range: {:.1} - {:.1}", top_p_range.min, top_p_range.max);
    }

    if let Some(top_k) = model.top_k {
        println!("Max Top-K: {top_k}");
    }

    if let Some(languages) = &model.supported_languages {
        println!("Supported Languages: {}", languages.join(", "));
    }

    Ok(())
}
