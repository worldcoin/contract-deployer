use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use common_keys::InitialRoot;
use ethers::prelude::k256::SecretKey;
use ethers::types::H256;
use semaphore::poseidon_tree::LazyPoseidonTree;
use semaphore::Field;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::instrument;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};
use typed_map::TypedMap;

pub mod common_keys;
pub mod forge_utils;
pub mod serde_utils;
pub mod typed_map;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    #[serde(with = "crate::serde_utils::secret_key")]
    pub private_key: SecretKey,
    pub tree_depth: usize,
    pub batch_size: usize,
    pub initial_leaf_value: H256,
}

#[derive(Debug)]
pub struct Context {
    pub cache_dir: PathBuf,
    pub typed_map: Arc<RwLock<TypedMap>>,
}

#[derive(Debug, Clone, Parser)]
#[clap(rename_all = "kebab-case")]
struct Cmd {
    #[clap(short, long, env)]
    pub config: PathBuf,

    /// The name of the deployment
    ///
    /// Should be something meaningful like 'prod-2023-04-18'
    #[clap(short, long, env)]
    pub deployment_name: String,
}

mod insertion_verifier;
mod semaphore_verifier;

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

    start().await
}

#[instrument]
async fn start() -> eyre::Result<()> {
    let cmd = Cmd::parse();

    let config: Config = serde_utils::read_deserialize(&cmd.config).await?;

    let cache_dir = PathBuf::from(cmd.deployment_name);
    tokio::fs::create_dir_all(&cache_dir).await?;

    let initial_leaf_value = Field::from_be_bytes(config.initial_leaf_value.0);

    let initial_root_hash = LazyPoseidonTree::new(config.tree_depth, initial_leaf_value).root();

    let initial_root_hash = H256(initial_root_hash.to_be_bytes());

    let mut typed_map = TypedMap::new();

    typed_map.insert(InitialRoot(initial_root_hash));

    let context = Context {
        cache_dir,
        typed_map: Arc::new(RwLock::new(typed_map)),
    };

    let (insertion, semaphore) = tokio::join!(
        insertion_verifier::deploy(&context, &config),
        semaphore_verifier::deploy(&context, &config),
    );

    semaphore?;
    insertion?;

    Ok(())
}
