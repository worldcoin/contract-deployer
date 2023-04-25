use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use clap::Parser;
use common_keys::{InitialRoot, RpcSigner};
use dependency_map::DependencyMap;
use ethers::prelude::k256::SecretKey;
use ethers::prelude::SignerMiddleware;
use ethers::providers::{Middleware, Provider};
use ethers::signers::{Signer, Wallet};
use ethers::types::H256;
use semaphore::poseidon_tree::LazyPoseidonTree;
use semaphore::Field;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, instrument};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub mod common_keys;
pub mod dependency_map;
pub mod forge_utils;
pub mod serde_utils;

mod identity_manager;
mod insertion_verifier;
mod lookup_tables;
mod semaphore_verifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    #[serde(with = "crate::serde_utils::secret_key")]
    pub private_key: SecretKey,
    pub tree_depth: usize,
    pub batch_size: usize,
    #[serde(default)]
    pub initial_leaf_value: H256,
}

#[derive(Debug)]
pub struct Context {
    pub cache_dir: PathBuf,
    pub dep_map: DependencyMap,
    pub nonce: AtomicU64,
}

impl Context {
    pub fn next_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
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

    let initial_root_hash =
        LazyPoseidonTree::new(config.tree_depth, initial_leaf_value).root();

    let initial_root_hash = H256(initial_root_hash.to_be_bytes());

    let dep_map = DependencyMap::new();

    dep_map.set(InitialRoot(initial_root_hash)).await;

    let provider = Provider::try_from(config.rpc_url.as_str())?;
    let chain_id = provider.get_chainid().await?;
    let wallet = Wallet::from(config.private_key.clone())
        .with_chain_id(chain_id.as_u64());

    let wallet_address = wallet.address();
    info!("wallet_address = {wallet_address}");
    let signer = SignerMiddleware::new(provider, wallet);

    let nonce = signer.get_transaction_count(wallet_address, None).await?;

    dep_map.set(RpcSigner(Arc::new(signer))).await;

    let context = Context {
        cache_dir,
        dep_map,
        nonce: AtomicU64::new(nonce.as_u64()),
    };

    // TODO: Futures unordered?
    let (insertion, semaphore, lookup, identity) = tokio::join!(
        insertion_verifier::deploy(&context, &config),
        semaphore_verifier::deploy(&context, &config),
        lookup_tables::deploy(&context, &config),
        identity_manager::deploy(&context, &config),
    );

    semaphore?;
    insertion?;
    lookup?;
    identity?;

    Ok(())
}
