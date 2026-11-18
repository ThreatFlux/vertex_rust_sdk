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
