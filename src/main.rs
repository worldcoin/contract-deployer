use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use args::PrivateKey;
use clap::{CommandFactory, Parser};
use common_keys::RpcSigner;
use config::Config;
use dependency_map::DependencyMap;
use deployment::steps::assemble_report::REPORT_PATH;
use ethers::prelude::SignerMiddleware;
use ethers::providers::{Middleware, Provider};
use ethers::signers::{Signer, Wallet};
use interactive::{run_interactive_session, InteractiveCmd};
use report::Report;
use reqwest::Url;
use tracing_error::ErrorLayer;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::deployment::steps::*;

pub mod common_keys;
pub mod dependency_map;
pub mod ethers_utils;
pub mod forge_utils;
pub mod serde_utils;
pub mod utils;

mod args;
mod config;
mod report;
mod types;

mod deployment;

mod interactive;

#[derive(Debug)]
pub struct DeploymentContext {
    pub deployment_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub dep_map: DependencyMap,
    pub nonce: AtomicU64,
    pub report: Report,
    pub private_key: PrivateKey,
    pub rpc_url: Url,
}

impl DeploymentContext {
    pub fn next_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn cache_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.cache_dir.join(path)
    }
}

pub struct Cmd {
    pub config: PathBuf,
    pub deployment_name: String,
    pub private_key: PrivateKey,
    pub rpc_url: Url,
}

impl Cmd {
    pub fn new(
        config: PathBuf,
        deployment_name: String,
        private_key: PrivateKey,
        rpc_url: Url,
    ) -> Self {
        Self {
            config,
            deployment_name,
            private_key,
            rpc_url,
        }
    }
}

async fn start() -> eyre::Result<()> {
    let initial_args = InteractiveCmd::parse();
    let cmd = run_interactive_session(initial_args).await?;

    let config: Config = serde_utils::read_deserialize(&cmd.config).await?;

    let deployment_dir = PathBuf::from(cmd.deployment_name);
    let cache_dir = deployment_dir.join(".cache");

    tokio::fs::create_dir_all(&cache_dir).await?;

    let dep_map = DependencyMap::new();

    let provider = Provider::try_from(cmd.rpc_url.as_str())?;
    let chain_id = provider.get_chainid().await?;
    let wallet = Wallet::from(cmd.private_key.key.clone())
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
        private_key: cmd.private_key,
        rpc_url: cmd.rpc_url,
    };

    let context = Arc::new(context);
    let config = Arc::new(config);

    let insertion_verifiers =
        insertion_verifier::deploy(context.clone(), config.clone()).await?;

    let lookup_tables = lookup_tables::deploy(
        context.clone(),
        config.clone(),
        &insertion_verifiers,
    )
    .await?;
    let semaphore_verifier =
        semaphore_verifier::deploy(context.clone(), config.clone()).await?;
    let identity_manager = identity_manager::deploy(
        context.clone(),
        config.clone(),
        &semaphore_verifier,
        &lookup_tables,
    )
    .await?;

    let world_id_router = world_id_router::deploy(
        context.clone(),
        config.clone(),
        &identity_manager,
    )
    .await?;

    assemble_report::assemble_report(
        context,
        config,
        &insertion_verifiers,
        &lookup_tables,
        &semaphore_verifier,
        &identity_manager,
        &world_id_router,
    )
    .await?;

    Ok(())
}

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

    match start().await {
        Ok(()) => Ok(()),
        Err(err) => {
            let report = eyre::ErrReport::from(err);
            tracing::error!("{:?}", report);
            std::process::exit(1)
        }
    }
}
