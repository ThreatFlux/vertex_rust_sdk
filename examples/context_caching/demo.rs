use std::time::{Duration, Instant};

use anyhow::Result;
use threatflux_vertex_rust_sdk::{
    cache::{CachedContent, ListCachedContentsResponse, UpdateCachedContentRequest},
    models::GenerateContentRequest,
};
use tokio::time::sleep;

use crate::client::ContextCacheClient;
use crate::content::{questions, uncached_prompt};
use crate::metrics::{preview_for, QuestionOutcome, RunSummary};

pub struct ContextCachingDemo<C> {
    client: C,
    model: String,
    questions: Vec<String>,
    pause_between_requests: Duration,
}

impl<C> ContextCachingDemo<C> {
    #[must_use]
    pub fn new(client: C, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            questions: questions(),
            pause_between_requests: crate::content::REQUEST_PAUSE,
        }
    }

    #[must_use]
    pub fn with_questions(mut self, questions: Vec<String>) -> Self {
        self.questions = questions;
        self
    }

    #[must_use]
    pub const fn with_pause_between(mut self, pause_between: Duration) -> Self {
        self.pause_between_requests = pause_between;
        self
    }
}

impl<C: ContextCacheClient> ContextCachingDemo<C> {
    pub async fn create_cache(&self, cached_content: CachedContent) -> Result<CachedContent> {
        Ok(self.client.create_cache(cached_content).await?)
    }

    pub async fn cache_details(&self, cache_id: &str) -> Result<CachedContent> {
        Ok(self.client.get_cache(cache_id).await?)
    }

    pub async fn list_caches(&self, page_size: Option<i32>) -> Result<ListCachedContentsResponse> {
        Ok(self.client.list_caches(page_size).await?)
    }

    pub async fn update_ttl_hours(&self, cache_id: &str, hours: u64) -> Result<CachedContent> {
        let request = UpdateCachedContentRequest::with_ttl_hours(hours);
        Ok(self.client.update_cache_ttl(cache_id, request).await?)
    }

    pub async fn cleanup(&self, cache_id: &str) -> Result<()> {
        Ok(self.client.delete_cache(cache_id).await?)
    }

    pub async fn run_questions(&self, cache_id: Option<&str>) -> Result<RunSummary> {
        let mut outcomes = Vec::with_capacity(self.questions.len());

        for question in &self.questions {
            let request = Self::build_request(question, cache_id);
            let start = Instant::now();
            let response = self.client.generate_content(&self.model, &request).await?;
            let elapsed = start.elapsed();

            let preview = preview_for(response.text());
            outcomes.push(QuestionOutcome { question: question.clone(), elapsed, preview });

            if self.pause_between_requests > Duration::ZERO {
                sleep(self.pause_between_requests).await;
            }
        }

        Ok(RunSummary { outcomes })
    }

    fn build_request(question: &str, cache_id: Option<&str>) -> GenerateContentRequest {
        let mut request = GenerateContentRequest::new(question);
        if let Some(cache) = cache_id {
            request = request.with_cached_content(cache);
        } else {
            request = request.with_system_text(uncached_prompt());
        }

        request
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use threatflux_vertex_rust_sdk::{
        cache::{CachedContent, ListCachedContentsResponse, UpdateCachedContentRequest},
        error::Result as VertexResult,
        models::{GenerateContentRequest, GenerateContentResponse},
        types::{Candidate, Content, Part},
    };

    #[derive(Clone)]
    struct MockContextCacheClient {
        state: Arc<Mutex<MockState>>,
    }

    struct MockState {
        cached_content: CachedContent,
        generated_requests: Vec<GenerateContentRequest>,
        deleted_ids: Vec<String>,
        ttl_updates: Vec<String>,
    }

    impl MockContextCacheClient {
        fn new() -> Self {
            let mut cached_content = CachedContent::from_text("cached");
            cached_content.name =
                Some("projects/test/locations/us-central1/cachedContents/mock-cache".to_string());
            cached_content.ttl = Some("7200s".to_string());

            Self {
                state: Arc::new(Mutex::new(MockState {
                    cached_content,
                    generated_requests: Vec::new(),
                    deleted_ids: Vec::new(),
                    ttl_updates: Vec::new(),
                })),
            }
        }

        fn generated_requests(&self) -> Vec<GenerateContentRequest> {
            self.state.lock().expect("lock poisoned").generated_requests.clone()
        }

        fn ttl_updates(&self) -> Vec<String> {
            self.state.lock().expect("lock poisoned").ttl_updates.clone()
        }

        fn deleted_ids(&self) -> Vec<String> {
            self.state.lock().expect("lock poisoned").deleted_ids.clone()
        }
    }

    #[async_trait]
    impl ContextCacheClient for MockContextCacheClient {
        async fn create_cache(&self, content: CachedContent) -> VertexResult<CachedContent> {
            let mut state = self.state.lock().expect("lock poisoned");
            state.cached_content = content;
            if state.cached_content.name.is_none() {
                state.cached_content.name = Some(
                    "projects/test/locations/us-central1/cachedContents/generated-id".to_string(),
                );
            }

            Ok(state.cached_content.clone())
        }

        async fn get_cache(&self, _cache_id: &str) -> VertexResult<CachedContent> {
            Ok(self.state.lock().expect("lock poisoned").cached_content.clone())
        }

        async fn list_caches(
            &self,
            _page_size: Option<i32>,
        ) -> VertexResult<ListCachedContentsResponse> {
            let state = self.state.lock().expect("lock poisoned");
            Ok(ListCachedContentsResponse {
                cached_contents: vec![state.cached_content.clone()],
                next_page_token: None,
            })
        }

        async fn update_cache_ttl(
            &self,
            cache_id: &str,
            request: UpdateCachedContentRequest,
        ) -> VertexResult<CachedContent> {
            let mut state = self.state.lock().expect("lock poisoned");
            state.ttl_updates.push(cache_id.to_string());
            if let Some(ttl) = request.ttl.clone() {
                state.cached_content.ttl = Some(ttl);
            }
            Ok(state.cached_content.clone())
        }

        async fn delete_cache(&self, cache_id: &str) -> VertexResult<()> {
            self.state.lock().expect("lock poisoned").deleted_ids.push(cache_id.to_string());
            Ok(())
        }

        async fn generate_content(
            &self,
            _model: &str,
            request: &GenerateContentRequest,
        ) -> VertexResult<GenerateContentResponse> {
            let mut state = self.state.lock().expect("lock poisoned");
            state.generated_requests.push(request.clone());

            let prompt = request
                .contents
                .first()
                .and_then(|content| {
                    content.parts.iter().find_map(|part| {
                        if let Part::Text { text } = part {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "missing prompt".to_string());

            let text = format!("answer for {prompt}");
            Ok(GenerateContentResponse {
                candidates: vec![Candidate {
                    content: Content::model_text(text),
                    finish_reason: None,
                    safety_ratings: vec![],
                    index: None,
                }],
                usage_metadata: None,
                grounding_metadata: None,
            })
        }
    }

    #[tokio::test]
    async fn run_questions_sets_cache_flag() {
        let mock = MockContextCacheClient::new();
        let demo = ContextCachingDemo::new(mock.clone(), "model")
            .with_questions(vec!["What is Rust?".to_string()])
            .with_pause_between(Duration::ZERO);

        let without_cache = demo.run_questions(None).await.unwrap();
        assert_eq!(without_cache.outcomes.len(), 1);
        assert!(mock.generated_requests()[0].cached_content.is_none());

        let with_cache = demo.run_questions(Some("cache-123")).await.unwrap();
        assert_eq!(with_cache.outcomes.len(), 1);
        assert_eq!(mock.generated_requests()[1].cached_content.as_deref(), Some("cache-123"));
    }

    #[tokio::test]
    async fn update_ttl_and_cleanup_are_recorded() {
        let mock = MockContextCacheClient::new();
        let demo = ContextCachingDemo::new(mock.clone(), "model");

        let updated = demo.update_ttl_hours("cache-123", 4).await.unwrap();
        assert_eq!(updated.ttl, Some("14400s".to_string()));
        assert_eq!(mock.ttl_updates(), vec!["cache-123".to_string()]);

        demo.cleanup("cache-123").await.unwrap();
        assert_eq!(mock.deleted_ids(), vec!["cache-123".to_string()]);
    }
}
