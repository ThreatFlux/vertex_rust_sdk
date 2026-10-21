# Container vulnerability remediation

Implemented 2026-10-21 in the consolidated dependency update. This supersedes
the original [review plan](README.md) and the earlier glibc-image scan results.

## Fix for all valid package findings

The published runtime previously contained vulnerable glibc and zlib packages
even though the CLI did not need zlib or most glibc interfaces. The Dockerfile
now builds the CLI against musl with explicit static CRT linkage and statically
vendored OpenSSL, preserving `--all-features` and PIE address randomization.
It copies a separately verified
static tini into pinned distroless/static, alongside CA certificates, nonroot
account data and the application SBOM. It ships no glibc, zlib, GCC shared
runtime, dynamic ELF loader, shell or package manager.

Rust 1.99.1's default self-contained musl archive is older than the maintained
Alpine system archive. The build explicitly sets `link-self-contained=no` and
`linker=cc`, requires the link map's libc archive to resolve to
`/usr/lib/libc.a`, and ships compiler identity, archive/init hashes and the
selected archive path in `native-build.txt`. A successful static ELF check or
empty scanner report alone would not catch selection of an older bundled CRT.
Only Rust's LLVM unwinder is retained in an isolated archive search directory,
so its older bundled libc cannot be selected accidentally. See the pinned
[compiler musl CRT selection logic](https://github.com/rust-lang/rust/blob/58cf8f9c35b355199727627d07ae34a4e2846d67/compiler/rustc_codegen_ssa/src/back/link.rs#L2298)
and [stable linker option documentation](https://doc.rust-lang.org/rustc/codegen-options/index.html#link-self-contained).

Both executable builds fail if ELF inspection finds an interpreter or shared
dependency; the CLI must also remain a position-independent executable. The final-image checker independently verifies the executables,
architecture, root ownership, nonroot runtime account, CA bundle and SBOM, and
rejects shared-library files and missing system-musl provenance. A known original glibc-linked executable was
correctly rejected by the ELF checker as a negative control.

This removes the affected components for **all 20 confirmed TP alerts**:

| Component removed | Alert IDs |
| --- | --- |
| glibc/libc6 | 23, 24, 25, 26, 27, 28, 29, 152, 153, 247, 249, 258, 263, 264, 265, 266, 325, 328, 329 |
| zlib1g | 98 |

The version-range disagreement for zlib alert **316 remains undetermined**;
fresh upstream discussion does not justify dismissing it as a proven FP.
Removing zlib also removes the package implicated by this alert.

Static linking is not a claim that embedded musl/OpenSSL/AWS-LC code can never
have vulnerabilities. Cargo advisories, locked native dependency provenance,
image scans and behavioral regression checks remain necessary. The build
toolchain stays in the builder rather than shipping in the runtime.

## Dismissed invalid findings

The following ten alerts were dismissed through GitHub's code-scanning API as
`false positive`, each with its own explanation and a link to the reviewed
artifact evidence. These dismissals apply to the original inspected amd64/arm64
images and must be reassessed if affected components or other targets appear.

| Alerts | Why invalid | Evidence |
| --- | --- | --- |
| 358 | Actual aligned-new implementations use unaffected `posix_memalign` path | [GCC G1](gcc-zlib-alert-review.md#g1-aligned-new-integer-overflow) |
| 349, 340, 336 | Other GCC binary packages do not implement the affected libstdc++ allocator | [GCC G1](gcc-zlib-alert-review.md#g1-aligned-new-integer-overflow) |
| 357, 348, 339, 335 | PBDS header/template flaw was mapped to runtime/metadata packages without that component | [GCC G2](gcc-zlib-alert-review.md#g2-pbds-binary-heap-use-after-free) |
| 337 | Power8 implementation is not in the published amd64/arm64 targets | [glibc review](glibc-alert-review.md) |
| 317 | Affected nscd component is absent from both published runtime filesystems | [glibc review](glibc-alert-review.md) |

No valid finding was dismissed as a false positive or accepted risk. GitHub's
main-branch TP alerts remain open until this PR is merged and the replacement
main-branch image scan records their absence. No production image has been
deployed by this change, and no merge is implied by local verification.

## Permanent regression checks

`make docker-check` runs both checks below against the image from
`make docker-build`. The Docker workflow runs them on its scan platform and
each additional published architecture; release dry runs also execute them.

- `scripts/check_static_image.py`: inspect the actual exported final image and
  both ELF executables, rather than trusting build flags or package metadata.
- `scripts/smoke_container_https.py`: use an ephemeral local HTTPS fixture and
  generated test-only RSA key/CA. Exercise bearer-token authentication,
  service-account JWT creation and signature verification, token exchange,
  hostname lookup and API response parsing. Require trusted HTTPS success,
  rejection of untrusted certificates and wrong hostnames, read-only/nonroot
  execution, and tini forwarding SIGTERM while a request is in flight.

The runtime explicitly sets `SSL_CERT_FILE` for the existing all-features native
TLS backend. Musl's resolver replaces glibc resolver/NSS behavior; glibc NSS
plugins and gcloud are not included. Auth/TLS tests use local fixtures, never
real Google credentials or production API calls.

## Artifact verification

Both final **linux/arm64 and linux/amd64** images passed static-PIE linkage,
nonroot ownership/entrypoint, native-musl provenance, CLI version/help/symlink,
embedded SBOM and all local HTTPS/auth/signal tests. Both embed Alpine
**musl 1.2.6-r2**, with Rust 1.99.1's LLVM unwinder isolated from its older CRT.
See [arm64 native provenance](native-build-arm64.txt) and
[amd64 native provenance](native-build-amd64.txt), exported from the actual
final images.

Trivy **0.76.0** all-severity scans (`UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL`, OS and
Cargo libraries) found **zero vulnerabilities on both architectures**. No CVE
ignore file or advisory exemption was added. Database schema 2 was updated
`2026-10-20T13:14:22.737606427Z`. The original scans reported all 31 CVE/package
pairs with the same scanner/database; their disappearance matches the verified
removal of the affected runtime implementations.

| Platform | Local Docker image ID | Scan findings |
| --- | --- | --- |
| linux/arm64 | `sha256:711bc9451d19c3f68ff070afd70b48e1210e5e5e4b8e673bb8c2aba0edb98dd6` | 0 |
| linux/amd64 | `sha256:084c712378d2c3f01c9b86ac7321f43c9e0d2a849808f2e07bcc4a1777067c6a` | 0 |

The native ELF/link-map checks are important: a static binary with an old
embedded CRT could show zero OS-package findings too. These images additionally
prove the selected maintained system archive and retain executable address
randomization. Original glibc-artifact results remain in the
[initial validation record](rollup-validation.md) for traceability.

The complete `make ci-local` gate passed again: 293 tests (11 ignored), all 81
feature combinations, Rust 1.96.0 MSRV, exact hosted Clippy flags, formatting,
warnings-as-errors documentation, benchmark compilation, Cargo audit and
cargo-deny. Final documentation, Python syntax, workflow and diff checks also
passed. Image behavior tests used generated local credentials and certificates;
no production requests were made. Local images have not been published.
