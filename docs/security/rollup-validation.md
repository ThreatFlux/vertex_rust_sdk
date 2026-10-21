# Stable dependency rollup validation

Validated locally on 2026-10-21. Base: main commit
`3b990a75b9cef5fc43dffa45ad8851078177998f`. See the
[review index](README.md) for verdicts and remediation plans.

## Refreshed container

Built the repository Dockerfile with `--platform linux/arm64` and tag
`vertex-stable-rollup:local`. CLI `--version`, `--help`, and the `vertex` symlink
entrypoint passed. The configured runtime user is `65532:65532`. The embedded
CycloneDX SBOM parsed successfully and contains 240 components. Package versions
below were confirmed directly from the final image's dpkg status fragments.

| Artifact | Digest |
| --- | --- |
| Local image index | `sha256:081023bf5f952c80b6331ac2ccc2562e029149b03154dd5956475db945224e7b` |
| arm64 manifest | `sha256:55c44c713689d01d8f88c8ead335a70f674cdec09ed413b937e65a4c9aac3ef4` |
| arm64 image config | `sha256:de771980338e30fcc18bb8d900fd58911885a66916882b3b83328e2866cb2f89` |

The local build preceded documentation-only additions; its application source,
lockfile, toolchain and Dockerfile match this update. Local images have not been
published or deployed. This is separate evidence from the original published
images inspected in the per-alert reviews.

## Full-severity vulnerability scan

Scanned a saved Docker image archive with Trivy **0.76.0**, vulnerability
scanning only, and `UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL` severities. The official
macOS ARM64 release archive SHA256 was verified against the release checksums.
Database schema 2, updated `2026-10-20T13:14:22.737606427Z`; scan completed
`2026-10-21T00:50:49-04:00`.

The image scan still reports **31 detections: 23 MEDIUM and 8 LOW**. Its
package/CVE pairs match the 31 original alerts. No HIGH or CRITICAL detection
was reported. Cargo-library scanning reported no vulnerabilities; Trivy warns
that third-party SBOMs can be inaccurate, so the separate Cargo audit remains
necessary.

Affected package versions remain:

- glibc/libc6: `2.41-12+deb13u4`.
- GCC runtime/base packages: `14.2.0-19`.
- zlib1g: `1:1.3.dfsg+really1.3.1-1+b1`.

The refreshed base digest and Rust/crate upgrades therefore do **not** establish
closure of the OS-package findings. The evidence-backed FP decisions are
documented in the reviews; the scanner results were not suppressed.

## Final amd64 verification

Rebuilt linux/amd64 after the feature powerset restored the complete manifest
and lockfile. CLI `--version`, `--help`, and the `vertex` symlink passed under
emulation. Runtime user is `65532:65532`; its embedded CycloneDX SBOM parsed
successfully and contains 242 components (target dependency counts differ).

| Artifact | Digest |
| --- | --- |
| Local amd64 image index | `sha256:a07999c0ec1b2ed4348c475ed48bacbd4df2e4153afad6b3486ac85f924b0691` |
| amd64 manifest | `sha256:32f50a6e602e4a64127729851a1685258c2c114c6f08496d40e2589d4135b3e3` |
| amd64 image config | `sha256:b20bb4ae3769fb5124440585b08886eaa94e642e7bb1aee19dd695edc0f94c5f` |

The final amd64 all-severity scan also reports 31 detections (23 MEDIUM,
8 LOW). Its CVE/package pairs exactly match the original exported alerts and
arm64 scan. Both refreshed images therefore retain the reviewed OS findings.

## Validation limits

The repository's full `make ci-local` gate passed: documentation contract,
formatting, exact hosted Clippy flags, 293 passing tests (11 ignored), all 81
feature combinations, Rust 1.96.0 MSRV, warnings-as-errors rustdoc, benchmark
compilation, Cargo audit with warnings denied, and Cargo deny's advisory,
license, source and dependency policies. `actionlint`, `git diff --check`, and
the final `cargo outdated --depth 100 --format json` check also passed; the
latter reports an empty update list within the resolved constraints.

No authenticated production API requests or exploit reproductions were run.
Hosted CI still supplies the operating-system test matrix and release-build
coverage. A passing filtered scan, Cargo audit or Rust test suite alone cannot
prove that a system-library finding has been remediated.
