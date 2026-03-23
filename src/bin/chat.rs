//! Interactive chat CLI for testing Vertex AI SDK

#[path = "chat/config.rs"]
mod config;

use anyhow::Result;
use clap::Parser;
use config::Cli;
use threatflux_vertex_rust_sdk::chat_core::{
    config::ChatConfig, io::ConsoleInput, io::ConsoleOutput, run_chat, service::VertexChatService,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = ChatConfig::try_from(cli)?;
    config.init_logging()?;

    let service = VertexChatService::connect(&config).await?;
    let mut input = ConsoleInput;
    let mut output = ConsoleOutput::default();

    run_chat(config, &service, &mut input, &mut output).await
}
