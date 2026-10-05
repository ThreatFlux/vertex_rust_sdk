# API coverage and scope

This document maps the crate's public Rust surface to the operations it
implements. It is derived from the source tree and is intentionally separate
from provider model catalogs, quotas, and regional availability.

> [!NOTE]
> "Implemented" means the SDK contains a typed request path and response
> handling for the operation. It does not mean every model supports that
> operation or that a project is entitled to use it.

Last audited **2026-10-05**. The [feature audit and proposed implementation
plan](sdk-feature-audit.md) compares this surface with current provider schemas.
Several existing wire paths are incomplete or incorrect; the modernization
change documents those gaps without implementing the follow-up plan.

## Implemented surface

| Capability | Public entry points | Endpoint shape | Notes |
| --- | --- | --- | --- |
| Client construction | `VertexClient`, `VertexClientBuilder`, `Config` | N/A | Async client with timeout/retries/project/region settings; requests hard-code API versions and selected model names override locations |
| Authentication | `AuthProvider`, `from_env`, `EnvAuth`, `ServiceAccountAuth`, `ApplicationDefaultCredentials` | OAuth token and metadata endpoints | Bearer-token authentication; built-in ADC does not cover the standard local ADC file or all credential-file types; exact current precedence is in the [configuration guide](configuration.md#authentication) |
| Content generation | `generate_content`, `GenerateContentRequest` | `:generateContent` | Typed legacy request/response subset; signed thoughts, blocked responses, current metadata, and multiple text parts have gaps; custom request metadata stays local |
| Content streaming | `stream_generate_content`, `ChatStream`, `SseParser` | `:streamGenerateContent?alt=sse` | Typed SSE path; split UTF-8 and skipped buffered events can corrupt or delay/lose data; finality currently uses presence of usage metadata |
| Function calling | `FunctionBuilder`, `generate_with_functions`, `execute_function_calling_flow` | `:generateContent` | Explicit caller callback with one follow-up; thought signatures and call IDs are not preserved; no bounded multi-turn runner |
| Structured output | `GenerationConfig`, response-schema types, `GenerateContentResponse::json_as` | `:generateContent` | Request schema support and typed response decoding |
| Grounding and code execution | Types in `types::grounding` and `types::code_execution` | `:generateContent` | Code execution representation; Search helper uses deprecated retrieval and grounding response nesting does not match current candidate schemas |
| Embeddings | `EmbeddingRequest`, `EmbeddingTaskType`, `VertexClient::embed`, `EmbeddingsApi` | `:predict` | Text predict with output dimensionality; task key is incorrectly `taskType`; arbitrary instances do not satisfy the single-input limit of gemini-embedding-001 |
| Token counting | `CountTokensRequest`, `count_tokens`, `count_text_tokens` | `:countTokens` | Gemini contents-only count and local approximate helpers; no tools/system-instruction projection, computeTokens, or Claude count operation |
| Chat helpers | `ChatMessage`, `ChatConversation`, `chat_impl`, `chat_with_context`, `stream_chat` | Generation endpoints | Text conversation helpers; system role is put into contents instead of systemInstruction and structured replay is not retained |
| Context caching | `CachedContent`, `CacheApi`, `VertexClient::cache` | `cachedContents` resources | All five methods exist; create has no model field, resource decoding requires input-only contents, and pagination tokens are not encoded |
| Model information | `ModelsApi`, `ModelDescriptor`, `ModelInfo` | Publisher model and project location resources | Service listing/lookup and static convenience tables; publisher DTO omits real capability metadata; full paths discard project/location |
| Claude on Vertex | `claude::MessageRequest`, `claude_message`, `claude_stream` | `:rawPredict` and `:streamRawPredict` | Hosted message/SSE path with selected beta headers; tool-choice, effort, citations, content unions, and multi-region routing have current-schema gaps |
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
Full resource input currently discards its project and location; the client
rebuilds them from configuration and model-specific routing. It must not be
treated as preserving the supplied resource identity or location constraint.

`ModelsApi::list_models`, `list_models_for_publisher`, `get_model`, and
`list_locations` make service-backed requests. By contrast,
`ModelsApi::get_gemini_models` returns a built-in snapshot from the crate and
must not be treated as current service discovery.
The Vertex publisher schema does not provide the Developer API's
`supportedGenerationMethods` field. An empty SDK capability list is therefore
unknown, not proof that a publisher model lacks generation support.

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

Vertex Cloud API keys/express mode and Gemini Developer API keys are distinct
surfaces. Developer API Files operations are not implied by Vertex multimodal
parts. Claude direct-API features unavailable on Vertex are not missing Vertex
operations; see the [audit's scope boundaries](sdk-feature-audit.md#scope-boundaries-and-historical-claims).

Open a focused [feature request](https://github.com/ThreatFlux/vertex_rust_sdk/issues/new?template=feature_request.yml)
when a missing operation belongs in this SDK. Include the provider endpoint,
request/response shape, intended error behavior, and a test strategy.

## Historical analysis

[`gap_analysis_mar_2026.md`](gap_analysis_mar_2026.md) is retained as a dated
project-planning snapshot. It is not the current support contract. This page,
the public Rust API, and compile-tested examples are authoritative for SDK
coverage.
