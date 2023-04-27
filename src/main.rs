use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use args::DeploymentArgs;
use clap::Parser;
use common_keys::RpcSigner;
use dependency_map::DependencyMap;
use ethers::prelude::SignerMiddleware;
use ethers::providers::{Middleware, Provider};
use ethers::signers::{Signer, Wallet};
use ethers::types::H256;
use report::Report;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};
use types::{BatchSize, GroupId, TreeDepth};

pub mod common_keys;
pub mod dependency_map;
pub mod forge_utils;
pub mod serde_utils;
pub mod utils;

mod args;
mod identity_manager;
mod insertion_verifier;
mod lookup_tables;
mod report;
mod semaphore_verifier;
mod types;
mod world_id_router;

pub const REPORT_PATH: &str = "report.yml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub groups: HashMap<GroupId, GroupConfig>,
    pub misc: MiscConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscConfig {
    #[serde(default)]
    pub initial_leaf_value: H256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub tree_depth: TreeDepth,
    pub batch_sizes: Vec<BatchSize>,
}

impl Config {
    pub fn unique_tree_depths_and_batch_sizes(
        &self,
    ) -> HashSet<(TreeDepth, BatchSize)> {
        let mut result = HashSet::new();

        for group in self.groups.values() {
            for batch_size in &group.batch_sizes {
                result.insert((group.tree_depth, *batch_size));
            }
        }

        result
    }
}

#[derive(Debug)]
pub struct DeploymentContext {
    pub deployment_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub dep_map: DependencyMap,
    pub nonce: AtomicU64,
    pub report: Report,
    pub args: DeploymentArgs,
}

impl DeploymentContext {
    pub fn next_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn cache_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.cache_dir.join(path)
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

    #[clap(flatten)]
    pub args: DeploymentArgs,
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

    let deployment_dir = PathBuf::from(cmd.deployment_name);
    let cache_dir = deployment_dir.join(".cache");

    tokio::fs::create_dir_all(&cache_dir).await?;

    let dep_map = DependencyMap::new();

    let provider = Provider::try_from(cmd.args.rpc_url.as_str())?;
    let chain_id = provider.get_chainid().await?;
    let wallet = Wallet::from(cmd.args.private_key.key.clone())
        .with_chain_id(chain_id.as_u64());

    let wallet_address = wallet.address();

    let signer = SignerMiddleware::new(provider, wallet);

    let nonce = signer.get_transaction_count(wallet_address, None).await?;

    // TODO: I think the RPC Signer should stay in the dep_map but it should eventually
    //       be replaced by some dyn Trait that can be used to sign transactions
    //       we might want to support multiple signers in the future
    dep_map.set(RpcSigner(Arc::new(signer))).await;

    let report_path = deployment_dir.join(REPORT_PATH);
    let report = if report_path.exists() {
        serde_utils::read_deserialize::<Report>(&report_path).await?
    } else {
        Report::default_with_config(&config)
    };

    let context = DeploymentContext {
        deployment_dir,
        cache_dir,
        dep_map,
        nonce: AtomicU64::new(nonce.as_u64()),
        report,
        args: cmd.args,
    };

    let context = Arc::new(context);

    // TODO: Futures unordered?
    let (insertion, semaphore, lookup, identity, world_id_router) = tokio::join!(
        insertion_verifier::deploy(context.clone(), &config),
        semaphore_verifier::deploy(context.as_ref(), &config),
        lookup_tables::deploy(context.as_ref(), &config),
        identity_manager::deploy(context.as_ref(), &config),
        world_id_router::deploy(context.as_ref(), &config),
    );

    semaphore?;
    insertion?;
    lookup?;
    identity?;
    world_id_router?;

    Ok(())
}
