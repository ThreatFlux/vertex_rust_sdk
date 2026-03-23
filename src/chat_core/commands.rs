#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Help,
    Clear,
    Stats,
    Quit,
    Temp(Option<f32>),
}

pub fn parse_command(input: &str) -> Option<Command> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();
    match lower.as_str() {
        "help" => Some(Command::Help),
        "clear" => Some(Command::Clear),
        "stats" => Some(Command::Stats),
        "quit" | "exit" | "bye" => Some(Command::Quit),
        _ if lower.starts_with("temp") => Some(parse_temp_command(trimmed)),
        _ => None,
    }
}

fn parse_temp_command(input: &str) -> Command {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() > 1 {
        if let Ok(value) = parts[1].parse::<f32>() {
            return Command::Temp(Some(value));
        }
    }
    Command::Temp(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_commands() {
        assert_eq!(parse_command("help"), Some(Command::Help));
        assert_eq!(parse_command("clear"), Some(Command::Clear));
        assert_eq!(parse_command("stats"), Some(Command::Stats));
        assert_eq!(parse_command("quit"), Some(Command::Quit));
        assert_eq!(parse_command("exit"), Some(Command::Quit));
        assert_eq!(parse_command("bye"), Some(Command::Quit));
    }

    #[test]
    fn parses_temp_without_value() {
        assert_eq!(parse_command("temp"), Some(Command::Temp(None)));
        assert_eq!(parse_command("temp   "), Some(Command::Temp(None)));
    }

    #[test]
    fn parses_temp_with_value() {
        assert_eq!(parse_command("temp 1.2"), Some(Command::Temp(Some(1.2))));
    }

    #[test]
    fn ignores_unknown() {
        assert_eq!(parse_command("hello"), None);
        assert_eq!(parse_command(""), None);
    }
}
