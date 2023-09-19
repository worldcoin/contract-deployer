use std::sync::Arc;

use tracing::instrument;

use super::identity_manager::WorldIDIdentityManagersDeployment;
use super::lookup_tables::LookupTables;
use super::semaphore_verifier::SemaphoreVerifierDeployment;
use super::verifiers::Verifiers;
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
    insertion_verifiers: &Verifiers,
    deletion_verifiers: &Verifiers,
    lookup_tables: &LookupTables,
    semaphore_verifier: Option<&SemaphoreVerifierDeployment>,
    identity_managers: &WorldIDIdentityManagersDeployment,
    world_id_router_deployment: Option<&WorldIdRouterDeployment>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: insertion_verifiers.clone(),
        deletion_verifiers: deletion_verifiers.clone(),
        lookup_tables: lookup_tables.clone(),
        semaphore_verifier: semaphore_verifier.cloned(),
        identity_managers: identity_managers.clone(),
        world_id_router: world_id_router_deployment.cloned(),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await?;

    Ok(())
}
