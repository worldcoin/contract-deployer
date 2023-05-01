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
) -> eyre::Result<()> {
    let verifiers = context.dep_map.get::<InsertionVerifiers>().await;
    let lookup_tables = context.dep_map.get::<LookupTables>().await;
    let semaphore_verifier =
        context.dep_map.get::<SemaphoreVerifierDeployment>().await;
    let identity_managers = context
        .dep_map
        .get::<WorldIDIdentityManagersDeployment>()
        .await;
    let world_id_router_deployment =
        context.dep_map.get::<WorldIdRouterDeployment>().await;

    let report = Report {
        config: config.as_ref().clone(),
        verifiers: verifiers.as_ref().clone(),
        lookup_tables: lookup_tables.as_ref().clone(),
        semaphore_verifier: Some(semaphore_verifier.as_ref().clone()),
        identity_managers: identity_managers.as_ref().clone(),
        world_id_router: Some(world_id_router_deployment.as_ref().clone()),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await?;

    Ok(())
}
