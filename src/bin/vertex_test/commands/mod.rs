mod all;
mod auth;
mod check;
mod function_call;
mod gemini2_flash;
mod generate;
mod locations;
mod models;
mod stream;

use crate::vertex_test::cli::{Cli, Commands};
use anyhow::Result;

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Auth => auth::run().await,
        Commands::Generate { prompt, model, prompt_words } => {
            generate::run(prompt, prompt_words, model).await
        }
        Commands::Stream { prompt, model, input_files } => {
            stream::run(prompt, model, input_files).await
        }
        Commands::Function { prompt, model } => function_call::run(prompt, model).await,
        Commands::All { model } => all::run(model).await,
        Commands::ListModels { gemini_only, detailed } => models::list(gemini_only, detailed).await,
        Commands::GetModel { model } => models::get(model).await,
        Commands::ListLocations => locations::list().await,
        Commands::TestGemini2Flash { prompt } => gemini2_flash::run(prompt).await,
        Commands::Check => check::run(),
    }
}

#[cfg(test)]
mod tests {
    use crate::vertex_test::attachments::InputFileArg;

    #[test]
    fn can_use_input_file_type() {
        let _value: InputFileArg =
            "/tmp/data.txt::text/plain".parse().expect("should parse input file arg");
    }
}
