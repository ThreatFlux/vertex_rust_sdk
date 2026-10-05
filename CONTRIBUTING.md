# Contributing to ThreatFlux Vertex Rust SDK

Contributions are welcome! This guide covers development setup, commit conventions, and PR guidelines.

## Getting Started

1. Fork the repository
2. Clone your fork: `gh repo fork ThreatFlux/vertex_rust_sdk --clone`
3. Create a branch: `git checkout -b feat/your-change`
4. Make your changes
5. Install repository hooks: `make hooks-install`
6. Run checks: `make ci-local`
7. Open a Pull Request

## Development Setup

```bash
# Consumer MSRV is Rust 1.96.0; repository development uses Rust 1.99.0
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run full CI locally
make build
make dev-setup
make hooks-install
make ci-local
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
- [ ] Hosted CI flags, feature powerset, MSRV, benchmarks, and security pass (`make ci-local`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow conventions

## Code Style

- **Clippy**: `make lint-ci` matches the hosted pedantic and nursery gate; `make lint-strict` additionally checks cargo lints (see `Makefile`)
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

Tests that require a real cloud project are ignored by default. Keep them ignored
for routine checks; `make ci-local` does not authorize billable API calls.

`make hooks-install` supports linked worktrees and preserves foreign hooks.
`make ci-local` uses the pinned toolchain, the actual consumer MSRV, the hosted
Clippy flags, the full feature powerset, strict Rustdoc, compiled benchmarks,
and security checks. Feature checks restore the committed lockfile after
`cargo-hack` temporarily removes development dependencies.

## Security Issues

Do not open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md).
