# CA trust-store alert 362

Reviewed 2026-10-21 after hosted scanning published
[alert 362](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/362),
rule `DSA-6572-1`, against the package metadata version `20250419`.

## Decision: false positive for the inspected runtime data

The [Debian advisory](https://www.debian.org/security/2026/dsa-6572) removes
two distrusted roots and identifies `20250419+deb13u1` as fixed. The runtime's
package metadata still identifies `20250419`, but its effective trust store
already matches the fixed package's trust store **byte for byte**.

Evidence gathered from the final arm64 and amd64 images:

- The sole file under `/etc/ssl/certs` is `ca-certificates.crt`; no alternate
  root files or hashed certificate symlinks are present. There is no separate
  `/usr/share/ca-certificates` tree to supply the removed roots.
- Both final runtime bundles contain **222073 bytes**, SHA256
  `e9c7b63ee2623386e9b0a3ee32f499d01a8ea23dc83b8383ddff9f8e749543b7`.
- An independent fixed reference was exported from official pinned image
  `rust:1.99.1-trixie@sha256:7e9eb23ac694dda816c76933ab63b2703e6becbf3b10da90970124656788d47b`.
  Its dpkg status identifies `ca-certificates 20250419+deb13u1`; apt's signed
  stable-security repository also identifies that version as the current
  installed/candidate version. Its trust bundle has the identical length and
  complete SHA256 above. The certificate DER fingerprint sets also match.
- The earlier rolled-up glibc image has the same bundle hash too, confirming
  that this was a data/metadata FP in the original alerted artifact as well.
- All **148** CA certificates were parsed. Neither the advisory's emSign ECC
  Root CA - G3 nor the AAA Certificate Services root is in the trusted set.
  Other unrelated COMODO and emSign roots are not grounds for this alert.
- The container explicitly selects this bundle through `SSL_CERT_FILE`, and
  the HTTPS tests prove trust enforcement rather than disabling verification.

This is an effective-data/version-attribution mismatch, not a decision that
vulnerable code is merely unreachable. No stale version was relabeled and no
trust material was removed solely to silence a scanner. The actual trust data
is the same as the fixed reference. The scoped GitHub dismissal records this
evidence; a future bundle change must be reassessed rather than globally
ignoring the advisory.

## Regression checks and fresh scan interpretation

The final-image validator parses the exported trust bundle with OpenSSL and
rejects the two distrusted root subjects. Both architecture images also pass
trusted HTTPS, untrusted-certificate rejection and hostname-mismatch rejection.

Explicitly downloading a fresh vulnerability database produced schema 2,
updated `2026-10-21T01:19:53.032404989Z`. With that database, Trivy 0.76.0 reports
**one raw finding on each architecture: this confirmed CA metadata FP**. The
original 31 CVE/package pairs remain absent. Earlier cached-database scans
showing zero raw findings are retained as historical evidence, not the latest
scan result. CI now pins the same Trivy version used for local validation.

Hosted CodeQL also highlighted a TLS-version default in the HTTPS fixture.
The fixture now explicitly sets minimum TLS 1.2; that warning was addressed in
source rather than dismissed. It does not change the SDK's production TLS
configuration.
