# Maintained Vertex timestamps

The next breaking SDK release replaces chrono with Jiff 1.0.1. Cache creation,
update and expiry fields and Claude `MessageResponse::created_at` use
`threatflux_vertex_rust_sdk::Timestamp`. Construct values with `Timestamp::now()`,
parse RFC3339 strings, or use fallible `Timestamp::from_second`.

JSON stays RFC3339. Offset instants and nanoseconds are preserved; formatting
uses canonical UTC and fractional precision. Optional fields retain their
existing omission behavior. Jiff accepts a lexical leap second 60 and normalizes
it to 59, preserving fractional digits. Invalid dates and missing offsets fail.
JWT claims remain numeric Unix seconds, and the CLI keeps its UTC display format.
Jiff's `as_second()` truncates negative fractional instants toward zero; code
that relied on chrono's floored `timestamp()` result before the Unix epoch must
handle that rounding explicitly. The timestamp instant and RFC3339 JSON retain
their original subsecond precision.

Provider-controlled token lifetimes use checked arithmetic. An unrepresentable
lifetime returns an error before a cached token is replaced. Refresh-margin
comparisons use elapsed durations rather than potentially overflowing timestamp
subtraction. Existing zero and negative lifetime behavior is retained.

The unused `gcp_auth` manifest dependency was removed: no source, test, example
or public re-export referenced it. The implemented environment-key,
service-account-key, access-token, gcloud and metadata-server paths remain.
No feature or credential mechanism is removed. The lockfile contains no chrono,
and no advisory exception is added.

GitHub owns runtime/HTTP/serialization/MSRV/security tests and release checks.
Fresh Countermeasure consumers must adopt the published breaking version and
its public timestamp type after all configured publication channels qualify.

The release preview selects 1.0.0 for this breaking change from 0.11.5. The
automatic release controller uses the reviewed proposal mode of the SHA-pinned
shared v0.7.13 workflow, rather than committing its version bump directly to
main. Exact-main CI/security must pass, then the version proposal must pass its
own checks and independent review before merge, tagging and publication. A
read-only preview does not publish a crate, native binary or container.
