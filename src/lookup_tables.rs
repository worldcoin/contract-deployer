use ethers::prelude::Contract;
use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::Eip1559TransactionRequest;
use eyre::{bail, Context as _, ContextCompat};
use tracing::{info, instrument};

use crate::common_keys::RpcSigner;
use crate::forge_utils::{ContractSpec, ForgeCreate, ForgeInspectAbi};
use crate::insertion_verifier::InsertionVerifier;
use crate::{Config, Context};

#[instrument(name = "Lookup Tables", skip_all)]
pub async fn deploy(context: &Context, config: &Config) -> eyre::Result<()> {
    let private_key_string =
        hex::encode(config.private_key.to_bytes().as_slice());

    let verifier_abi =
        ForgeInspectAbi::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .run()
            .await?;

    let insert_lookup_table =
        ForgeCreate::new(ContractSpec::name("VerifierLookupTable"))
            .with_cwd("./world-id-contracts")
            .with_private_key(private_key_string.clone())
            .with_rpc_url(config.rpc_url.clone())
            .with_override_nonce(context.next_nonce())
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

    if tx.status != Some(1.into()) {
        bail!("Failed!");
    }

    Ok(())
}
