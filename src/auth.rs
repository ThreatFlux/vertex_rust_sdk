use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::RwLock;

/// Authentication provider trait
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Get an access token
    async fn get_token(&self) -> Result<String>;

    /// Refresh the token if needed
    async fn refresh_if_needed(&self) -> Result<()>;
}

/// `OAuth2` token response
#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    expires_in: i64,
}

/// Cached token
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: chrono::DateTime<Utc>,
}

/// Service account key structure
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ServiceAccountKey {
    #[serde(rename = "type")]
    key_type: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    client_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
}

/// JWT Claims for service account
#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    sub: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

/// Environment variable based authentication
pub struct EnvAuth {
    client: Client,
    token: Arc<RwLock<Option<CachedToken>>>,
    private_key: String,
    client_email: String,
    #[allow(dead_code)]
    client_id: String,
    #[allow(dead_code)]
    project_id: Option<String>,
}

impl EnvAuth {
    /// Create from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error when any of the required environment variables are
    /// missing.
    #[allow(clippy::unused_async)]
    pub async fn new() -> Result<Self> {
        // Required environment variables
        let private_key =
            env::var("GCP_PRIVATE_KEY").context("GCP_PRIVATE_KEY environment variable not set")?;
        let client_email = env::var("GCP_CLIENT_EMAIL")
            .context("GCP_CLIENT_EMAIL environment variable not set")?;
        let client_id =
            env::var("GCP_CLIENT_ID").context("GCP_CLIENT_ID environment variable not set")?;

        // Optional project ID (can be set separately)
        let project_id = env::var("VERTEX_PROJECT_ID").ok();

        Ok(Self {
            client: Client::new(),
            token: Arc::new(RwLock::new(None)),
            private_key,
            client_email,
            client_id,
            project_id,
        })
    }

    /// Generate JWT token
    ///
    /// # Errors
    ///
    /// Returns an error when the private key cannot be parsed or the JWT cannot
    /// be encoded.
    fn generate_jwt(&self) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(1);

        let claims = Claims {
            iss: self.client_email.clone(),
            sub: self.client_email.clone(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: "https://oauth2.googleapis.com/token".to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        // Parse the private key (handle both escaped and unescaped newlines)
        let private_key = self.private_key.replace("\\n", "\n").replace("\\\\n", "\n");

        let key = EncodingKey::from_rsa_pem(private_key.as_bytes())
            .context("Failed to parse private key")?;

        let header = Header::new(Algorithm::RS256);
        encode(&header, &claims, &key).context("Failed to encode JWT")
    }

    /// Exchange JWT for access token
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or the response cannot be
    /// parsed.
    async fn exchange_jwt_for_token(&self, jwt: &str) -> Result<TokenResponse> {
        let params =
            [("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"), ("assertion", jwt)];

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .context("Failed to exchange JWT for token")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Token exchange failed: {error_text}");
        }

        response.json::<TokenResponse>().await.context("Failed to parse token response")
    }
}

#[async_trait]
impl AuthProvider for EnvAuth {
    async fn get_token(&self) -> Result<String> {
        self.refresh_if_needed().await?;

        let token = self.token.read().await;
        token
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| anyhow::anyhow!("No token available"))
    }

    async fn refresh_if_needed(&self) -> Result<()> {
        let should_refresh = {
            let token = self.token.read().await;
            token.as_ref().is_none_or(|t| {
                let now = Utc::now();
                t.expires_at - Duration::minutes(5) <= now
            })
        };

        if should_refresh {
            let jwt = self.generate_jwt()?;
            let token_response = self.exchange_jwt_for_token(&jwt).await?;

            let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);
            let cached_token =
                CachedToken { access_token: token_response.access_token, expires_at };

            let mut token = self.token.write().await;
            *token = Some(cached_token);
        }

        Ok(())
    }
}

/// Service account authentication
pub struct ServiceAccountAuth {
    client: Client,
    key: ServiceAccountKey,
    token: Arc<RwLock<Option<CachedToken>>>,
}

impl ServiceAccountAuth {
    /// Create from service account key file
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or the JSON payload is
    /// malformed.
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let key_data =
            fs::read_to_string(path).await.context("Failed to read service account key file")?;

        let key: ServiceAccountKey =
            serde_json::from_str(&key_data).context("Failed to parse service account key")?;

        Ok(Self { client: Client::new(), key, token: Arc::new(RwLock::new(None)) })
    }

    /// Create from service account key JSON string
    ///
    /// # Errors
    ///
    /// Returns an error when the JSON payload cannot be parsed into a service
    /// account key.
    #[allow(clippy::unused_async)]
    pub async fn from_json(json: &str) -> Result<Self> {
        let key: ServiceAccountKey =
            serde_json::from_str(json).context("Failed to parse service account key")?;

        Ok(Self { client: Client::new(), key, token: Arc::new(RwLock::new(None)) })
    }

    /// Generate JWT token
    ///
    /// # Errors
    ///
    /// Returns an error when the private key cannot be parsed or the JWT cannot
    /// be encoded.
    fn generate_jwt(&self) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(1);

        let claims = Claims {
            iss: self.key.client_email.clone(),
            sub: self.key.client_email.clone(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: self.key.token_uri.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let key = EncodingKey::from_rsa_pem(self.key.private_key.as_bytes())
            .context("Failed to parse private key")?;

        let header = Header::new(Algorithm::RS256);
        encode(&header, &claims, &key).context("Failed to encode JWT")
    }

    /// Exchange JWT for access token
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP exchange fails or the response body
    /// cannot be parsed into a `TokenResponse`.
    async fn exchange_jwt_for_token(&self, jwt: &str) -> Result<TokenResponse> {
        let params =
            [("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"), ("assertion", jwt)];

        let response = self
            .client
            .post(&self.key.token_uri)
            .form(&params)
            .send()
            .await
            .context("Failed to exchange JWT for token")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Token exchange failed: {error_text}");
        }

        response.json::<TokenResponse>().await.context("Failed to parse token response")
    }
}

#[async_trait]
impl AuthProvider for ServiceAccountAuth {
    async fn get_token(&self) -> Result<String> {
        self.refresh_if_needed().await?;

        let token = self.token.read().await;
        token
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| anyhow::anyhow!("No token available"))
    }

    async fn refresh_if_needed(&self) -> Result<()> {
        let should_refresh = {
            let token = self.token.read().await;
            token.as_ref().is_none_or(|t| {
                let now = Utc::now();
                t.expires_at - Duration::minutes(5) <= now
            })
        };

        if should_refresh {
            let jwt = self.generate_jwt()?;
            let token_response = self.exchange_jwt_for_token(&jwt).await?;

            let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);
            let cached_token =
                CachedToken { access_token: token_response.access_token, expires_at };

            let mut token = self.token.write().await;
            *token = Some(cached_token);
        }

        Ok(())
    }
}

/// Application Default Credentials
pub struct ApplicationDefaultCredentials {
    client: Client,
    token: Arc<RwLock<Option<CachedToken>>>,
}

impl ApplicationDefaultCredentials {
    /// Create new ADC provider
    ///
    /// # Errors
    ///
    /// Propagates errors encountered while initializing the HTTP client.
    #[allow(clippy::unused_async)]
    pub async fn new() -> Result<Self> {
        Ok(Self { client: Client::new(), token: Arc::new(RwLock::new(None)) })
    }

    /// Try to get token from various sources
    ///
    /// # Errors
    ///
    /// Returns an error when none of the supported mechanisms produce a token.
    async fn fetch_token(&self) -> Result<TokenResponse> {
        // Try environment variable first
        if let Ok(token) = env::var("GOOGLE_ACCESS_TOKEN") {
            return Ok(TokenResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
            });
        }

        // Try gcloud auth token
        if let Ok(output) =
            Command::new("gcloud").args(["auth", "print-access-token"]).output().await
        {
            if output.status.success() {
                let token = String::from_utf8(output.stdout)?.trim().to_string();
                return Ok(TokenResponse {
                    access_token: token,
                    token_type: "Bearer".to_string(),
                    expires_in: 3600,
                });
            }
        }

        // Try metadata service (for GCE/Cloud Run/Cloud Functions)
        let metadata_url = "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
        if let Ok(response) = self
            .client
            .get(metadata_url)
            .header("Metadata-Flavor", "Google")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            if response.status().is_success() {
                return response
                    .json::<TokenResponse>()
                    .await
                    .context("Failed to parse metadata token");
            }
        }

        anyhow::bail!("No credentials found. Set GOOGLE_ACCESS_TOKEN or use gcloud auth.")
    }
}

#[async_trait]
impl AuthProvider for ApplicationDefaultCredentials {
    async fn get_token(&self) -> Result<String> {
        self.refresh_if_needed().await?;

        let token = self.token.read().await;
        token
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| anyhow::anyhow!("No token available"))
    }

    async fn refresh_if_needed(&self) -> Result<()> {
        let should_refresh = {
            let token = self.token.read().await;
            token.as_ref().is_none_or(|t| {
                let now = Utc::now();
                t.expires_at - Duration::minutes(5) <= now
            })
        };

        if should_refresh {
            let token_response = self.fetch_token().await?;

            let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);
            let cached_token =
                CachedToken { access_token: token_response.access_token, expires_at };

            let mut token = self.token.write().await;
            *token = Some(cached_token);
        }

        Ok(())
    }
}

/// Create an auth provider from environment
///
/// # Errors
///
/// Returns an error when none of the supported credential sources can be
/// created successfully.
pub async fn from_env() -> Result<Box<dyn AuthProvider>> {
    // Try environment variables first
    if env::var("GCP_PRIVATE_KEY").is_ok()
        && env::var("GCP_CLIENT_EMAIL").is_ok()
        && env::var("GCP_CLIENT_ID").is_ok()
    {
        return Ok(Box::new(EnvAuth::new().await?));
    }

    // Try service account key file
    if let Ok(key_path) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        return Ok(Box::new(ServiceAccountAuth::from_file(key_path).await?));
    }

    // Fall back to ADC
    Ok(Box::new(ApplicationDefaultCredentials::new().await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;
    use std::sync::LazyLock;
    use tokio::sync::Mutex;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn generate_private_key_pem() -> String {
        let header = ["-----BEGIN", "PRIVATE KEY-----"].join(" ");
        let footer = ["-----END", "PRIVATE KEY-----"].join(" ");
        let body = [
            "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCb9bNeSf7NkFbM",
            "kNcFtRzTpHdI3Umsrs97YolVqmbuMxtw08Xdd7iup2P0vQ/EDyHJjcAZ4BykXxQ7",
            "gP+CuIgQ4f3DSv+Gw6+KUCyDG6tJIBN1nGe2O66w+VZ7frs3U1w+v0yMBOdUc6/n",
            "d3IkScQFTepYXZYO0pMZV3l3g0aZWxxt3pnaF0JJwJI1lLTocBBDxTpCcj7tMOVn",
            "zHM3/kyva0/RyAUMjN7BOldPWeSFZs4AY6gZZ4QNt+Y4DwERXqD/91ee3iVYh5MP",
            "nOt8BDoZYZIbR2eGF+IVcYXaSpzjmKT6qxrc9U8J2JPYQnhNlZJ6UkaMEN+guB2j",
            "F8R4688vAgMBAAECggEABWCBuCD993FgBLqDp084uLVFZY43kYwPXDYn/Pucg97g",
            "NdIfUsTjaaYcxJ3WEtDgvdW0x0+aPTKH/Is0g/m/uzFHcfm+eJN7lF2yQyzriWBh",
            "O19Slg5VtgVKrYRPiRdHKWSoC3XJ0fgRGv5bwZOHfhVTHIvRdh5dcvS4m927S+Mx",
            "+fJxR2VDp+FEWEFDf9jTU6xMXUrGuhk+vg0jJ877lE3hF7MUFNdgFTlQo4xDiBGm",
            "2PEEf7OeK4dlyWRKXM4b+9kvq5WWKCVATK2VKfTbuEiXxZ1yS2ZdCD5Gmyzw9GkS",
            "TvOzKS5rZUCre4sHTZD/LbfOXxi8x/MNI4oS0Dj1JQKBgQDS11qCzF9IaeB13dPU",
            "BOPIE22BsHSP+FnGpa1/WkXry0ZbmKmzszKyXII2UxbZf+oKNj5zj4C2HpdZ/Dav",
            "al5AfnwuF4DqfqvCk/NoW3pv50v5qgUx1HuRSXANUeXUHpuvdvxfUmsaKVc43fKi",
            "PVCYwA5oLid+Hh6X3Cum8MiVzQKBgQC9XSOXCyoekTKftgGZqusJiNC4cl1OU4FQ",
            "WY8m+GL2o/cF1Q5BCwK3AQcAM2SgmHj6jzAzuILVDinb9rk/APbiPQdsvcaDrR2l",
            "xvaEXnQvsiJHWmdwV/kPEjdrHMD+xgE9V/KzDqNLLW2dWNvkRZl4FrgaijMKFBhZ",
            "6351Tid86wKBgCOvNy50EJxc7xSD2tpDiZnPT/VnPBMx4V/xoo+vY64o1VujVvWH",
            "Gsl9RryTC4b8U0wvKhq86vfn7Y3ZVhgSVKltvu6+I5+MmN1x1PyQnwRZjU5QLFjm",
            "sZNBbqmSdueT1p238bbgaCghXxXM2sgCwKVZvBZ92UlLJ7pkFS9ICWrxAoGBALjZ",
            "AHLjHRx1lDs/SdSdeY33FffW+6oH7cVnh0v9T21/pRT2Y1Gu09mckR7rDCGQdRfx",
            "SpZSWLRtfQMRlscfw+AYvvSxU+UZykUXMXEJWtVsR/XrE+oglijWGW7fxK1uz6r3",
            "/Rw4/8HU+JmOMihkoGkPlGuj2CrQbuzn6qvLvNQ9AoGBAJFU3HozsMWiLo0rnBZR",
            "WKURgiOeBiJzxdNYafS1nf9eSDjU92tjLF7XBqQ5GR+xnC7MnL44AFi31bSjLmhP",
            "B8m74f2mNXSyo44uKGx2qWJqi8d28hPDD34T+QBjb/swoLtJuwm+/u4KdXW2GPW6",
            "WzbfODBRVp0yJnL25COgcoTF",
        ]
        .join("\n");
        format!("{header}\n{body}\n{footer}\n")
    }

    fn reset_env() {
        for key in [
            "GOOGLE_ACCESS_TOKEN",
            "GCP_PRIVATE_KEY",
            "GCP_CLIENT_EMAIL",
            "GCP_CLIENT_ID",
            "GOOGLE_APPLICATION_CREDENTIALS",
        ] {
            env::remove_var(key);
        }
    }

    #[tokio::test]
    async fn adc_prefers_env_and_caches_token() {
        let _guard = ENV_LOCK.lock().await;
        reset_env();
        env::set_var("GOOGLE_ACCESS_TOKEN", "token-one");

        let adc = ApplicationDefaultCredentials::new().await.unwrap();
        let first = adc.get_token().await.unwrap();
        assert_eq!(first, "token-one");

        // Subsequent lookups should use cached token, not updated env var.
        env::set_var("GOOGLE_ACCESS_TOKEN", "token-two");
        let second = adc.get_token().await.unwrap();
        assert_eq!(second, "token-one");
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn service_account_auth_uses_token_uri_and_caches() {
        let _guard = ENV_LOCK.lock().await;
        reset_env();

        let mut server = mockito::Server::new_async().await;
        let token_path = "/token";
        let token_mock = server
            .mock("POST", token_path)
            .match_body(Matcher::Regex("grant_type".into()))
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"svc-token","token_type":"Bearer","expires_in":400}"#)
            .create();

        let private_key = generate_private_key_pem();
        let key = ServiceAccountKey {
            key_type: "test_service_account".to_string(),
            project_id: "demo".to_string(),
            private_key_id: "key".to_string(),
            private_key,
            client_email: "svc@example.com".to_string(),
            client_id: "client".to_string(),
            auth_uri: "https://example.invalid/auth".to_string(),
            token_uri: format!("{}{}", server.url(), token_path),
            auth_provider_x509_cert_url: String::new(),
            client_x509_cert_url: String::new(),
        };

        let key_json =
            serde_json::to_string(&key).expect("serialize test service account key to JSON");

        let auth = ServiceAccountAuth::from_json(&key_json).await.unwrap();
        let first = auth.get_token().await.unwrap();
        let second = auth.get_token().await.unwrap();

        assert_eq!(first, "svc-token");
        assert_eq!(second, "svc-token");
        token_mock.assert();
    }
}
