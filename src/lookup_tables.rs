use ethers::prelude::Contract;
use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::Eip1559TransactionRequest;
use eyre::{bail, Context as _, ContextCompat};
use tracing::instrument;

use crate::common_keys::RpcSigner;
use crate::forge_utils::{
    ContractSpec, ForgeCreate, ForgeInspectAbi, ForgeOutput,
};
use crate::insertion_verifier::InsertionVerifier;
use crate::{Config, Context};

pub struct UpdateLookupTable {
    pub deploy_info: ForgeOutput,
}

pub struct InsertLookupTable {
    pub deploy_info: ForgeOutput,
}

#[instrument(skip_all)]
async fn deploy_insert_lookup_table(
    context: &Context,
    config: &Config,
) -> eyre::Result<()> {
    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let insert_lookup_table =
        ForgeCreate::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .with_private_key(private_key_string.clone())
            .with_rpc_url(config.rpc_url.clone())
            .with_override_nonce(context.next_nonce())
            .run()
            .await?;

    let verifier_abi =
        ForgeInspectAbi::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .run()
            .await?;

    let signer = context.dep_map.get::<RpcSigner>().await.0.clone();

    let insert_lookup = Contract::new(
        insert_lookup_table.deployed_to,
        verifier_abi,
        signer.clone(),
    );

    let insertion_verifier = context.dep_map.get::<InsertionVerifier>().await;

    let add_verifier = insert_lookup.encode(
        "addVerifier",
        (
            config.batch_size as u64,
            insertion_verifier.deployment.deployed_to,
        ),
    )?;

    let mut tx = TypedTransaction::Eip1559(
        Eip1559TransactionRequest::new()
            .from(signer.address())
            .to(insert_lookup_table.deployed_to)
            .data(add_verifier)
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

    context
        .dep_map
        .set(InsertLookupTable {
            deploy_info: insert_lookup_table,
        })
        .await;

    if tx.status != Some(1.into()) {
        bail!("Failed!");
    }

    Ok(())
}

#[instrument(skip_all)]
async fn deploy_update_lookup_table(
    context: &Context,
    config: &Config,
) -> eyre::Result<()> {
    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let update_lookup_table =
        ForgeCreate::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .with_private_key(private_key_string.clone())
            .with_rpc_url(config.rpc_url.clone())
            .with_override_nonce(context.next_nonce())
            .run()
            .await?;

    context
        .dep_map
        .set(UpdateLookupTable {
            deploy_info: update_lookup_table,
        })
        .await;

    Ok(())
}

pub async fn deploy(context: &Context, config: &Config) -> eyre::Result<()> {
    let (insert, update) = tokio::join!(
        deploy_insert_lookup_table(context, config),
        deploy_update_lookup_table(context, config)
    );

    insert.context("Insert lookup table")?;
    update.context("Update lookup table")?;

    Ok(())
}
