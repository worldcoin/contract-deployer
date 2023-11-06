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
    insertion_verifiers: Option<&Verifiers>,
    deletion_verifiers: Option<&Verifiers>,
    lookup_tables: Option<&LookupTables>,
    semaphore_verifier: Option<&SemaphoreVerifierDeployment>,
    identity_managers: Option<&WorldIDIdentityManagersDeployment>,
    world_id_router: Option<&WorldIdRouterDeployment>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: insertion_verifiers.cloned(),
        deletion_verifiers: deletion_verifiers.cloned(),
        lookup_tables: lookup_tables.cloned(),
        semaphore_verifier: semaphore_verifier.cloned(),
        identity_managers: identity_managers.cloned(),
        world_id_router: world_id_router.cloned(),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await?;

    Ok(())
}
