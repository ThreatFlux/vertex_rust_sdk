# ThreatFlux Vertex Rust SDK

A comprehensive Rust SDK for Google Cloud Vertex AI API, providing access to Gemini models and other AI services.

[![Crates.io](https://img.shields.io/crates/v/threatflux-vertex-rust-sdk.svg)](https://crates.io/crates/threatflux-verte
x-rust-sdk)
[![Documentation](https://docs.rs/threatflux-vertex-rust-sdk/badge.svg)](https://docs.rs/threatflux-vertex-rust-sdk)

## Features

- **Authentication**: OAuth2, Service Account, and Application Default Credentials
- **Gemini Models**: Content generation with streaming and non-streaming support
- **Function Calling**: Tool/function calling capabilities
- **Token Counting**: Count tokens in content
- **Chat Completions**: Multi-turn conversations
- **CLI Interface**: Command-line tool for easy interaction
- **Async/Await**: Built with Tokio for high-performance async operations

## Repository

This repository publishes the standalone Vertex AI SDK extracted from the original MIT-licensed `ThreatFlux/core`
crate. It remains the shared Vertex/Gemini client used by the broader ThreatFlux stack, but it now has its own release
and CI lifecycle.

## Core packages

- `reqwest` + `tokio` + `tokio-stream`: async HTTP client and streaming foundations.
- `serde`, `serde_json`, `bytes`: typed request/response structures and payload handling.
- `gcp_auth`, `jsonwebtoken`, `base64`: Google Cloud authentication providers and JWT helpers.
- `anyhow`, `thiserror`, `futures`, `async-trait`: ergonomic error handling and trait-based abstractions.
- `clap`, `colored`, `indicatif`: optional CLI feature set backing the bundled binaries.

## Crate layout

```text
.
├── src/
│   ├── lib.rs               # Public client exports
│   ├── client.rs            # HTTP client + request builders
│   ├── auth.rs              # Service account + ADC helpers
│   ├── models.rs            # Generated request/response structs
│   ├── builders.rs          # Request builder helpers
│   ├── cache.rs             # Response caching
│   ├── config.rs            # SDK configuration
│   ├── error.rs             # Error types
│   ├── media.rs             # Media upload/download helpers
│   ├── model_descriptor.rs  # Model descriptor types
│   ├── model_info.rs        # Model metadata and info
│   ├── streaming.rs         # Streaming abstractions
│   ├── streaming_support.rs # Streaming support utilities
│   ├── api/                 # API endpoint modules
│   ├── chat_core/           # Core chat/conversation logic
│   ├── claude/              # Claude-on-Vertex helpers
│   ├── types/               # Shared enums + data models
│   └── bin/                 # CLI binaries (`vertex`, `vertex-test`, `vertex-chat`)
├── examples/                # End-to-end code samples
├── tests/                   # Integration tests (require --features integration-tests)
├── benches/                 # Criterion benchmarks
└── CLI.md                   # Additional CLI usage notes
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
threatflux-vertex-rust-sdk = "0.4"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

### Library Usage

```rust
use threatflux_vertex_rust_sdk::{VertexClient, GenerateContentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (uses Application Default Credentials)
    let client = VertexClient::new("your-project-id", "us-central1").await?;

    // Generate content
    let request = GenerateContentRequest::new("Why is the sky blue?");
    let response = client.generate_content("gemini-2.0-flash-001", &request).await?;

    if let Some(text) = response.text() {
        println!("Response: {}", text);
    }

    Ok(())
}
```

### CLI Usage

Install the CLI:

```bash
cargo install threatflux-vertex-rust-sdk --features=cli
```

Set up authentication:

```bash
# Using gcloud CLI
gcloud auth application-default login

# Or set service account key
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account-key.json"
```

Use the CLI:

```bash
# Generate content
vertex -p your-project-id generate "Explain quantum computing"

# Streaming generation
vertex -p your-project-id generate "Write a poem" --stream

# Count tokens
vertex -p your-project-id tokens "How many tokens is this?"

# Interactive chat
vertex -p your-project-id chat

# Test authentication
vertex -p your-project-id test auth
```

## Authentication

The SDK supports multiple authentication methods:

### Application Default Credentials (Recommended)

```bash
gcloud auth application-default login
```

### Service Account File

```rust
use threatflux_vertex_rust_sdk::{VertexClientBuilder, GcpAuthProvider};

let auth_provider = GcpAuthProvider::from_service_account_file("/path/to/key.json").await?;
let client = VertexClientBuilder::new("project-id", "us-central1")
    .with_auth_provider(Box::new(auth_provider))
    .build()
    .await?;
```

### Service Account JSON

```rust
let json_key = std::fs::read_to_string("/path/to/key.json")?;
let auth_provider = GcpAuthProvider::from_service_account_json(&json_key).await?;
let client = VertexClientBuilder::new("project-id", "us-central1")
    .with_auth_provider(Box::new(auth_provider))
    .build()
    .await?;
```

## Advanced Usage

### Streaming Responses

```rust
use tokio_stream::StreamExt;

let request = GenerateContentRequest::new("Tell me a long story");
let mut stream = client.stream_generate_content("gemini-2.0-flash-001", &request).await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(response) => {
            if let Some(text) = response.text() {
                print!("{}", text);
            }
        }
        Err(e) => eprintln!("Stream error: {}", e),
    }
}
```

### Function Calling

```rust
use threatflux_vertex_rust_sdk::{Tool, FunctionDeclaration, GenerationConfig};
use serde_json::json;

// Define a function
let weather_function = FunctionDeclaration {
    name: "get_weather".to_string(),
    description: "Get current weather for a location".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "City name"
            }
        },
        "required": ["location"]
    }),
};

let tool = Tool {
    function_declarations: vec![weather_function],
};

let request = GenerateContentRequest::new("What's the weather in Boston?")
    .with_tools(vec![tool])
    .with_generation_config(GenerationConfig {
        temperature: Some(0.0), // Use 0 for function calling
        ..GenerationConfig::default()
    });

let response = client.generate_content("gemini-2.0-flash-001", &request).await?;

// Check for function calls
for function_call in response.function_calls() {
    println!("Function called: {}", function_call.name);
    println!("Arguments: {:?}", function_call.args);
}
```

### Claude Haiku 4.5 on Vertex (Anthropic)

Claude Haiku 4.5 delivers near-frontier quality with 200k token input windows, 64k token outputs, and Anthropic's
extended thinking capabilities. It is a great default choice for latency-sensitive workflows and high-volume assistants,
while higher-tier Claude variants remain available for heavier reasoning.

```rust,no_run
use futures::StreamExt;
use serde_json::json;
use threatflux_vertex_rust_sdk::{claude::{MessageRequest, StreamEvent, WebSearchTool}, config::Config, VertexClient};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = Config {
    project_id: "project-id".into(),
    region: "global".into(),
    ..Config::default()
};

let client = VertexClient::new(config).await?;

// Non-streaming invocation
let request = MessageRequest::new()
    .max_tokens(1024)
    .system("You are a concise assistant")
    .add_user_message("Summarise the latest release notes.")
    .add_web_search_tool(WebSearchTool::new().with_max_uses(Some(3)));

let response = client
    .claude_message("claude-haiku-4-5", &request)
    .await?;

println!("Claude: {}", response.text());

// Responses include citation metadata when the web search tool is enabled
if let Some(content) = response.content.first() {
    println!("Citations: {:?}", content);
}

// Streaming with optional beta/memory parameters
let streaming_request = MessageRequest::new()
    .with_param("memory", json!({"store": true}))
    .add_user_message("Draft an executive summary for the attached PDF.");

let mut stream = client
    .claude_stream("claude-haiku-4-5", &streaming_request)
    .await?;

while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::ContentBlockDelta { delta, .. } => {
            if let Some(text) = delta.text {
                print!("{}", text);
            }
        }
        StreamEvent::MessageStop => println!("\n--- done ---"),
        _ => {}
    }
}
# Ok(())
# }
```

> Tip: integration tests accept short model names. Set `VERTEX_ANTHROPIC_MODELS=haiku-4.5,sonnet-4.5,opus-4.1` and
> optionally override specific revisions with environment variables such as
> `VERTEX_ANTHROPIC_MODEL_HAIKU_4_5=claude-haiku-4-5` or `VERTEX_ANTHROPIC_MODEL_SONNET_4_5=claude-sonnet-4-5`. You can
> still supply a fully versioned identifier if needed.

### Chat Conversations

```rust
use threatflux_vertex_rust_sdk::{ChatMessage, Content};

let messages = vec![
    ChatMessage::system("You are a helpful assistant."),
    ChatMessage::user("Hello!"),
];

let response = client.chat("gemini-2.0-flash-001", messages).await?;
println!("Assistant: {}", response);
```

### Token Counting

```rust
use threatflux_vertex_rust_sdk::CountTokensRequest;

let request = CountTokensRequest::new("Count tokens in this text");
let response = client.count_tokens("gemini-2.0-flash-001", &request).await?;
println!("Token count: {}", response.total_tokens);
```

## Configuration

### Environment Variables

- `GOOGLE_CLOUD_PROJECT`: Default project ID
- `GOOGLE_CLOUD_LOCATION`: Default location (defaults to `us-central1`)
- `VERTEX_REGION` / `VERTEX_LOCATION`: Region override for the SDK and CLI (e.g. `global` for Claude)
- `GOOGLE_APPLICATION_CREDENTIALS`: Path to service account key file

### Supported Models

- `gemini-2.0-flash-001` (latest)
- `gemini-1.5-pro`
- `gemini-1.5-flash`
- `gemini-pro`
- And more...

### Supported Locations

- `us-central1`
- `us-east1`
- `europe-west1`
- `asia-southeast1`
- And others...

## Error Handling

The SDK provides comprehensive error types:

```rust
use threatflux_vertex_rust_sdk::VertexError;

match client.generate_content("model", &request).await {
    Ok(response) => println!("Success: {:?}", response),
    Err(VertexError::Authentication { message }) => {
        eprintln!("Auth error: {}", message);
    }
    Err(VertexError::Http { status, message }) => {
        eprintln!("HTTP error {}: {}", status, message);
    }
    Err(VertexError::Api { code, message }) => {
        eprintln!("API error {}: {}", code, message);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Examples

See the `examples/` directory for more comprehensive examples:

- `examples/basic_generation.rs` - Simple content generation
- `examples/streaming/` - Streaming responses
- `examples/function_calling/` - Tool/function calling
- `examples/chat.rs` - Multi-turn conversations
- `examples/token_counting.rs` - Token counting

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Authors

- Wyatt Roersma
- Claude Code
- Codex

## Disclaimer

This is an unofficial SDK and is not affiliated with or endorsed by Google Cloud.
