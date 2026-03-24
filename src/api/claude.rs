use crate::claude::{
    ClaudeSseParser, MessageRequest, MessageResponse, RequestTool, StreamEvent, WebSearchToolType,
    CLAUDE_LONG_CONTEXT_BETA_TAG, CLAUDE_WEB_SEARCH_BETA_TAG, CLAUDE_WEB_SEARCH_V2_BETA_TAG,
};
use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::streaming_support::SseStreamState;
use futures_util::stream::{self, Stream, TryStreamExt};
use reqwest::header;
use std::pin::Pin;

impl VertexClient {
    /// Invoke a Claude model via Vertex Anthropic integration.
    ///
    /// # Errors
    ///
    /// Returns an error if the model descriptor is invalid, the authenticated
    /// request fails, or the Claude payload cannot be parsed.
    pub async fn claude_message(
        &self,
        model: &str,
        request: &MessageRequest,
    ) -> Result<MessageResponse> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        // Claude on Vertex exposes predict/stream endpoints under the v1 API.
        let path = format!("/v1/{}:rawPredict", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);

        let mut payload = request.clone();
        let beta_value = beta_header_value(&payload);
        payload.beta = None;
        if payload.stream == Some(true) {
            payload.stream = None;
        }

        let mut extra_headers: Vec<(String, String)> = Vec::new();
        if let Some(beta_value) = beta_value {
            extra_headers.push((ANTHROPIC_BETA_HEADER.to_string(), beta_value));
        }

        let response =
            self.make_authenticated_request_with_headers(&url, &payload, &extra_headers).await?;
        let status = response.status();
        let body = response.text().await.map_err(VertexError::Request)?;

        if !status.is_success() {
            return Err(VertexError::http(status.as_u16(), body));
        }

        parse_claude_response(&body)
    }

    /// Stream Claude responses over SSE.
    ///
    /// # Errors
    ///
    /// Returns an error if the request cannot be prepared, the SSE connection
    /// fails, or the service returns a non-success status.
    pub async fn claude_stream(
        &self,
        model: &str,
        request: &MessageRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let path = format!("/v1/{}:streamRawPredict?alt=sse", context.resource_path);
        let url = self.build_url_for_endpoint(&context.endpoint, &path);

        let mut payload = request.clone();
        let beta_value = beta_header_value(&payload);
        payload.beta = None;
        payload.stream = Some(true);

        let mut extra_headers: Vec<(String, String)> = Vec::new();
        if let Some(beta_value) = beta_value {
            extra_headers.push((ANTHROPIC_BETA_HEADER.to_string(), beta_value));
        }

        tracing::info!(
            target: "threatflux_vertex_rust_sdk::claude",
            model = %descriptor.relative_path(),
            endpoint = %context.endpoint,
            resource_path = %context.resource_path,
            url = %url,
            "vertex claude stream request"
        );

        let response = self
            .send_with_retry(|| {
                let url = url.clone();
                let payload = payload.clone();
                let extra_headers = extra_headers.clone();
                async move {
                    let token = self.get_auth_token().await?;
                    let mut request_builder = self
                        .http_client()
                        .post(&url)
                        .header(header::AUTHORIZATION, format!("Bearer {token}"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::ACCEPT, "text/event-stream");

                    for (key, value) in &extra_headers {
                        request_builder = request_builder.header(key, value);
                    }

                    let response = request_builder
                        .json(&payload)
                        .send()
                        .await
                        .map_err(VertexError::Request)?;

                    Ok(response)
                }
            })
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.map_err(VertexError::Request)?;
            return Err(VertexError::http(status, error_text));
        }

        let byte_stream = response.bytes_stream();
        let state = SseStreamState::new(Box::pin(byte_stream), ClaudeSseParser::new());

        let stream = stream::try_unfold(state, |mut state| async move {
            loop {
                if let Some(event) = state.try_parsed_event()? {
                    return Ok(Some((event, state)));
                }

                if !state.advance().await? {
                    if let Some(event) = state.try_parsed_event()? {
                        return Ok(Some((event, state)));
                    }
                    return Ok(None);
                }
            }
        })
        .into_stream();

        Ok(Box::pin(stream))
    }
}

fn parse_claude_response(body: &str) -> Result<MessageResponse> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(VertexError::Serialization)?;

    if let Ok(message) = serde_json::from_value::<MessageResponse>(value.clone()) {
        return Ok(message);
    }

    if let Some(predictions) = value.get("predictions").and_then(|p| p.as_array()) {
        if let Some(first) = predictions.first() {
            return serde_json::from_value(first.clone()).map_err(VertexError::Serialization);
        }
    }

    Err(VertexError::generic("Unexpected Claude response payload"))
}

const ANTHROPIC_BETA_HEADER: &str = "anthropic-beta";

fn beta_header_value(request: &MessageRequest) -> Option<String> {
    let mut tags: Vec<String> = Vec::new();

    if let Some(features) = &request.beta {
        for feature in features {
            if let Some(normalized) = normalize_beta_feature(feature) {
                maybe_push_beta_tag(&mut tags, normalized);
            }
        }
    }

    match web_search_tool_version(request) {
        Some(WebSearchToolType::WebSearch) => {
            maybe_push_beta_tag(&mut tags, CLAUDE_WEB_SEARCH_BETA_TAG.to_string());
        }
        Some(WebSearchToolType::WebSearchV2) => {
            maybe_push_beta_tag(&mut tags, CLAUDE_WEB_SEARCH_V2_BETA_TAG.to_string());
        }
        None => {}
    }

    if tags.is_empty() {
        None
    } else {
        tags.sort_by_key(|value| value.to_ascii_lowercase());
        tags.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        Some(tags.join(","))
    }
}

fn normalize_beta_feature(feature: &str) -> Option<String> {
    let trimmed = feature.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.eq_ignore_ascii_case("web-search")
        || trimmed.eq_ignore_ascii_case(CLAUDE_WEB_SEARCH_BETA_TAG)
    {
        return Some(CLAUDE_WEB_SEARCH_BETA_TAG.to_string());
    }

    if trimmed.eq_ignore_ascii_case("web-search-v2")
        || trimmed.eq_ignore_ascii_case(CLAUDE_WEB_SEARCH_V2_BETA_TAG)
    {
        return Some(CLAUDE_WEB_SEARCH_V2_BETA_TAG.to_string());
    }

    if trimmed.eq_ignore_ascii_case("context-1m")
        || trimmed.eq_ignore_ascii_case(CLAUDE_LONG_CONTEXT_BETA_TAG)
    {
        return Some(CLAUDE_LONG_CONTEXT_BETA_TAG.to_string());
    }

    Some(trimmed.to_string())
}

fn maybe_push_beta_tag(tags: &mut Vec<String>, candidate: String) {
    if !tags.iter().any(|value| value.eq_ignore_ascii_case(&candidate)) {
        tags.push(candidate);
    }
}

fn web_search_tool_version(request: &MessageRequest) -> Option<WebSearchToolType> {
    request.tools.as_ref().and_then(|tools| {
        tools.iter().find_map(|tool| match tool {
            RequestTool::WebSearch(ws) => Some(ws.tool_type.clone()),
            RequestTool::Function(_) => None,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_direct_response() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello"}],
            "stop_reason": "end_turn"
        }"#;

        let response = parse_claude_response(json).expect("parse direct response");
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.text(), "Hello");
    }

    #[test]
    fn parses_predictions_array() {
        let json = r#"{
            "predictions": [
                {
                    "id": "msg_456",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Hi"}]
                }
            ]
        }"#;

        let response = parse_claude_response(json).expect("parse predictions wrapper");
        assert_eq!(response.id, "msg_456");
        assert_eq!(response.text(), "Hi");
    }
}
