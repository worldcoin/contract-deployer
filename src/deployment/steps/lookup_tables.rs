use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ethers::types::Address;
use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use super::verifiers::Verifiers;
use crate::common_keys::RpcSigner;
use crate::config::GroupConfig;
use crate::deployment::DeploymentContext;
use crate::ethers_utils::TransactionBuilder;
use crate::forge_utils::{ContractSpec, ForgeInspectAbi};
use crate::report::contract_deployment::ContractDeployment;
use crate::types::{BatchSize, GroupId, TreeDepth};
use crate::Config;

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct LookupTables {
    pub groups: HashMap<GroupId, GroupLookupTables>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct GroupLookupTables {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert: Option<LookupTable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<LookupTable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<LookupTable>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LookupTable {
    pub deployment: ContractDeployment,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub entries: HashMap<BatchSize, Address>,
}

#[instrument(skip_all)]
async fn deploy_lookup_table(
    context: &DeploymentContext,
) -> eyre::Result<ContractDeployment> {
    let insert_lookup_table = context
        .forge_create(ContractSpec::name("VerifierLookupTable"))
        .with_cwd("./world-id-contracts")
        .run()
        .await?;

    Ok(insert_lookup_table.into())
}

#[instrument(skip(context))]
async fn deploy_lookup_tables(
    context: Arc<DeploymentContext>,
    group_id: GroupId,
) -> eyre::Result<GroupLookupTables> {
    let mut lookup_tables = if let Some(lookup_tables) =
        context.report.lookup_tables.groups.get(&group_id)
    {
        info!("Found existing lookup tables for group {group_id}");
        lookup_tables.clone()
    } else {
        info!("No existing lookup tables found for group {group_id}");
        GroupLookupTables::default()
    };

    if lookup_tables.insert.is_none() {
        lookup_tables.insert = Some(LookupTable {
            deployment: deploy_lookup_table(context.as_ref()).await?,
            entries: HashMap::new(),
        });
    }

    if lookup_tables.update.is_none() {
        lookup_tables.update = Some(LookupTable {
            deployment: deploy_lookup_table(context.as_ref()).await?,
            entries: HashMap::new(),
        });
    }

    if lookup_tables.delete.is_none() {
        lookup_tables.delete = Some(LookupTable {
            deployment: deploy_lookup_table(context.as_ref()).await?,
            entries: HashMap::new(),
        });
    }

    Ok(lookup_tables)
}

#[instrument(skip(context, verifier_abi, verifiers))]
async fn associate_group_batch_size_verifier(
    context: Arc<DeploymentContext>,
    verifier_abi: ethers::abi::Abi,
    lookup_table_address: Address,
    group_id: GroupId,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
    verifiers: &Verifiers,
) -> eyre::Result<Address> {
    let verifier = verifiers
        .verifiers
        .get(&(tree_depth, batch_size))
        .with_context(|| format!("Failed to get verifier for batch size {batch_size} and tree_depth {tree_depth}"))?;

    let signer = context.dep_map.get::<RpcSigner>().await;

    TransactionBuilder::default()
        .signer(signer)
        .abi(verifier_abi.clone())
        .function_name("updateVerifier")
        .args((batch_size.0 as u64, verifier.deployment.address))
        .to(lookup_table_address)
        .context(context.as_ref())
        .build()?
        .send()
        .await?;

    Ok(verifier.deployment.address)
}

#[instrument(name = "lookup_tables", skip_all)]
pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    insertion_verifiers: &Verifiers,
    deletion_verifiers: &Verifiers,
) -> eyre::Result<LookupTables> {
    let mut by_group = HashMap::new();

    for group in config.groups.keys() {
        let lookup_tables =
            deploy_lookup_tables(context.clone(), *group).await?;

        by_group.insert(*group, lookup_tables);
    }

    let lookup_abi =
        ForgeInspectAbi::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .run()
            .await?;

    // New or existing verifiers
    for (group_id, group_config) in &config.groups {
        let group = by_group.get(group_id).unwrap();

        let group_id = *group_id;

        let mut insert_updates = HashMap::new();
        let mut delete_updates = HashMap::new();

        if let Some(insert) = group.insert.as_ref() {
            let config_batch_sizes: HashSet<_> =
                group_config.insertion_batch_sizes.iter().copied().collect();

            insert_updates = update_lookup_table(
                context.clone(),
                insertion_verifiers,
                group_id,
                group_config,
                insert,
                &config_batch_sizes,
                &lookup_abi,
            )
            .await?;
        }

        if let Some(delete) = group.delete.as_ref() {
            let config_batch_sizes: HashSet<_> =
                group_config.deletion_batch_sizes.iter().copied().collect();

            delete_updates = update_lookup_table(
                context.clone(),
                deletion_verifiers,
                group_id,
                group_config,
                delete,
                &config_batch_sizes,
                &lookup_abi,
            )
            .await?;
        }

        for ((group_id, batch_size), address) in insert_updates {
            by_group
                .get_mut(&group_id)
                .unwrap()
                .insert
                .as_mut()
                .unwrap()
                .entries
                .insert(batch_size, address);
        }

        for ((group_id, batch_size), address) in delete_updates {
            by_group
                .get_mut(&group_id)
                .unwrap()
                .delete
                .as_mut()
                .unwrap()
                .entries
                .insert(batch_size, address);
        }
    }

    Ok(LookupTables { groups: by_group })
}

async fn update_lookup_table(
    context: Arc<DeploymentContext>,
    verifiers: &Verifiers,
    group_id: GroupId,
    group_config: &GroupConfig,
    table: &LookupTable,
    config_batch_sizes: &HashSet<BatchSize>,
    lookup_abi: &ethers::abi::Abi,
) -> eyre::Result<HashMap<(GroupId, BatchSize), Address>> {
    let report_batch_sizes =
        table.entries.keys().copied().collect::<HashSet<_>>();

    let batch_sizes_to_add_or_update =
        config_batch_sizes.difference(&report_batch_sizes);
    let batch_sizes_to_disable =
        report_batch_sizes.difference(config_batch_sizes);

    info!("Going to update batch sizes for group {group_id}: {batch_sizes_to_add_or_update:?}");
    for batch_size_to_disable in batch_sizes_to_disable {
        warn!("Insertion batch size {batch_size_to_disable} for group {group_id} will not be disabled - remove it manually");
    }

    let table_deployment_address = table.deployment.address;

    let mut updates = HashMap::new();

    for batch_size in batch_sizes_to_add_or_update {
        let tree_depth = group_config.tree_depth;
        let batch_size = *batch_size;

        let address = associate_group_batch_size_verifier(
            context.clone(),
            lookup_abi.clone(),
            table_deployment_address,
            group_id,
            tree_depth,
            batch_size,
            verifiers,
        )
        .await?;

        updates.insert((group_id, batch_size), address);
    }

    Ok(updates)
}
