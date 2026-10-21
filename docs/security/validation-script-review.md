# Container validation script review

Reviewed 2026-10-21 against Codacy GitHub check run `119059337772` on PR [105](https://github.com/ThreatFlux/vertex_rust_sdk/pull/105). Source evidence was all 44 annotations returned by `gh api repos/ThreatFlux/vertex_rust_sdk/check-runs/119059337772/annotations --paginate` for commit `8e6096a430e836756b2ee130244e1e44397ca57f`. This document records a code audit and fixes; it does not claim any annotation was dismissed in the Codacy dashboard.

## Valid findings fixed

Four annotations identified excessive length and complexity in the two scripts' `main` methods: each had cyclomatic complexity 22; the static checker had 61 lines and the HTTPS fixture had 91 lines, above the configured 50-line limit. Refactored the methods into focused helpers for ELF/configuration checks, CA package/trust checks, filesystem export, certificate generation, token exchange, TLS cases, and process shutdown. The resulting `main` methods occupy 17 and 7 physical lines, respectively. The longest helper occupies 22 lines, and all functions have at most seven explicit branch nodes. This structural count is a local sanity check, not a claim that Codacy has already rerun its complexity calculation.

The static checker still verifies the app and tini ELF architecture, absence of interpreter/shared dependencies, app PIE, root ownership, no writable/privileged executable modes, no unexpected ELF files/shared libraries, nonroot account and expected entrypoint, explicit CA bundle, distrusted-root removal, the installed CA supplier package record and its refreshed version, application SBOM, and system-musl linker provenance. The HTTPS fixture still verifies token authentication, actual RS256 signature and service-account token exchange, API parsing, trusted success, certificate-specific untrusted/hostname failures, read-only operation, and tini TERM forwarding. Refactoring preserved the existing uncommitted CA supplier record validation.

As additional input hardening, Docker `create`/`run` commands now separate options from the image with `--`, and `--platform` accepts only the amd64/arm64 platforms supported by the ELF checker.

## Audited false positives

The remaining 40 annotations are security audit heuristics without a demonstrated vulnerability in this fixture. Counts below cover every remaining annotation; duplicate tools' reports are counted separately.

| Annotation group | Count | Audit evidence and disposition |
| --- | ---: | --- |
| Subprocess module import warnings | 2 | Each script intentionally invokes Docker/OpenSSL to inspect images or exercise real container TLS. Importing the module does not execute input. FP. |
| Generic subprocess untrusted-input audit warnings | 20 | Every invocation supplies an argument list with a fixed `docker` or `openssl` executable and fixed subcommand. None uses a shell, `eval`, or executable text derived from HTTP/user input. Docker IDs are opaque CLI outputs. OpenSSL verifies the received JWT signature as data; fixture paths are created in a secure temporary directory. FP. |
| “Python possesses many mechanisms to invoke an external executable” | 2 | These identify intentional Docker inspection calls, not a demonstrated injection path. List arguments and `shell=False` behavior preserve argument boundaries. FP. |
| Dynamic/non-static subprocess argv warnings | 12 | Six invocations each received both a user-controlled-data warning and a non-static-string warning: two `check_output` call sites and four `run` call sites. Their variable data is the operator/CI-selected image/platform, Docker container IDs, or fixture-generated paths. Running the selected image is the explicit purpose of this tool and requires the caller's existing Docker authority. No remote input selects an executable. The added `--` image boundary also prevents an option-like image argument from becoming Docker flags. FP. |
| All-interface bind | 1 | The temporary HTTPS test server must bind `0.0.0.0` for containers to reach the Linux bridge/Docker Desktop host through `host-gateway`; loopback-only binding breaks that path. It serves only fabricated token/API responses with a fresh test certificate and receives no real cloud credentials. Its OS-assigned port exists only during the check and is shut down in `finally`. This is intentional test connectivity, not a production listener. FP. |
| Insecure temporary-file/directory heuristic | 1 | Original annotated line 98 is Docker's `--tmpfs /tmp:rw,nosuid,noexec` option, which creates a private container tmpfs; it does not create a predictable host temporary file. Host fixture files live under `tempfile.TemporaryDirectory`, with secure random allocation and automatic cleanup. FP. |
| Google service-account/secret detections | 2 | The matched JSON has a fabricated `fixture@example.invalid` identity and a newly generated ephemeral RSA key. It is constructed at runtime for the mock token endpoint, contains no issued Google credential, and is removed with the temporary directory. Both regex matches are FP; no real credential is committed. |

Total: 4 valid maintainability findings fixed + 40 audited false positives = 44 annotations. No global rule exclusions or scanner suppressions were added.

## Validation

`python3 -m py_compile scripts/check_static_image.py scripts/smoke_container_https.py` passed. Both refactored scripts passed against `vertex-final:arm64` using `--platform linux/arm64`:

- Static runtime checks, including the CA supplier package record, trust-root removal, PIE, both static ELF files, and system-musl provenance, passed.
- Real HTTPS fixture passed trusted token/API requests, RS256 service-account authentication, explicit self-signed-certificate and hostname-mismatch rejection, read-only/nonroot operation, and TERM forwarding.

The same refactored scripts subsequently passed against `vertex-final:amd64` with `--platform linux/amd64`, including all TLS/authentication/signal cases. Fresh all-severity Trivy scans report zero findings on both final architectures; see [final artifacts](remediation.md#artifact-verification).
