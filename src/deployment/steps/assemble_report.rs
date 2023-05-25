use std::sync::Arc;

use tracing::instrument;

use super::identity_manager::WorldIDIdentityManagersDeployment;
use super::insertion_verifier::InsertionVerifiers;
use super::lookup_tables::LookupTables;
use super::semaphore_verifier::SemaphoreVerifierDeployment;
use super::world_id_router::WorldIdRouterDeployment;
use crate::config::Config;
use crate::deployment::DeploymentContext;
use crate::report::Report;
use crate::serde_utils;

pub const REPORT_PATH: &str = "report.yml";

#[instrument(skip_all)]
pub async fn assemble_report(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    verifiers: &InsertionVerifiers,
    lookup_tables: &LookupTables,
    semaphore_verifier: &SemaphoreVerifierDeployment,
    identity_managers: &WorldIDIdentityManagersDeployment,
    world_id_router_deployment: &WorldIdRouterDeployment,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        verifiers: verifiers.clone(),
        lookup_tables: lookup_tables.clone(),
        semaphore_verifier: Some(semaphore_verifier.clone()),
        identity_managers: identity_managers.clone(),
        world_id_router: Some(world_id_router_deployment.clone()),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await?;

    Ok(())
}
