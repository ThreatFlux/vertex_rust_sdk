# Code-scanning review and remediation plan

Reviewed 2026-10-21 by three independent review agents, with a combined coverage
check. Scope: all **31 open alerts** exported from GitHub for main commit
`3b990a75b9cef5fc43dffa45ad8851078177998f`. All were container OS-package
findings; none was a source-code or Cargo advisory. That initial review did not
dismiss alerts. See the [implemented remediation and current dispositions](remediation.md)
for the static runtime fix and ten subsequent evidence-backed dismissals.
The [subsequent CA alert review](ca-alert-review.md) documents one additional
valid trust-store finding on the original published image, found after the
original 31-alert export and fixed by importing maintained CA data/provenance.
The [validation script review](validation-script-review.md) also accounts for
all subsequent Codacy annotations, including four maintainability fixes.

## Decisions

| Review | Alerts | Result |
| --- | --- | --- |
| [Modern glibc](glibc-alert-review.md) | 14 | 12 TPs; 2 scoped FPs (Power8-only code and absent nscd component) |
| [GCC and zlib](gcc-zlib-alert-review.md) | 10 | 8 GCC component/target FPs; 1 undetermined zlib finding; 1 zlib TP |
| [Historical glibc](historical-alert-review.md) | 7 | 7 valid package detections, without demonstrated SDK exploitation |

Totals: **20 TP package detections, 10 scoped FPs, 1 undetermined finding**. A TP
package detection is not proof of an exploitable SDK call path. The reports
record attack prerequisites, source/package evidence, platform limits, and a
fix or mitigation plan with validation for every TP. The disputed zlib finding
remains open because upstream source evidence conflicts with Debian's current
affected-version classification. The architecture-specific glibc decision must
be revisited if additional platforms are published.

## Original prioritized plan

The [static runtime remediation](remediation.md) implements component removal
for every TP and the undetermined zlib finding. The original plan below explains
the alternatives considered when stable Debian package fixes were unavailable.

1. **Resolver alert 325:** verify deployment-controlled resolver search domains
   and `LOCALDOMAIN`, then adopt a supported stable runtime with the glibc fix
   when available. Exercise SDK DNS resolution with the upstream regression
   fixture. This is the most plausible SDK path identified in the glibc review.
2. **zlib alert 98:** evaluate a maintained runtime containing only the actual
   executable dependencies, or a supported vendor zlib package carrying both
   the CRC combine fix and the later gzip writer fix. Validate both architectures
   and both regressions; upgrading to plain zlib 1.3.2 is insufficient.
3. **Other glibc TPs:** follow each report's trigger-specific mitigation and
   regression plan. Debian 13 currently has no stable fixed package for these
   reported issues. Refreshing Rust or the base digest does not itself fix
   glibc. Historical vendor-disputed issues require documented applicability
   or risk decisions rather than an invented fixed version.
4. **FP handling:** preserve evidence for the exact image, package and platform;
   obtain vendor clarification for the undetermined finding before dismissal.
   Ten confirmed FPs were subsequently dismissed with individual explanations.
   No blanket CVE ignores or advisory exemptions were added.
5. **Artifact validation:** rebuild, export and scan each published architecture
   with current vulnerability data at every severity. Check the actual package
   versions, ELF dependencies, user, entrypoint, TLS/auth/DNS behavior and CLI
   smoke tests. Resolve findings only against replacement artifact evidence.

## Dependency consolidation

This update supersedes Dependabot PRs
[102](https://github.com/ThreatFlux/vertex_rust_sdk/pull/102),
[103](https://github.com/ThreatFlux/vertex_rust_sdk/pull/103), and
[104](https://github.com/ThreatFlux/vertex_rust_sdk/pull/104).

- Stable Rust is **1.99.1**, verified against the official
  [Rust channel manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml).
  Toolchain, CI, release builds, Makefile, Docker builder and current setup docs
  use it. The consumer MSRV stays 1.96.0 and is tested separately.
- All direct and development dependency requirements were checked against the
  [crates.io sparse index](https://index.crates.io/config.json), excluding yanked
  and prerelease versions. Updated Tokio to 1.54.0, serde_json to 1.0.154, UUID
  to 1.28.0 and TOML to 1.1.8. The remaining requirements already name the latest
  stable releases. Cargo resolves transitive dependencies within upstream
  constraints; incompatible transitive major versions are not forced by patches.
- Refreshed the lockfile with `cargo update`, immutable Actions pins (including
  TruffleHog beyond the existing PR), and both Docker base image digests.
  Repository Cargo tool pins were also checked and already name current stable
  releases. No application release version was changed manually.
- Trivy SARIF now includes all severities, preserving visibility of the medium
  and low findings reviewed here. Reporting visibility is not remediation.

The individual reports describe evidence from the original published artifacts;
they do not claim that a later build has fixed unresolved system vulnerabilities.
