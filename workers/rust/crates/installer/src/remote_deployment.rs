const REMOTE_DEPLOYMENT_SCHEMA_VERSION: &str = "kyuubiki.remote-deployment-roadmap/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeploymentRoadmap {
    pub schema_version: String,
    pub current_maturity: String,
    pub target_maturity: String,
    pub principles: Vec<String>,
    pub stages: Vec<RemoteDeploymentStage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeploymentStage {
    pub id: String,
    pub title: String,
    pub status: String,
    pub exit_criteria: Vec<String>,
}

impl RemoteDeploymentRoadmap {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote deployment roadmap".to_string(),
            format!("schema: {}", self.schema_version),
            format!("current_maturity: {}", self.current_maturity),
            format!("target_maturity: {}", self.target_maturity),
            "principles:".to_string(),
        ];
        for principle in &self.principles {
            lines.push(format!("  - {principle}"));
        }
        lines.push("stages:".to_string());
        for stage in &self.stages {
            lines.push(format!(
                "  [{}] {} ({})",
                stage.status, stage.title, stage.id
            ));
            for criterion in &stage.exit_criteria {
                lines.push(format!("    - {criterion}"));
            }
        }
        lines.join("\n")
    }
}

pub fn remote_deployment_roadmap() -> RemoteDeploymentRoadmap {
    RemoteDeploymentRoadmap {
        schema_version: REMOTE_DEPLOYMENT_SCHEMA_VERSION.to_string(),
        current_maturity: "pilot: bounded SSH wrapper with policy validation".to_string(),
        target_maturity: "service: idempotent remote deployment protocol with durable journal"
            .to_string(),
        principles: vec![
            "Installer owns deployment and repair, not modeling workflow semantics".to_string(),
            "SSH is transport; deployment intent must be represented as a plan".to_string(),
            "Remote actions must be policy-bounded, auditable, and replay-safe".to_string(),
            "Remote hosts should pull signed or staged artifacts instead of relying on ad-hoc shell state".to_string(),
        ],
        stages: vec![
            stage(
                "remote-policy",
                "Policy-bounded SSH transport",
                "implemented",
                &[
                    "allowed hosts and workspace roots are visible",
                    "host, user, workspace, URL, and agent identifiers are shape-validated",
                    "remote node registry records operator-visible action snapshots",
                ],
            ),
            stage(
                "deployment-plan",
                "Structured deployment plan and step contract",
                "started",
                &[
                    "bootstrap, package sync, verify, start, health-check, and rollback steps are explicit",
                    "each step has idempotency and failure classification metadata",
                    "CLI and GUI can preview the same plan before execution",
                ],
            ),
            stage(
                "remote-journal",
                "Durable remote deployment journal",
                "started",
                &[
                    "every remote action emits structured local and remote journal records",
                    "operators can resume or retry from the last safe step",
                    "logs avoid secrets and preserve host, component, and artifact identity",
                ],
            ),
            stage(
                "artifact-delivery",
                "Installer-managed artifact delivery",
                "started",
                &[
                    "remote host pulls staged packages from the deploy/update source",
                    "checksums and component integrity contracts are verified before start",
                    "old residue is cleaned only through component integrity rules",
                ],
            ),
            stage(
                "dry-run-preflight",
                "Read-only dry-run and preflight report",
                "started",
                &[
                    "plan, journal, artifacts, and integrity blockers are summarized together",
                    "dry-run output stays read-only and never opens SSH sessions",
                    "GUI can reuse the same readiness report before enabling execution",
                ],
            ),
            stage(
                "host-trust",
                "Host trust and credential hardening",
                "started",
                &[
                    "known-host policy exposes the dev accept-new path and managed pinned-host path",
                    "certificate and node identity binding are required for managed agents",
                    "remote deployment never stores SSH passwords in project files",
                ],
            ),
            stage(
                "integration-tests",
                "Physical and containerized remote deployment test matrix",
                "started",
                &[
                    "containerized SSH fixture validates command construction and journals",
                    "physical lab host validates bootstrap and agent lifecycle",
                    "installer tests guard policy and plan compatibility",
                ],
            ),
        ],
    }
}

fn stage(id: &str, title: &str, status: &str, exit_criteria: &[&str]) -> RemoteDeploymentStage {
    RemoteDeploymentStage {
        id: id.to_string(),
        title: title.to_string(),
        status: status.to_string(),
        exit_criteria: exit_criteria
            .iter()
            .map(|criterion| (*criterion).to_string())
            .collect(),
    }
}
