use ethers::prelude::Contract;
use ethers::types::{Address, U256};
use tracing::{info, instrument};

use crate::common_keys::{InitialRoot, RpcSigner};
use crate::forge_utils::{
    ContractSpec, ExternalDep, ForgeCreate, ForgeInspectAbi, ForgeOutput,
};
use crate::{Config, Context};

#[instrument(skip_all)]
async fn deploy_identity_manager_impl(
    context: &Context,
    config: &Config,
) -> eyre::Result<ForgeOutput> {
    let contract_spec = ContractSpec::name("WorldIDIdentityManagerImplV1");

    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let output = ForgeCreate::new(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_private_key(private_key_string)
        .with_rpc_url(config.rpc_url.clone())
        .with_override_nonce(context.next_nonce())
        .run()
        .await?;

    Ok(output)
}

#[instrument(skip_all)]
async fn deploy_world_id_identity_manager(
    context: &Context,
    config: &Config,
    impl_address: Address,
) -> eyre::Result<()> {
    let identity_manager_spec = ContractSpec::name("WorldIDIdentityManager");
    let impl_spec = ContractSpec::name("WorldIDIdentityManagerImplV1");

    let identity_manager_abi =
        ForgeInspectAbi::new(identity_manager_spec.clone())
            .with_cwd("./world-id-contracts")
            .run()
            .await?;

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let typed_map = context.typed_map.read().await;

    let initial_root = typed_map.get::<InitialRoot>();
    let signer = typed_map.get::<RpcSigner>().0.clone();

    let impl_contract = Contract::new(impl_address, impl_abi, signer.clone());

    let initial_root =
        U256::from_big_endian(&initial_root.clone().0.to_fixed_bytes());

    let call_data = impl_contract.encode(
        "initialize",
        (
            config.tree_depth as u64,
            initial_root,
            Address::default(),
            Address::default(),
            Address::default(),
            false,
            Address::default(),
        ),
    )?;

    // let identity_manager_abi

    // Initialize method args:
    // config.treeDepth,
    // config.initialRoot,
    // config[insertLUTTargetField],
    // config[updateLUTTargetField],
    // config.semaphoreVerifierContractAddress,
    // config.enableStateBridge,
    // processedStateBridgeAddress,

    // Constructor args:
    // config.identityManagerImplementationContractAddress,
    // callData = `Initialize method args`

    let output = ForgeCreate::new(identity_manager_spec)
        .with_cwd("./world-id-contracts")
        .with_private_key(private_key_string)
        .with_rpc_url(config.rpc_url.clone())
        .with_override_nonce(context.next_nonce())
        .with_constructor_arg(format!("{impl_address:?}"))
        .with_constructor_arg(call_data)
        .run()
        .await?;

    info!("Deployed IdentityManager: {:?}", output);

    Ok(())
}

pub async fn deploy(context: &Context, config: &Config) -> eyre::Result<()> {
    let identity_manager_impl =
        deploy_identity_manager_impl(context, config).await?;

    deploy_world_id_identity_manager(
        context,
        config,
        identity_manager_impl.deployed_to,
    )
    .await?;

    Ok(())
}
