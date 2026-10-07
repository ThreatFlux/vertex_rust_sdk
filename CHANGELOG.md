# Changelog

All notable changes to `threatflux-vertex-rust-sdk` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `release.yml` writes the Windows archive's `.sha256` file with an LF line
  ending, like the Unix archives' files. Every `vertex-windows-amd64.zip.sha256`
  asset published so far (0.8.0 to 0.10.2) ends in CRLF, so `shasum -a 256 -c`
  and macOS `sha256sum -c` report the archive as missing; check one with
  `tr -d '\r' < vertex-windows-amd64.zip.sha256 | shasum -a 256 -c` (the hash
  itself is correct).

## [0.10.2] - 2026-10-06

Container images now ship for `linux/arm64` as well as `linux/amd64`. The SDK
API and behavior are unchanged from 0.10.1.

### Changed

- The Docker workflow publishes multi-platform images for `linux/amd64` and
  `linux/arm64`, matching rust-cicd-template. Images up to 0.10.1 were
  `linux/amd64` only. The `arm64` image is built under QEMU emulation, and
  before the image is signed the workflow checks that the pushed index holds
  every platform and runs each platform's CLI (`--version`, `--help`, and the
  non-root user check). Pull requests still build `linux/amd64` only. The
  `RUST_TEMPLATE_DOCKER_PLATFORMS` repository variable overrides the list.

### Security

- `release.yml` no longer has a `source_ref` dispatch input. Every job builds
  the commit the run started on: the pushed tag, or the branch or tag picked
  with `gh workflow run release.yml --ref <ref>`. A dispatch can no longer
  point the build, SBOM, and publish jobs at another revision, such as an
  unreviewed pull request head, while they run with the default branch's cache
  scope. To rebuild an existing tag, dispatch on that tag. Auto Release is
  unaffected: the App's tag push starts `release.yml` on the tag, and the
  `GITHUB_TOKEN` fallback dispatches on the tag without the input.

## [0.10.1] - 2026-10-06

A maintenance release covering the toolchain, dependencies, container image,
and release automation. The SDK API and behavior are unchanged from 0.10.0.

### Changed

- Updated the development toolchain and Docker builder to stable Rust 1.99.0,
  retaining the Rust 1.96.0 consumer MSRV and existing Cargo features.
- The Docker image builds on `rust:1.99.0-trixie` and runs on distroless
  `gcr.io/distroless/cc-debian13:nonroot`, both pinned by digest. The runtime
  has no shell or package manager and runs as uid/gid 65532 in `/data`. Its
  entrypoint is now the CLI itself, so arguments go straight to it
  (`docker run <image> --help`); `/usr/local/bin/vertex` links to the binary.
- Refreshed stable dependencies, transitive security fixes, immutable GitHub
  Actions pins, and development tool versions.
- Added worktree-aware repository hooks and a local gate matching hosted
  Clippy, feature powerset, MSRV, documentation, benchmark, and security checks.
- Removed the stale RSA advisory exemption; RSA is absent from the dependency
  graph.
- The release workflow resolves its source ref to one commit and builds,
  generates SBOMs for, and publishes that exact commit, matching the release
  tag even if a source branch moves mid-release. It refuses to run when the
  release tag already exists on a different commit or does not point to a
  commit. It creates a missing tag through the API and checks the tag on
  GitHub before publishing, so a concurrent run cannot publish under another
  commit's tag.
- Manual `release.yml` dispatches accept `dry_run`, which builds and packages
  every target, generates SBOMs, runs `cargo publish --dry-run`, and builds the
  Docker image without creating a tag or GitHub Release, uploading assets,
  publishing to crates.io, or pushing an image. A real release stops before
  tagging when the requested version differs from the `Cargo.toml` version; a
  dry run only warns. Manual `auto-release.yml` dispatches accept `dry_run` to
  report the next release without committing, tagging, or releasing.
- Releases publish to crates.io through trusted publishing: the publish job
  runs in the `crates-io` environment and exchanges its GitHub OIDC identity
  for a short-lived token instead of reading a registry token secret. A failed
  publish fails the release, and a version already on crates.io is skipped.
  The release workflow token is read-only except in the jobs that create the
  tag and release or upload assets.
- Automatic releases are cut with the organization's GitHub App token through
  `github_actions` v0.7.7. The App's release tag starts `release.yml` and
  `docker.yml` through their tag triggers instead of an explicit dispatch, so
  neither runs twice for one tag. Its release commit on `main` now also gets a
  branch Docker run, so the `main` and `latest` images follow the release.
  Docker push runs for the same commit wait for each other, so neither
  overwrites the short-SHA image tag the other is scanning.

### Added

- A current Vertex AI feature audit and staged implementation plan. The audit
  documents existing behavior and gaps; it does not add new provider features.

### Security

- The Docker runtime image moves off Debian 12 (bookworm), whose packages
  carried open Trivy findings with no Debian 12 fix, to distroless Debian 13
  (trixie). The CLI binary and SBOM are root-owned and read-only to the runtime
  user.

## [0.10.0] - 2026-08-12

These changes shipped as 0.10.0. The manifest was set to 0.9.0, but the
automated release bumped the minor version again for the `feat` commit, so
0.9.0 was never tagged or published.

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
