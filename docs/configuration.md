# Authentication and configuration

This guide documents the behavior implemented by crate version 0.6.x. Google
Cloud project policy, IAM, model availability, quotas, and locations remain
provider-controlled.

## Minimal environment

The quickest explicit configuration is:

```bash
export VERTEX_PROJECT_ID="your-project-id"
export VERTEX_REGION="us-central1"
export VERTEX_MODEL="a-model-available-in-that-location"
```

`Config::from_env()` requires a project ID. Region and model have SDK defaults,
but applications should set them explicitly so deployment behavior does not
depend on a crate default.

## Authentication

Every API request uses an OAuth bearer token. API-key authentication is not
implemented.

### Resolution order

`auth::from_env()` selects an authentication provider in this exact order:

1. If `GCP_PRIVATE_KEY`, `GCP_CLIENT_EMAIL`, and `GCP_CLIENT_ID` are all set,
   `EnvAuth` signs a service-account JWT and exchanges it for a token.
2. If `GOOGLE_APPLICATION_CREDENTIALS` is set, `ServiceAccountAuth` reads that
   file as a service-account key JSON document.
3. Otherwise, `ApplicationDefaultCredentials` is constructed. When a token is
   first needed, it tries:

   1. `GOOGLE_ACCESS_TOKEN`.
   2. The active gcloud CLI identity through `gcloud auth print-access-token`.
   3. The Google Cloud metadata service for the attached default service
      account.

Partial inline service-account variables do not select `EnvAuth`; resolution
continues to the next source. Client construction is lazy: the fallback
provider can be constructed successfully even when no token source will work,
and the first API request then returns an authentication error.

### Local gcloud behavior

For this release, verify the authentication mechanism the SDK actually calls:

```bash
gcloud auth login
gcloud auth print-access-token >/dev/null
```

The `ApplicationDefaultCredentials` type does **not** currently read the local
ADC JSON file created by `gcloud auth application-default login`. Google Cloud
documents the distinction between gcloud CLI credentials and
[Application Default Credentials](https://cloud.google.com/docs/authentication/provide-credentials-adc).
Do not assume that configuring local ADC alone configures this SDK.

If another credential strategy is required, implement `AuthProvider` and pass
it with `VertexClientBuilder::with_auth_provider` or
`VertexClient::with_config_and_auth_provider`.

### Production guidance

- Prefer an attached, least-privilege service account on Google Cloud so the
  metadata-service fallback can provide short-lived tokens.
- Prefer a custom short-lived credential provider over downloadable service
  account keys outside Google Cloud.
- If a key file is unavoidable, store it outside the repository, restrict file
  permissions, rotate it, and set `GOOGLE_APPLICATION_CREDENTIALS` to its path.
- Treat `GOOGLE_ACCESS_TOKEN`, `GCP_PRIVATE_KEY`, and service-account JSON as
  secrets. Never commit them, put them in CLI arguments, or include them in
  diagnostic output.
- The project used in request paths comes from `Config`; it is not copied from
  the service-account key. Configure the intended project explicitly.

Token providers cache tokens and refresh them before their modeled expiry.
Tokens sourced from `GOOGLE_ACCESS_TOKEN` or gcloud are treated as valid for one
hour, so applications supplying those values must keep them renewable.

## Configuration precedence

### Project

The first set variable wins:

1. `VERTEX_PROJECT_ID`
2. `GCP_PROJECT_ID`
3. `GOOGLE_CLOUD_PROJECT`

A missing project is an error.

### Region

The first set variable wins:

1. `VERTEX_REGION`
2. `VERTEX_LOCATION`
3. `VERTEX_ANTHROPIC_LOCATION`
4. `GCP_REGION`
5. `GOOGLE_CLOUD_REGION`
6. SDK default: `us-central1`

`GOOGLE_CLOUD_LOCATION` is used by some repository examples but is not read by
`Config::from_env()`. Prefer `VERTEX_REGION` for the primary client.

### Other variables

| Variable | Default | Implemented behavior |
| --- | --- | --- |
| `VERTEX_MODEL` | `gemini-1.5-flash` | Stored in `Config::model`; operations still receive a model argument explicitly |
| `VERTEX_API_VERSION` | `v1` | Accepted values are `v1` and `v1beta1`; individual API methods currently choose their endpoint version in code |
| `VERTEX_TIMEOUT` | `60` | Whole-client request timeout in seconds; invalid values fall back to 60 |
| `VERTEX_MAX_RETRIES` | `3` | Additional attempts for selected HTTP responses; invalid values fall back to 3 |
| `VERTEX_DEBUG` | unset | Sets `Config::debug` when present; does not initialize a logger by itself |
| `DEBUG` | unset | Also sets `Config::debug` when present |
| `VERTEX_BASE_URL` | regional Vertex endpoint | Overrides the endpoint base URL; intended for controlled tests or proxies |
| `VERTEX_ANTHROPIC_LOCATION` | publisher default | Anthropic publisher-location override |
| `VERTEX_ANTHROPIC_REGION` | publisher default | Alias for the Anthropic publisher-location override |

The `Config::new()` helper reads a smaller subset than `Config::from_env()`.
Use `from_env()` when environment-driven timeout, API-version, and debug
settings are expected.

## Explicit configuration

Construct `Config` directly when deployment configuration should not come from
ambient environment variables:

```rust,no_run
use threatflux_vertex_rust_sdk::{config::Config, VertexClient};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = Config {
    project_id: "your-project-id".into(),
    region: "us-central1".into(),
    timeout_secs: 30,
    max_retries: 2,
    ..Config::default()
};

let client = VertexClient::new(config).await?;
# Ok(())
# }
```

[`vertex.toml.example`](../vertex.toml.example) shows the serialized shape used
by `Config::from_file`. Environment variables can still override the base URL
and Anthropic publisher location after a file is loaded.

## Publisher and model routing

The primary region is used for model requests unless a publisher-specific or
model-specific rule applies. `publisher_locations` can map publisher names to
other locations. The SDK inserts an Anthropic default and supports the two
Anthropic environment overrides above.

Some recognized model families have model-specific routing or version rules in
the client. Use `VertexClient::context_for_model` with a `ModelDescriptor` to
inspect the endpoint and resource path the SDK will use before making a request.
Do not derive current provider availability from these routing rules.

## Timeouts and retries

The client creates one reusable `reqwest::Client` with `timeout_secs` and
`no_proxy()`.

Automatic retries occur only when the service returns one of these statuses:

- `429 Too Many Requests`
- `500 Internal Server Error`
- `502 Bad Gateway`
- `503 Service Unavailable`
- `504 Gateway Timeout`

`max_retries` counts retries after the initial request. A value of `0` disables
automatic retries. Numeric `Retry-After` seconds are honored. Otherwise delays
start at 500 milliseconds, double per attempt, and are capped at 10 seconds.
There is currently no jitter.

Connection, DNS, TLS, timeout, and body-stream errors are returned immediately
instead of being retried. Build application-level retry policy around the
operation's idempotency and your latency budget. Retried POST requests can
consume quota or produce another model response even when they are safe from a
data-mutation perspective.

## Error handling and diagnostics

Public operations return `VertexError`, including these categories:

- `Authentication`
- `Configuration`
- `Http` and `Api`
- `Request`
- `Serialization`
- `Token`
- `Streaming`
- `Io`
- `Generic`

Some error variants include provider response bodies. Those bodies can contain
request details or sensitive model output, so redact them before logging or
returning them to end users.

The library emits limited `log` and `tracing` events at selected streaming
paths. `Config::debug` does not install a logger or subscriber; applications
remain responsible for observability configuration and secret redaction.

## Base URL and network controls

`VERTEX_BASE_URL` and `Config::base_url_override` bypass normal regional
endpoint construction. Only use an allowlisted HTTPS endpoint under your
control. Never accept this value from an untrusted tenant or request parameter.

The internal HTTP client calls `no_proxy()`, so conventional proxy environment
variables are ignored. Environments that require an outbound proxy need a code
change or a supported custom transport before deployment.
