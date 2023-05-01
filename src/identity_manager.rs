use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ethers::prelude::encode_function_data;
use ethers::types::{Address, U256};
use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::forge_utils::{
    ContractSpec, ForgeCreate, ForgeInspectAbi, ForgeOutput,
};
use crate::lookup_tables::LookupTables;
use crate::semaphore_verifier::SemaphoreVerifierDeployment;
use crate::types::GroupId;
use crate::{Config, DeploymentContext};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WorldIDIdentityManagersDeployment {
    pub groups: HashMap<GroupId, WorldIdIdentityManagerDeployment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldIdIdentityManagerDeployment {
    pub impl_deployment: ForgeOutput,
    pub proxy_deployment: ForgeOutput,
}

#[instrument(skip(context, config))]
async fn deploy_world_id_identity_manager_for_group(
    context: &DeploymentContext,
    config: &Config,
    group_id: GroupId,
) -> eyre::Result<WorldIdIdentityManagerDeployment> {
    if let Some(deployment) =
        context.report.identity_managers.groups.get(&group_id)
    {
        // TODO: Upgradeability
        info!("Existing world id identity manager deployment found for group {:?}. Skipping.", group_id);
        return Ok(deployment.clone());
    }

    let group_config = config
        .groups
        .get(&group_id)
        .context("Missing group id in config")?;

    let identity_manager_spec = ContractSpec::name("WorldIDIdentityManager");
    let impl_spec = ContractSpec::name("WorldIDIdentityManagerImplV1");

    let impl_deployment = ForgeCreate::new(impl_spec.clone())
        .with_cwd("./world-id-contracts")
        .with_private_key(context.args.private_key.to_string())
        .with_rpc_url(context.args.rpc_url.to_string())
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

    let semaphore_verifier_deployment =
        context.dep_map.get::<SemaphoreVerifierDeployment>().await;
    let lookup_tables = context.dep_map.get::<LookupTables>().await;

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
        .with_private_key(context.args.private_key.to_string())
        .with_rpc_url(context.args.rpc_url.to_string())
        .with_override_nonce(context.next_nonce())
        .with_constructor_arg(format!("{:?}", impl_deployment.deployed_to))
        .with_constructor_arg(call_data)
        .run()
        .await?;

    Ok(WorldIdIdentityManagerDeployment {
        impl_deployment,
        proxy_deployment,
    })
}

pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    let config_groups: HashSet<_> = config.groups.keys().copied().collect();
    let report_groups: HashSet<_> =
        context.report.config.groups.keys().copied().collect();

    let mut groups = HashMap::new();

    let groups_to_remove = report_groups.difference(&config_groups);
    let groups_to_add_or_update = config_groups.clone();

    for group_id in groups_to_add_or_update {
        let group_deployment = deploy_world_id_identity_manager_for_group(
            context.as_ref(),
            config.as_ref(),
            group_id,
        )
        .await?;

        groups.insert(group_id, group_deployment);
    }

    for group_id in groups_to_remove {
        warn!("Removing groups is not implemented yet, group_id = {group_id}");
    }

    context
        .dep_map
        .set(WorldIDIdentityManagersDeployment { groups })
        .await;

    Ok(())
}
