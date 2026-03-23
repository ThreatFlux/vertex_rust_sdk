use std::io::{self, Write};

use colored::Colorize;
use threatflux_vertex_rust_sdk::{
    models::GenerateContentResponse,
    types::{GroundingMetadata, GroundingSupport},
};

pub fn display_grounding_info(response: &GenerateContentResponse) {
    if let Some(metadata) = response.grounding_metadata() {
        let _ = write_grounding_metadata(metadata, &mut io::stdout());
    }
}

pub fn display_grounding_metadata(metadata: &GroundingMetadata) {
    let _ = write_grounding_metadata(metadata, &mut io::stdout());
}

pub fn write_grounding_metadata(
    metadata: &GroundingMetadata,
    writer: &mut impl Write,
) -> io::Result<()> {
    if metadata.web_search_queries.is_none()
        && metadata.grounding_chunks.is_none()
        && metadata.grounding_supports.is_none()
    {
        return Ok(());
    }

    writeln!(writer, "\n{}", "🔍 Grounding Information:".bold().blue())?;
    writeln!(writer, "{}", "─".repeat(60).blue())?;

    if let Some(queries) = &metadata.web_search_queries {
        if !queries.is_empty() {
            writeln!(writer, "{}", "Search Queries:".bold().yellow())?;
            for (i, query) in queries.iter().enumerate() {
                writeln!(writer, "  {}. {}", i + 1, query.italic().cyan())?;
            }
            writeln!(writer)?;
        }
    }

    if let Some(chunks) = &metadata.grounding_chunks {
        if !chunks.is_empty() {
            writeln!(writer, "{}", "Sources:".bold().yellow())?;
            for (i, chunk) in chunks.iter().enumerate() {
                writeln!(writer, "  {}. {}", i + 1, "Source".green().bold())?;
                if let Some(title) = &chunk.title {
                    writeln!(writer, "     Title: {}", title.bold())?;
                }
                if let Some(uri) = &chunk.uri {
                    writeln!(writer, "     URL: {}", uri.blue().underline())?;
                }
                if let Some(content) = &chunk.content {
                    let preview = if content.len() > 150 {
                        format!("{}...", &content[..150])
                    } else {
                        content.clone()
                    };
                    writeln!(writer, "     Preview: {}", preview.italic().dimmed())?;
                }
                writeln!(writer)?;
            }
        }
    }

    if let Some(supports) = &metadata.grounding_supports {
        if !supports.is_empty() {
            writeln!(writer, "{}", "Grounding Support:".bold().yellow())?;
            for (i, support) in supports.iter().enumerate() {
                write_support(writer, i, support)?;
            }
        }
    }

    writeln!(writer, "{}", "─".repeat(60).blue())
}

fn write_support(
    writer: &mut impl Write,
    index: usize,
    support: &GroundingSupport,
) -> io::Result<()> {
    writeln!(writer, "  {}. {}", index + 1, "Support".green().bold())?;
    if let Some(score) = support.confidence_score {
        writeln!(writer, "     Confidence: {:.2}%", score * 100.0)?;
    }
    if let Some(text) = &support.text {
        writeln!(writer, "     Supported Text: \"{}\"", text.italic())?;
    }
    if let Some(indices) = &support.grounding_chunk_indices {
        if !indices.is_empty() {
            let source_refs: Vec<String> = indices.iter().map(|i| (i + 1).to_string()).collect();
            writeln!(writer, "     Sources: {}", source_refs.join(", ").cyan())?;
        }
    }
    writeln!(writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_grounding_metadata_to_writer() {
        let metadata = GroundingMetadata {
            web_search_queries: Some(vec!["rust async".into()]),
            search_entry_point: None,
            grounding_chunks: Some(vec![threatflux_vertex_rust_sdk::types::GroundingChunk {
                content: Some("Example content from a source".into()),
                uri: Some("https://example.com".into()),
                title: Some("Example".into()),
            }]),
            grounding_supports: Some(vec![GroundingSupport {
                grounding_chunk_indices: Some(vec![0]),
                confidence_score: Some(0.8),
                start_index: None,
                end_index: None,
                text: Some("supported text".into()),
            }]),
        };

        let mut buffer = Vec::new();
        write_grounding_metadata(&metadata, &mut buffer).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("Grounding Information"));
        assert!(output.contains("Search Queries"));
        assert!(output.contains("Sources"));
        assert!(output.contains("Grounding Support"));
    }

    #[test]
    fn no_output_when_metadata_empty() {
        let metadata = GroundingMetadata {
            web_search_queries: None,
            search_entry_point: None,
            grounding_chunks: None,
            grounding_supports: None,
        };

        let mut buffer = Vec::new();
        write_grounding_metadata(&metadata, &mut buffer).unwrap();

        assert!(buffer.is_empty());
    }
}
