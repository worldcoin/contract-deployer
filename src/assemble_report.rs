use std::sync::Arc;

use tracing::instrument;

use crate::config::Config;
use crate::insertion_verifier::InsertionVerifiers;
use crate::lookup_tables::LookupTables;
use crate::report::Report;
use crate::semaphore_verifier::SemaphoreVerifierDeployment;
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

    let report = Report {
        config: config.as_ref().clone(),
        verifiers: verifiers.as_ref().clone(),
        lookup_tables: lookup_tables.as_ref().clone(),
        semaphore_verifier: Some(semaphore_verifier.as_ref().clone()),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await?;

    Ok(())
}
