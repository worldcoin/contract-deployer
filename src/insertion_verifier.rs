use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{info, instrument};

use crate::config::Config;
use crate::forge_utils::{ContractSpec, ForgeCreate, ForgeOutput};
use crate::types::{BatchSize, TreeDepth};
use crate::DeploymentContext;

const MTB_BIN: &str = "mtb";
const KEYS_DIR: &str = "keys";
const VERIFIER_CONTRACTS_DIR: &str = "verifier_contracts";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InsertionVerifier {
    pub deployment: ForgeOutput,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct InsertionVerifiers {
    pub verifiers: HashMap<(TreeDepth, BatchSize), InsertionVerifier>,
}

#[instrument(skip_all)]
pub async fn download_semaphore_mtb_binary(
    context: &DeploymentContext,
    _config: &Config,
) -> eyre::Result<()> {
    let mtb_bin = context.cache_path(MTB_BIN);

    if mtb_bin.exists() {
        return Ok(());
    }

    let info = os_info::get();

    let os = match info.os_type() {
        os_info::Type::Windows => "windows",
        os_info::Type::Macos => "darwin",
        os_info::Type::Linux => "linux",
        unsupported => {
            eyre::bail!("Unsupported os type: {unsupported}")
        }
    };

    let arch = info
        .architecture()
        .ok_or_else(|| eyre::eyre!("Missing architecture"))?;

    if arch == "x86" {
        eyre::bail!("32 bit architectures are not supported, got: {arch}")
    }

    let arch = if arch == "x64" { "amd64" } else { arch };

    const MTB_RELEASES_URL: &str =
        "https://github.com/worldcoin/semaphore-mtb/releases/download";
    const MTB_VERSION: &str = "1.0.2";

    let url = format!("{MTB_RELEASES_URL}/{MTB_VERSION}/mtb-{os}-{arch}");

    let response = reqwest::get(url).await?;

    let status = response.status();

    if !status.is_success() {
        let error = response.text().await?;
        eyre::bail!("Failed to download mtb binary: {status} - {error}");
    }

    let bytes = response.bytes().await?;

    tokio::fs::write(&mtb_bin, bytes).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let meta = tokio::fs::metadata(&mtb_bin).await?;

        let mut permissions = meta.permissions();
        permissions.set_mode(0o755);

        tokio::fs::set_permissions(&mtb_bin, permissions).await?;
    }

    Ok(())
}

fn keys_file_format(tree_depth: TreeDepth, batch_size: BatchSize) -> PathBuf {
    PathBuf::from(format!("keys_{tree_depth}_{batch_size}"))
}

#[instrument(skip_all)]
pub async fn generate_keys(
    mtb_binary: impl AsRef<OsStr>,
    keys_dir: impl AsRef<Path>,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> eyre::Result<PathBuf> {
    let keys_file = keys_dir
        .as_ref()
        .join(keys_file_format(tree_depth, batch_size));

    if keys_file.exists() {
        return Ok(keys_file);
    }

    let output = tokio::process::Command::new(mtb_binary)
        .arg("setup")
        .arg("--tree-depth")
        .arg(tree_depth.to_string())
        .arg("--batch-size")
        .arg(batch_size.to_string())
        .arg("--output")
        .arg(&keys_file)
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("Failed to generate verifier contract: {error}");
    }

    Ok(keys_file)
}

#[instrument(skip_all)]
pub async fn generate_verifier_contract(
    mtb_binary: impl AsRef<OsStr>,
    keys_file: impl AsRef<Path>,
    verifier_contracts_dir: impl AsRef<Path>,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> eyre::Result<PathBuf> {
    let keys_file = keys_file.as_ref();

    let verifier_contract = verifier_contracts_dir
        .as_ref()
        .join(verifier_contract_filename(tree_depth, batch_size));

    if verifier_contract.exists() {
        return Ok(verifier_contract);
    }

    let output = tokio::process::Command::new(mtb_binary)
        .arg("export-solidity")
        .arg("--keys-file")
        .arg(keys_file)
        .arg("--output")
        .arg(&verifier_contract)
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("Failed to generate verifier contract: {error}");
    }

    Ok(verifier_contract)
}

fn verifier_contract_filename(
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> PathBuf {
    PathBuf::from(format!("verifier_{batch_size}_{tree_depth}.sol"))
}

#[instrument(skip_all)]
pub async fn deploy_verifier_contract(
    context: &DeploymentContext,
    verifier_contract: impl AsRef<Path>,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> eyre::Result<ForgeOutput> {
    let verifier_contract = verifier_contract.as_ref().canonicalize()?;

    if let Some(existing_deployment) = context
        .report
        .verifiers
        .verifiers
        .get(&(tree_depth, batch_size))
    {
        info!("Found previous verifier deployment for tree depth {tree_depth} and batch size {batch_size} at {:?}", existing_deployment.deployment.deployed_to);
        return Ok(existing_deployment.deployment.clone());
    }

    let verifier_contract_parent = verifier_contract
        .parent()
        .context("Missing verifier contract parent directory")?;

    let contract_spec =
        ContractSpec::path_name(verifier_contract.clone(), "Verifier");

    let output = ForgeCreate::new(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_override_contract_source(verifier_contract_parent)
        .with_override_nonce(context.next_nonce())
        .with_private_key(context.args.private_key.to_string())
        .with_rpc_url(context.args.rpc_url.to_string())
        .run()
        .await?;

    Ok(output)
}

pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    let mtb_bin_path = context.cache_dir.join(MTB_BIN);

    download_semaphore_mtb_binary(context.as_ref(), config.as_ref()).await?;

    let verifier_contracts_dir = context.cache_path(VERIFIER_CONTRACTS_DIR);
    let keys_dir = context.cache_path(KEYS_DIR);

    tokio::fs::create_dir_all(&verifier_contracts_dir).await?;
    tokio::fs::create_dir_all(&keys_dir).await?;

    let mut deployment_tasks: Vec<JoinHandle<eyre::Result<_>>> = vec![];
    for (tree_depth, batch_size) in config.unique_tree_depths_and_batch_sizes()
    {
        let mtb_bin_path = mtb_bin_path.clone();
        let keys_dir = keys_dir.clone();
        let verifier_contracts_dir = verifier_contracts_dir.clone();

        // We don't parallelize on generating keys as it's a process that will likely consume 100% of the CPU
        // so we'd see little benefit
        let keys_file =
            generate_keys(&mtb_bin_path, &keys_dir, tree_depth, batch_size)
                .await?;

        let context = context.clone();

        // but we can parallelize verifier contract generation and deployment
        deployment_tasks.push(tokio::spawn(async move {
            let verifier_contract_path = generate_verifier_contract(
                mtb_bin_path,
                keys_file,
                verifier_contracts_dir,
                tree_depth,
                batch_size,
            )
            .await?;

            let deployment = deploy_verifier_contract(
                context.as_ref(),
                verifier_contract_path,
                tree_depth,
                batch_size,
            )
            .await?;

            let key = (tree_depth, batch_size);

            Ok((key, deployment))
        }));
    }

    let deployments = futures::future::try_join_all(deployment_tasks).await?;

    let mut verifiers = HashMap::new();
    for deployment in deployments {
        let (key, deployment) = deployment?;
        verifiers.insert(key, InsertionVerifier { deployment });
    }

    context.dep_map.set(InsertionVerifiers { verifiers }).await;

    Ok(())
}
