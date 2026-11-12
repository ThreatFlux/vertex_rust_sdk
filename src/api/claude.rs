use crate::claude::{
    ClaudeSseParser, MessageRequest, MessageResponse, RequestTool, StreamEvent, ToolChoice,
    WebSearchToolType, CLAUDE_LONG_CONTEXT_BETA_TAG, CLAUDE_WEB_SEARCH_BETA_TAG,
    CLAUDE_WEB_SEARCH_V2_BETA_TAG,
};
use crate::client::VertexClient;
use crate::error::{Result, VertexError};
use crate::model_descriptor::ModelDescriptor;
use crate::streaming_support::SseStreamState;
use futures_util::stream::{self, Stream, TryStreamExt};
use reqwest::header;
use serde_json::{json, Value};
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

        let payload = claude_payload(request, false)?;

        let mut extra_headers: Vec<(String, String)> = Vec::new();
        if let Some(beta_value) = beta_header_value(request) {
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

        let payload = claude_payload(request, true)?;

        let mut extra_headers: Vec<(String, String)> = Vec::new();
        if let Some(beta_value) = beta_header_value(request) {
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

fn claude_payload(request: &MessageRequest, streaming: bool) -> Result<Value> {
    let mut payload = serde_json::to_value(request).map_err(VertexError::Serialization)?;
    let fields = payload
        .as_object_mut()
        .ok_or_else(|| VertexError::configuration("Claude Messages payload must be an object"))?;
    fields.remove("beta");
    if streaming {
        fields.insert("stream".to_string(), Value::Bool(true));
    } else if fields.get("stream") == Some(&Value::Bool(true)) {
        fields.remove("stream");
    }

    if let Some(choice) = &request.tool_choice {
        let value = match choice {
            ToolChoice::Auto => json!({"type": "auto"}),
            ToolChoice::Tool { name } => json!({"type": "tool", "name": name}),
            ToolChoice::Any => json!({"type": "any"}),
            ToolChoice::None => json!({"type": "none"}),
        };
        fields.insert("tool_choice".to_string(), value);
    }

    let effort = fields
        .get_mut("thinking")
        .and_then(Value::as_object_mut)
        .filter(|thinking| {
            matches!(
                thinking.get("type").and_then(Value::as_str),
                Some("adaptive" | "between_tools")
            )
        })
        .and_then(|thinking| {
            thinking.remove("allow_tool_use");
            thinking.remove("effort")
        });
    if let Some(effort) = effort {
        let output =
            fields.entry("output_config").or_insert_with(|| json!({})).as_object_mut().ok_or_else(
                || VertexError::configuration("Claude output_config must be an object"),
            )?;
        output.entry("effort").or_insert(effort);
    }

    if let Some(format) = payload
        .pointer_mut("/output_config/format")
        .and_then(Value::as_object_mut)
        .filter(|_| request.output_config.is_some())
    {
        format.remove("name");
        format.remove("description");
    }

    Ok(payload)
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
        Some(WebSearchToolType::VertexWebSearch) | None => {}
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
    use crate::claude::{OutputConfig, StopReason, ThinkingConfig, ThinkingEffort, WebSearchTool};

    #[test]
    fn vertex_web_search_uses_google_tool_type_without_beta() {
        let request = MessageRequest::new().add_web_search_tool(WebSearchTool::new_vertex());
        let payload = claude_payload(&request, false).expect("serialize Claude request");
        assert_eq!(payload["tools"], json!([{"type": "web_search", "name": "web_search"}]));
        assert!(beta_header_value(&request).is_none());
    }

    #[test]
    fn sends_adaptive_effort_with_structured_output() {
        let request = MessageRequest::new()
            .add_user_message("Extract the name")
            .thinking(ThinkingConfig::adaptive_with_tools(ThinkingEffort::Xhigh))
            .output_config(OutputConfig::json_schema_with_description(
                "result",
                "A name",
                json!({"type": "string"}),
            ))
            .tool_choice(ToolChoice::Auto)
            .stream(true);

        let payload = claude_payload(&request, false).expect("serialize Claude request");
        assert_eq!(payload["thinking"], json!({"type": "adaptive"}));
        assert_eq!(
            payload["output_config"],
            json!({"effort": "xhigh", "format": {"type": "json_schema", "schema": {"type": "string"}}})
        );
        assert_eq!(payload["tool_choice"], json!({"type": "auto"}));
        assert!(payload.get("stream").is_none());
    }

    #[test]
    fn sends_between_tools_thinking_in_streaming_requests() {
        let request = MessageRequest::new()
            .thinking(ThinkingConfig::between_tools(ThinkingEffort::Medium))
            .enable_beta_features(vec!["example-beta".to_string()]);

        let payload = claude_payload(&request, true).expect("serialize streaming Claude request");
        assert_eq!(payload["thinking"], json!({"type": "between_tools"}));
        assert_eq!(payload["output_config"], json!({"effort": "medium"}));
        assert_eq!(payload["stream"], true);
        assert!(payload.get("beta").is_none());
    }

    #[test]
    fn preserves_legacy_thinking_budget() {
        let request = MessageRequest::new().thinking(ThinkingConfig::enabled(4096));
        let payload = claude_payload(&request, false).expect("serialize legacy Claude request");
        assert_eq!(payload["thinking"], json!({"type": "enabled", "budget_tokens": 4096}));
        assert!(payload.get("output_config").is_none());
    }

    #[test]
    fn parses_refusal_from_current_claude_models() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "thinking", "thinking": "", "signature": "sig"}],
            "stop_reason": "refusal"
        }"#;
        let response = parse_claude_response(json).expect("parse refusal response");
        assert_eq!(response.stop_reason, Some(StopReason::Refusal));
    }

    #[test]
    fn parses_direct_response() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello", "citations": null}],
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
