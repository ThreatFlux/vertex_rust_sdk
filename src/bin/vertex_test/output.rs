use crate::vertex_test::config::{extract_query_from_value, host_from_url_str};
use colored::Colorize;
use serde_json::Value;
use std::collections::HashSet;
use std::io::{self, Write};
use threatflux_vertex_rust_sdk::claude::{
    Citation, ContentBlock, WebSearchErrorCode, WebSearchToolContent,
};

pub fn print_claude_blocks(blocks: &[ContentBlock]) {
    let mut text_segments: Vec<String> = Vec::new();
    let mut citations: Vec<Citation> = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::Text { text, citations: block_citations } => {
                if !text.trim().is_empty() {
                    text_segments.push(text.clone());
                }
                if !block_citations.is_empty() {
                    citations.extend(block_citations.clone());
                }
            }
            other => display_non_text_block(other),
        }
    }

    if !text_segments.is_empty() {
        println!("{}", text_segments.join("\n\n"));
    }

    if !citations.is_empty() {
        display_citations(&citations);
    }
}

pub fn display_non_text_block(block: &ContentBlock) {
    match block {
        ContentBlock::ServerToolUse { name, input, .. } => display_server_tool_use(name, input),
        ContentBlock::ToolUse { name, input, .. } => display_custom_tool_use(name, input),
        ContentBlock::ToolResult { tool_use_id, content, is_error } => {
            display_tool_result(tool_use_id, content.as_deref(), *is_error);
        }
        ContentBlock::WebSearchToolResult { content, .. } => display_web_search_content(content),
        _ => {}
    }
}

pub fn display_server_tool_use(name: &str, input: &Value) {
    println!("\n{}", format!("Tool request: {name}").bold().yellow());

    if let Some(query) = extract_query_from_value(input) {
        println!("  Query: {}", query.cyan());
        if is_query_only(input) {
            return;
        }
    }

    println!("  Input: {}", pretty_json(input).dimmed());
}

pub fn display_custom_tool_use(name: &str, input: &Value) {
    println!("\n{}", format!("Custom tool call requested: {name}").bold().yellow());
    println!("  Input: {}", pretty_json(input).dimmed());
}

pub fn display_tool_result(tool_use_id: &str, content: Option<&str>, is_error: Option<bool>) {
    let header = format!("Tool result [{tool_use_id}]");
    let formatted_header =
        if is_error.unwrap_or(false) { header.bold().red() } else { header.bold().green() };

    println!("\n{formatted_header}");

    if let Some(body) = content {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return;
        }

        match serde_json::from_str::<Value>(trimmed) {
            Ok(json) => println!("  {}", pretty_json(&json).dimmed()),
            Err(_) => println!("  {trimmed}"),
        }
    }
}

pub fn display_web_search_content(content: &WebSearchToolContent) {
    match content {
        WebSearchToolContent::Results(results) => {
            let mut seen = HashSet::new();
            let mut indexed = 0usize;

            for result in results {
                let url = result.url.trim();
                if url.is_empty() || !seen.insert(url.to_string()) {
                    continue;
                }

                if indexed == 0 {
                    println!("\n{}", "Web search results:".bold().yellow());
                }

                indexed += 1;
                let title =
                    if result.title.trim().is_empty() { url } else { result.title.as_str() };

                let host = host_from_url_str(url).unwrap_or_else(|| url.to_string());

                println!("  {}. {}", indexed, title.bold());
                println!("     {}", url.blue().underline());
                println!("     {}", host.italic().dimmed());

                if let Some(page_age) = &result.page_age {
                    if !page_age.trim().is_empty() {
                        println!("     Updated: {}", page_age.dimmed());
                    }
                }
            }

            if indexed == 0 {
                println!("\n{}", "Web search returned no usable results.".italic().dimmed());
            }
        }
        WebSearchToolContent::Error(error) => {
            println!("\n{}", "Web search error:".bold().red());
            println!("  Code: {}", web_search_error_code_to_str(&error.error_code).italic());
        }
    }
}

pub fn display_citations(citations: &[Citation]) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut unique: Vec<Citation> = Vec::new();

    for citation in citations {
        let url = citation.url.trim();
        if url.is_empty() {
            continue;
        }

        if seen.insert(url.to_string()) {
            unique.push(citation.clone());
        }
    }

    if unique.is_empty() {
        return;
    }

    println!("\n{}", "Citations:".bold().yellow());
    for (index, citation) in unique.iter().enumerate() {
        let url = citation.url.trim();
        let title = if citation.title.trim().is_empty() { url } else { citation.title.trim() };

        println!("  {}. {} {}", index + 1, title.bold(), url.blue().underline());

        if let Some(excerpt) = &citation.cited_text {
            let trimmed = excerpt.trim();
            if !trimmed.is_empty() {
                println!("     \"{}\"", trimmed.italic());
            }
        }
    }
}

pub fn is_query_only(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            if map.len() != 1 {
                return false;
            }
            map.get("query").and_then(Value::as_str).is_some_and(|s| !s.trim().is_empty())
        }
        _ => false,
    }
}

pub fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

pub const fn web_search_error_code_to_str(code: &WebSearchErrorCode) -> &'static str {
    match code {
        WebSearchErrorCode::TooManyRequests => "too_many_requests",
        WebSearchErrorCode::InvalidInput => "invalid_input",
        WebSearchErrorCode::MaxUsesExceeded => "max_uses_exceeded",
        WebSearchErrorCode::QueryTooLong => "query_too_long",
        WebSearchErrorCode::Unavailable => "unavailable",
    }
}

pub fn print_stream_delta(delta: &str) -> io::Result<()> {
    if delta.is_empty() {
        return Ok(());
    }
    print!("{delta}");
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use threatflux_vertex_rust_sdk::claude::{WebSearchResult, WebSearchToolError};

    #[test]
    fn detects_query_only_inputs() {
        assert!(is_query_only(&json!({"query": "hello"})));
        assert!(!is_query_only(&json!({"query": ""})));
        assert!(!is_query_only(&json!({"query": "hi", "other": 1})));
        assert!(!is_query_only(&json!(["query"])));
    }

    #[test]
    fn formats_web_search_results_and_errors() {
        let results = WebSearchToolContent::Results(vec![WebSearchResult {
            result_type: "web".to_string(),
            title: "Example".to_string(),
            url: "https://example.com".to_string(),
            encrypted_content: None,
            page_age: Some("Today".to_string()),
        }]);
        display_web_search_content(&results);

        let error = WebSearchToolContent::Error(WebSearchToolError {
            error_type: "error".to_string(),
            error_code: WebSearchErrorCode::Unavailable,
        });
        display_web_search_content(&error);
    }

    #[test]
    fn prints_tool_results_without_crashing_on_invalid_json() {
        display_tool_result("id", Some("{invalid"), Some(false));
        display_tool_result("id", Some("{\"a\":1}"), Some(true));
    }
}
