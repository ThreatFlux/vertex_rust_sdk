# Vertex AI model & feature gap analysis (March 2026)

This note captures the latest Vertex AI model lineup and highlights gaps between upstream capabilities and the current SDK surface. Sources are linked for each new model/feature.

## Current SDK coverage (0.4.x)

- Gemini 2.5 Flash, Gemini 2.5 Pro
- Gemini 3 Pro Preview
- Gemini 3.1 Pro, Gemini 3.1 Flash, Gemini 3.1 Flash Lite (Preview)
- Claude 4.5 Sonnet, Haiku, Opus
- Gemini Embedding 001 (text/vision embeddings via `EmbeddingsApi`)
- Multimodal input (text, image/video/pdf via inline or file parts), streaming, thinking mode, function calling, grounding, code execution, context caching

## Latest Vertex AI models & features

- **Gemini 3.1 Pro (Preview)** — up to a 2M token context window; successor to Gemini 3 Pro Preview and recommended for long-context workloads.[^1][^2]
- **Gemini 3.1 Flash / Flash Lite (Preview)** — latency-optimized 1M context variants for high-volume or cost-sensitive flows.[^1][^3]
- **Lifecycle guidance** — Google recommends migrating from 2.5 to 3.x models as they become generally available.[^4]
- **Embeddings** — `gemini-embedding-001` supersedes prior `text-embedding-004`/`005` models in the Vertex text embeddings API.[^5]

## Resolved gaps

1. **Model metadata** (resolved): `src/model_info.rs` now tracks Gemini 3.1 Pro, Flash, Flash Lite, and `gemini-embedding-001` with context/output limits and aliases.
2. **Quick start defaults** (resolved): README examples updated from the retired `gemini-2.0` family to `gemini-2.5-flash`.
3. **Embeddings client** (resolved): `EmbeddingsApi` module (`src/api/embeddings.rs`) wraps the Vertex predict endpoint for `gemini-embedding-001` with request/response types, batch support, task types, and output dimensionality control.

## Remaining gaps

1. **Vector/RAG helpers**: There is no convenience integration for Vertex AI Vector Search/RAG engine. Consider thin helpers (index upsert/query) paired with embeddings once the embeddings API has been validated in production.

The above items keep the SDK aligned with the March 2026 Vertex AI surface while preserving existing behavior for Gemini 2.5/3.0 users.

<!-- Sources -->
[^1]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/models/gemini/3-1-pro
[^2]: https://ai.google.dev/gemini-api/docs/models/gemini-3.1-pro-preview
[^3]: https://ai.google.dev/gemini-api/docs/models
[^4]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/migrate
[^5]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/model-reference/text-embeddings-api
