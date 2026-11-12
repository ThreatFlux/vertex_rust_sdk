//! Bidirectional Gemini Live API sessions over WebSocket.

use crate::{ModelDescriptor, Result, VertexClient, VertexError};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

const LIVE_PATH: &str = "/ws/google.cloud.aiplatform.v1beta1.LlmBidiService/BidiGenerateContent";

/// An authenticated Gemini Live session. Messages use the Vertex Live JSON schema.
pub struct LiveSession {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl LiveSession {
    /// Send a Live API message, such as `realtimeInput` or `toolResponse`.
    ///
    /// # Errors
    ///
    /// Returns an error if the WebSocket write fails.
    pub async fn send(&mut self, message: &Value) -> Result<()> {
        self.socket.send(Message::Text(message.to_string().into())).await.map_err(live_error)
    }

    /// Read the next Live API message. Returns `None` when the server closes the session.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket, server, or JSON decoder reports a failure.
    pub async fn receive(&mut self) -> Result<Option<Value>> {
        while let Some(frame) = self.socket.next().await {
            let message: Value = match frame.map_err(live_error)? {
                Message::Text(text) => serde_json::from_str(text.as_str())?,
                Message::Binary(bytes) => serde_json::from_slice(&bytes)?,
                Message::Close(_) => return Ok(None),
                _ => continue,
            };
            if let Some(error) = message.get("error") {
                return Err(live_error(error));
            }
            return Ok(Some(message));
        }
        Ok(None)
    }

    /// Initiate a graceful WebSocket close.
    ///
    /// # Errors
    ///
    /// Returns an error if the close frame cannot be sent.
    pub async fn close(mut self) -> Result<()> {
        self.socket.close(None).await.map_err(live_error)
    }
}

impl VertexClient {
    /// Open a Live API session and wait for its setup acknowledgement.
    ///
    /// `setup` contains the fields of `BidiGenerateContentSetup`, such as
    /// `generationConfig` and `systemInstruction`. The SDK sets its `model` field.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication, connection, or setup fails.
    pub async fn connect_live(
        &self,
        model: &str,
        mut setup: Map<String, Value>,
    ) -> Result<LiveSession> {
        let descriptor = ModelDescriptor::parse(model)?;
        let context = self.model_request_context(&descriptor);
        let mut url =
            url::Url::parse(&format!("{}{LIVE_PATH}", context.endpoint.trim_end_matches('/')))
                .map_err(live_error)?;
        let websocket_scheme = match url.scheme() {
            "https" => "wss",
            "http" => "ws",
            _ => return Err(VertexError::configuration("Live API requires an HTTP(S) endpoint")),
        };
        url.set_scheme(websocket_scheme)
            .map_err(|()| VertexError::configuration("Invalid Live API endpoint scheme"))?;
        let mut request = url.as_str().into_client_request().map_err(live_error)?;
        let token = self.get_auth_token().await?;
        request.headers_mut().insert(
            http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().map_err(live_error)?,
        );
        setup.insert("model".to_string(), Value::String(context.resource_path));

        timeout(self.request_timeout(), async {
            let connector =
                if websocket_scheme == "wss" { Some(live_tls_connector()?) } else { None };
            let (socket, _) = connect_async_tls_with_config(request, None, false, connector)
                .await
                .map_err(live_error)?;
            let mut session = LiveSession { socket };
            session.send(&json!({"setup": setup})).await?;
            if session
                .receive()
                .await?
                .is_some_and(|message| message.get("setupComplete").is_some())
            {
                Ok(session)
            } else {
                Err(live_error("server did not acknowledge session setup"))
            }
        })
        .await
        .map_err(live_error)?
    }
}

fn live_tls_connector() -> Result<Connector> {
    let mut roots = rustls::RootCertStore::empty();
    let certificates = rustls_native_certs::load_native_certs();
    roots.add_parsable_certificates(certificates.certs);
    if roots.is_empty() {
        return Err(live_error("no trusted root certificates are available"));
    }
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(live_error)?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

fn live_error(error: impl std::fmt::Display) -> VertexError {
    VertexError::streaming(format!("Gemini Live: {error}"))
}
