use std::path::PathBuf;

use clap::Parser;
use ethers::prelude::k256::SecretKey;
use serde::{Deserialize, Serialize};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub mod cache;
pub mod forge_utils;
pub mod serde_utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    #[serde(with = "crate::serde_utils::secret_key")]
    pub private_key: SecretKey,
    pub tree_depth: usize,
    pub batch_size: usize,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub cache_dir: PathBuf,
}

#[derive(Debug, Clone, Parser)]
#[clap(rename_all = "kebab-case")]
struct Cmd {
    #[clap(short, long, env)]
    pub config: PathBuf,

    /// The name of the deployment
    ///
    /// Should be something meaningfull like 'prod-2023-04-18'
    #[clap(short, long, env)]
    pub deployment_name: String,
}

mod insertion_verifier;

#[tokio::main]
async fn main() -> eyre::Result<()> {
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
        .init();

    let cmd = Cmd::parse();

    let config = serde_utils::read_deserialize(&cmd.config).await?;

    let cache_dir = PathBuf::from(cmd.deployment_name);
    tokio::fs::create_dir_all(&cache_dir).await?;

    let context = Context { cache_dir };

    insertion_verifier::deploy(&context, &config).await?;

    Ok(())
}
