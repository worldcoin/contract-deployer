use ethers::types::Address;
use tracing::{info, instrument};

use crate::forge_utils::{ContractSpec, ExternalDep, ForgeCreate, ForgeOutput};
use crate::{Config, Context};

#[instrument(skip_all)]
pub async fn deploy_semaphore_pairing_library(
    context: &Context,
    config: &Config,
) -> eyre::Result<ForgeOutput> {
    let contract_spec = ContractSpec::name("Pairing");
    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let output = ForgeCreate::new(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_private_key(private_key_string)
        .with_override_nonce(context.next_nonce())
        .with_rpc_url(config.rpc_url.clone())
        .run()
        .await?;

    Ok(output)
}

#[instrument(skip_all)]
pub async fn deploy_semaphore_verifier(
    context: &Context,
    config: &Config,
    pairing_address: Address,
) -> eyre::Result<()> {
    let contract_spec = ContractSpec::name("SemaphoreVerifier");

    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let output = ForgeCreate::new(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_private_key(private_key_string)
        .with_rpc_url(config.rpc_url.clone())
        .with_override_nonce(context.next_nonce())
        .with_external_dep(ExternalDep::path_name_address(
            "./lib/semaphore/packages/contracts/contracts/base/Pairing.sol",
            "Pairing",
            pairing_address,
        ))
        .run()
        .await?;

    info!("output = {output:#?}");

    Ok(())
}

#[instrument(name = "Semaphore Verifier", skip_all)]
pub async fn deploy(context: &Context, config: &Config) -> eyre::Result<()> {
    let output = deploy_semaphore_pairing_library(context, config).await?;

    let pairing_address = output.deployed_to;

    deploy_semaphore_verifier(context, config, pairing_address).await?;

    Ok(())
}
