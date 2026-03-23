use anyhow::Result;

#[path = "vertex_test/mod.rs"]
mod vertex_test;

#[tokio::main]
async fn main() -> Result<()> {
    vertex_test::run().await
}
