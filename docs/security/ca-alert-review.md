# CA trust-store alert 362

Reviewed 2026-10-21 after hosted scanning published
[alert 362](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/362),
rule `DSA-6572-1`, against package metadata version `20250419`.

## Decision: valid against the original published image

The [Debian advisory](https://www.debian.org/security/2026/dsa-6572) removes
emSign ECC Root CA - G3 and AAA Certificate Services and identifies
`20250419+deb13u1` as fixed. The original published amd64 image,
`ghcr.io/threatflux/vertex_rust_sdk@sha256:13578551123923484a9ed91632f25ff81b1c272093c407da7baf0927bf5ca9d5`,
still contains the distrusted emSign root. Its CA bundle SHA256 is
`714d457d580922dbf1d0be8bd35ba236a842b50b0072ae791582a19adef772a5`.
The final-image CA regression check correctly rejects that original bundle.
An affected trust anchor can permit certificates issued under a root that the
supplier has distrusted; exploitability depends on certificate issuance and
an application's TLS peer. A valid package/trust finding does not prove a
successful attack on this SDK.

## Artifact scope and dismissal correction

The refreshed distroless images already contained fixed trust data, despite
retaining the old package version. Both architectures' bundles have 148 roots,
222073 bytes and SHA256
`e9c7b63ee2623386e9b0a3ee32f499d01a8ea23dc83b8383ddff9f8e749543b7`.
Neither distrusted subject is present. The sole trust file is
`/etc/ssl/certs/ca-certificates.crt`, explicitly selected by `SSL_CERT_FILE`;
there are no alternate root files under `/usr/share/ca-certificates`.
An independently exported official `rust:1.99.1-trixie` image, pinned at
`sha256:7e9eb23ac694dda816c76933ab63b2703e6becbf3b10da90970124656788d47b`,
contains installed fixed package `20250419+deb13u1` and a byte-identical bundle.
Complete certificate DER fingerprint sets also match.

The alert was briefly dismissed based on that refreshed artifact's metadata
mismatch. Inspecting the original published image demonstrated the narrower
scope of that evidence, so the alert was reopened. It must remain visible
until replacement scans resolve it; the ten unrelated, proven FPs remain
dismissed. The earlier artifact-specific evidence does not justify a global FP
disposition for older published images.

## Implemented fix and validation

A pinned Debian trixie trust-store stage installs the maintained CA package
from signed repositories, requires version at least `20250419+deb13u1`, and
copies both its actual bundle and exact installed-package record into the
runtime. This updates trust data and provenance together, rather than merely
changing a version label. Only architecture-independent trust data is copied;
no Debian shared runtime is introduced.

The image validator rejects the two distrusted CA subjects. Both architectures
must also pass trusted HTTPS, untrusted-certificate rejection and hostname
mismatch rejection, then fresh all-severity Trivy scans. See
[final validation](remediation.md#artifact-verification) for the results.

Hosted CodeQL also highlighted a TLS-version default in the HTTPS fixture.
The fixture now explicitly sets minimum TLS 1.2; that warning was fixed in
source. It does not change the SDK's production TLS configuration.
