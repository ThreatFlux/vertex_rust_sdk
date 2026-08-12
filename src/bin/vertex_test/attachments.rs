use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::{path::PathBuf, str::FromStr};
use threatflux_vertex_rust_sdk::claude::{
    ContentBlock, DocumentSource, ImageSource, Message, Role, Usage as ClaudeUsage,
};
use threatflux_vertex_rust_sdk::{
    classify_inline_data, models::StreamingResponse, types::Part, InlineDataKind,
};
use tokio::fs;

pub const MAX_INLINE_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const DEFAULT_INLINE_FILE_MIME: &str = "application/pdf";

#[derive(Clone, Debug)]
pub struct InputFileArg {
    pub path: PathBuf,
    pub mime_type: String,
}

impl FromStr for InputFileArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("input file argument cannot be empty".to_string());
        }

        let (path, mime) = match trimmed.split_once("::") {
            Some((path, _)) if path.trim().is_empty() => {
                return Err("input file path cannot be empty".to_string());
            }
            Some((path, mime)) if mime.trim().is_empty() => (path.trim(), DEFAULT_INLINE_FILE_MIME),
            Some((path, mime)) => (path.trim(), mime.trim()),
            None => (trimmed, DEFAULT_INLINE_FILE_MIME),
        };

        Ok(Self { path: PathBuf::from(path), mime_type: mime.to_string() })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineAttachment {
    pub filename: String,
    pub mime_type: String,
    pub data_base64: String,
    pub text_content: Option<String>,
    pub kind: InlineDataKind,
    pub size_bytes: usize,
}

impl InlineAttachment {
    pub fn descriptor(&self) -> String {
        format!("Attachment: {} ({}, {} bytes)", self.filename, self.mime_type, self.size_bytes)
    }

    pub fn gemini_parts(&self) -> Vec<Part> {
        match self.kind {
            InlineDataKind::Text => {
                let body = self.text_content.as_deref().unwrap_or_default();
                let descriptor = self.descriptor();
                if body.is_empty() {
                    vec![Part::text(descriptor)]
                } else {
                    vec![Part::text(format!("{descriptor}\n\n{body}"))]
                }
            }
            _ => vec![
                Part::text(self.descriptor()),
                Part::inline_data(self.data_base64.clone(), self.mime_type.clone()),
            ],
        }
    }

    pub fn claude_blocks(&self) -> Vec<ContentBlock> {
        match self.kind {
            InlineDataKind::Image => vec![
                ContentBlock::text(self.descriptor()),
                ContentBlock::Image {
                    source: ImageSource::base64(self.mime_type.clone(), self.data_base64.clone()),
                },
            ],
            InlineDataKind::Pdf => vec![
                ContentBlock::text(self.descriptor()),
                ContentBlock::Document {
                    source: DocumentSource::base64(
                        self.mime_type.clone(),
                        self.data_base64.clone(),
                    ),
                },
            ],
            InlineDataKind::Text => {
                let body = self.text_content.as_deref().unwrap_or_default();
                let descriptor = self.descriptor();
                if body.is_empty() {
                    vec![ContentBlock::text(descriptor)]
                } else {
                    vec![ContentBlock::text(format!("{descriptor}\n\n{body}"))]
                }
            }
            InlineDataKind::Binary => vec![ContentBlock::Document {
                source: DocumentSource::base64(self.mime_type.clone(), self.data_base64.clone()),
            }],
        }
    }
}

pub async fn load_inline_attachments(files: &[InputFileArg]) -> Result<Vec<InlineAttachment>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut attachments = Vec::with_capacity(files.len());

    for file in files {
        let metadata = fs::metadata(&file.path)
            .await
            .with_context(|| format!("failed to read metadata for {}", file.path.display()))?;

        if metadata.len() > MAX_INLINE_FILE_BYTES {
            return Err(anyhow!(
                "{} exceeds the inline upload limit of {} bytes; upload with the Vertex File API instead",
                file.path.display(),
                MAX_INLINE_FILE_BYTES
            ));
        }

        let bytes = fs::read(&file.path)
            .await
            .with_context(|| format!("failed to read {}", file.path.display()))?;

        let filename = file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "attachment".to_string());

        let classification = classify_inline_data(
            Some(file.mime_type.as_str()),
            file.path.file_name().and_then(|name| name.to_str()),
            &bytes,
        );

        if classification.kind == InlineDataKind::Binary {
            return Err(anyhow!(
                "{} has unsupported inline type {}; supported types are PDF, images, and UTF-8 text",
                file.path.display(),
                classification.mime_type
            ));
        }

        let encoded = BASE64_STANDARD.encode(&bytes);
        attachments.push(InlineAttachment {
            filename,
            mime_type: classification.mime_type,
            data_base64: encoded,
            text_content: classification.text,
            kind: classification.kind,
            size_bytes: bytes.len(),
        });
    }

    Ok(attachments)
}

pub fn build_claude_user_message(prompt: &str, attachments: &[InlineAttachment]) -> Message {
    if attachments.is_empty() {
        return Message::user(prompt.to_string());
    }

    let mut content = Vec::with_capacity(1 + attachments.len() * 2);
    content.push(ContentBlock::text(prompt.to_string()));

    for attachment in attachments {
        content.extend(attachment.claude_blocks());
    }

    Message::new(Role::User, content)
}

pub fn aggregate_stream_text(response: &StreamingResponse) -> String {
    let mut aggregated = String::new();

    if let Some(candidate) = response.candidates.first() {
        for part in &candidate.content.parts {
            if let Part::Text { text } = part {
                aggregated.push_str(text);
            }
        }
    }

    aggregated
}

pub fn merge_claude_usage(existing: Option<ClaudeUsage>, new_usage: &ClaudeUsage) -> ClaudeUsage {
    let mut usage = existing.unwrap_or_default();
    usage.input_tokens = usage.input_tokens.max(new_usage.input_tokens);
    usage.output_tokens = usage.output_tokens.max(new_usage.output_tokens);
    usage
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use threatflux_vertex_rust_sdk::types::Content;

    #[test]
    fn aggregate_stream_text_concatenates_parts() {
        let response = StreamingResponse {
            candidates: vec![threatflux_vertex_rust_sdk::types::Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![
                        Part::Text { text: "Hello".to_string() },
                        Part::Text { text: ", world".to_string() },
                    ],
                },
                finish_reason: None,
                safety_ratings: Vec::new(),
                index: Some(0),
            }],
            usage_metadata: None,
            grounding_metadata: None,
        };

        let aggregated = aggregate_stream_text(&response);
        assert_eq!(aggregated, "Hello, world");
    }

    #[test]
    fn parses_input_file_default_mime() {
        let arg = InputFileArg::from_str("/tmp/sample.pdf").expect("failed to parse input file");
        assert_eq!(arg.path, PathBuf::from("/tmp/sample.pdf"));
        assert_eq!(arg.mime_type, DEFAULT_INLINE_FILE_MIME);
    }

    #[test]
    fn parses_input_file_custom_mime() {
        let arg = InputFileArg::from_str("/tmp/sample.txt::text/plain")
            .expect("failed to parse custom mime input file");
        assert_eq!(arg.path, PathBuf::from("/tmp/sample.txt"));
        assert_eq!(arg.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn load_inline_attachments_encodes_bytes() {
        let mut temp = NamedTempFile::new().expect("failed to create temp file");
        temp.write_all(b"hello world").expect("failed to write temp file");

        let files = vec![InputFileArg {
            path: temp.path().to_path_buf(),
            mime_type: "application/pdf".to_string(),
        }];

        let attachments =
            load_inline_attachments(&files).await.expect("expected inline attachments");

        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].kind, InlineDataKind::Text);
        assert_eq!(attachments[0].mime_type, "text/plain");
        assert_eq!(attachments[0].text_content.as_deref(), Some("hello world"));
        let expected_name =
            temp.path().file_name().and_then(|name| name.to_str()).unwrap_or("attachment");
        assert_eq!(attachments[0].filename, expected_name);
        assert_eq!(attachments[0].size_bytes, 11);
        let decoded = BASE64_STANDARD
            .decode(attachments[0].data_base64.as_bytes())
            .expect("failed to decode base64 data");
        assert_eq!(decoded, b"hello world");
    }

    #[tokio::test]
    async fn load_inline_attachments_rejects_large_files() {
        let temp = NamedTempFile::new().expect("failed to create temp file");
        temp.as_file().set_len(MAX_INLINE_FILE_BYTES + 1).expect("failed to resize temp file");

        let files = vec![InputFileArg {
            path: temp.path().to_path_buf(),
            mime_type: DEFAULT_INLINE_FILE_MIME.to_string(),
        }];

        let err = load_inline_attachments(&files).await.expect_err("expected oversized file error");

        assert!(err.to_string().contains("inline upload limit"));
    }

    #[test]
    fn build_claude_message_adds_pdf_attachment_as_document() {
        let attachments = vec![InlineAttachment {
            filename: "report.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            data_base64: "ZGF0YQ==".to_string(),
            text_content: None,
            kind: InlineDataKind::Pdf,
            size_bytes: 4,
        }];

        let message = build_claude_user_message("Summarize", &attachments);
        assert_eq!(message.role, Role::User);
        assert_eq!(message.content.len(), 3);
        assert!(matches!(message.content[0], ContentBlock::Text { .. }));
        match &message.content[1] {
            ContentBlock::Text { text, .. } => {
                assert!(text.contains("report.pdf"));
                assert!(text.contains("application/pdf"));
            }
            other => panic!("expected descriptor text block, got {other:?}"),
        }
        assert!(matches!(message.content[2], ContentBlock::Document { .. }));
    }

    #[test]
    fn build_claude_message_adds_image_attachment_as_image() {
        let attachments = vec![InlineAttachment {
            filename: "diagram.png".to_string(),
            mime_type: "image/png".to_string(),
            data_base64: "ZGF0YQ==".to_string(),
            text_content: None,
            kind: InlineDataKind::Image,
            size_bytes: 4,
        }];

        let message = build_claude_user_message("Summarize", &attachments);
        assert_eq!(message.role, Role::User);
        assert_eq!(message.content.len(), 3);
        assert!(matches!(message.content[0], ContentBlock::Text { .. }));
        assert!(matches!(message.content[2], ContentBlock::Image { .. }));
        if let ContentBlock::Text { text, .. } = &message.content[1] {
            assert!(text.contains("diagram.png"));
            assert!(text.contains("image/png"));
        } else {
            panic!("expected descriptor text block");
        }
    }

    #[test]
    fn build_claude_message_adds_text_attachment_as_text() {
        let attachments = vec![InlineAttachment {
            filename: "notes.txt".to_string(),
            mime_type: "text/plain".to_string(),
            data_base64: "ZGF0YQ==".to_string(),
            text_content: Some("data".to_string()),
            kind: InlineDataKind::Text,
            size_bytes: 4,
        }];

        let message = build_claude_user_message("Summarize", &attachments);
        assert_eq!(message.role, Role::User);
        assert_eq!(message.content.len(), 2);
        assert!(matches!(message.content[0], ContentBlock::Text { .. }));
        match &message.content[1] {
            ContentBlock::Text { text, .. } => {
                assert!(text.starts_with("Attachment: notes.txt"));
                assert!(text.contains("\n\ndata"));
            }
            other => panic!("expected text block with contents, got {other:?}"),
        }
    }

    #[test]
    fn merge_usage_prefers_largest_counts() {
        let merged = merge_claude_usage(
            Some(ClaudeUsage { input_tokens: 10, output_tokens: 5, ..Default::default() }),
            &ClaudeUsage { input_tokens: 12, output_tokens: 3, ..Default::default() },
        );
        assert_eq!(merged.input_tokens, 12);
        assert_eq!(merged.output_tokens, 5);

        let merged = merge_claude_usage(
            None,
            &ClaudeUsage { input_tokens: 1, output_tokens: 2, ..Default::default() },
        );
        assert_eq!(merged.input_tokens, 1);
        assert_eq!(merged.output_tokens, 2);
    }
}
