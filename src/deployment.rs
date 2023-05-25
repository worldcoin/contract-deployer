use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use ethers::prelude::SignerMiddleware;
use ethers::providers::{Middleware, Provider};
use ethers::signers::{Signer, Wallet};

use self::steps::assemble_report::REPORT_PATH;
use self::steps::*;
use crate::common_keys::RpcSigner;
use crate::config::Config;
use crate::dependency_map::DependencyMap;
use crate::report::Report;
use crate::serde_utils;

pub mod cmd;
pub mod deployment_context;
pub mod steps;

pub use self::cmd::Cmd;
pub use self::deployment_context::DeploymentContext;

pub async fn run_deployment(cmd: Cmd) -> eyre::Result<()> {
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
        etherscan_api_key: cmd.etherscan_api_key,
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
