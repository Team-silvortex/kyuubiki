use crate::{RemoteDeploymentPlan, default_remote_deployment_plan};

const REMOTE_DEPLOYMENT_JOURNAL_SCHEMA_VERSION: &str = "kyuubiki.remote-deployment-journal/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeploymentJournal {
    pub schema_version: String,
    pub plan_id: String,
    pub target_ref: String,
    pub records: Vec<RemoteDeploymentJournalRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeploymentJournalRecord {
    pub step_id: String,
    pub phase: String,
    pub status: String,
    pub idempotency_key: String,
    pub failure_class: String,
    pub local_record_path: String,
    pub remote_record_path: String,
}

impl RemoteDeploymentJournal {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote deployment journal preview".to_string(),
            format!("schema: {}", self.schema_version),
            format!("plan_id: {}", self.plan_id),
            format!("target_ref: {}", self.target_ref),
            "records:".to_string(),
        ];
        for record in &self.records {
            lines.push(format!("  - {} [{}]", record.step_id, record.status));
            lines.push(format!("    phase: {}", record.phase));
            lines.push(format!("    idempotency_key: {}", record.idempotency_key));
            lines.push(format!("    failure_class: {}", record.failure_class));
            lines.push(format!(
                "    local_record_path: {}",
                record.local_record_path
            ));
            lines.push(format!(
                "    remote_record_path: {}",
                record.remote_record_path
            ));
        }
        lines.join("\n")
    }
}

pub fn default_remote_deployment_journal() -> RemoteDeploymentJournal {
    remote_deployment_journal_for_plan(&default_remote_deployment_plan(), "lab-remote-agent")
}

pub fn remote_deployment_journal_for_plan(
    plan: &RemoteDeploymentPlan,
    target_ref: &str,
) -> RemoteDeploymentJournal {
    RemoteDeploymentJournal {
        schema_version: REMOTE_DEPLOYMENT_JOURNAL_SCHEMA_VERSION.to_string(),
        plan_id: plan.plan_id.clone(),
        target_ref: target_ref.to_string(),
        records: plan
            .steps
            .iter()
            .map(|step| RemoteDeploymentJournalRecord {
                step_id: step.id.clone(),
                phase: step.phase.clone(),
                status: "pending".to_string(),
                idempotency_key: format!("{}:{}", target_ref, step.idempotency_key),
                failure_class: step.failure_class.clone(),
                local_record_path: format!(
                    ".kyuubiki/remote-journal/{}/{}.jsonl",
                    plan.plan_id, step.id
                ),
                remote_record_path: format!(
                    ".kyuubiki/remote-journal/{}/{}.jsonl",
                    target_ref, step.id
                ),
            })
            .collect(),
    }
}
