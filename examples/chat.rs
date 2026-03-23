//! Interactive chat example

#[path = "common/mod.rs"]
mod common;

use threatflux_vertex_rust_sdk::chat_core::{
    config::ChatConfig,
    io::{ConsoleInput, ConsoleOutput},
    run_chat,
    service::VertexChatService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    common::init_logging().ok();

    let environment = common::ExampleEnvironment::from_env()?;
    let client = environment.new_client().await?;

    let config = ChatConfig::with_defaults(environment.project_id, environment.location);
    let service = VertexChatService::from_client(client);
    let mut input = ConsoleInput;
    let mut output = ConsoleOutput::default();

    run_chat(config, &service, &mut input, &mut output).await?;
    Ok(())
}
