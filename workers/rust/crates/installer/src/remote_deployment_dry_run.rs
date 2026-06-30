use crate::{
    Platform, RemoteArtifactDeliveryManifest, RemoteDeploymentJournal, RemoteDeploymentPlan,
    default_remote_deployment_journal, default_remote_deployment_plan,
    installation_integrity_report, remote_artifact_delivery_manifest,
};

const REMOTE_DRY_RUN_SCHEMA_VERSION: &str = "kyuubiki.remote-deployment-dry-run/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDeploymentDryRunReport {
    pub schema_version: String,
    pub status: String,
    pub plan: RemoteDeploymentPlan,
    pub journal: RemoteDeploymentJournal,
    pub artifact_manifest: Option<RemoteArtifactDeliveryManifest>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub next_actions: Vec<String>,
}

impl RemoteDeploymentDryRunReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote deployment dry-run".to_string(),
            format!("schema: {}", self.schema_version),
            format!("status: {}", self.status),
            format!("plan_id: {}", self.plan.plan_id),
            format!("plan_steps: {}", self.plan.steps.len()),
            format!("journal_records: {}", self.journal.records.len()),
            format!(
                "artifact_count: {}",
                self.artifact_manifest
                    .as_ref()
                    .map(|manifest| manifest.artifacts.len())
                    .unwrap_or(0)
            ),
        ];
        append_section(&mut lines, "blockers", &self.blockers);
        append_section(&mut lines, "warnings", &self.warnings);
        append_section(&mut lines, "next_actions", &self.next_actions);
        lines.push(
            "note: dry-run is read-only and does not open SSH sessions or mutate hosts".to_string(),
        );
        lines.join("\n")
    }
}

pub fn default_remote_deployment_dry_run() -> RemoteDeploymentDryRunReport {
    remote_deployment_dry_run(None, Platform::current())
}

pub fn remote_deployment_dry_run(
    channel: Option<String>,
    platform: Platform,
) -> RemoteDeploymentDryRunReport {
    let plan = default_remote_deployment_plan();
    let journal = default_remote_deployment_journal();
    let artifact_manifest = remote_artifact_delivery_manifest(channel, platform);
    let integrity = installation_integrity_report();
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if plan.steps.is_empty() {
        blockers.push("deployment plan has no steps".to_string());
    }
    if journal.records.len() != plan.steps.len() {
        blockers.push(format!(
            "journal record count {} does not match plan step count {}",
            journal.records.len(),
            plan.steps.len()
        ));
    }
    if !integrity.issues.is_empty() {
        blockers.push(format!(
            "installation integrity has {} blocking issue(s)",
            integrity.issues.len()
        ));
    }
    let missing_required_paths = integrity
        .layout
        .iter()
        .filter(|entry| entry.required && !entry.present)
        .count();
    if missing_required_paths > 0 {
        blockers.push(format!(
            "installation layout is missing {missing_required_paths} required path(s)"
        ));
    }
    let version_mismatches = integrity
        .version_checks
        .iter()
        .filter(|check| !check.ok)
        .count();
    if version_mismatches > 0 {
        blockers.push(format!(
            "brand/version alignment has {version_mismatches} mismatch(es)"
        ));
    }

    let artifact_manifest = match artifact_manifest {
        Ok(manifest) => {
            if manifest.artifacts.is_empty() {
                blockers.push("remote artifact manifest is empty".to_string());
            }
            Some(manifest)
        }
        Err(error) => {
            blockers.push(format!("remote artifact manifest is unavailable: {error}"));
            None
        }
    };

    let pending_records = journal
        .records
        .iter()
        .filter(|record| record.status == "pending")
        .count();
    if pending_records == journal.records.len() {
        warnings.push("all journal records are preview-only pending entries".to_string());
    }
    warnings.push("host trust remains dev-oriented until known-host pinning is added".to_string());

    let status = if blockers.is_empty() {
        "ready_for_preflight"
    } else {
        "blocked"
    }
    .to_string();
    let next_actions = if blockers.is_empty() {
        vec![
            "connect dry-run to Installer GUI preflight panel".to_string(),
            "add containerized SSH fixture before enabling execution".to_string(),
            "add host-key pinning policy before non-dev remote deployments".to_string(),
        ]
    } else {
        vec![
            "resolve blockers before enabling remote execution".to_string(),
            "run installation-integrity and remote-artifacts for details".to_string(),
        ]
    };

    RemoteDeploymentDryRunReport {
        schema_version: REMOTE_DRY_RUN_SCHEMA_VERSION.to_string(),
        status,
        plan,
        journal,
        artifact_manifest,
        blockers,
        warnings,
        next_actions,
    }
}

fn append_section(lines: &mut Vec<String>, label: &str, values: &[String]) {
    lines.push(format!("{label}:"));
    if values.is_empty() {
        lines.push("  - none".to_string());
        return;
    }
    for value in values {
        lines.push(format!("  - {value}"));
    }
}
