# Changelog

All notable changes to `threatflux-vertex-rust-sdk` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
