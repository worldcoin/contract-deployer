use clap::Parser;
use cli::Args;
use config::Config;
use deployment::run_deployment;
use interactive::run_interactive_session;
use tracing_error::ErrorLayer;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub mod common_keys;
pub mod dependency_map;
pub mod ethers_utils;
pub mod forge_utils;
pub mod serde_utils;
pub mod utils;

mod cli;
mod config;
mod report;
mod types;

mod deployment;

mod interactive;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    dotenv::dotenv().ok();

    let indicatif_layer = IndicatifLayer::new();

    let filter = EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif_layer.get_stderr_writer())
                .with_filter(filter),
        )
        .with(indicatif_layer)
        .with(ErrorLayer::default())
        .init();

    let initial_args = Args::parse();

    let cmd = run_interactive_session(initial_args).await?;

    match run_deployment(cmd).await {
        Ok(()) => Ok(()),
        Err(err) => {
            let report = eyre::ErrReport::from(err);
            tracing::error!("{:?}", report);
            std::process::exit(1)
        }
    }
}
