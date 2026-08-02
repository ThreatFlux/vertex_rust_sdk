# Contributing to ThreatFlux Vertex Rust SDK

Contributions are welcome! This guide covers development setup, commit conventions, and PR guidelines.

## Getting Started

1. Fork the repository
2. Clone your fork: `gh repo fork ThreatFlux/vertex_rust_sdk --clone`
3. Create a branch: `git checkout -b feat/your-change`
4. Make your changes
5. Run checks: `make ci`
6. Open a Pull Request

## Development Setup

```bash
# Install Rust 1.96.0+ (the repository toolchain is pinned separately)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run full CI locally
make build
make ci
```

## Commit Guidelines

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat`: new feature
- `fix`: bug fix
- `docs`: documentation only
- `refactor`: code refactoring
- `test`: adding or updating tests
- `chore`: maintenance

## Pull Request Process

- Use a conventional-commit title
- Explain what changed and why
- Add tests or validation where applicable
- Update documentation when public API changes

### PR Checklist

- [ ] Code follows project style (`make fmt`)
- [ ] All tests pass (`make test`)
- [ ] Linting passes (`make lint`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow conventions

## Code Style

- **Clippy**: Strict — pedantic, nursery, and cargo lints are all denied (see `Makefile`)
- **Formatting**: `rustfmt` with `use_small_heuristics = "Max"` (see `rustfmt.toml`)
- **Errors**: Use `VertexError` via `thiserror`; avoid `unwrap()` in library code
- **Public API**: Re-export new public types from `src/lib.rs`

## Documentation changes

- Keep README capability claims tied to exported code or tested behavior.
- Update `docs/api-coverage.md` when adding or removing an operation.
- Update `docs/configuration.md` when authentication, environment variables,
  timeouts, or retry behavior changes.
- Edit `examples/quickstart.rs` and copy it exactly between the README's
  `BEGIN QUICKSTART` and `END QUICKSTART` markers.
- Run `make docs-check` and `make test-doc` before opening a pull request.

The documentation contract checks the README MSRV and Cargo feature table
against `Cargo.toml`, verifies release-safe installation guidance and the
synchronized quickstart, and resolves local Markdown links.

## Running Tests

```bash
make test               # Unit tests (all features)
make test-features      # Test feature combinations
cargo test --doc        # Doc tests only
make docs-check         # README contract and local links

# Integration tests (require GCP credentials)
cargo test --all-features --features integration-tests
```

## Security Issues

Do not open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md).
