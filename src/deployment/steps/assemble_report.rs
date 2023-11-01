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
pub async fn assemble_report_full(
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

#[instrument(skip_all)]
pub async fn assemble_report_identity_manager(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    insertion_verifiers: Option<&Verifiers>,
    deletion_verifiers: Option<&Verifiers>,
    lookup_tables: Option<&LookupTables>,
    semaphore_verifier: Option<&SemaphoreVerifierDeployment>,
    identity_manager: Option<&WorldIDIdentityManagersDeployment>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: insertion_verifiers.cloned(),
        deletion_verifiers: deletion_verifiers.cloned(),
        lookup_tables: lookup_tables.cloned(),
        semaphore_verifier: semaphore_verifier.cloned(),
        identity_managers: identity_manager.cloned(),
        world_id_router: None,
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}

#[instrument(skip_all)]
pub async fn assemble_report_world_id_router(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    identity_manager: Option<&WorldIDIdentityManagersDeployment>,
    world_id_router: Option<&WorldIdRouterDeployment>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: None,
        deletion_verifiers: None,
        lookup_tables: None,
        semaphore_verifier: None,
        identity_managers: identity_manager.cloned(),
        world_id_router: world_id_router.cloned(),
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}
#[instrument(skip_all)]
pub async fn assemble_report_semaphore_verifier(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    semaphore_verifier: Option<&SemaphoreVerifierDeployment>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: None,
        deletion_verifiers: None,
        lookup_tables: None,
        semaphore_verifier: semaphore_verifier.cloned(),
        identity_managers: None,
        world_id_router: None,
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}

#[instrument(skip_all)]
pub async fn assemble_report_lookup_tables(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    insertion_verifiers: Option<&Verifiers>,
    deletion_verifiers: Option<&Verifiers>,
    lookup_tables: Option<&LookupTables>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: insertion_verifiers.cloned(),
        deletion_verifiers: deletion_verifiers.cloned(),
        lookup_tables: lookup_tables.cloned(),
        semaphore_verifier: None,
        identity_managers: None,
        world_id_router: None,
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}

#[instrument(skip_all)]
pub async fn assemble_report_insertion_verifiers(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    insertion_verifiers: Option<&Verifiers>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: insertion_verifiers.cloned(),
        deletion_verifiers: None,
        lookup_tables: None,
        semaphore_verifier: None,
        identity_managers: None,
        world_id_router: None,
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}

#[instrument(skip_all)]
pub async fn assemble_report_deletion_verifiers(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    deletion_verifiers: Option<&Verifiers>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: None,
        deletion_verifiers: deletion_verifiers.cloned(),
        lookup_tables: None,
        semaphore_verifier: None,
        identity_managers: None,
        world_id_router: None,
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}

#[instrument(skip_all)]
pub async fn assemble_report_verifiers(
    context: Arc<DeploymentContext>,
    config: Arc<Config>,
    deletion_verifiers: Option<&Verifiers>,
    insertion_verifiers: Option<&Verifiers>,
) -> eyre::Result<()> {
    let report = Report {
        config: config.as_ref().clone(),
        insertion_verifiers: insertion_verifiers.cloned(),
        deletion_verifiers: deletion_verifiers.cloned(),
        lookup_tables: None,
        semaphore_verifier: None,
        identity_managers: None,
        world_id_router: None,
    };

    let path = context.deployment_dir.join(REPORT_PATH);
    serde_utils::write_serialize(path, report).await.unwrap();

    Ok(())
}
