use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::config::Config;
use crate::deployment::mtb_utils::{
    download_semaphore_mtb_binary, generate_keys, generate_verifier_contract,
    ProverMode, MTB_BIN,
};
use crate::deployment::{DeploymentContext, KEYS_DIR, VERIFIER_CONTRACTS_DIR};
use crate::forge_utils::ContractSpec;
use crate::report::contract_deployment::ContractDeployment;
use crate::types::{BatchSize, TreeDepth};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Verifiers {
    pub verifiers: HashMap<(TreeDepth, BatchSize), VerifierDeployment>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VerifierDeployment {
    pub deployment: ContractDeployment,
}

#[instrument(skip(context, verifier_contract))]
pub async fn deploy_verifier_contract(
    context: &DeploymentContext,
    verifier_contract: impl AsRef<Path>,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
    mode: ProverMode,
) -> eyre::Result<ContractDeployment> {
    let verifier_contract = verifier_contract.as_ref().canonicalize()?;

    if let Some(existing_deployment) = context
        .report
        .insertion_verifiers
        .verifiers
        .get(&(tree_depth, batch_size))
    {
        info!("Found previous verifier deployment for tree depth {tree_depth} and batch size {batch_size} at {:?}", existing_deployment.deployment.address);
        return Ok(existing_deployment.deployment.clone());
    }

    let verifier_contract_parent = verifier_contract
        .parent()
        .context("Missing verifier contract parent directory")?;

    let contract_spec =
        ContractSpec::path_name(verifier_contract.clone(), "Verifier");

    tracing::info!("Deploying Verifier with {contract_spec}");

    let output = context
        .forge_create(contract_spec.clone())
        .with_cwd("./world-id-contracts")
        .with_override_contract_source(verifier_contract_parent)
        .no_verify()
        .run()
        .await?;

    Ok(output.into())
}

#[instrument(name = "verifiers", skip(context, config))]
pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    mode: ProverMode,
) -> eyre::Result<Verifiers> {
    let mtb_bin_path = context.cache_dir.join(MTB_BIN);

    download_semaphore_mtb_binary(context.as_ref(), config.as_ref()).await?;

    let verifier_contracts_dir = context.cache_path(VERIFIER_CONTRACTS_DIR);
    let keys_dir = context.cache_path(KEYS_DIR);

    tokio::fs::create_dir_all(&verifier_contracts_dir).await?;
    tokio::fs::create_dir_all(&keys_dir).await?;

    let mut verifiers = HashMap::new();
    for (tree_depth, batch_size) in
        config.unique_tree_depths_and_batch_sizes(mode)
    {
        let mtb_bin_path = mtb_bin_path.clone();
        let keys_dir = keys_dir.clone();
        let verifier_contracts_dir = verifier_contracts_dir.clone();

        let keys_file = generate_keys(
            &mtb_bin_path,
            &keys_dir,
            tree_depth,
            batch_size,
            mode,
        )
        .await?;

        let context = context.clone();

        let verifier_contract_path = generate_verifier_contract(
            mtb_bin_path,
            keys_file,
            verifier_contracts_dir,
            tree_depth,
            batch_size,
            mode,
        )
        .await?;

        let deployment = deploy_verifier_contract(
            context.as_ref(),
            verifier_contract_path,
            tree_depth,
            batch_size,
            mode,
        )
        .await?;

        let key = (tree_depth, batch_size);
        verifiers.insert(key, VerifierDeployment { deployment });
    }

    Ok(Verifiers { verifiers })
}
