use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use ethers::prelude::SignerMiddleware;
use ethers::providers::{Middleware, Provider};
use ethers::signers::{Signer, Wallet};
use eyre::ContextCompat;

use self::mtb_utils::ProverMode;
use self::steps::assemble_report::{self, REPORT_PATH};
use self::steps::{
    identity_manager, lookup_tables, semaphore_verifier, verifiers,
    world_id_router,
};
use crate::cli::{Args, DeploymentType};
use crate::common_keys::RpcSigner;
use crate::config::Config;
use crate::report::Report;
use crate::serde_utils;

pub mod deployment_context;
pub mod mtb_utils;
pub mod steps;

pub const KEYS_DIR: &str = "keys";
pub const VERIFIER_CONTRACTS_DIR: &str = "verifier_contracts";

pub use self::deployment_context::DeploymentContext;

pub async fn run_deployment(cmd: Args) -> eyre::Result<()> {
    let config: Config = serde_utils::read_deserialize(&cmd.config).await?;

    let deployment_dir = PathBuf::from(cmd.deployment_name);
    let cache_dir: PathBuf = deployment_dir.join(".cache");

    tokio::fs::create_dir_all(&cache_dir).await?;

    let provider = Provider::try_from(cmd.rpc_url.as_str())?;
    let chain_id = provider.get_chainid().await?;
    let wallet = Wallet::from(cmd.private_key.key.clone())
        .with_chain_id(chain_id.as_u64());

    let wallet_address = wallet.address();

    let signer = SignerMiddleware::new(provider, wallet);

    let nonce = signer.get_transaction_count(wallet_address, None).await?;

    // TODO: should eventually be replaced by some dyn Trait that can be used to sign transactions
    //       we might want to support multiple signers in the future
    let rpc_signer = Arc::new(RpcSigner(Arc::new(signer)));

    let report_path = deployment_dir.join(REPORT_PATH);

    let report: Report;

    if report_path.exists() {
        report = serde_utils::read_deserialize(&report_path).await?;

        let cache_path = report_path.join(".cache");
        serde_utils::write_serialize(cache_path, &report).await?;
    } else {
        report = Report::default_with_config(&config);
    };

    let context = DeploymentContext {
        deployment_dir,
        cache_dir,
        nonce: AtomicU64::new(nonce.as_u64()),
        report,
        private_key: cmd.private_key,
        rpc_url: cmd.rpc_url,
        rpc_signer,
        etherscan_api_key: cmd.etherscan_api_key,
    };

    let context = Arc::new(context);
    let config = Arc::new(config);

    let insertion_verifiers = Some(
        verifiers::deploy(
            context.clone(),
            config.clone(),
            ProverMode::Insertion,
        )
        .await?,
    );

    assemble_report::assemble_report(
        context.clone(),
        config.clone(),
        insertion_verifiers.as_ref(),
        None,
        None,
        None,
        None,
        None,
    )
    .await?;

    if cmd.target == DeploymentType::InsertionVerifiers {
        return Ok(());
    }

    let deletion_verifiers = Some(
        verifiers::deploy(
            context.clone(),
            config.clone(),
            ProverMode::Deletion,
        )
        .await?,
    );

    assemble_report::assemble_report(
        context.clone(),
        config.clone(),
        insertion_verifiers.as_ref(),
        deletion_verifiers.as_ref(),
        None,
        None,
        None,
        None,
    )
    .await?;

    if cmd.target == DeploymentType::DeletionVerifiers
        || cmd.target == DeploymentType::Verifiers
    {
        return Ok(());
    }

    let lookup_tables = Some(
        lookup_tables::deploy(
            context.clone(),
            config.clone(),
            insertion_verifiers
                .as_ref()
                .context("Missing insertion verifiers")?,
            deletion_verifiers
                .as_ref()
                .context("Missing deletion verifiers")?,
        )
        .await?,
    );

    assemble_report::assemble_report(
        context.clone(),
        config.clone(),
        insertion_verifiers.as_ref(),
        deletion_verifiers.as_ref(),
        lookup_tables.as_ref(),
        None,
        None,
        None,
    )
    .await?;

    if cmd.target == DeploymentType::LookupTables {
        return Ok(());
    }

    let semaphore_verifier = Some(
        semaphore_verifier::deploy(context.clone(), config.clone()).await?,
    );

    assemble_report::assemble_report(
        context.clone(),
        config.clone(),
        insertion_verifiers.as_ref(),
        deletion_verifiers.as_ref(),
        lookup_tables.as_ref(),
        semaphore_verifier.as_ref(),
        None,
        None,
    )
    .await?;

    if cmd.target == DeploymentType::SemaphoreVerifier {
        return Ok(());
    }

    let identity_manager: Option<
        identity_manager::WorldIDIdentityManagersDeployment,
    > = Some(
        identity_manager::deploy(
            context.clone(),
            config.clone(),
            semaphore_verifier
                .as_ref()
                .context("Missing semaphore verifier")?,
            lookup_tables.as_ref().context("Missing lookup tables")?,
        )
        .await?,
    );

    assemble_report::assemble_report(
        context.clone(),
        config.clone(),
        insertion_verifiers.as_ref(),
        deletion_verifiers.as_ref(),
        lookup_tables.as_ref(),
        semaphore_verifier.as_ref(),
        identity_manager.as_ref(),
        None,
    )
    .await?;

    if cmd.target == DeploymentType::IdentityManager {
        return Ok(());
    }

    let world_id_router = Some(
        world_id_router::deploy(
            context.clone(),
            config.clone(),
            identity_manager
                .as_ref()
                .context("Missing identity manager")?,
        )
        .await?,
    );

    assemble_report::assemble_report(
        context,
        config,
        insertion_verifiers.as_ref(),
        deletion_verifiers.as_ref(),
        lookup_tables.as_ref(),
        semaphore_verifier.as_ref(),
        identity_manager.as_ref(),
        world_id_router.as_ref(),
    )
    .await?;

    if cmd.target == DeploymentType::WorldIdRouter
        || cmd.target == DeploymentType::Full
    {
        return Ok(());
    }

    Ok(())
}
