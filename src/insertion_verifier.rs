use std::ffi::OsStr;
use std::path::Path;

use ethers::types::Address;
use eyre::ContextCompat;
use tracing::{info, instrument};

use crate::forge_utils::{ContractSpec, ForgeCreate};
use crate::{Config, Context};

const MTB_BIN: &str = "mtb";
const KEYS: &str = "keys";
const VERIFIER_CONTRACT: &str = "verifier.sol";

#[instrument(skip(mtb_bin))]
pub async fn download_semaphore_mtb_binary(mtb_bin: impl AsRef<Path>) -> eyre::Result<()> {
    let mtb_bin = mtb_bin.as_ref();

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

    const MTB_RELEASES_URL: &str = "https://github.com/worldcoin/semaphore-mtb/releases/download";
    const MTB_VERSION: &str = "1.0.2";

    let url = format!("{MTB_RELEASES_URL}/{MTB_VERSION}/mtb-{os}-{arch}");

    let response = reqwest::get(url).await?;

    let status = response.status();

    if !status.is_success() {
        let error = response.text().await?;
        eyre::bail!("Failed to download mtb binary: {status} - {error}");
    }

    let bytes = response.bytes().await?;

    tokio::fs::write(mtb_bin, bytes).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let meta = tokio::fs::metadata(mtb_bin).await?;

        let mut permissions = meta.permissions();
        permissions.set_mode(0o755);

        tokio::fs::set_permissions(mtb_bin, permissions).await?;
    }

    Ok(())
}

#[instrument(skip(mtb_binary, keys_file))]
pub async fn generate_keys(
    mtb_binary: impl AsRef<OsStr>,
    keys_file: impl AsRef<Path>,
    tree_depth: usize,
    batch_size: usize,
) -> eyre::Result<()> {
    let keys_file = keys_file.as_ref();

    if keys_file.exists() {
        return Ok(());
    }

    let output = tokio::process::Command::new(mtb_binary)
        .arg("setup")
        .arg("--tree-depth")
        .arg(tree_depth.to_string())
        .arg("--batch-size")
        .arg(batch_size.to_string())
        .arg("--output")
        .arg(keys_file)
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("Failed to generate verifier contract: {error}");
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn generate_verifier_contract(
    mtb_binary: impl AsRef<OsStr>,
    keys_file: impl AsRef<Path>,
) -> eyre::Result<()> {
    let keys_file = keys_file.as_ref();

    if keys_file.exists() {
        return Ok(());
    }

    let output = tokio::process::Command::new(mtb_binary)
        .arg("export-solidity")
        .arg("--keys-file")
        .arg(keys_file)
        .arg("--output")
        .arg("./verifier.sol")
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("Failed to generate verifier contract: {error}");
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn deploy_verifier_contract(
    config: &Config,
    verifier_contract: impl AsRef<Path>,
) -> eyre::Result<Address> {
    let verifier_contract = verifier_contract.as_ref();

    let verifier_contract = verifier_contract.canonicalize()?;
    let verifier_contract_parent = verifier_contract
        .parent()
        .context("Missing verifier contract parent directory")?;

    let contract_spec = ContractSpec::path_name(verifier_contract.clone(), "Verifier");

    let private_key_string = hex::encode(config.private_key.to_bytes().as_slice());

    let forge = ForgeCreate::new(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_override_contract_source(verifier_contract_parent)
        .with_private_key(private_key_string)
        .with_rpc_url(config.rpc_url.clone())
        .run()
        .await?;

    Ok(forge.deployed_to)
}

#[instrument(name = "Insertion Verifier", skip_all)]
pub async fn deploy(context: &Context, config: &Config) -> eyre::Result<()> {
    let mtb_bin_path = context.cache_dir.join(MTB_BIN);
    let keys_file = context.cache_dir.join(KEYS);
    let verifier_contract = context.cache_dir.join(VERIFIER_CONTRACT);

    download_semaphore_mtb_binary(&mtb_bin_path).await?;

    generate_keys(
        &mtb_bin_path,
        &keys_file,
        config.tree_depth,
        config.batch_size,
    )
    .await?;
    generate_verifier_contract(&mtb_bin_path, &keys_file).await?;

    let verifier_address = deploy_verifier_contract(config, &verifier_contract).await?;
    info!("verifier_address = {verifier_address:?}");

    Ok(())
}
