use ethers::prelude::encode_function_data;
use ethers::types::Address;
use eyre::Context as _;
use tracing::instrument;

use crate::forge_utils::{
    ContractSpec, ForgeCreate, ForgeInspectAbi, ForgeOutput,
};
use crate::identity_manager::WorldIDIdentityManagerDeployment;
use crate::{Config, DeploymentContext};

#[instrument(skip_all)]
async fn deploy_world_id_router_implementation(
    context: &DeploymentContext,
    config: &Config,
) -> eyre::Result<ForgeOutput> {
    let contract_spec = ContractSpec::name("WorldIDRouterImplV1");

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
async fn deploy_world_id_router_proxy(
    context: &DeploymentContext,
    config: &Config,
    impl_address: Address,
) -> eyre::Result<ForgeOutput> {
    let contract_spec = ContractSpec::name("WorldIDRouter");
    let impl_spec = ContractSpec::name("WorldIDRouterImplV1");

    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let initialize_func = impl_abi.function("initialize")?;

    let world_id_deployment = context
        .dep_map
        .get::<WorldIDIdentityManagerDeployment>()
        .await;

    let call_data = encode_function_data(
        initialize_func,
        world_id_deployment.deploy_info.deployed_to,
    )?;

    let world_id_router = ForgeCreate::new(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_private_key(private_key_string)
        .with_rpc_url(config.rpc_url.clone())
        .with_override_nonce(context.next_nonce())
        .with_constructor_arg(format!("{impl_address:?}"))
        .with_constructor_arg(call_data)
        .run()
        .await?;

    Ok(world_id_router)
}

pub async fn deploy(
    context: &DeploymentContext,
    config: &Config,
) -> eyre::Result<()> {
    let world_id_router =
        deploy_world_id_router_implementation(context, config)
            .await
            .context("deploying world id router implementation")?;

    let _world_id_router = deploy_world_id_router_proxy(
        context,
        config,
        world_id_router.deployed_to,
    )
    .await?;

    Ok(())
}
