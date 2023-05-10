use std::sync::Arc;

use tracing::instrument;

use crate::config::Config;
use crate::identity_manager::WorldIDIdentityManagersDeployment;
use crate::insertion_verifier::InsertionVerifiers;
use crate::lookup_tables::LookupTables;
use crate::report::Report;
use crate::semaphore_verifier::SemaphoreVerifierDeployment;
use crate::world_id_router::WorldIdRouterDeployment;
use crate::{serde_utils, DeploymentContext};

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
