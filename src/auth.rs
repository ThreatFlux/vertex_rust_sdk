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
        use rsa::{
            pkcs8::{EncodePrivateKey, LineEnding},
            rand_core::OsRng,
            RsaPrivateKey,
        };

        let mut rng = OsRng;
        let key = RsaPrivateKey::new(&mut rng, 2048).expect("generate test key");
        key.to_pkcs8_pem(LineEnding::LF).expect("serialize test key").to_string()
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
