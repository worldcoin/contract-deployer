use std::collections::HashMap;
use std::sync::Arc;

use ethers::prelude::encode_function_data;
use ethers::types::Address;
use eyre::{Context as _, ContextCompat};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::identity_manager::WorldIDIdentityManagersDeployment;
use crate::deployment::DeploymentContext;
use crate::ethers_utils::TransactionBuilder;
use crate::forge_utils::{ContractSpec, ForgeInspectAbi};
use crate::report::contract_deployment::ContractDeployment;
use crate::types::GroupId;
use crate::Config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldIdRouterDeployment {
    pub impl_v1_deployment: ContractDeployment,
    pub proxy_deployment: ContractDeployment,
    pub entries: HashMap<GroupId, Address>,
}

#[instrument(skip_all)]
async fn deploy_world_id_router_v1(
    context: &DeploymentContext,
    first_group_address: Address,
) -> eyre::Result<WorldIdRouterDeployment> {
    if let Some(previous_deployment) = context.report.world_id_router.as_ref() {
        return Ok(previous_deployment.clone());
    }

    let contract_spec = ContractSpec::name("WorldIDRouter");
    let impl_spec = ContractSpec::name("WorldIDRouterImplV1");

    let impl_v1_deployment = context
        .forge_create(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let initialize_func = impl_abi.function("initialize")?;

    let call_data = encode_function_data(initialize_func, first_group_address)?;

    let proxy_deployment = context
        .forge_create(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_constructor_arg(format!("{:?}", impl_v1_deployment.deployed_to))
        .with_constructor_arg(call_data)
        .run()
        .await?;

    Ok(WorldIdRouterDeployment {
        impl_v1_deployment: impl_v1_deployment.into(),
        proxy_deployment: proxy_deployment.into(),
        entries: maplit::hashmap! {
            GroupId(0) => first_group_address
        },
    })
}

#[instrument(skip(context))]
async fn update_group_route(
    context: &DeploymentContext,
    world_id_router_address: Address,
    group_id: GroupId,
    new_target_address: Address,
) -> eyre::Result<()> {
    let impl_spec = ContractSpec::name("WorldIDRouterImplV1");

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let signer = &context.rpc_signer;

    let tx = TransactionBuilder::default()
        .signer(signer.clone())
        .abi(impl_abi.clone())
        .function_name("updateGroup")
        .args((group_id.0 as u64, new_target_address))
        .to(world_id_router_address)
        .context(context)
        .build()?;

    tx.send().await?;

    Ok(())
}

#[instrument(skip(context))]
async fn add_group_route(
    context: &DeploymentContext,
    world_id_router_address: Address,
    group_id: GroupId,
    new_target_address: Address,
) -> eyre::Result<()> {
    let impl_spec = ContractSpec::name("WorldIDRouterImplV1");

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let signer = &context.rpc_signer;

    let tx = TransactionBuilder::default()
        .signer(signer.clone())
        .abi(impl_abi.clone())
        .function_name("addGroup")
        .args(new_target_address)
        .to(world_id_router_address)
        .context(context)
        .build()?;

    tx.send().await?;

    Ok(())
}

#[instrument(skip(context))]
async fn remove_group_route(
    context: &DeploymentContext,
    world_id_router_address: Address,
    group_id: GroupId,
) -> eyre::Result<()> {
    let impl_spec = ContractSpec::name("WorldIDRouterImplV1");

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let signer = &context.rpc_signer;

    let tx = TransactionBuilder::default()
        .signer(signer.clone())
        .abi(impl_abi.clone())
        .function_name("disableGroup")
        .args(group_id.0 as u64)
        .to(world_id_router_address)
        .context(context)
        .build()?;

    tx.send().await?;

    Ok(())
}

#[instrument(name = "world_id_router", skip_all)]
pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    identity_managers: &WorldIDIdentityManagersDeployment,
) -> eyre::Result<WorldIdRouterDeployment> {
    let first_group = identity_managers
        .groups
        .get(&GroupId(0))
        .context("Missing group 0")?;

    let mut world_id_router_deployment = deploy_world_id_router_v1(
        context.as_ref(),
        first_group.proxy_deployment.address,
    )
    .await
    .context("deploying world id router implementation")?;

    let mut group_ids: Vec<_> = config.groups.keys().copied().collect();
    group_ids.sort();

    for group_id in group_ids {
        let group_identity_manager_address = identity_managers
            .groups
            .get(&group_id)
            .context("Missing group")?
            .proxy_deployment
            .address;

        if let Some(current_group_address) =
            world_id_router_deployment.entries.get_mut(&group_id)
        {
            if *current_group_address != group_identity_manager_address {
                update_group_route(
                    context.as_ref(),
                    world_id_router_deployment.proxy_deployment.address,
                    group_id,
                    group_identity_manager_address,
                )
                .await?;

                *current_group_address = group_identity_manager_address;
            }
        } else {
            add_group_route(
                context.as_ref(),
                world_id_router_deployment.proxy_deployment.address,
                group_id,
                group_identity_manager_address,
            )
            .await?;

            world_id_router_deployment
                .entries
                .insert(group_id, group_identity_manager_address);
        }

        let deployment_group_ids: Vec<_> =
            world_id_router_deployment.entries.keys().copied().collect();
        for deployment_group_id in deployment_group_ids {
            if !config.groups.contains_key(&deployment_group_id) {
                remove_group_route(
                    context.as_ref(),
                    world_id_router_deployment.proxy_deployment.address,
                    deployment_group_id,
                )
                .await?;

                world_id_router_deployment
                    .entries
                    .remove(&deployment_group_id);
            }
        }
    }

    Ok(world_id_router_deployment)
}
