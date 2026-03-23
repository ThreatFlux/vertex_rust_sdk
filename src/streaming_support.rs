use crate::error::{Result, VertexError};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use std::pin::Pin;

/// Trait for parsing raw SSE payloads into strongly typed events.
pub trait SsePayloadParser<T> {
    /// Attempt to parse a raw SSE payload into an event.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload cannot be parsed into the target type.
    fn parse(&self, payload: &str) -> Result<Option<T>>;
}

/// Shared SSE stream state used by streaming APIs.
pub struct SseStreamState<P, T> {
    pub byte_stream: Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
    pub buffer: String,
    pub parser: P,
    pub finished: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<P, T> SseStreamState<P, T> {
    pub fn new(
        byte_stream: Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
        parser: P,
    ) -> Self {
        Self {
            byte_stream,
            buffer: String::new(),
            parser,
            finished: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Attempt to parse the next buffered event.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying parser fails.
    pub fn try_parsed_event(&mut self) -> Result<Option<T>>
    where
        P: SsePayloadParser<T>,
    {
        if let Some(event) = Self::next_event(&mut self.buffer, self.finished) {
            return self.parser.parse(&event);
        }
        Ok(None)
    }

    /// Advance the underlying byte stream by one chunk.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying byte stream produces an error.
    pub async fn advance(&mut self) -> Result<bool> {
        match self.byte_stream.as_mut().next().await {
            Some(Ok(bytes)) => {
                let chunk = String::from_utf8_lossy(&bytes);
                self.buffer.push_str(&chunk);
                Ok(true)
            }
            Some(Err(e)) => Err(VertexError::streaming(format!("Stream error: {e}"))),
            None => {
                self.finished = true;
                Ok(false)
            }
        }
    }

    fn next_event(buffer: &mut String, finished: bool) -> Option<String> {
        if buffer.is_empty() {
            if finished {
                let taken = std::mem::take(buffer);
                return Self::clean_event(&taken);
            }
            return None;
        }

        if let Some(idx) = buffer.find("\r\n\r\n") {
            let event = buffer[..idx].to_string();
            buffer.drain(..idx + 4);
            return Self::clean_event(&event);
        }

        if let Some(idx) = buffer.find("\n\n") {
            let event = buffer[..idx].to_string();
            buffer.drain(..idx + 2);
            return Self::clean_event(&event);
        }

        if finished {
            let taken = std::mem::take(buffer);
            return Self::clean_event(&taken);
        }

        None
    }

    fn clean_event(event: &str) -> Option<String> {
        let trimmed = event.trim_matches(|c| c == '\r' || c == '\n');
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures_util::stream;

    #[derive(Clone, Default)]
    struct PassthroughParser;

    impl SsePayloadParser<String> for PassthroughParser {
        fn parse(&self, payload: &str) -> Result<Option<String>> {
            Ok(Some(payload.to_string()))
        }
    }

    #[tokio::test]
    async fn parses_events_from_stream_chunks() {
        let chunks = vec![
            Ok(Bytes::from_static(b"data: first\n\n")),
            Ok(Bytes::from_static(b"data: second\n\n")),
        ];
        let byte_stream = Box::pin(stream::iter(chunks));
        let mut state = SseStreamState::new(byte_stream, PassthroughParser);

        assert!(state.advance().await.unwrap());
        let event = state.try_parsed_event().unwrap();
        assert_eq!(event, Some("data: first".to_string()));

        assert!(state.advance().await.unwrap());
        let event = state.try_parsed_event().unwrap();
        assert_eq!(event, Some("data: second".to_string()));
    }

    #[tokio::test]
    async fn indicates_end_of_stream() {
        let byte_stream = Box::pin(stream::iter(vec![Ok(Bytes::from_static(b"data: final"))]));
        let mut state = SseStreamState::new(byte_stream, PassthroughParser);

        // Consume the last chunk and drain buffer without terminator.
        assert!(state.advance().await.unwrap());
        state.finished = true;
        let event = state.try_parsed_event().unwrap();
        assert_eq!(event, Some("data: final".to_string()));

        // No more data should be available after draining.
        assert!(!state.advance().await.unwrap());
        assert!(state.try_parsed_event().unwrap().is_none());
    }

    #[test]
    fn ignores_empty_events() {
        let byte_stream = Box::pin(stream::iter(Vec::<reqwest::Result<Bytes>>::new()));
        let mut state = SseStreamState::new(byte_stream, PassthroughParser);

        state.buffer = "\n\n".to_string();
        state.finished = true;
        assert!(state.try_parsed_event().unwrap().is_none());
    }
}
