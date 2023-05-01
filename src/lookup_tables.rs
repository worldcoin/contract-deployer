use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ethers::prelude::Contract;
use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Eip1559TransactionRequest};
use eyre::{bail, Context as _, ContextCompat};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::common_keys::RpcSigner;
use crate::forge_utils::{
    ContractSpec, ForgeCreate, ForgeInspectAbi, ForgeOutput,
};
use crate::insertion_verifier::InsertionVerifiers;
use crate::types::{BatchSize, GroupId, TreeDepth};
use crate::{Config, DeploymentContext};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct LookupTables {
    pub groups: HashMap<GroupId, GroupLookupTables>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GroupLookupTables {
    pub insert: InsertLookupTable,
    pub update: UpdateLookupTable,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpdateLookupTable {
    pub deployment: ForgeOutput,
    // TODO: Support entries
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InsertLookupTable {
    pub deployment: ForgeOutput,
    #[serde(default)]
    pub entries: HashMap<BatchSize, Address>,
}

#[instrument(skip_all)]
async fn deploy_lookup_table(
    context: &DeploymentContext,
) -> eyre::Result<ForgeOutput> {
    let insert_lookup_table =
        ForgeCreate::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .with_private_key(context.args.private_key.to_string())
            .with_rpc_url(context.args.rpc_url.to_string())
            .with_override_nonce(context.next_nonce())
            .run()
            .await?;

    Ok(insert_lookup_table)
}

#[instrument(skip(context))]
async fn deploy_lookup_tables(
    context: Arc<DeploymentContext>,
    group_id: GroupId,
) -> eyre::Result<(GroupId, GroupLookupTables)> {
    if let Some(lookup_tables) =
        context.report.lookup_tables.groups.get(&group_id)
    {
        info!("Found existing lookup tables for group {group_id}");
        return Ok((group_id, lookup_tables.clone()));
    }

    let insert_lookup_deployment = deploy_lookup_table(context.as_ref());
    let update_lookup_deployment = deploy_lookup_table(context.as_ref());

    let (insert_lookup_deployment, update_lookup_deployment) =
        futures::join!(insert_lookup_deployment, update_lookup_deployment);

    let insert_lookup_deployment = insert_lookup_deployment?;
    let update_lookup_deployment = update_lookup_deployment?;

    let lookup_tables = GroupLookupTables {
        insert: InsertLookupTable {
            deployment: insert_lookup_deployment,
            entries: HashMap::new(),
        },
        update: UpdateLookupTable {
            deployment: update_lookup_deployment,
        },
    };

    Ok((group_id, lookup_tables))
}

#[instrument(skip(context, verifier_abi, verifiers))]
async fn associate_group_batch_size_verifier(
    context: Arc<DeploymentContext>,
    verifier_abi: ethers::abi::Abi,
    lookup_table_address: Address,
    group_id: GroupId,
    tree_depth: TreeDepth,
    batch_size: BatchSize,
    verifiers: Arc<InsertionVerifiers>,
) -> eyre::Result<Address> {
    if let Some(group_verifiers) =
        context.report.lookup_tables.groups.get(&group_id)
    {
        if let Some(verifier_address) =
            group_verifiers.insert.entries.get(&batch_size)
        {
            info!("Early return!");
            return Ok(*verifier_address);
        }
    }

    let verifier = verifiers
        .verifiers
        .get(&(tree_depth, batch_size))
        .with_context(|| format!("Failed to get verifier for batch size {batch_size} and tree_depth {tree_depth}"))?;

    let signer = context.dep_map.get::<RpcSigner>().await.0.clone();

    let insert_lookup =
        Contract::new(lookup_table_address, verifier_abi, signer.clone());

    let update_verifier = insert_lookup.encode(
        "updateVerifier",
        (batch_size.0 as u64, verifier.deployment.deployed_to),
    )?;

    let mut tx = TypedTransaction::Eip1559(
        Eip1559TransactionRequest::new()
            .from(signer.address())
            .to(lookup_table_address)
            .data(update_verifier)
            .nonce(context.next_nonce()),
    );

    info!("Filling tx!");
    signer.fill_transaction(&mut tx, None).await?;

    info!("Sending tx!");
    let tx = signer
        .send_transaction(tx, None)
        .await
        .context("Send transaction")?;

    info!("Waitig for receipt!");
    let tx = tx
        .await
        .context("Awaiting receipt")?
        .context("Failed to execute")?;

    if tx.status != Some(1.into()) {
        bail!("Failed!");
    }

    Ok(verifier.deployment.deployed_to)
}

#[instrument(skip(context, verifier_abi))]
async fn remove_group_batch_size_verifier(
    context: Arc<DeploymentContext>,
    verifier_abi: ethers::abi::Abi,
    lookup_table_address: Address,
    group_id: GroupId,
    batch_size: BatchSize,
) -> eyre::Result<()> {
    let signer = context.dep_map.get::<RpcSigner>().await.0.clone();

    let insert_lookup =
        Contract::new(lookup_table_address, verifier_abi, signer.clone());

    let update_verifier =
        insert_lookup.encode("disableVerifier", batch_size.0 as u64)?;

    let mut tx = TypedTransaction::Eip1559(
        Eip1559TransactionRequest::new()
            .from(signer.address())
            .to(lookup_table_address)
            .data(update_verifier)
            .nonce(context.next_nonce()),
    );

    signer.fill_transaction(&mut tx, None).await?;

    let tx = signer
        .send_transaction(tx, None)
        .await
        .context("Send transaction")?
        .await
        .context("Awaiting receipt")?
        .context("Failed to execute")?;

    if tx.status != Some(1.into()) {
        bail!("Failed!");
    }

    Ok(())
}

#[instrument(name = "lookup_tables", skip_all)]
pub async fn deploy(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    let mut lookup_table_deployments = vec![];

    for group in config.groups.keys() {
        lookup_table_deployments
            .push(tokio::spawn(deploy_lookup_tables(context.clone(), *group)));
    }

    let lookup_table_deployments =
        futures::future::try_join_all(lookup_table_deployments).await?;

    let mut by_group = HashMap::new();
    for result in lookup_table_deployments {
        let (group, lookup_tables) = result?;
        by_group.insert(group, lookup_tables);
    }

    let verifier_abi =
        ForgeInspectAbi::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .run()
            .await?;

    info!("Waiting for insertion verifiers to deploy...");
    let verifiers = context.dep_map.get::<InsertionVerifiers>().await;

    // New or existing verifiers
    for (group_id, group_config) in &config.groups {
        let group = by_group.get(group_id).unwrap();

        let group_id = *group_id;

        let lookup_table_address = group.insert.deployment.deployed_to;

        let config_batch_sizes: HashSet<_> =
            group_config.batch_sizes.iter().copied().collect();
        let report_batch_sizes =
            group.insert.entries.keys().copied().collect::<HashSet<_>>();

        let batch_sizes_to_add_or_update =
            config_batch_sizes.difference(&report_batch_sizes);
        let batch_sizes_to_disable =
            report_batch_sizes.difference(&config_batch_sizes);

        info!("Going to update batch sizes for group {group_id}: {batch_sizes_to_add_or_update:?}");
        info!("Going to disable batch sizes for group {group_id}: {batch_sizes_to_disable:?}");

        for batch_size in batch_sizes_to_add_or_update {
            let tree_depth = group_config.tree_depth;
            let batch_size = *batch_size;

            let address = associate_group_batch_size_verifier(
                context.clone(),
                verifier_abi.clone(),
                lookup_table_address,
                group_id,
                tree_depth,
                batch_size,
                verifiers.clone(),
            )
            .await?;

            by_group
                .get_mut(&group_id)
                .unwrap()
                .insert
                .entries
                .insert(batch_size, address);
        }

        for batch_size in batch_sizes_to_disable {
            let batch_size = *batch_size;

            remove_group_batch_size_verifier(
                context.clone(),
                verifier_abi.clone(),
                lookup_table_address,
                group_id,
                batch_size,
            )
            .await?;

            by_group
                .get_mut(&group_id)
                .unwrap()
                .insert
                .entries
                .remove(&batch_size);
        }
    }

    context.dep_map.set(LookupTables { groups: by_group }).await;

    Ok(())
}
