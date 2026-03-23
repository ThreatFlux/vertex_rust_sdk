use crate::vertex_test::attachments::InputFileArg;
use clap::{Parser, Subcommand};

/// Vertex AI SDK Test CLI
#[derive(Parser)]
#[command(name = "vertex-test")]
#[command(about = "Test Vertex AI SDK functionality", long_about = None)]
pub struct Cli {
    /// Enable debug output
    #[arg(short, long)]
    pub debug: bool,

    /// Override the maximum number of retries for retryable HTTP errors
    #[arg(long, value_name = "N", global = true)]
    pub max_retries: Option<u32>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Test authentication
    Auth,

    /// Test non-streaming generation
    Generate {
        /// Prompt to send (may also be provided positionally)
        #[arg(short, long)]
        prompt: Option<String>,

        /// Model to use
        #[arg(short, long, default_value = "gemini-3-pro-preview")]
        model: String,

        /// Positional prompt words
        #[arg(value_name = "PROMPT", trailing_var_arg = true)]
        prompt_words: Vec<String>,
    },

    /// Test streaming generation
    Stream {
        /// Prompt to send
        #[arg(short, long, default_value = "Write a short story about AI")]
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-3-pro-preview")]
        model: String,

        /// Attach a local file as inline data (use PATH or `PATH::MIME_TYPE`)
        #[arg(long = "input-file", value_name = "PATH[::MIME]", action = clap::ArgAction::Append)]
        input_files: Vec<InputFileArg>,
    },

    /// Test function/tool calling
    Function {
        /// Initial prompt
        #[arg(short, long, default_value = "What's the weather in New York and San Francisco?")]
        prompt: String,

        /// Model to use
        #[arg(short, long, default_value = "gemini-3-pro-preview")]
        model: String,
    },

    /// Run all tests
    All {
        /// Model to use
        #[arg(short, long, default_value = "gemini-3-pro-preview")]
        model: String,
    },

    /// List available models
    ListModels {
        /// Show only Gemini models
        #[arg(long)]
        gemini_only: bool,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Get details for a specific model
    GetModel {
        /// Model name (e.g., "gemini-2.0-flash-001")
        model: String,
    },

    /// List available regions/locations
    ListLocations,

    /// Test gemini-2.0-flash specifically
    TestGemini2Flash {
        /// Prompt to send
        #[arg(short, long, default_value = "Hello, what model are you?")]
        prompt: String,
    },

    /// Check environment configuration
    Check,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_expectations() {
        let cli = Cli::parse_from(["vertex-test", "auth"]);
        matches!(cli.command, Commands::Auth);

        let generate = Cli::parse_from(["vertex-test", "generate"]);
        if let Commands::Generate { model, .. } = generate.command {
            assert_eq!(model, "gemini-3-pro-preview");
        } else {
            panic!("expected generate command");
        }

        let stream = Cli::parse_from(["vertex-test", "stream"]);
        if let Commands::Stream { prompt, model, .. } = stream.command {
            assert_eq!(prompt, "Write a short story about AI");
            assert_eq!(model, "gemini-3-pro-preview");
        } else {
            panic!("expected stream command");
        }
    }
}
