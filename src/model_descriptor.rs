use crate::error::VertexError;

/// Represents a Vertex AI model reference including its publisher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDescriptor {
    publisher: String,
    model: String,
}

impl ModelDescriptor {
    /// Create a new descriptor from publisher and model name.
    #[must_use]
    pub fn new<P: Into<String>, M: Into<String>>(publisher: P, model: M) -> Self {
        Self { publisher: publisher.into(), model: model.into() }
    }

    /// Parse a model string into a descriptor.
    ///
    /// Accepts any of the following formats:
    /// - `publishers/{publisher}/models/{model}`
    /// - `projects/{project}/locations/{location}/publishers/{publisher}/models/{model}`
    /// - `models/{model}` (defaults to Google publisher)
    /// - `{model}` where publisher is inferred (Anthropic for Claude*, Google otherwise)
    /// - `{publisher}/{model}` or `{publisher}:{model}`
    ///
    /// # Errors
    ///
    /// Returns an error when the provided model identifier is empty or does not
    /// match a supported format.
    pub fn parse(model: &str) -> Result<Self, VertexError> {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            return Err(VertexError::configuration("Model name cannot be empty"));
        }

        if let Some(idx) = trimmed.find("/publishers/") {
            // Strip leading project/locations prefix if present
            let publisher_segment = &trimmed[idx + 1..];
            return Self::parse(publisher_segment);
        }

        if trimmed.starts_with("publishers/") {
            return Self::parse_publisher_path(trimmed);
        }

        if trimmed.starts_with("models/") {
            let model_name = trimmed.trim_start_matches("models/");
            return Ok(Self::new("google", model_name));
        }

        if let Some((publisher, model_name)) = trimmed.split_once('/') {
            if !publisher.is_empty() && !model_name.is_empty() {
                return Ok(Self::new(publisher, model_name));
            }
        }

        if let Some((publisher, model_name)) = trimmed.split_once(':') {
            if !publisher.is_empty() && !model_name.is_empty() {
                return Ok(Self::new(publisher, model_name));
            }
        }

        let publisher = Self::infer_publisher(trimmed);
        Ok(Self::new(publisher, trimmed))
    }

    /// Returns the Vertex relative path `publishers/{publisher}/models/{model}`.
    #[must_use]
    pub fn relative_path(&self) -> String {
        format!("publishers/{}/models/{}", self.publisher, self.model)
    }

    /// Returns the full resource path `projects/{project}/locations/{location}/publishers/{publisher}/models/{model}`.
    #[must_use]
    pub fn resource_path(&self, project: &str, location: &str) -> String {
        format!("projects/{}/locations/{}/{}", project, location, self.relative_path())
    }

    /// Publisher identifier (e.g., `google`, `anthropic`).
    #[must_use]
    pub fn publisher(&self) -> &str {
        &self.publisher
    }

    /// Model identifier (e.g., `gemini-2.0-flash-001`).
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    fn parse_publisher_path(path: &str) -> Result<Self, VertexError> {
        // Expected: publishers/{publisher}/models/{model}
        let mut segments = path.split('/');
        let prefix = segments.next();
        if prefix != Some("publishers") {
            return Err(VertexError::configuration("Model path must start with 'publishers/'"));
        }

        let publisher = segments
            .next()
            .ok_or_else(|| VertexError::configuration("Missing publisher segment"))?;

        let models_keyword = segments
            .next()
            .ok_or_else(|| VertexError::configuration("Missing 'models' segment"))?;
        if models_keyword != "models" {
            return Err(VertexError::configuration("Model path must include 'models/'"));
        }

        let model_segments: Vec<&str> = segments.collect();
        if model_segments.is_empty() {
            return Err(VertexError::configuration("Missing model identifier"));
        }

        Ok(Self::new(publisher, model_segments.join("/")))
    }

    fn infer_publisher(model: &str) -> String {
        let lower = model.to_ascii_lowercase();
        if lower.contains("claude")
            || lower.contains("opus")
            || lower.contains("sonnet")
            || lower.contains("haiku")
        {
            "anthropic".to_string()
        } else {
            "google".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_path() {
        let descriptor = ModelDescriptor::parse(
            "projects/demo/locations/us/publishers/anthropic/models/claude-sonnet-4-5@20250929",
        )
        .unwrap();
        assert_eq!(descriptor.publisher(), "anthropic");
        assert_eq!(descriptor.model(), "claude-sonnet-4-5@20250929");
        assert_eq!(
            descriptor.relative_path(),
            "publishers/anthropic/models/claude-sonnet-4-5@20250929"
        );
    }

    #[test]
    fn parse_relative_path() {
        let descriptor =
            ModelDescriptor::parse("publishers/google/models/gemini-2.5-flash").unwrap();
        assert_eq!(descriptor.publisher(), "google");
        assert_eq!(descriptor.model(), "gemini-2.5-flash");
    }

    #[test]
    fn parse_infers_anthropic() {
        let descriptor = ModelDescriptor::parse("claude-sonnet-4-5@20250929").unwrap();
        assert_eq!(descriptor.publisher(), "anthropic");
    }

    #[test]
    fn parse_short_google() {
        let descriptor = ModelDescriptor::parse("gemini-2.5-flash").unwrap();
        assert_eq!(descriptor.publisher(), "google");
        assert_eq!(descriptor.relative_path(), "publishers/google/models/gemini-2.5-flash");
    }

    #[test]
    fn parse_custom_separator() {
        let descriptor = ModelDescriptor::parse("anthropic:claude-sonnet-4-5").unwrap();
        assert_eq!(descriptor.publisher(), "anthropic");
        assert_eq!(descriptor.model(), "claude-sonnet-4-5");
    }

    #[test]
    fn resource_path_composes() {
        let descriptor = ModelDescriptor::parse("claude-sonnet-4-5@20250929").unwrap();
        assert_eq!(
            descriptor.resource_path("demo", "global"),
            "projects/demo/locations/global/publishers/anthropic/models/claude-sonnet-4-5@20250929"
        );
    }
}
