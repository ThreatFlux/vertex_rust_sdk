use std::io::{self, Write};

#[cfg(test)]
use std::collections::VecDeque;

pub trait Input {
    fn read_line(&mut self) -> io::Result<String>;
}

pub trait Output {
    fn print(&mut self, text: &str) -> io::Result<()>;
    fn println(&mut self, text: &str) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

#[derive(Default)]
pub struct ConsoleInput;

impl Input for ConsoleInput {
    fn read_line(&mut self) -> io::Result<String> {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        Ok(buffer)
    }
}

pub struct ConsoleOutput {
    stdout: io::Stdout,
}

impl Default for ConsoleOutput {
    fn default() -> Self {
        Self { stdout: io::stdout() }
    }
}

impl Output for ConsoleOutput {
    fn print(&mut self, text: &str) -> io::Result<()> {
        self.stdout.write_all(text.as_bytes())
    }

    fn println(&mut self, text: &str) -> io::Result<()> {
        self.stdout.write_all(text.as_bytes())?;
        self.stdout.write_all(b"\n")
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

#[cfg(test)]
pub struct QueueInput {
    queue: VecDeque<String>,
}

#[cfg(test)]
impl QueueInput {
    pub fn new(inputs: Vec<String>) -> Self {
        Self { queue: inputs.into() }
    }
}

#[cfg(test)]
impl Input for QueueInput {
    fn read_line(&mut self) -> io::Result<String> {
        self.queue
            .pop_front()
            .map(|mut s| {
                if !s.ends_with('\n') {
                    s.push('\n');
                }
                s
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more input"))
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct BufferOutput {
    buffer: Vec<u8>,
}

#[cfg(test)]
impl BufferOutput {
    pub fn into_string(self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }
}

#[cfg(test)]
impl Output for BufferOutput {
    fn print(&mut self, text: &str) -> io::Result<()> {
        self.buffer.extend_from_slice(text.as_bytes());
        Ok(())
    }

    fn println(&mut self, text: &str) -> io::Result<()> {
        self.print(text)?;
        self.print("\n")
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_input_returns_lines() {
        let mut input = QueueInput::new(vec!["first".to_string(), "second".to_string()]);
        assert_eq!(input.read_line().unwrap().trim(), "first");
        assert_eq!(input.read_line().unwrap().trim(), "second");
        assert!(input.read_line().is_err());
    }

    #[test]
    fn buffer_output_collects_text() {
        let mut output = BufferOutput::default();
        output.print("hi ").unwrap();
        output.println("there").unwrap();
        assert_eq!(output.into_string(), "hi there\n");
    }
}
