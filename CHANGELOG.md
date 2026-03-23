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

### Changed

- Updated all README examples from retired `gemini-2.0-flash-001` to `gemini-2.5-flash`.
- Refreshed Supported Models section in README with full model lineup including Claude variants.
- Registered Gemini 3.1 models as global-location models in client routing.

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
