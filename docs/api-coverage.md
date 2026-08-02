# API coverage and scope

This document maps the crate's public Rust surface to the operations it
implements. It is derived from the source tree and is intentionally separate
from provider model catalogs, quotas, and regional availability.

> [!NOTE]
> "Implemented" means the SDK contains a typed request path and response
> handling for the operation. It does not mean every model supports that
> operation or that a project is entitled to use it.

## Implemented surface

| Capability | Public entry points | Endpoint shape | Notes |
| --- | --- | --- | --- |
| Client construction | `VertexClient`, `VertexClientBuilder`, `Config` | N/A | Async client with configurable timeout, retries, project, region, and publisher overrides |
| Authentication | `AuthProvider`, `from_env`, `EnvAuth`, `ServiceAccountAuth`, `ApplicationDefaultCredentials` | OAuth token and metadata endpoints | Bearer-token authentication; exact precedence is in the [configuration guide](configuration.md#authentication) |
| Content generation | `generate_content`, `GenerateContentRequest` | `:generateContent` | Typed content, generation, safety, tools, metadata, cache, and grounding request fields |
| Content streaming | `stream_generate_content`, `ChatStream`, `SseParser` | `:streamGenerateContent?alt=sse` | Server-sent event parsing and typed stream chunks |
| Function calling | `FunctionBuilder`, `generate_with_functions`, `execute_function_calling_flow` | `:generateContent` | Tool declarations, function calls, and function responses |
| Structured output | `GenerationConfig`, response-schema types, `GenerateContentResponse::json_as` | `:generateContent` | Request schema support and typed response decoding |
| Grounding and code execution | Types in `types::grounding` and `types::code_execution` | `:generateContent` | Request/response representation; actual support is model-dependent |
| Embeddings | `EmbeddingRequest`, `EmbeddingTaskType`, `VertexClient::embed`, `EmbeddingsApi` | `:predict` | Single or batched instances and optional output dimensionality |
| Token counting | `CountTokensRequest`, `count_tokens`, `count_text_tokens` | `:countTokens` | Service-backed count plus local estimate helpers |
| Chat helpers | `ChatMessage`, `ChatConversation`, `chat_impl`, `chat_with_context`, `stream_chat` | Generation endpoints | Conversation assembly on top of content generation |
| Context caching | `CachedContent`, `CacheApi`, `VertexClient::cache` | `cachedContents` resources | Create, get, list, update TTL, and delete |
| Model information | `ModelsApi`, `ModelDescriptor`, `ModelInfo` | Publisher model and project location resources | Listing, lookup, filtering, and model path normalization |
| Claude on Vertex | `claude::MessageRequest`, `claude_message`, `claude_stream` | `:rawPredict` and `:streamRawPredict` | Anthropic message and stream types, tool definitions, and selected beta headers |
| Media request helpers | `media`, inline/file data types | Generation endpoints | MIME classification and typed multimodal request parts |
| Command-line applications | `vertex`, `vertex-chat`, `vertex-test` | Multiple | Available with the `cli` Cargo feature |

The implementation lives primarily in [`src/client.rs`](../src/client.rs),
[`src/api/`](../src/api/), [`src/cache.rs`](../src/cache.rs), and
[`src/claude/`](../src/claude/). Public re-exports are defined in
[`src/lib.rs`](../src/lib.rs).

## Model identifiers and discovery

`ModelDescriptor::parse` accepts these forms:

- A short model ID, with Google inferred unless the name resembles a Claude
  family name.
- `models/{model}`.
- `{publisher}/{model}` or `{publisher}:{model}`.
- `publishers/{publisher}/models/{model}`.
- A full
  `projects/{project}/locations/{location}/publishers/{publisher}/models/{model}`
  resource path.

Publisher inference is a convenience, not validation against a live provider
catalog. Use an explicit publisher path when inference would be ambiguous.

`ModelsApi::list_models`, `list_models_for_publisher`, `get_model`, and
`list_locations` make service-backed requests. By contrast,
`ModelsApi::get_gemini_models` returns a built-in snapshot from the crate and
must not be treated as current service discovery.

## Scope boundaries

The following Vertex AI product areas do not currently have first-class client
operations in this crate:

| Area | Current status |
| --- | --- |
| Batch prediction and batch generation jobs | Not implemented |
| Model tuning, training, and pipelines | Not implemented |
| Endpoint deployment and custom model serving | Not implemented |
| Vector Search, RAG Engine, and evaluation services | Not implemented |
| Live or bidirectional real-time sessions | Not implemented |
| Dedicated image, video, speech, and music generation APIs | Not implemented; multimodal content parts are available for supported generation calls |
| API-key authentication | Not implemented; the client sends OAuth bearer tokens |
| Synchronous SDK facade | Not implemented; public network operations are async even when the `blocking` feature is enabled |

Open a focused [feature request](https://github.com/ThreatFlux/vertex_rust_sdk/issues/new?template=feature_request.yml)
when a missing operation belongs in this SDK. Include the provider endpoint,
request/response shape, intended error behavior, and a test strategy.

## Historical analysis

[`gap_analysis_mar_2026.md`](gap_analysis_mar_2026.md) is retained as a dated
project-planning snapshot. It is not the current support contract. This page,
the public Rust API, and compile-tested examples are authoritative for SDK
coverage.
