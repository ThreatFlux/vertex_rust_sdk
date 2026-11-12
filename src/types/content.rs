use serde::{Deserialize, Serialize};

use super::code_execution::{CodeExecutionResult, ExecutableCode, Language, Outcome};
use super::function_calling::{FunctionCall, FunctionResponse};

/// Thinking content from models with thinking mode enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingPart {
    /// The thinking content/process text.
    pub content: String,
}

/// Content part - can be text, file data, function call, code execution, or thinking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    /// A content part with Vertex thought or speech metadata.
    Annotated(AnnotatedPart),

    /// Text content.
    Text { text: String },

    /// Inline data content.
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },

    /// File data content.
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileData,
    },

    /// Function call.
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },

    /// Function response.
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },

    /// Executable code.
    ExecutableCode {
        #[serde(rename = "executableCode")]
        executable_code: ExecutableCode,
    },

    /// Code execution result.
    CodeExecutionResult {
        #[serde(rename = "codeExecutionResult")]
        code_execution_result: CodeExecutionResult,
    },

    /// Thinking process content.
    Thinking {
        #[serde(rename = "thought")]
        thought: String,
    },
}

impl Part {
    /// Create a text part.
    #[must_use]
    pub fn text<S: Into<String>>(text: S) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an inline data part.
    #[must_use]
    pub fn inline_data<D: Into<String>, M: Into<String>>(data: D, mime_type: M) -> Self {
        Self::InlineData {
            inline_data: InlineData { data: data.into(), mime_type: mime_type.into() },
        }
    }

    /// Create a file data part.
    #[must_use]
    pub const fn file_data(file_uri: String, mime_type: String) -> Self {
        Self::FileData { file_data: FileData { file_uri, mime_type } }
    }

    /// Create an executable code part.
    #[must_use]
    pub const fn executable_code(language: Language, code: String) -> Self {
        Self::ExecutableCode { executable_code: ExecutableCode { language, code } }
    }

    /// Create a code execution result part.
    #[must_use]
    pub const fn code_execution_result(outcome: Outcome, output: String) -> Self {
        Self::CodeExecutionResult { code_execution_result: CodeExecutionResult { outcome, output } }
    }

    /// Create a thinking part.
    #[must_use]
    pub fn thinking<S: Into<String>>(thought: S) -> Self {
        Self::Thinking { thought: thought.into() }
    }

    /// Create a text-to-speech part with an optional speaker and speaking style.
    #[must_use]
    pub fn speech_text(text: impl Into<String>, metadata: SpeechMetadata) -> Self {
        Self::Annotated(AnnotatedPart {
            metadata: PartMetadata { speech_metadata: Some(metadata), ..Default::default() },
            part: Some(Box::new(Self::text(text))),
        })
    }

    /// The opaque thought signature to preserve when sending this part back to Vertex.
    #[must_use]
    pub fn thought_signature(&self) -> Option<&str> {
        match self {
            Self::Annotated(part) => part.metadata.thought_signature.as_deref(),
            _ => None,
        }
    }

    /// Access normal text, excluding model thinking summaries.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            Self::Annotated(part) if part.metadata.thought != Some(true) => part
                .part
                .as_deref()
                .and_then(Self::as_text)
                .or_else(|| part.metadata.audio_transcription.as_ref()?.get("text")?.as_str()),
            _ => None,
        }
    }

    /// Access a model thinking summary.
    #[must_use]
    pub fn as_thinking(&self) -> Option<&str> {
        match self {
            Self::Thinking { thought } => Some(thought),
            Self::Annotated(part) if part.metadata.thought == Some(true) => {
                part.part.as_deref()?.as_text()
            }
            _ => None,
        }
    }

    /// Access a function call, including any call ID.
    #[must_use]
    pub fn as_function_call(&self) -> Option<&FunctionCall> {
        match self.payload()? {
            Self::FunctionCall { function_call } => Some(function_call),
            _ => None,
        }
    }

    /// Access generated or supplied inline media, including images and audio.
    #[must_use]
    pub fn as_inline_data(&self) -> Option<&InlineData> {
        match self.payload()? {
            Self::InlineData { inline_data } => Some(inline_data),
            _ => None,
        }
    }

    /// Access transcription details such as speaker labels and word timings.
    #[must_use]
    pub const fn audio_transcription(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Annotated(part) => part.metadata.audio_transcription.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn payload(&self) -> Option<&Self> {
        match self {
            Self::Annotated(part) => part.part.as_deref(),
            _ => Some(self),
        }
    }
}

/// A part with its model-provided metadata retained for subsequent turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedPart {
    #[serde(flatten)]
    metadata: PartMetadata,
    #[serde(
        flatten,
        serialize_with = "serialize_payload",
        deserialize_with = "deserialize_payload",
        skip_serializing_if = "Option::is_none"
    )]
    part: Option<Box<Part>>,
}

// Convert through a JSON object to keep the recursive part on a single
// serializer and deserializer type when flattened.
#[allow(clippy::ref_option)]
fn serialize_payload<S: serde::Serializer>(
    part: &Option<Box<Part>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let value = serde_json::to_value(part).map_err(serde::ser::Error::custom)?;
    value.serialize(serializer)
}

fn deserialize_payload<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Box<Part>>, D::Error> {
    let payload = serde_json::Map::deserialize(deserializer)?;
    if payload.is_empty() {
        return Ok(None);
    }
    let part = serde_json::from_value(serde_json::Value::Object(payload))
        .map_err(serde::de::Error::custom)?;
    Ok(Some(part))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "PartMetadataWire")]
struct PartMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    thought_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thought: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_metadata: Option<SpeechMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_transcription: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartMetadataWire {
    thought_signature: Option<String>,
    thought: Option<bool>,
    speech_metadata: Option<SpeechMetadata>,
    audio_transcription: Option<serde_json::Value>,
}

impl TryFrom<PartMetadataWire> for PartMetadata {
    type Error = &'static str;

    fn try_from(wire: PartMetadataWire) -> Result<Self, Self::Error> {
        let PartMetadataWire { thought_signature, thought, speech_metadata, audio_transcription } =
            wire;
        if thought_signature.is_none()
            && thought.is_none()
            && speech_metadata.is_none()
            && audio_transcription.is_none()
        {
            return Err("part has no annotations");
        }
        Ok(Self { thought_signature, thought, speech_metadata, audio_transcription })
    }
}

/// Directions for a Gemini text-to-speech segment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpeechMetadata {
    /// Speaker name matching the speech configuration for multi-speaker audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// Natural-language directions for how to speak this segment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

/// File data for multimodal content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    #[serde(rename = "fileUri")]
    pub file_uri: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// Inline data for multimodal content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

/// Content with role and parts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Content {
    /// Role of the content (user, model, system).
    #[serde(default)]
    pub role: String,
    /// Parts of the content.
    #[serde(default)]
    pub parts: Vec<Part>,
}

impl Content {
    /// Create user content with text.
    #[must_use]
    pub fn user_text<S: Into<String>>(text: S) -> Self {
        Self { role: "user".to_string(), parts: vec![Part::text(text)] }
    }

    /// Create model content with text.
    #[must_use]
    pub fn model_text<S: Into<String>>(text: S) -> Self {
        Self { role: "model".to_string(), parts: vec![Part::text(text)] }
    }

    /// Create system content with text.
    #[must_use]
    pub fn system_text<S: Into<String>>(text: S) -> Self {
        Self { role: "system".to_string(), parts: vec![Part::text(text)] }
    }
}
