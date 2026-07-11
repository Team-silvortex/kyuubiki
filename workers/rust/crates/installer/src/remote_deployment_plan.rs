use serde::{Deserialize, Serialize};

const REMOTE_DEPLOYMENT_PLAN_SCHEMA_VERSION: &str = "kyuubiki.remote-deployment-plan/v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeploymentPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub target_profile: String,
    pub steps: Vec<RemoteDeploymentPlanStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeploymentPlanStep {
    pub id: String,
    pub title: String,
    pub phase: String,
    pub idempotency_key: String,
    pub rollback_hint: String,
    pub failure_class: String,
}

impl RemoteDeploymentPlan {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote deployment plan".to_string(),
            format!("schema: {}", self.schema_version),
            format!("plan_id: {}", self.plan_id),
            format!("target_profile: {}", self.target_profile),
            "steps:".to_string(),
        ];
        for step in &self.steps {
            lines.push(format!("  - {}: {}", step.id, step.title));
            lines.push(format!("    phase: {}", step.phase));
            lines.push(format!("    idempotency_key: {}", step.idempotency_key));
            lines.push(format!("    failure_class: {}", step.failure_class));
            lines.push(format!("    rollback_hint: {}", step.rollback_hint));
        }
        lines.join("\n")
    }
}

pub fn default_remote_deployment_plan() -> RemoteDeploymentPlan {
    RemoteDeploymentPlan {
        schema_version: REMOTE_DEPLOYMENT_PLAN_SCHEMA_VERSION.to_string(),
        plan_id: "remote-agent-lab-pilot".to_string(),
        target_profile: "policy-bounded SSH remote agent".to_string(),
        steps: vec![
            step(
                "policy-check",
                "Validate host, user, workspace, and authority policy",
                "preflight",
                "remote.policy",
                "stop before touching remote state",
                "policy",
            ),
            step(
                "bootstrap-workspace",
                "Prepare the remote workspace root and runtime directory",
                "bootstrap",
                "remote.workspace",
                "leave existing workspace untouched unless integrity cleanup allows it",
                "environment",
            ),
            step(
                "sync-artifacts",
                "Stage installer-managed artifacts for the selected platform",
                "delivery",
                "remote.artifacts",
                "delete only staged artifacts owned by this plan",
                "artifact",
            ),
            step(
                "verify-integrity",
                "Verify checksums and component integrity before start",
                "verify",
                "remote.integrity",
                "refuse start and surface integrity report",
                "integrity",
            ),
            step(
                "start-agent",
                "Start the Rust agent under the selected authority mode",
                "runtime",
                "remote.agent",
                "stop the started agent session if health-check fails",
                "runtime",
            ),
            step(
                "health-check",
                "Confirm the remote agent can report liveness and identity",
                "verify",
                "remote.health",
                "mark deployment incomplete and keep journal for retry",
                "health",
            ),
            step(
                "rollback-guide",
                "Emit safe rollback and cleanup instructions",
                "rollback",
                "remote.rollback",
                "operator-visible cleanup plan remains available",
                "recovery",
            ),
        ],
    }
}

fn step(
    id: &str,
    title: &str,
    phase: &str,
    idempotency_key: &str,
    rollback_hint: &str,
    failure_class: &str,
) -> RemoteDeploymentPlanStep {
    RemoteDeploymentPlanStep {
        id: id.to_string(),
        title: title.to_string(),
        phase: phase.to_string(),
        idempotency_key: idempotency_key.to_string(),
        rollback_hint: rollback_hint.to_string(),
        failure_class: failure_class.to_string(),
    }
}
