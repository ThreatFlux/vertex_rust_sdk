use std::borrow::Cow;

/// Classification of inline data for Vertex requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineDataKind {
    /// Image data (e.g., PNG, JPEG).
    Image,
    /// PDF document data.
    Pdf,
    /// UTF-8 text content.
    Text,
    /// Any other binary payload.
    Binary,
}

/// Result of classifying inline bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineDataClassification {
    pub mime_type: String,
    pub kind: InlineDataKind,
    pub text: Option<String>,
}

/// Determine the inline data kind based on MIME type, filename, and byte content.
///
/// The classifier normalizes common cases so callers can route PDFs, images, and
/// UTF-8 text appropriately while falling back to binary for everything else.
pub fn classify_inline_data(
    provided_mime: Option<&str>,
    filename: Option<&str>,
    bytes: &[u8],
) -> InlineDataClassification {
    let initial_mime = provided_mime
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let mut mime = normalize_mime(&initial_mime);
    let extension = filename
        .and_then(|name| name.rsplit('.').next())
        .map(|ext| ext.trim().to_ascii_lowercase());
    let ext_inferred = extension.as_deref().and_then(|ext| infer_mime_from_extension(ext));
    let utf8_guess = utf8_text(bytes);
    let pdf_signature = bytes.starts_with(b"%PDF");

    if mime == "application/octet-stream" {
        if let Some(inferred) = ext_inferred {
            mime = inferred.to_string();
        }
    } else if let Some(inferred) = ext_inferred {
        let provided_kind = kind_from_mime(&mime);
        let inferred_kind = kind_from_mime(inferred);
        if provided_kind == InlineDataKind::Binary && inferred_kind != InlineDataKind::Binary {
            mime = inferred.to_string();
        } else if provided_kind != inferred_kind {
            match inferred_kind {
                InlineDataKind::Pdf if pdf_signature => mime = inferred.to_string(),
                InlineDataKind::Image => mime = inferred.to_string(),
                InlineDataKind::Text if utf8_guess.is_some() => mime = inferred.to_string(),
                _ => {}
            }
        }
    }

    if mime == "application/pdf" && pdf_signature {
        return InlineDataClassification { mime_type: mime, kind: InlineDataKind::Pdf, text: None };
    }

    if mime.starts_with("image/") {
        return InlineDataClassification {
            mime_type: mime,
            kind: InlineDataKind::Image,
            text: None,
        };
    }

    let ext_is_text = extension.as_deref().is_some_and(is_text_extension);
    let mime_is_text = is_text_mime(&mime);

    if let Some(text) = utf8_guess.clone() {
        if mime_is_text
            || ext_is_text
            || provided_mime.is_none()
            || (mime == "application/pdf" && !pdf_signature)
        {
            let canonical_mime = if mime_is_text {
                mime
            } else if ext_is_text {
                ext_inferred.map_or_else(|| "text/plain".to_string(), ToString::to_string)
            } else {
                "text/plain".to_string()
            };

            return InlineDataClassification {
                mime_type: canonical_mime,
                kind: InlineDataKind::Text,
                text: Some(text.into_owned()),
            };
        }
    }

    InlineDataClassification { mime_type: mime, kind: InlineDataKind::Binary, text: None }
}

fn normalize_mime(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    lower.find(';').map_or_else(|| lower.trim().to_string(), |idx| lower[..idx].trim().to_string())
}

fn kind_from_mime(mime: &str) -> InlineDataKind {
    if mime == "application/pdf" {
        InlineDataKind::Pdf
    } else if mime.starts_with("image/") {
        InlineDataKind::Image
    } else if is_text_mime(mime) {
        InlineDataKind::Text
    } else {
        InlineDataKind::Binary
    }
}

fn infer_mime_from_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "pdf" => Some("application/pdf"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "tif" | "tiff" => Some("image/tiff"),
        "heic" => Some("image/heic"),
        "heif" => Some("image/heif"),
        "txt" | "log" | "text" | "ini" | "conf" | "config" | "cfg" | "py" | "rs" | "js" | "ts"
        | "tsx" | "jsx" | "java" | "kt" | "kts" | "go" | "rb" | "php" | "swift" | "scala"
        | "sql" | "c" | "cc" | "cpp" | "h" | "hpp" | "sh" | "bash" | "zsh" | "ps1" | "psm1"
        | "lua" => Some("text/plain"),
        "md" | "markdown" => Some("text/markdown"),
        "json" => Some("application/json"),
        "yml" | "yaml" => Some("application/x-yaml"),
        "toml" => Some("application/toml"),
        "csv" => Some("text/csv"),
        "tsv" => Some("text/tab-separated-values"),
        "html" | "htm" => Some("text/html"),
        "css" => Some("text/css"),
        "xml" => Some("application/xml"),
        _ => None,
    }
}

fn is_text_mime(mime: &str) -> bool {
    if mime.starts_with("text/") {
        return true;
    }

    matches!(
        mime,
        "application/json"
            | "application/ld+json"
            | "application/xml"
            | "application/xhtml+xml"
            | "application/javascript"
            | "application/x-javascript"
            | "application/typescript"
            | "application/x-typescript"
            | "application/x-sh"
            | "application/x-shellscript"
            | "application/x-python"
            | "application/sql"
            | "application/x-sql"
            | "application/x-www-form-urlencoded"
            | "application/x-yaml"
            | "application/yaml"
            | "application/toml"
            | "application/csv"
    ) || mime.ends_with("+json")
        || mime.ends_with("+xml")
}

fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "log"
            | "text"
            | "md"
            | "markdown"
            | "json"
            | "yml"
            | "yaml"
            | "toml"
            | "csv"
            | "tsv"
            | "html"
            | "htm"
            | "css"
            | "xml"
            | "ini"
            | "conf"
            | "config"
            | "cfg"
            | "py"
            | "rs"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "java"
            | "kt"
            | "kts"
            | "go"
            | "rb"
            | "php"
            | "swift"
            | "scala"
            | "sql"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "sh"
            | "bash"
            | "zsh"
            | "ps1"
            | "psm1"
            | "lua"
    )
}

fn utf8_text(bytes: &[u8]) -> Option<Cow<'_, str>> {
    if bytes.contains(&0) {
        return None;
    }

    std::str::from_utf8(bytes).map(Cow::from).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_pdf_with_signature() {
        let bytes = b"%PDF-sample";
        let result = classify_inline_data(Some("application/pdf"), Some("file.pdf"), bytes);
        assert_eq!(result.mime_type, "application/pdf");
        assert_eq!(result.kind, InlineDataKind::Pdf);
        assert!(result.text.is_none());
    }

    #[test]
    fn rejects_pdf_without_signature() {
        let bytes = b"plain text masquerading as pdf";
        let result = classify_inline_data(Some("application/pdf"), Some("notes.txt"), bytes);
        assert_eq!(result.kind, InlineDataKind::Text);
        assert_eq!(result.mime_type, "text/plain");
        assert_eq!(result.text.as_deref(), Some("plain text masquerading as pdf"));
    }

    #[test]
    fn infers_image_from_extension() {
        let bytes = [0u8; 10];
        let result = classify_inline_data(None, Some("image.PNG"), &bytes);
        assert_eq!(result.mime_type, "image/png");
        assert_eq!(result.kind, InlineDataKind::Image);
    }

    #[test]
    fn detects_utf8_text_without_mime() {
        let bytes = b"hello world";
        let result = classify_inline_data(None, Some("notes.txt"), bytes);
        assert_eq!(result.kind, InlineDataKind::Text);
        assert_eq!(result.mime_type, "text/plain");
        assert_eq!(result.text.as_deref(), Some("hello world"));
    }

    #[test]
    fn honors_text_mime() {
        let bytes = b"{}";
        let result = classify_inline_data(Some("application/json"), None, bytes);
        assert_eq!(result.kind, InlineDataKind::Text);
        assert_eq!(result.mime_type, "application/json");
    }

    #[test]
    fn falls_back_to_binary() {
        let bytes = [0u8, 159, 255, 0];
        let result = classify_inline_data(None, Some("data.bin"), &bytes);
        assert_eq!(result.kind, InlineDataKind::Binary);
        assert_eq!(result.mime_type, "application/octet-stream");
        assert!(result.text.is_none());
    }
}
