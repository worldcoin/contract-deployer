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
use crate::forge_utils::{ContractSpec, ForgeInspectAbi};
use crate::report::contract_deployment::ContractDeployment;
use crate::types::GroupId;
use crate::Config;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WorldIDIdentityManagersDeployment {
    pub groups: HashMap<GroupId, WorldIdIdentityManagerDeployment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldIdIdentityManagerDeployment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_v1_deployment: Option<ContractDeployment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_v2_deployment: Option<ContractDeployment>,
    pub proxy_deployment: ContractDeployment,
}

#[instrument(skip_all)]
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
        if deployment.impl_v1_deployment.is_some()
            && deployment.impl_v2_deployment.is_none()
        {
            info!("Existing world id identity manager deployment found for group {:?}. Upgrading to v2.", group_id);
            return upgrade_v1_to_v2(
                context,
                config,
                group_id,
                semaphore_verifier_deployment,
                lookup_tables,
                deployment,
            )
            .await;
        } else {
            info!("Existing world id identity manager deployment found for group {:?}. Skipping.", group_id);
            return Ok(deployment.clone());
        }
    }

    let group_config = config
        .groups
        .get(&group_id)
        .context("Missing group id in config")?;

    let identity_manager_spec = ContractSpec::name("WorldIDIdentityManager");
    let impl_spec = ContractSpec::name("WorldIDIdentityManagerImplV1");

    let impl_v1_deployment = context
        .forge_create(impl_spec.clone())
        .with_cwd("./world-id-contracts")
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

    let initial_root_u256 =
        if let Some(initial_root) = group_config.initial_root {
            U256::from(initial_root.0)
        } else {
            U256::from_big_endian(initial_root.as_bytes())
        };

    let insert_lookup_table_address = group_lookup_tables
        .insert
        .as_ref()
        .expect("TODO")
        .deployment
        .address;

    let update_lookup_table_address = group_lookup_tables
        .update
        .as_ref()
        .expect("TODO")
        .deployment
        .address;

    let call_data = encode_function_data(
        initialize_func,
        (
            group_config.tree_depth.0 as u64,
            initial_root_u256,
            insert_lookup_table_address,
            update_lookup_table_address,
            semaphore_verifier_deployment.verifier_deployment.address,
        ),
    )?;

    let proxy_deployment = context
        .forge_create(identity_manager_spec)
        .with_cwd("./world-id-contracts")
        .with_constructor_arg(format!("{:?}", impl_v1_deployment.deployed_to))
        .with_constructor_arg(call_data)
        .run()
        .await?;

    Ok(WorldIdIdentityManagerDeployment {
        impl_v1_deployment: Some(impl_v1_deployment.into()),
        impl_v2_deployment: None,
        proxy_deployment: proxy_deployment.into(),
    })
}

#[instrument(skip_all)]
async fn upgrade_v1_to_v2(
    context: &DeploymentContext,
    config: &Config,
    group_id: GroupId,
    semaphore_verifier_deployment: &SemaphoreVerifierDeployment,
    lookup_tables: &LookupTables,
    v1_deployment: &WorldIdIdentityManagerDeployment,
) -> eyre::Result<WorldIdIdentityManagerDeployment> {
    let identity_manager_spec = ContractSpec::name("WorldIDIdentityManager");
    let impl_v2_spec = ContractSpec::name("WorldIDIdentityManagerImplV2");

    let impl_v2_deployment = context
        .forge_create(impl_v2_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let impl_abi = ForgeInspectAbi::new(impl_v2_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    todo!()
}

#[instrument(skip_all)]
async fn initialize_v2(
    context: &DeploymentContext,
    config: &Config,
    group_id: GroupId,
    semaphore_verifier_deployment: &SemaphoreVerifierDeployment,
    lookup_tables: &LookupTables,
) -> eyre::Result<()> {
    let impl_v2_spec = ContractSpec::name("WorldIDIdentityManagerImplV2");

    let impl_abi = ForgeInspectAbi::new(impl_v2_spec.clone())
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    let initialize_v2_func = impl_abi.function("initializeV2")?;

    let group_lookup_tables =
        lookup_tables.groups.get(&group_id).with_context(|| {
            format!("Missing lookup tables for group {group_id}")
        })?;

    let call_data = encode_function_data(
        initialize_v2_func,
        group_lookup_tables
            .delete
            .as_ref()
            .expect("TODO")
            .deployment
            .address,
    )?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use ethers::types::H160;
    use indoc::indoc;

    use super::*;

    const ONLY_PROXY_DEPLOYMENT: &'static str = indoc! { r#"
        proxy_deployment:
          address: '0x0000000000000000000000000000000000000000'
    "# };

    #[test]
    fn only_proxy() {
        let actual: WorldIdIdentityManagerDeployment =
            serde_yaml::from_str(ONLY_PROXY_DEPLOYMENT).unwrap();

        let expected = WorldIdIdentityManagerDeployment {
            impl_v1_deployment: None,
            impl_v2_deployment: None,
            proxy_deployment: ContractDeployment {
                address: H160::zero(),
            },
        };

        let serialized_actual = serde_yaml::to_string(&actual).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(serialized_actual, ONLY_PROXY_DEPLOYMENT);
    }
}
