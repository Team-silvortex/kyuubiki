use crate::{
    RemoteDeploymentJournal, RemoteDeploymentPlan, interrupt_remote_deployment_step,
    verify_remote_deployment_journal,
};
use serde::{Deserialize, Serialize};

pub const REMOTE_DEPLOYMENT_RESUME_PLAN_SCHEMA_VERSION: &str =
    "kyuubiki.remote-deployment-resume-plan/v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeploymentResumePlan {
    pub schema_version: String,
    pub status: String,
    pub plan_id: String,
    pub target_ref: String,
    pub source_revision: u64,
    pub source_digest: String,
    pub resume_step_id: Option<String>,
    pub completed_step_ids: Vec<String>,
    pub pending_step_ids: Vec<String>,
    pub reason_code: String,
}

pub fn prepare_remote_deployment_resume(
    plan: &RemoteDeploymentPlan,
    journal: &RemoteDeploymentJournal,
) -> Result<(RemoteDeploymentJournal, RemoteDeploymentResumePlan), String> {
    verify_remote_deployment_journal(plan, journal)?;
    let mut recovered = journal.clone();
    if let Some(step_id) = recovered
        .records
        .iter()
        .find(|record| record.status == "running")
        .map(|record| record.step_id.clone())
    {
        interrupt_remote_deployment_step(
            plan,
            &mut recovered,
            &step_id,
            "installer_process_interrupted",
        )?;
    }
    verify_remote_deployment_journal(plan, &recovered)?;

    let completed_step_ids = recovered.completed_step_ids();
    let pending_step_ids = recovered
        .records
        .iter()
        .filter(|record| record.status != "completed")
        .map(|record| record.step_id.clone())
        .collect::<Vec<_>>();
    let resume_step_id = pending_step_ids.first().cloned();
    let (status, reason_code) = if resume_step_id.is_some() {
        ("ready_to_resume", "resume_from_first_incomplete_step")
    } else {
        ("completed", "no_replay_required")
    };
    let resume = RemoteDeploymentResumePlan {
        schema_version: REMOTE_DEPLOYMENT_RESUME_PLAN_SCHEMA_VERSION.to_string(),
        status: status.to_string(),
        plan_id: recovered.plan_id.clone(),
        target_ref: recovered.target_ref.clone(),
        source_revision: recovered.revision,
        source_digest: recovered.journal_digest.clone(),
        resume_step_id,
        completed_step_ids,
        pending_step_ids,
        reason_code: reason_code.to_string(),
    };
    Ok((recovered, resume))
}
