#![allow(clippy::too_many_arguments)]

use clap::Parser;
use cli::Args;
use deployment::run_deployment;
use tracing_subscriber::EnvFilter;

pub mod common_keys;
pub mod ethers_utils;
pub mod forge_utils;
pub mod serde_utils;
pub mod utils;

mod cli;
mod config;
mod report;
mod types;

mod deployment;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    dotenv::dotenv().ok();

    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();

    match run_deployment(args).await {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::error!("{:?}", err);
            std::process::exit(1)
        }
    }
}
