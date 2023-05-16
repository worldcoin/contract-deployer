use std::collections::HashMap;
use std::sync::Arc;

use ethers::prelude::encode_function_data;
use ethers::types::{Address, U256};
use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::lookup_tables::LookupTables;
use super::semaphore_verifier::SemaphoreVerifierDeployment;
use crate::deployment::DeploymentContext;
use crate::forge_utils::{
    ContractSpec, ForgeCreate, ForgeInspectAbi, ForgeOutput,
};
use crate::types::GroupId;
use crate::Config;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WorldIDIdentityManagersDeployment {
    pub groups: HashMap<GroupId, WorldIdIdentityManagerDeployment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldIdIdentityManagerDeployment {
    pub impl_v1_deployment: ForgeOutput,
    pub proxy_deployment: ForgeOutput,
}

#[instrument(skip(context, config))]
async fn deploy_world_id_identity_manager_v1_for_group(
    context: &DeploymentContext,
    config: &Config,
    group_id: GroupId,
    semaphore_verifier_deployment: &SemaphoreVerifierDeployment,
    lookup_tables: &LookupTables,
) -> eyre::Result<WorldIdIdentityManagerDeployment> {
    if let Some(deployment) =
        context.report.identity_managers.groups.get(&group_id)
    {
        info!("Existing world id identity manager deployment found for group {:?}. Skipping.", group_id);
        return Ok(deployment.clone());
    }

    let group_config = config
        .groups
        .get(&group_id)
        .context("Missing group id in config")?;

    let identity_manager_spec = ContractSpec::name("WorldIDIdentityManager");
    let impl_spec = ContractSpec::name("WorldIDIdentityManagerImplV1");

    let impl_v1_deployment = ForgeCreate::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .with_private_key(context.private_key.clone())
        .with_rpc_url(context.rpc_url.to_string())
        .with_override_nonce(context.next_nonce())
        .run()
        .await?;

    let impl_abi = ForgeInspectAbi::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let initial_root = crate::utils::initial_root_hash(
        group_config.tree_depth,
        config.misc.initial_leaf_value,
    );

    let group_lookup_tables =
        lookup_tables.groups.get(&group_id).with_context(|| {
            format!("Missing lookup tables for group {group_id}")
        })?;

    let initialize_func = impl_abi.function("initialize")?;

    let initial_root_u256 = U256::from_big_endian(initial_root.as_bytes());

    let call_data = encode_function_data(
        initialize_func,
        (
            group_config.tree_depth.0 as u64,
            initial_root_u256,
            group_lookup_tables.insert.deployment.deployed_to,
            group_lookup_tables.update.deployment.deployed_to,
            semaphore_verifier_deployment
                .verifier_deployment
                .deployed_to,
            false,
            Address::default(), // TODO: processedStateBridgeAddress
        ),
    )?;

    let proxy_deployment = ForgeCreate::new(identity_manager_spec)
        .with_cwd("./world-id-contracts")
        .with_private_key(context.private_key.clone())
        .with_rpc_url(context.rpc_url.to_string())
        .with_override_nonce(context.next_nonce())
        .with_constructor_arg(format!("{:?}", impl_v1_deployment.deployed_to))
        .with_constructor_arg(call_data)
        .run()
        .await?;

    Ok(WorldIdIdentityManagerDeployment {
        impl_v1_deployment,
        proxy_deployment,
    })
}

pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    semaphore_verifier_deployment: &SemaphoreVerifierDeployment,
    lookup_tables: &LookupTables,
) -> eyre::Result<WorldIDIdentityManagersDeployment> {
    let mut groups = HashMap::new();

    for group_id in config.groups.keys().copied() {
        let group_deployment = deploy_world_id_identity_manager_v1_for_group(
            context.as_ref(),
            config.as_ref(),
            group_id,
            semaphore_verifier_deployment,
            lookup_tables,
        )
        .await?;

        groups.insert(group_id, group_deployment);
    }

    Ok(WorldIDIdentityManagersDeployment { groups })
}
