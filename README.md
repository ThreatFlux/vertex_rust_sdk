# ThreatFlux Vertex Rust SDK

[![Crates.io](https://img.shields.io/crates/v/threatflux-vertex-rust-sdk.svg)](https://crates.io/crates/threatflux-vertex-rust-sdk)
[![Documentation](https://docs.rs/threatflux-vertex-rust-sdk/badge.svg)](https://docs.rs/threatflux-vertex-rust-sdk)
[![MSRV](https://img.shields.io/badge/MSRV-1.96.0-orange.svg)](Cargo.toml)
[![CI](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/ci.yml)
[![Security](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/security.yml/badge.svg)](https://github.com/ThreatFlux/vertex_rust_sdk/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An async Rust client for generative AI APIs on Google Cloud Vertex AI. The
crate provides Gemini content generation, streaming, tools, embeddings, token
counting, chat helpers, context caching, Claude on Vertex, and optional command
line applications.

> [!IMPORTANT]
> This is a community-maintained ThreatFlux project. It is not an official
> Google, Google Cloud, Vertex AI, Gemini, Anthropic, or Claude SDK.

The crate is pre-1.0, so minor releases may include public API changes. Review
the [changelog](CHANGELOG.md) when upgrading.

## Why this SDK?

- One `VertexClient` for Google and Anthropic publisher model paths.
- Typed request, response, streaming, tool, safety, grounding, and cache data.
- Configurable timeouts and bounded retries for selected HTTP status codes.
- Explicit authentication providers for workload, service-account, and custom
  credential strategies.
- Three optional CLIs plus focused, runnable examples.
- Rust 1.96.0 minimum supported Rust version (MSRV), enforced in CI.

See the [API coverage guide](docs/api-coverage.md) for implemented operations
and known scope boundaries.

## Quick start

### Requirements

- Rust 1.96.0 or newer.
- A Google Cloud project with the Vertex AI API enabled and suitable IAM
  permissions.
- A model available to that project in the selected location. Model IDs and
  regional availability are controlled by the provider and can change
  independently of this crate.
- One of the authentication sources described in the
  [configuration guide](docs/configuration.md#authentication).

For a library-only application, add the crate without its default CLI features:

```toml
[dependencies]
threatflux-vertex-rust-sdk = { version = "0.6", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Configure a project, location, and model. The gcloud fallback uses the active
gcloud CLI identity, so verify that `gcloud auth print-access-token` succeeds:

```bash
gcloud auth login
gcloud auth print-access-token >/dev/null

export VERTEX_PROJECT_ID="your-project-id"
export VERTEX_REGION="us-central1"
export VERTEX_MODEL="a-model-available-in-that-location"
```

The following program is kept identical to
[`examples/quickstart.rs`](examples/quickstart.rs) and compiled in the
documentation workflow.

<!-- BEGIN QUICKSTART -->
```rust
use threatflux_vertex_rust_sdk::{config::Config, GenerateContentRequest, VertexClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let model = config.model.clone();
    let client = VertexClient::new(config).await?;

    let request = GenerateContentRequest::new("Explain why observability matters.");
    let response = client.generate_content(&model, &request).await?;

    if let Some(text) = response.text() {
        println!("{text}");
    }

    Ok(())
}
```
<!-- END QUICKSTART -->

Run it with:

```bash
cargo run --example quickstart --no-default-features
```

Client construction validates configuration but fetches credentials lazily on
the first API request. See [authentication and configuration](docs/configuration.md)
before deploying to production.

## API coverage

| Area | Primary surface | Status |
| --- | --- | --- |
| Content generation | `VertexClient::generate_content` | Implemented |
| Server-sent event streaming | `VertexClient::stream_generate_content` | Implemented |
| Tools and function calling | `GenerateContentRequest`, `FunctionBuilder`, `execute_function_calling_flow` | Implemented |
| Structured output, grounding, safety, and thinking request types | `types` module | Implemented |
| Embeddings | `VertexClient::embed`, `VertexClient::embeddings` | Implemented |
| Token counting | `VertexClient::count_tokens`, `count_text_tokens` | Implemented |
| Multi-turn chat helpers | `ChatConversation`, `chat_with_context`, `stream_chat` | Implemented |
| Context caching | `VertexClient::cache` | Implemented |
| Models and locations | `VertexClient::models` | Implemented; see discovery caveat below |
| Claude on Vertex | `claude_message`, `claude_stream` | Implemented |
| Command-line applications | `vertex`, `vertex-chat`, `vertex-test` | Optional `cli` feature |

`ModelsApi::get_gemini_models` returns a built-in snapshot; it is not live
service discovery. Prefer service-backed listing methods when current project
availability matters. Coverage describes SDK code, not entitlement or model
availability in a particular project or region.

The detailed [coverage matrix](docs/api-coverage.md) also lists intentionally
unsupported Vertex AI product areas.

## Common examples

Live examples are gated by the `examples` feature and require credentials,
project access, and a suitable model.

| Example | Demonstrates | Run command |
| --- | --- | --- |
| [`quickstart.rs`](examples/quickstart.rs) | Minimal environment-configured generation | `cargo run --example quickstart --no-default-features` |
| [`basic_generation.rs`](examples/basic_generation.rs) | Generation configuration and usage metadata | `cargo run --example basic_generation --features examples` |
| [`streaming/`](examples/streaming/) | Modular SSE streaming client | `cargo run --example streaming --features examples` |
| [`function_calling/`](examples/function_calling/) | Tool declarations and tool-result loop | `cargo run --example function_calling --features examples` |
| [`token_counting.rs`](examples/token_counting.rs) | Token counts for varied content | `cargo run --example token_counting --features examples` |
| [`chat.rs`](examples/chat.rs) | Interactive multi-turn chat | `cargo run --example chat --features examples` |
| [`context_caching/`](examples/context_caching/) | Cache lifecycle and reuse | `cargo run --example context_caching --features examples` |

Examples intentionally avoid promising a permanently available default model.
Check each example's environment variables before running it.

## Cargo features

| Feature | Default | Effect |
| --- | --- | --- |
| `default` | Yes | Enables `blocking` and `cli` |
| `blocking` | Yes | Forwards `reqwest/blocking`; the SDK's public request API remains async |
| `cli` | Yes | Builds `vertex`, `vertex-chat`, and `vertex-test` with their UI dependencies |
| `native-tls` | No | Adds reqwest's vendored native-TLS backend; does not disable rustls |
| `rustls-tls` | No | Compatibility feature forwarding `reqwest/rustls`; rustls is already enabled by the base dependency |
| `examples` | No | Enables the credentialed examples registered in `Cargo.toml` |
| `integration-tests` | No | Compiles the credentialed integration-test target |

For applications that only need the async library, prefer
`default-features = false`. Add `cli` only when installing or embedding the
command-line programs.

## Configuration and reliability

`Config::from_env()` recognizes project, location, model, timeout, retry,
publisher-location, API-version, debug, and base-URL settings. The
[configuration guide](docs/configuration.md) documents exact precedence,
authentication resolution, defaults, and security considerations.

Operational behavior worth knowing:

- The default request timeout is 60 seconds.
- `max_retries` defaults to three retries after the initial attempt.
- Retries apply to HTTP 429, 500, 502, 503, and 504 responses. Transport errors
  are returned immediately.
- Backoff honors numeric `Retry-After` seconds; otherwise it uses capped
  exponential delays without jitter.
- The HTTP client disables environment proxy discovery with `no_proxy()`.
- `VERTEX_BASE_URL` is intended for controlled test or proxy endpoints; never
  populate it from untrusted input.

The crate returns [`VertexError`](https://docs.rs/threatflux-vertex-rust-sdk/latest/threatflux_vertex_rust_sdk/error/enum.VertexError.html)
for authentication, configuration, HTTP, API, serialization, token, streaming,
and I/O failures. Callers should decide which application-level operations are
safe to retry; tool executions and other side effects may not be idempotent.

## Command-line applications

Install the optional binaries from crates.io:

```bash
cargo install --locked threatflux-vertex-rust-sdk --features cli
vertex --help
vertex-chat --help
vertex-test --help
```

Use runtime help as the authoritative command reference. [`CLI.md`](CLI.md)
provides a longer guide to the interactive `vertex-chat` binary.

## Documentation

- [API reference](https://docs.rs/threatflux-vertex-rust-sdk)
- [API coverage and boundaries](docs/api-coverage.md)
- [Authentication, configuration, retries, and security](docs/configuration.md)
- [Interactive chat CLI](CLI.md)
- [Contributing guide](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## Development

```bash
make ci-quick     # documentation contract, formatting, lint, and cargo check
make test         # all feature-enabled tests
make test-doc     # rustdoc examples
make docs         # rustdoc with warnings denied
```

`make docs-check` validates README feature/MSRV claims, the synchronized
quickstart, and local documentation links. See [CONTRIBUTING.md](CONTRIBUTING.md)
for the complete workflow.

## Support and security

Use [GitHub issues](https://github.com/ThreatFlux/vertex_rust_sdk/issues) for
reproducible bugs and focused feature requests. Do not include access tokens,
service-account JSON, prompts containing sensitive data, or provider response
bodies that may contain private content.

Report vulnerabilities privately using the process in [SECURITY.md](SECURITY.md).

## License

Licensed under the [MIT License](LICENSE).
