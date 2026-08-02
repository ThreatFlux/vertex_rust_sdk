# ThreatFlux README standards

These standards describe the documentation contract used by this repository.
They are useful defaults for other Rust SDKs, but provider-specific claims must
always be derived from each project's code and tests.

## Required content

A public SDK README should make these facts discoverable without requiring a
reader to inspect source code:

1. Project name, concise purpose, maintenance status, and affiliation status.
2. Package, generated API docs, MSRV, CI, security, and license badges.
3. Prerequisites and a minimal quickstart that is compiled in CI.
4. A code-backed API coverage summary with explicit scope boundaries.
5. Every Cargo feature, including defaults and non-obvious limitations.
6. Authentication and configuration precedence.
7. Timeout, retry, proxy, error, and secret-handling behavior.
8. Runnable examples and commands.
9. Contribution, support, vulnerability-reporting, changelog, and license links.

## Accuracy rules

- Describe SDK implementation separately from provider availability,
  entitlement, quota, and regional support.
- Avoid absolute terms such as "all APIs" or "fully supported" unless a testable
  contract proves them.
- Do not publish a static model table as an availability guarantee. Provider
  catalogs change independently of crate releases.
- Call out compatibility features that do not expose a corresponding public
  SDK mode. For example, forwarding `reqwest/blocking` does not make an async
  SDK synchronous.
- Document credential discovery in the same order implemented by the code.
- Link deeper operational details instead of allowing the README to become an
  unmaintainable manual.

## Machine-checked contract

Run:

```bash
make docs-check
```

The checker verifies that:

- README MSRV claims match `package.rust-version`.
- The dependency snippet matches the crate's current major/minor version.
- The Cargo feature table matches `[features]` exactly.
- The README quickstart is identical to `examples/quickstart.rs`.
- Required affiliation and navigation text remains present.
- Obsolete API and authentication snippets do not return.
- Local Markdown links resolve inside the repository.
- Template placeholders have not leaked into published files.

The documentation workflow also lints maintained Markdown, checks external
links, compiles the quickstart on the MSRV and stable Rust, runs doctests, and
builds rustdoc with warnings denied.

## Historical documents

Dated planning notes may be excluded from style checks when fixing them would
rewrite the historical record. They must carry a visible historical banner and
link to the current support contract.
