use std::sync::Arc;

use ethers::types::Address;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::config::Config;
use crate::deployment::DeploymentContext;
use crate::forge_utils::{ContractSpec, ExternalDep};
use crate::report::contract_deployment::ContractDeployment;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemaphoreVerifierDeployment {
    pub verifier_deployment: ContractDeployment,
    pub pairing_deployment: ContractDeployment,
}

#[instrument(skip_all)]
async fn deploy_semaphore_pairing_library(
    context: &DeploymentContext,
) -> eyre::Result<ContractDeployment> {
    if let Some(previous_deployment) =
        context.report.semaphore_verifier.as_ref()
    {
        return Ok(previous_deployment.pairing_deployment.clone());
    }

    let contract_spec = ContractSpec::name("Pairing");

    let output = context
        .forge_create(contract_spec)
        .with_cwd("./world-id-contracts")
        .no_verify()
        .run()
        .await?;

    Ok(output.into())
}

#[instrument(skip_all)]
async fn deploy_semaphore_verifier(
    context: &DeploymentContext,
    pairing_address: Address,
) -> eyre::Result<ContractDeployment> {
    if let Some(previous_deployment) =
        context.report.semaphore_verifier.as_ref()
    {
        return Ok(previous_deployment.verifier_deployment.clone());
    }

    let contract_spec: ContractSpec = ContractSpec::name("SemaphoreVerifier");

    let output = context
        .forge_create(contract_spec)
        .with_cwd("./world-id-contracts")
        .with_external_dep(ExternalDep::path_name_address(
            "./lib/semaphore/packages/contracts/contracts/base/Pairing.sol",
            "Pairing",
            pairing_address,
        ))
        .no_verify()
        .run()
        .await?;

    Ok(output.into())
}

pub async fn deploy(
    context: Arc<DeploymentContext>,
    _config: Arc<Config>,
) -> eyre::Result<SemaphoreVerifierDeployment> {
    let pairing_deployment =
        deploy_semaphore_pairing_library(context.as_ref()).await?;

    let pairing_address = pairing_deployment.address;

    let verifier_deployment =
        deploy_semaphore_verifier(context.as_ref(), pairing_address).await?;

    Ok(SemaphoreVerifierDeployment {
        verifier_deployment,
        pairing_deployment,
    })
}
