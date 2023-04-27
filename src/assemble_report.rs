use std::sync::Arc;

use crate::config::Config;
use crate::insertion_verifier::InsertionVerifiers;
use crate::report::Report;
use crate::{serde_utils, DeploymentContext};

pub const REPORT_PATH: &str = "report.yml";

pub async fn assemble_report(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    let verifiers = context.dep_map.get::<InsertionVerifiers>().await;

    let report = Report {
        config: config.as_ref().clone(),
        verifiers: verifiers.as_ref().clone(),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await?;

    Ok(())
}
