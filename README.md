<div align="center">

# ThreatFlux Vertex Rust SDK

[![CI](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/ci.yml)
[![Security](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/security.yml/badge.svg)](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/threatflux-vertex-rust-sdk.svg)](https://crates.io/crates/threatflux-vertex-rust-sdk)
[![Documentation](https://docs.rs/threatflux-vertex-rust-sdk/badge.svg)](https://docs.rs/threatflux-vertex-rust-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.95.0%2B-orange.svg)](https://www.rust-lang.org)

**A comprehensive Rust SDK for Google Cloud Vertex AI — Gemini models, Claude on Vertex, streaming, function calling, and more.**

[Quick Start](#quick-start) · [Features](#features) · [Documentation](https://docs.rs/threatflux-vertex-rust-sdk) · [Contributing](CONTRIBUTING.md)

</div>

---

Async Rust client for the Google Cloud Vertex AI API built on `reqwest` and `tokio`. Supports Gemini content generation, Claude on Vertex (Anthropic), streaming, function calling, embeddings, token counting, and multi-turn chat — with optional CLI binaries.

## Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
- [Installation](#installation)
- [Authentication](#authentication)
- [Usage](#usage)
- [Supported Models](#supported-models)
- [CLI Usage](#cli-usage)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Error Handling](#error-handling)
- [Examples](#examples)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)

## Quick Start

```rust
use threatflux_vertex_rust_sdk::{VertexClient, GenerateContentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = VertexClient::new("your-project-id", "us-central1").await?;

    let request = GenerateContentRequest::new("Why is the sky blue?");
    let response = client.generate_content("gemini-2.5-flash", &request).await?;

    if let Some(text) = response.text() {
        println!("Response: {}", text);
    }

    Ok(())
}
```

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Features

- **Authentication** — OAuth2, Service Account, and Application Default Credentials
- **Gemini Models** — Content generation with streaming and non-streaming support
- **Claude on Vertex** — Anthropic Claude models via Vertex AI (Haiku, Sonnet, Opus)
- **Function Calling** — Tool/function calling capabilities
- **Embeddings** — Text and vision embeddings with task types and dimensionality control
- **Token Counting** — Count tokens in content before sending requests
- **Chat** — Multi-turn conversations with context management
- **Streaming** — SSE-based streaming for both Gemini and Claude responses
- **CLI** — Command-line tools for quick interaction (`vertex`, `vertex-chat`, `vertex-test`)
- **Async/Await** — Built with Tokio for high-performance async operations

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
threatflux-vertex-rust-sdk = "0.6"
tokio = { version = "1.0", features = ["full"] }
```

### Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `blocking` | Yes | Enables `reqwest/blocking` for synchronous usage |
| `cli` | Yes | Enables CLI binaries (`clap`, `colored`, `indicatif`) |
| `native-tls` | No | Use native TLS backend |
| `rustls-tls` | No | Use rustls TLS backend |
| `integration-tests` | No | Gates integration test compilation |
| `examples` | No | Gates example compilation |

<p align="right"><a href="#table-of-contents">back to top</a></p>

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

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Usage

### Content Generation

```rust
use threatflux_vertex_rust_sdk::{VertexClient, GenerateContentRequest};

let client = VertexClient::new("your-project-id", "us-central1").await?;
let request = GenerateContentRequest::new("Explain quantum computing");
let response = client.generate_content("gemini-2.5-flash", &request).await?;
```

### Streaming Responses

```rust
use tokio_stream::StreamExt;

let request = GenerateContentRequest::new("Tell me a long story");
let mut stream = client.stream_generate_content("gemini-2.5-flash", &request).await?;

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
        temperature: Some(0.0),
        ..GenerationConfig::default()
    });

let response = client.generate_content("gemini-2.5-flash", &request).await?;

for function_call in response.function_calls() {
    println!("Function: {} Args: {:?}", function_call.name, function_call.args);
}
```

### Claude on Vertex (Anthropic)

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

// Non-streaming
let request = MessageRequest::new()
    .max_tokens(1024)
    .system("You are a concise assistant")
    .add_user_message("Summarise the latest release notes.")
    .add_web_search_tool(WebSearchTool::new().with_max_uses(Some(3)));

let response = client.claude_message("claude-haiku-4-5", &request).await?;
println!("Claude: {}", response.text());

// Streaming
let streaming_request = MessageRequest::new()
    .with_param("memory", json!({"store": true}))
    .add_user_message("Draft an executive summary.");

let mut stream = client.claude_stream("claude-haiku-4-5", &streaming_request).await?;

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

### Chat Conversations

```rust
use threatflux_vertex_rust_sdk::{ChatMessage, Content};

let messages = vec![
    ChatMessage::system("You are a helpful assistant."),
    ChatMessage::user("Hello!"),
];

let response = client.chat("gemini-2.5-flash", messages).await?;
println!("Assistant: {}", response);
```

### Token Counting

```rust
use threatflux_vertex_rust_sdk::CountTokensRequest;

let request = CountTokensRequest::new("Count tokens in this text");
let response = client.count_tokens("gemini-2.5-flash", &request).await?;
println!("Token count: {}", response.total_tokens);
```

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Supported Models

### Gemini

| Model | Context | Output | Notes |
|-------|---------|--------|-------|
| `gemini-3.1-pro` | 2M | 64K | Preview |
| `gemini-3.1-flash` | 1M | 8K | Preview |
| `gemini-3.1-flash-lite` | 1M | 8K | Preview |
| `gemini-3-pro-preview` | 1M | 64K | |
| `gemini-2.5-pro` | 2M | 8K | |
| `gemini-2.5-flash` | 1M | 8K | |
| `gemini-embedding-001` | — | — | Text/vision embeddings |

### Claude on Vertex (Anthropic)

| Model | Context | Output |
|-------|---------|--------|
| `claude-opus-4-6` | 1M | 128K |
| `claude-sonnet-4-6` | 1M | 64K |
| `claude-sonnet-4-5` | 200K | 64K |
| `claude-haiku-4-5` | 200K | 64K |
| `claude-opus-4-5` | 200K | 32K |

### Supported Locations

`us-central1` · `us-east1` · `europe-west1` · `asia-southeast1` · `global` (Claude)

See [`docs/gap_analysis_mar_2026.md`](docs/gap_analysis_mar_2026.md) for the latest Vertex AI model/support coverage.

<p align="right"><a href="#table-of-contents">back to top</a></p>

## CLI Usage

Install the CLI:

```bash
cargo install threatflux-vertex-rust-sdk --features=cli
```

Set up authentication:

```bash
gcloud auth application-default login
# Or: export GOOGLE_APPLICATION_CREDENTIALS="/path/to/key.json"
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

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `GOOGLE_CLOUD_PROJECT` | Default project ID |
| `GOOGLE_CLOUD_LOCATION` | Default location (defaults to `us-central1`) |
| `VERTEX_REGION` / `VERTEX_LOCATION` | Region override (e.g. `global` for Claude) |
| `GOOGLE_APPLICATION_CREDENTIALS` | Path to service account key file |

### Build & Test

```bash
make build          # cargo build --all-features
make test           # cargo test --all-features
make lint           # cargo clippy --all-features --all-targets -- -D warnings
make lint-strict    # clippy with pedantic/nursery/cargo lints
make fmt            # cargo fmt --all
make fmt-check      # format check without modifying
make ci             # full CI: fmt-check, lint, test, test-features, docs, security
make ci-quick       # quick CI: fmt-check, lint, check
```

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Architecture

```text
src/
├── lib.rs               # Public re-exports
├── client.rs            # VertexClient + VertexClientBuilder
├── auth.rs              # AuthProvider trait, ADC, service account
├── config.rs            # SDK configuration
├── error.rs             # VertexError (thiserror)
├── models.rs            # Model metadata and constants
├── builders.rs          # Request builder helpers
├── streaming.rs         # SSE parser + ChatStream
├── api/                 # API trait impls on VertexClient
│   ├── generate.rs      #   Content generation
│   ├── stream.rs        #   Streaming generation
│   ├── chat.rs          #   Multi-turn chat
│   ├── claude.rs        #   Claude on Vertex (Anthropic)
│   ├── embeddings.rs    #   Text/vision embeddings
│   ├── tokens.rs        #   Token counting
│   ├── functions.rs     #   Function calling
│   └── models.rs        #   Model listing/metadata
├── claude/              # Claude-specific types and streaming
├── types/               # Shared request/response structs
├── chat_core/           # Interactive CLI chat engine
└── bin/                 # CLI binaries (vertex, vertex-test, vertex-chat)
```

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Error Handling

The SDK provides comprehensive error types via `VertexError`:

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

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Examples

See the [`examples/`](examples/) directory for comprehensive examples:

| Example | Description |
|---------|-------------|
| [`basic_generation.rs`](examples/basic_generation.rs) | Simple content generation |
| [`streaming/`](examples/streaming/) | Streaming responses |
| [`function_calling/`](examples/function_calling/) | Tool/function calling |
| [`chat.rs`](examples/chat.rs) | Multi-turn conversations |
| [`token_counting.rs`](examples/token_counting.rs) | Token counting |
| [`context_caching/`](examples/context_caching/) | Context caching |

Run an example:

```bash
cargo run --example basic_generation --features examples
```

<p align="right"><a href="#table-of-contents">back to top</a></p>

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, commit conventions, and PR guidelines.

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting instructions.

## License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">

Built and maintained by [ThreatFlux](https://github.com/ThreatFlux)

</div>
