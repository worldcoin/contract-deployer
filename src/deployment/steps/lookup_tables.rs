use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ethers::types::Address;
use eyre::ContextCompat;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use super::insertion_verifier::InsertionVerifiers;
use crate::common_keys::RpcSigner;
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
    verifiers: &InsertionVerifiers,
) -> eyre::Result<Address> {
    if let Some(group_verifiers) =
        context.report.lookup_tables.groups.get(&group_id)
    {
        if let Some(insert) = group_verifiers.insert.as_ref() {
            if let Some(verifier_address) = insert.entries.get(&batch_size) {
                info!("Early return!");
                return Ok(*verifier_address);
            }
        }
    }

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
    verifiers: &InsertionVerifiers,
) -> eyre::Result<LookupTables> {
    let mut by_group = HashMap::new();

    for group in config.groups.keys() {
        let lookup_tables =
            deploy_lookup_tables(context.clone(), *group).await?;

        by_group.insert(*group, lookup_tables);
    }

    let verifier_abi =
        ForgeInspectAbi::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .run()
            .await?;

    // New or existing verifiers
    for (group_id, group_config) in &config.groups {
        let group = by_group.get(group_id).unwrap();

        let group_id = *group_id;

        let Some(insert) = group.insert.as_ref() else {
            continue;
        };

        let config_batch_sizes: HashSet<_> =
            group_config.batch_sizes.iter().copied().collect();
        let report_batch_sizes =
            insert.entries.keys().copied().collect::<HashSet<_>>();

        let batch_sizes_to_add_or_update =
            config_batch_sizes.difference(&report_batch_sizes);
        let batch_sizes_to_disable =
            report_batch_sizes.difference(&config_batch_sizes);

        info!("Going to update batch sizes for group {group_id}: {batch_sizes_to_add_or_update:?}");
        for batch_size_to_disable in batch_sizes_to_disable {
            warn!("Insertion batch size {batch_size_to_disable} for group {group_id} will not be disabled - remove it manually");
        }

        let insert_deployment_address = insert.deployment.address;

        drop(insert);
        drop(group);

        for batch_size in batch_sizes_to_add_or_update {
            let tree_depth = group_config.tree_depth;
            let batch_size = *batch_size;

            let address = associate_group_batch_size_verifier(
                context.clone(),
                verifier_abi.clone(),
                insert_deployment_address,
                group_id,
                tree_depth,
                batch_size,
                verifiers,
            )
            .await?;

            let group = by_group.get_mut(&group_id).unwrap();

            group
                .insert
                .as_mut()
                .unwrap()
                .entries
                .insert(batch_size, address);
        }
    }

    Ok(LookupTables { groups: by_group })
}
