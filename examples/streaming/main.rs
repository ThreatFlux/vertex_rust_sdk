#[path = "../common/mod.rs"]
mod common;

mod config;
mod output;
mod runner;

use crate::config::{StreamingArgs, StreamingConfig};
use crate::output::ConsolePrinter;
use crate::runner::{StreamingRunner, VertexStreamer};
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    common::init_logging().ok();

    let args = StreamingArgs::parse(std::env::args())?;
    let mut config = StreamingConfig::from_env()?;
    config.apply_args(&args)?;

    let environment = config.environment.clone();
    let request = config.build_request();

    let client = environment.new_client().await.map_err(|err| {
        eprintln!("Failed to create Vertex AI client: {err}");
        eprintln!("Make sure you have authenticated with:");
        eprintln!("  gcloud auth application-default login");
        err
    })?;

    let runner = StreamingRunner::new(VertexStreamer::new(client));
    let mut printer = ConsolePrinter::stdout();

    printer.intro(&environment, &config.model_id)?;

    match runner.run(&config.model_id, &request, &mut printer).await {
        Ok(summary) => {
            printer.summary(&summary)?;

            if summary.full_response.is_empty() {
                printer.no_response()?;
            } else {
                printer.success()?;
            }
        }
        Err(err) => {
            printer.error(&err)?;
            return Err(err.into());
        }
    }

    printer.into_inner().flush()?;
    Ok(())
}
