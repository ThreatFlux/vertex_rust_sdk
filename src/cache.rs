//! Context caching functionality for Vertex AI
//!
//! Context caching allows caching large contexts (like documents, system instructions)
//! for reuse across multiple requests, reducing latency and costs.

use crate::error::{Result, VertexError};
use crate::types::Content;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;

/// Cached content for reuse across requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedContent {
    /// Unique cache identifier
    pub name: Option<String>,
    /// Display name for the cache
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    /// Content to be cached
    pub contents: Vec<Content>,
    /// System instruction to be cached
    #[serde(rename = "systemInstruction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Tools available for cached content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::types::Tool>>,
    /// Time to live (TTL) in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    /// Cache expiration time
    #[serde(rename = "expireTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<DateTime<Utc>>,
    /// Cache creation time
    #[serde(rename = "createTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<DateTime<Utc>>,
    /// Cache last update time
    #[serde(rename = "updateTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<DateTime<Utc>>,
    /// Usage metadata
    #[serde(rename = "usageMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<CacheUsageMetadata>,
}

#[allow(clippy::missing_const_for_fn)]
impl CachedContent {
    /// Create a new cache with contents
    #[must_use]
    pub fn new(contents: Vec<Content>) -> Self {
        Self {
            name: None,
            display_name: None,
            contents,
            system_instruction: None,
            tools: None,
            ttl: Some("3600s".to_string()), // Default 1 hour TTL
            expire_time: None,
            create_time: None,
            update_time: None,
            usage_metadata: None,
        }
    }

    /// Create a cache from a single text content
    #[must_use]
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        let content = Content::user_text(text);
        Self::new(vec![content])
    }

    /// Create a cache from file content
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read.
    pub fn from_file<P: AsRef<std::path::Path>>(file_path: P) -> Result<Self> {
        let content = fs::read_to_string(file_path).map_err(VertexError::Io)?;
        Ok(Self::from_text(content))
    }

    /// Set display name
    #[must_use]
    pub fn with_display_name<S: Into<String>>(mut self, display_name: S) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Set system instruction
    #[must_use]
    pub fn with_system_instruction(mut self, instruction: Content) -> Self {
        self.system_instruction = Some(instruction);
        self
    }

    /// Set system instruction from text
    #[must_use]
    pub fn with_system_text<S: Into<String>>(mut self, text: S) -> Self {
        self.system_instruction = Some(Content::system_text(text));
        self
    }

    /// Set tools
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<crate::types::Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set TTL (time to live) in seconds
    #[must_use]
    pub fn with_ttl_seconds(mut self, ttl_seconds: u64) -> Self {
        self.ttl = Some(format!("{ttl_seconds}s"));
        self
    }

    /// Set TTL (time to live) in minutes
    #[must_use]
    pub fn with_ttl_minutes(self, ttl_minutes: u64) -> Self {
        self.with_ttl_seconds(ttl_minutes * 60)
    }

    /// Set TTL (time to live) in hours
    #[must_use]
    pub fn with_ttl_hours(self, ttl_hours: u64) -> Self {
        self.with_ttl_seconds(ttl_hours * 3600)
    }

    /// Get the cache ID (extracted from name)
    #[must_use]
    pub fn cache_id(&self) -> Option<String> {
        self.name.as_ref().and_then(|name| {
            // Extract cache ID from full resource name
            // Format: projects/{project}/locations/{location}/cachedContents/{cache_id}
            name.split('/').next_back().map(str::to_string)
        })
    }

    /// Check if cache has expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expire_time.is_some_and(|expire_time| Utc::now() > expire_time)
    }

    /// Get remaining TTL in seconds
    #[must_use]
    pub fn remaining_ttl_seconds(&self) -> Option<i64> {
        self.expire_time.map(|expire_time| (expire_time - Utc::now()).num_seconds().max(0))
    }
}

/// Request to create a cached content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCachedContentRequest {
    /// Cached content to create
    #[serde(flatten)]
    pub cached_content: CachedContent,
}

impl CreateCachedContentRequest {
    /// Create a new request
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(cached_content: CachedContent) -> Self {
        Self { cached_content }
    }
}

/// Request to update cached content TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCachedContentRequest {
    /// Updated TTL
    pub ttl: Option<String>,
    /// Updated expiration time
    #[serde(rename = "expireTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<DateTime<Utc>>,
}

impl UpdateCachedContentRequest {
    /// Create a new request with TTL in seconds
    #[must_use]
    pub fn with_ttl_seconds(ttl_seconds: u64) -> Self {
        Self { ttl: Some(format!("{ttl_seconds}s")), expire_time: None }
    }

    /// Create a new request with TTL in minutes
    #[must_use]
    pub fn with_ttl_minutes(ttl_minutes: u64) -> Self {
        Self::with_ttl_seconds(ttl_minutes * 60)
    }

    /// Create a new request with TTL in hours
    #[must_use]
    pub fn with_ttl_hours(ttl_hours: u64) -> Self {
        Self::with_ttl_seconds(ttl_hours * 3600)
    }
}

/// Response from listing cached contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCachedContentsResponse {
    /// List of cached contents
    #[serde(rename = "cachedContents")]
    pub cached_contents: Vec<CachedContent>,
    /// Token for next page of results
    #[serde(rename = "nextPageToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Usage metadata for cached content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheUsageMetadata {
    /// Total token count for cached content
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: i32,
}

/// Reference to cached content for use in requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedContentRef {
    /// Name/ID of the cached content
    pub name: String,
}

impl CachedContentRef {
    /// Create a new cached content reference
    #[must_use]
    pub fn new<S: Into<String>>(cache_id: S) -> Self {
        Self { name: cache_id.into() }
    }

    /// Create from full resource name
    #[must_use]
    pub fn from_full_name<S: Into<String>>(full_name: S) -> Self {
        Self { name: full_name.into() }
    }
}

/// Cache API implementation
pub struct CacheApi<'a> {
    client: &'a crate::client::VertexClient,
}

impl<'a> CacheApi<'a> {
    /// Create a new cache API instance
    #[must_use]
    pub const fn new(client: &'a crate::client::VertexClient) -> Self {
        Self { client }
    }

    /// Create a new cached content
    ///
    /// # Errors
    ///
    /// Returns an error when building the request URL fails or the API returns
    /// an unsuccessful response.
    pub async fn create_cache(&self, cached_content: CachedContent) -> Result<CachedContent> {
        let url = self.client.build_url(&format!(
            "/v1/projects/{}/locations/{}/cachedContents",
            self.client.project_id(),
            self.client.location()
        ));

        let request = CreateCachedContentRequest::new(cached_content);
        let response = self.client.make_authenticated_request(&url, &request).await?;

        if response.status().is_success() {
            let result = response.json::<CachedContent>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!("Failed to create cache: Status {status}: {error_text}"),
                code: status.to_string(),
            })
        }
    }

    /// Get cached content by ID
    ///
    /// # Errors
    ///
    /// Returns an error when the cache cannot be fetched or the response cannot
    /// be parsed.
    pub async fn get_cache(&self, cache_id: &str) -> Result<CachedContent> {
        let url = self.client.build_url(&format!(
            "/v1/projects/{}/locations/{}/cachedContents/{}",
            self.client.project_id(),
            self.client.location(),
            cache_id
        ));

        let response = self.client.make_authenticated_get_request(&url).await?;

        if response.status().is_success() {
            let result = response.json::<CachedContent>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!("Failed to get cache {cache_id}: Status {status}: {error_text}"),
                code: status.to_string(),
            })
        }
    }

    /// Delete cached content by ID
    ///
    /// # Errors
    ///
    /// Returns an error when the deletion request fails or the service responds
    /// with a non-success status.
    pub async fn delete_cache(&self, cache_id: &str) -> Result<()> {
        let url = self.client.build_url(&format!(
            "/v1/projects/{}/locations/{}/cachedContents/{}",
            self.client.project_id(),
            self.client.location(),
            cache_id
        ));

        let token = self.client.get_auth_token().await?;

        let response = self
            .client
            .http_client()
            .delete(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(VertexError::Request)?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!(
                    "Failed to delete cache {cache_id}: Status {status}: {error_text}"
                ),
                code: status.to_string(),
            })
        }
    }

    /// List cached contents
    ///
    /// # Errors
    ///
    /// Returns an error when the list request fails or the response cannot be
    /// parsed successfully.
    pub async fn list_caches(
        &self,
        page_size: Option<i32>,
        page_token: Option<&str>,
    ) -> Result<ListCachedContentsResponse> {
        let mut url = self.client.build_url(&format!(
            "/v1/projects/{}/locations/{}/cachedContents",
            self.client.project_id(),
            self.client.location()
        ));

        let mut params = Vec::new();
        if let Some(size) = page_size {
            params.push(format!("pageSize={size}"));
        }
        if let Some(token) = page_token {
            params.push(format!("pageToken={token}"));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self.client.make_authenticated_get_request(&url).await?;

        if response.status().is_success() {
            let result = response
                .json::<ListCachedContentsResponse>()
                .await
                .map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!("Failed to list caches: Status {status}: {error_text}"),
                code: status.to_string(),
            })
        }
    }

    /// Update cached content TTL
    ///
    /// # Errors
    ///
    /// Returns an error when the update call fails or the service returns a
    /// non-success status.
    pub async fn update_cache_ttl(
        &self,
        cache_id: &str,
        update_request: UpdateCachedContentRequest,
    ) -> Result<CachedContent> {
        let url = self.client.build_url(&format!(
            "/v1/projects/{}/locations/{}/cachedContents/{}",
            self.client.project_id(),
            self.client.location(),
            cache_id
        ));

        let token = self.client.get_auth_token().await?;

        let response = self
            .client
            .http_client()
            .patch(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .query(&[("updateMask", "ttl")])
            .json(&update_request)
            .send()
            .await
            .map_err(VertexError::Request)?;

        if response.status().is_success() {
            let result = response.json::<CachedContent>().await.map_err(VertexError::Request)?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(VertexError::Api {
                message: format!(
                    "Failed to update cache {cache_id}: Status {status}: {error_text}"
                ),
                code: status.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_content_creation() {
        let content = Content::user_text("Test content");
        let cached =
            CachedContent::new(vec![content]).with_display_name("Test Cache").with_ttl_hours(2);

        assert_eq!(cached.display_name, Some("Test Cache".to_string()));
        assert_eq!(cached.ttl, Some("7200s".to_string()));
        assert_eq!(cached.contents.len(), 1);
    }

    #[test]
    fn test_cached_content_from_text() {
        let cached = CachedContent::from_text("Hello, world!");
        assert_eq!(cached.contents.len(), 1);

        if let crate::types::Part::Text { text } = &cached.contents[0].parts[0] {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text part");
        }
    }

    #[test]
    fn test_ttl_helpers() {
        let cached = CachedContent::from_text("test").with_ttl_seconds(120);
        assert_eq!(cached.ttl, Some("120s".to_string()));

        let cached = CachedContent::from_text("test").with_ttl_minutes(5);
        assert_eq!(cached.ttl, Some("300s".to_string()));

        let cached = CachedContent::from_text("test").with_ttl_hours(1);
        assert_eq!(cached.ttl, Some("3600s".to_string()));
    }

    #[test]
    fn test_cache_id_extraction() {
        let mut cached = CachedContent::from_text("test");
        cached.name =
            Some("projects/test-project/locations/us-central1/cachedContents/abc123".to_string());

        assert_eq!(cached.cache_id(), Some("abc123".to_string()));
    }

    #[test]
    fn test_cached_content_ref() {
        let cache_ref = CachedContentRef::new("abc123");
        assert_eq!(cache_ref.name, "abc123");

        let cache_ref = CachedContentRef::from_full_name(
            "projects/test/locations/us-central1/cachedContents/def456",
        );
        assert_eq!(cache_ref.name, "projects/test/locations/us-central1/cachedContents/def456");
    }

    #[test]
    fn test_update_request() {
        let update = UpdateCachedContentRequest::with_ttl_hours(3);
        assert_eq!(update.ttl, Some("10800s".to_string()));
    }
}
