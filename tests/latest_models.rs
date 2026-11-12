use futures_util::{SinkExt, StreamExt};
use mockito::Matcher;
use serde_json::{json, Map, Value};
use threatflux_vertex_rust_sdk::{
    config::Config, AuthProvider, EmbeddingRequest, EmbeddingTaskType, VertexClient,
};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};

struct TestAuth;

#[async_trait::async_trait]
impl AuthProvider for TestAuth {
    async fn get_token(&self) -> anyhow::Result<String> {
        Ok("test-token".to_string())
    }

    async fn refresh_if_needed(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

async fn client(base_url: String) -> VertexClient {
    let config = Config {
        project_id: "test-project".to_string(),
        base_url_override: Some(base_url),
        timeout_secs: 5,
        max_retries: 0,
        ..Config::default()
    };
    VertexClient::with_config_and_auth_provider(config, Box::new(TestAuth)).await.unwrap()
}

#[tokio::test]
#[allow(clippy::significant_drop_tightening, reason = "The mock server must outlive its requests")]
async fn embedding_2_returns_a_vector_for_each_text() {
    let mut server = mockito::Server::new_async().await;
    let mut mocks = Vec::new();
    for (input, vector) in [("first", json!([1.0, 0.0])), ("second", json!([0.0, 1.0]))] {
        let request = json!({
            "content": {"role": "user", "parts": [{"text": format!("task: search result | query: {input}")}]},
            "embedContentConfig": {"outputDimensionality": 2}
        });
        let response =
            json!({"embedding": {"values": vector}, "usageMetadata": {"promptTokenCount": 8}});
        mocks.push(server.mock("POST", "/v1/projects/test-project/locations/global/publishers/google/models/gemini-embedding-2:embedContent")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(request))
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create_async().await);
    }

    let request = EmbeddingRequest::batch(vec!["first".into(), "second".into()])
        .with_task_type(EmbeddingTaskType::RetrievalQuery)
        .with_output_dimensionality(2);
    let response = client(server.url()).await.embed("gemini-embedding-2", &request).await.unwrap();
    assert_eq!(response.embeddings(), [&[1.0, 0.0], &[0.0, 1.0]]);
    assert_eq!(response.predictions[0].embeddings.statistics.as_ref().unwrap().token_count, 8);
    for mock in mocks {
        mock.assert_async().await;
    }
}

#[tokio::test]
#[allow(clippy::significant_drop_tightening, reason = "The mock server must outlive its requests")]
async fn veo_submits_and_polls_the_same_publisher_model() {
    let mut server = mockito::Server::new_async().await;
    let model = "veo-3.1-generate-001";
    let resource =
        format!("projects/test-project/locations/us-central1/publishers/google/models/{model}");
    let operation = format!("{resource}/operations/test-operation");
    let request = json!({"instances": [{"prompt": "A leaf floating down a stream"}]});
    let submit = server
        .mock("POST", format!("/v1/{resource}:predictLongRunning").as_str())
        .match_body(Matcher::Json(request.clone()))
        .with_body(json!({"name": operation}).to_string())
        .create_async()
        .await;
    let poll = server
        .mock("POST", format!("/v1/{resource}:fetchPredictOperation").as_str())
        .match_body(Matcher::Json(json!({"operationName": operation})))
        .with_body(json!({"done": true}).to_string())
        .create_async()
        .await;

    let client = client(server.url()).await;
    let job = client.predict_long_running(model, &request).await.unwrap();
    let result =
        client.fetch_predict_operation(model, job["name"].as_str().unwrap()).await.unwrap();
    assert_eq!(result["done"], true);
    submit.assert_async().await;
    poll.assert_async().await;
}

#[tokio::test]
#[allow(clippy::result_large_err, reason = "Tungstenite defines the handshake callback error type")]
async fn gemini_live_uses_authenticated_setup_and_receives_server_content() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = client(format!("http://{}", listener.local_addr().unwrap())).await;
    let server = tokio::spawn(async move {
        let (connection, _) = listener.accept().await.unwrap();
        let mut socket = accept_hdr_async(connection, |request: &Request, response: Response| {
            assert_eq!(
                request.uri().path(),
                "/ws/google.cloud.aiplatform.v1beta1.LlmBidiService/BidiGenerateContent"
            );
            assert_eq!(request.headers()["authorization"], "Bearer test-token");
            Ok(response)
        })
        .await
        .unwrap();
        let setup = socket.next().await.unwrap().unwrap();
        let setup: Value = serde_json::from_str(&setup.into_text().unwrap()).unwrap();
        assert_eq!(
            setup["setup"]["model"],
            "projects/test-project/locations/us-central1/publishers/google/models/gemini-3.8-live"
        );
        socket.send(Message::Text(json!({"setupComplete": {}}).to_string().into())).await.unwrap();
        let input = socket.next().await.unwrap().unwrap();
        let input: Value = serde_json::from_str(&input.into_text().unwrap()).unwrap();
        assert_eq!(input["realtimeInput"]["text"], "Hello");
        socket
            .send(Message::Binary(
                json!({"serverContent": {"turnComplete": true}}).to_string().into_bytes().into(),
            ))
            .await
            .unwrap();
    });

    let mut session = client.connect_live("gemini-3.8-live", Map::new()).await.unwrap();
    session.send(&json!({"realtimeInput": {"text": "Hello"}})).await.unwrap();
    let response = session.receive().await.unwrap().unwrap();
    assert_eq!(response["serverContent"]["turnComplete"], true);
    server.await.unwrap();
}
