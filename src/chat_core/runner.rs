use crate::chat_core::commands::{parse_command, Command};
use crate::chat_core::config::{validate_temperature, ChatConfig};
use crate::chat_core::io::{Input, Output};
use crate::chat_core::service::ChatService;
use crate::chat_core::session::{ChatSession, SessionStats};
use crate::{GenerateContentRequest, UsageMetadata};
use anyhow::{anyhow, Result};
use futures_util::StreamExt;

pub async fn run_chat<S, I, O>(
    config: ChatConfig,
    service: &S,
    input: &mut I,
    output: &mut O,
) -> Result<()>
where
    S: ChatService + Sync,
    I: Input,
    O: Output,
{
    render_header(&config, output)?;
    let mut session = ChatSession::new(&config);

    loop {
        prompt_user(output)?;
        let line = input.read_line()?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(command) = parse_command(trimmed) {
            match handle_command(command, &mut session, &config, input, output) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => render_error(&e, output)?,
            }
            continue;
        }

        session.add_user_message(trimmed.to_string());
        output.print(&format!("{} ", style_text("Assistant:", Style::AssistantLabel)))?;
        output.flush()?;

        let request = session.build_request();
        match stream_response(&config.model, service, request, output).await {
            Ok(Some(response_text)) => session.add_model_message(response_text),
            Ok(None) => session.rollback_last(),
            Err(e) => {
                session.rollback_last();
                render_error(&e, output)?;
            }
        }
    }

    Ok(())
}

fn render_header<O: Output>(config: &ChatConfig, output: &mut O) -> Result<()> {
    output.println(&style_text("=== Vertex AI Chat ===", Style::Header))?;
    output.println(&format!("Project: {}", config.project))?;
    output.println(&format!("Location: {}", config.location))?;
    output.println(&format!("Model: {}", config.model))?;
    output.println(&format!("Temperature: {:.1}", config.temperature))?;
    output.println("")?;
    output
        .println(&style_text("Commands: 'help', 'clear', 'stats', 'temp', 'quit'", Style::Hint))?;
    output.println("")?;
    Ok(())
}

fn prompt_user<O: Output>(output: &mut O) -> Result<()> {
    output.print(&format!("{} ", style_text("You:", Style::UserLabel)))?;
    output.flush()?;
    Ok(())
}

fn handle_command<I, O>(
    command: Command,
    session: &mut ChatSession,
    config: &ChatConfig,
    input: &mut I,
    output: &mut O,
) -> Result<bool>
where
    I: Input,
    O: Output,
{
    match command {
        Command::Help => {
            render_help(output)?;
            Ok(false)
        }
        Command::Clear => {
            session.clear();
            output.println(&style_text("Conversation cleared.", Style::Warning))?;
            Ok(false)
        }
        Command::Stats => {
            let stats = session.stats();
            render_stats(&stats, &config.model, output)?;
            Ok(false)
        }
        Command::Quit => {
            output.println(&style_text("Goodbye!", Style::Success))?;
            Ok(true)
        }
        Command::Temp(value) => {
            handle_temperature_command(value, session, input, output)?;
            Ok(false)
        }
    }
}

fn handle_temperature_command<I: Input, O: Output>(
    value: Option<f32>,
    session: &mut ChatSession,
    input: &mut I,
    output: &mut O,
) -> Result<()> {
    let target = if let Some(val) = value {
        val
    } else {
        output.print("New temperature (0.0-2.0): ")?;
        output.flush()?;
        let line = input.read_line()?;
        line.trim().parse::<f32>().map_err(|_| anyhow!("Invalid temperature (must be 0.0-2.0)"))?
    };

    validate_temperature(target)?;
    session.set_temperature(target);
    output.println(&style_text(&format!("Temperature set to {target:.1}"), Style::Success))?;
    Ok(())
}

fn render_help<O: Output>(output: &mut O) -> Result<()> {
    output.println(&style_text("Available commands:", Style::Header))?;
    output.println("  help    - Show this help")?;
    output.println("  clear   - Clear conversation history")?;
    output.println("  stats   - Show conversation stats")?;
    output.println("  temp    - Change temperature")?;
    output.println("  quit    - Exit (also: exit, bye)")?;
    output.println("")?;
    Ok(())
}

fn render_stats<O: Output>(stats: &SessionStats, model: &str, output: &mut O) -> Result<()> {
    output.println(&style_text("Statistics:", Style::Header))?;
    output.println(&format!("  Messages: {}", stats.messages))?;
    output.println(&format!("  Temperature: {:.1}", stats.temperature))?;
    output.println(&format!("  Model: {model}"))?;
    Ok(())
}

async fn stream_response<S: ChatService + Sync, O: Output>(
    model: &str,
    service: &S,
    request: GenerateContentRequest,
    output: &mut O,
) -> Result<Option<String>> {
    let mut stream = service.stream_chat(model, request).await?;
    let mut response_text = String::new();
    let mut usage = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        if !chunk.text.is_empty() {
            output.print(&chunk.text)?;
            response_text.push_str(&chunk.text);
        }

        if chunk.is_final {
            usage = chunk.usage_metadata;
        }
    }

    output.println("")?;
    output.println("")?;
    render_usage(usage, output)?;

    if response_text.is_empty() {
        output.println(&style_text("No response generated", Style::Warning))?;
        return Ok(None);
    }

    Ok(Some(response_text))
}

fn render_usage<O: Output>(usage: Option<UsageMetadata>, output: &mut O) -> Result<()> {
    if let Some(usage) = usage {
        let usage_str = format!(
            "(tokens: {} in, {} out, {} total)",
            usage.prompt_token_count,
            usage.candidates_token_count.unwrap_or(0),
            usage.total_token_count
        );
        output.println(&style_text(&usage_str, Style::Subtle))?;
        output.println("")?;
    }
    Ok(())
}

fn render_error<O: Output>(error: &anyhow::Error, output: &mut O) -> Result<()> {
    output.println(&format!("{} {error}", style_text("Error:", Style::Error)))?;
    output.println("")?;

    let lower = error.to_string().to_lowercase();
    if lower.contains("authentication") {
        output.println(&style_text(
            "Authentication failed. Check your credentials.",
            Style::Warning,
        ))?;
        output.println(&style_text(
            "Make sure these environment variables are set:",
            Style::Warning,
        ))?;
        output.println("  GCP_PRIVATE_KEY")?;
        output.println("  GCP_CLIENT_EMAIL")?;
        output.println("  GCP_CLIENT_ID")?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Style {
    Header,
    UserLabel,
    AssistantLabel,
    Hint,
    Warning,
    Success,
    Error,
    Subtle,
}

fn style_text(text: &str, style: Style) -> String {
    #[cfg(feature = "cli")]
    {
        use colored::Colorize;

        match style {
            Style::Header => text.bold().cyan().to_string(),
            Style::UserLabel => text.bold().blue().to_string(),
            Style::AssistantLabel | Style::Success => text.bold().green().to_string(),
            Style::Hint | Style::Subtle => text.dimmed().to_string(),
            Style::Warning => text.yellow().to_string(),
            Style::Error => text.red().bold().to_string(),
        }
    }

    #[cfg(not(feature = "cli"))]
    {
        let _ = style;
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_core::io::{BufferOutput, QueueInput};
    use crate::chat_core::service::{MockChatService, MockChunk};
    use crate::chat_core::session::ChatSession;
    use crate::ChatStreamChunk;

    fn test_config() -> ChatConfig {
        ChatConfig {
            project: "p".to_string(),
            location: "l".to_string(),
            model: "m".to_string(),
            temperature: 0.9,
            max_tokens: 10,
            system: None,
            debug: false,
        }
    }

    #[tokio::test]
    async fn runs_basic_flow() {
        let service = MockChatService::new(vec![MockChunk::Ok(ChatStreamChunk {
            text: "hello".to_string(),
            is_final: true,
            usage_metadata: None,
        })]);
        let mut input =
            QueueInput::new(vec!["hi".to_string(), "stats".to_string(), "quit".to_string()]);
        let mut output = BufferOutput::default();

        run_chat(test_config(), &service, &mut input, &mut output).await.expect("chat runs");

        let rendered = output.into_string();
        assert!(rendered.contains("Vertex AI Chat"));
        assert!(rendered.contains("Assistant"));
        assert!(rendered.contains("Statistics"));
        assert!(rendered.contains("Goodbye"));
    }

    #[tokio::test]
    async fn handles_invalid_temperature_command() {
        let service = MockChatService::new(vec![]);
        let mut input = QueueInput::new(vec!["temp 3.5".to_string(), "quit".to_string()]);
        let mut output = BufferOutput::default();

        let result = run_chat(test_config(), &service, &mut input, &mut output).await;
        assert!(result.is_ok());

        let rendered = output.into_string();
        assert!(rendered.contains("Temperature must be between"));
    }

    #[tokio::test]
    async fn renders_stream_errors() {
        let service = MockChatService::new(vec![MockChunk::Err("fail".to_string())]);
        let mut input = QueueInput::new(vec!["hello".to_string(), "quit".to_string()]);
        let mut output = BufferOutput::default();

        let result = run_chat(test_config(), &service, &mut input, &mut output).await;
        assert!(result.is_ok());

        let rendered = output.into_string();
        assert!(rendered.contains("Error:"));
    }

    #[tokio::test]
    async fn handle_temperature_parses_interactive_input() {
        let mut session = ChatSession::new(&test_config());
        let mut input = QueueInput::new(vec!["1.2".to_string()]);
        let mut output = BufferOutput::default();

        handle_temperature_command(None, &mut session, &mut input, &mut output)
            .expect("temperature updates");
        assert!((session.temperature() - 1.2).abs() < f32::EPSILON);
    }
}
