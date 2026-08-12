# Changelog

All notable changes to `threatflux-vertex-rust-sdk` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Gemini 3.1 Pro, Flash, and Flash Lite model metadata and aliases in `model_info.rs`.
- `gemini-embedding-001` model metadata for the Vertex text embeddings API.
- `EmbeddingsApi` module (`src/api/embeddings.rs`) wrapping the Vertex predict endpoint with
  request/response types, batch support, task types, and output dimensionality control.
- Public re-exports for all embedding types from the crate root.
- Gap analysis document (`docs/gap_analysis_mar_2026.md`) tracking Vertex AI coverage.
- Claude Opus 4.6 and Sonnet 4.6 model metadata (1M context, 128K/64K output, bare IDs).
- Claude Opus 4.1, Sonnet 4, and Opus 4 model metadata with version overrides.
- Adaptive thinking support (`ThinkingConfig::adaptive()`) with effort levels for Claude 4.6+.
- Thinking display omission (`with_display_omitted()`) for faster streaming.
- Structured output support (`OutputConfig` with JSON schema) for Claude 4.5+.
- Citations configuration (`CitationsConfig`, `enable_citations()`) for document content.
- `ContentBlock::Thinking` variant for extended thinking response blocks.
- `StopReason::ModelContextWindowExceeded` variant.
- `WebSearchToolType::WebSearchV2` and `WebSearchTool::new_v2()` for 4.6 models with dynamic
  filtering.
- `ToolChoice::None` variant to disable tool use.
- Thinking delta support in `ContentBlockDelta` for streaming.
- `Usage.cache_creation_input_tokens: Option<u32>` and `Usage.cache_read_input_tokens: Option<u32>`
  for Anthropic prompt-cache accounting on Vertex Claude responses.
- `ContentBlockDelta.signature: Option<String>` field carrying the cryptographic signature chunks
  emitted after extended-thinking text during SSE streaming.

### Changed

- Updated all README examples from retired `gemini-2.0-flash-001` to `gemini-2.5-flash`.
- Refreshed Supported Models section in README with full model lineup including Claude variants.
- Registered Gemini 3.1 models as global-location models in client routing.
- Claude 4.6 models use bare IDs (no `@date` suffix) in version override logic.
- Claude 4.1/4.0 models get appropriate `@date` version suffixes.
- All new Claude model families route through global location.
- Web search beta header auto-detection distinguishes v1 and v2 tool variants.

## [0.7.0] - 2026-08-10

### Added

- Claude 5 model support for Vertex: model metadata and aliases in `model_info.rs`, descriptor entries in
  `model_descriptor.rs`, and Claude 5 coverage in the `vertex_test` binary configuration.

### Changed

- Client routing recognizes the Claude 5 model family.
- Overhauled the Vertex SDK onboarding documentation.

## [0.4.0] - 2026-03-23

### Changed

- Extracted the SDK from `ThreatFlux/core` into a standalone repository with dedicated CI, release, and security
  automation.
- Updated crate metadata and repository references for standalone publishing.

### Fixed

- Switched RSA key generation to `rsa::rand_core::OsRng` for compatibility with the current RSA crate stack.

## [0.3.2] - 2025-12-01

### Added

- Initial tracked release within `ThreatFlux/core`.
