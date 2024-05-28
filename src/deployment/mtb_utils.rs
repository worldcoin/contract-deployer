use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use strum::{Display, EnumString};
use tracing::instrument;

use crate::config::Config;
use crate::deployment::DeploymentContext;
use crate::types::{BatchSize, TreeDepth};

pub const MTB_BIN: &str = "mtb";

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum ProverMode {
    Insertion,
    Deletion,
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
        os_info::Type::Ubuntu | os_info::Type::Linux => "linux",
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

    let arch = if arch == "x64" || arch == "x86_64" { "amd64" } else { arch };

    const MTB_RELEASES_URL: &str =
        "https://github.com/worldcoin/semaphore-mtb/releases/download";
    const MTB_VERSION: &str = "1.2.1";

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

#[instrument(skip(mtb_binary, keys_dir))]
pub async fn generate_keys(
    mtb_binary: impl AsRef<OsStr>,
    keys_dir: impl AsRef<Path>,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
    mode: ProverMode,
) -> eyre::Result<PathBuf> {
    let filename = match mode {
        ProverMode::Deletion => {
            deletion_keys_file_format(tree_depth, batch_size)
        }
        ProverMode::Insertion => {
            insertion_keys_file_format(tree_depth, batch_size)
        }
    };

    let mode_str = mode.to_string();

    let keys_file = keys_dir.as_ref().join(filename);

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
        .arg("--mode")
        .arg(&mode_str)
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("Failed to generate verifier contract: {error}");
    }

    Ok(keys_file)
}

#[instrument(skip(mtb_binary, keys_file, verifier_contracts_dir))]
pub async fn generate_verifier_contract(
    mtb_binary: impl AsRef<OsStr>,
    keys_file: impl AsRef<Path>,
    verifier_contracts_dir: impl AsRef<Path>,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
    mode: ProverMode,
) -> eyre::Result<PathBuf> {
    let keys_file = keys_file.as_ref();

    let filename = match mode {
        ProverMode::Deletion => {
            deletion_verifier_contract_filename(tree_depth, batch_size)
        }
        ProverMode::Insertion => {
            insertion_verifier_contract_filename(tree_depth, batch_size)
        }
    };

    let verifier_contract = verifier_contracts_dir.as_ref().join(filename);

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

fn insertion_keys_file_format(
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> PathBuf {
    PathBuf::from(format!("keys_insertion_{tree_depth}_{batch_size}"))
}

fn deletion_keys_file_format(
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> PathBuf {
    PathBuf::from(format!("keys_deletion_{tree_depth}_{batch_size}"))
}

fn insertion_verifier_contract_filename(
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> PathBuf {
    PathBuf::from(format!("insertion_{tree_depth}_{batch_size}.sol"))
}

fn deletion_verifier_contract_filename(
    tree_depth: TreeDepth,
    batch_size: BatchSize,
) -> PathBuf {
    PathBuf::from(format!("deletion_{tree_depth}_{batch_size}.sol"))
}
