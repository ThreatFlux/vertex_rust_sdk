use threatflux_vertex_rust_sdk::{config::Config, GenerateContentRequest, VertexClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let model = config.model.clone();
    let client = VertexClient::new(config).await?;

    let request = GenerateContentRequest::new("Explain why observability matters.");
    let response = client.generate_content(&model, &request).await?;

    if let Some(text) = response.text() {
        println!("{text}");
    }

    Ok(())
}
