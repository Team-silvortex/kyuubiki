use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SdkError, SdkResult};

pub const MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle/v1";

const POSTURE: &str = "screening_research_bundle";
const EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
const NEXT_ROUND_EXECUTION_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round-execution/v1";
const CHAIN_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-chain/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialResearchBundle {
    pub schema_version: String,
    pub bundle_id: String,
    pub generated_at_utc: String,
    pub posture: String,
    pub study: String,
    pub artifact_checksums: MaterialResearchBundleArtifactChecksums,
    pub reproducibility: MaterialResearchBundleReproducibility,
    pub execution_trace: Value,
    pub summary: MaterialResearchBundleSummary,
    pub initial_exploration: Value,
    pub next_round_execution_plan: Value,
    pub next_exploration: Value,
    pub chain: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialResearchBundleArtifactChecksums {
    pub initial_exploration_sha256: String,
    pub next_round_execution_plan_sha256: String,
    pub next_exploration_sha256: String,
    pub chain_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialResearchBundleReproducibility {
    pub workspace: String,
    pub initial_command: Vec<String>,
    pub plan_next_command_template: Vec<String>,
    pub run_next_command_template: Vec<String>,
    pub chain_next_command_template: Vec<String>,
    #[serde(default)]
    pub transient_work_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialResearchBundleSummary {
    pub winner_candidate_id: String,
    pub reliability_decision: String,
    pub next_round_decision: String,
    #[serde(default)]
    pub runnable_next_step_count: Option<usize>,
    #[serde(default)]
    pub next_iteration: Option<usize>,
    pub chain_stop_reason: String,
    #[serde(default)]
    pub chain_convergence_state: Option<String>,
    #[serde(default)]
    pub chain_round_count: Option<usize>,
}

impl MaterialResearchBundle {
    pub fn validate(&self) -> SdkResult<()> {
        validate_material_research_bundle(self)
    }
}

pub fn validate_material_research_bundle(bundle: &MaterialResearchBundle) -> SdkResult<()> {
    let mut errors = Vec::new();
    require_equal(
        &mut errors,
        &bundle.schema_version,
        MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION,
        "schema_version",
    );
    require_equal(&mut errors, &bundle.posture, POSTURE, "posture");
    require_non_empty(&mut errors, &bundle.bundle_id, "bundle_id");
    require_non_empty(&mut errors, &bundle.generated_at_utc, "generated_at_utc");
    require_non_empty(&mut errors, &bundle.study, "study");
    validate_checksums(&mut errors, &bundle.artifact_checksums);
    validate_reproducibility(&mut errors, &bundle.reproducibility);
    require_artifact_schema(
        &mut errors,
        &bundle.initial_exploration,
        EXPLORATION_SCHEMA_VERSION,
        "initial_exploration",
    );
    require_artifact_schema(
        &mut errors,
        &bundle.next_round_execution_plan,
        NEXT_ROUND_EXECUTION_SCHEMA_VERSION,
        "next_round_execution_plan",
    );
    require_artifact_schema(
        &mut errors,
        &bundle.next_exploration,
        EXPLORATION_SCHEMA_VERSION,
        "next_exploration",
    );
    require_artifact_schema(&mut errors, &bundle.chain, CHAIN_SCHEMA_VERSION, "chain");
    validate_summary_artifact_consistency(&mut errors, bundle);
    require_non_empty(
        &mut errors,
        &bundle.summary.winner_candidate_id,
        "summary.winner_candidate_id",
    );
    require_non_empty(
        &mut errors,
        &bundle.summary.reliability_decision,
        "summary.reliability_decision",
    );
    require_non_empty(
        &mut errors,
        &bundle.summary.next_round_decision,
        "summary.next_round_decision",
    );
    require_non_empty(
        &mut errors,
        &bundle.summary.chain_stop_reason,
        "summary.chain_stop_reason",
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SdkError::Validation { errors })
    }
}

fn validate_summary_artifact_consistency(
    errors: &mut Vec<String>,
    bundle: &MaterialResearchBundle,
) {
    require_value_str_equal(
        errors,
        &bundle.next_round_execution_plan,
        "decision",
        &bundle.summary.next_round_decision,
        "next_round_execution_plan.decision",
    );
    if let Some(expected) = bundle.summary.runnable_next_step_count {
        require_value_u64_equal(
            errors,
            &bundle.next_round_execution_plan,
            "runnable_step_count",
            expected as u64,
            "next_round_execution_plan.runnable_step_count",
        );
    }
    if let Some(expected) = bundle.summary.next_iteration {
        require_value_u64_equal(
            errors,
            &bundle.next_round_execution_plan,
            "iteration",
            expected as u64,
            "next_round_execution_plan.iteration",
        );
        require_value_u64_equal(
            errors,
            &bundle.next_exploration,
            "iteration",
            expected as u64,
            "next_exploration.iteration",
        );
    }
    require_value_str_equal(
        errors,
        &bundle.chain,
        "stop_reason",
        &bundle.summary.chain_stop_reason,
        "chain.stop_reason",
    );
}

fn validate_checksums(
    errors: &mut Vec<String>,
    checksums: &MaterialResearchBundleArtifactChecksums,
) {
    require_sha256(
        errors,
        &checksums.initial_exploration_sha256,
        "artifact_checksums.initial_exploration_sha256",
    );
    require_sha256(
        errors,
        &checksums.next_round_execution_plan_sha256,
        "artifact_checksums.next_round_execution_plan_sha256",
    );
    require_sha256(
        errors,
        &checksums.next_exploration_sha256,
        "artifact_checksums.next_exploration_sha256",
    );
    require_sha256(
        errors,
        &checksums.chain_sha256,
        "artifact_checksums.chain_sha256",
    );
}

fn validate_reproducibility(
    errors: &mut Vec<String>,
    reproducibility: &MaterialResearchBundleReproducibility,
) {
    require_non_empty(
        errors,
        &reproducibility.workspace,
        "reproducibility.workspace",
    );
    require_argv(
        errors,
        &reproducibility.initial_command,
        "reproducibility.initial_command",
    );
    require_argv(
        errors,
        &reproducibility.plan_next_command_template,
        "reproducibility.plan_next_command_template",
    );
    require_argv(
        errors,
        &reproducibility.run_next_command_template,
        "reproducibility.run_next_command_template",
    );
    require_argv(
        errors,
        &reproducibility.chain_next_command_template,
        "reproducibility.chain_next_command_template",
    );
}

fn require_artifact_schema(errors: &mut Vec<String>, value: &Value, expected: &str, field: &str) {
    match value.get("schema_version").and_then(Value::as_str) {
        Some(actual) => require_equal(errors, actual, expected, &format!("{field}.schema_version")),
        None => errors.push(format!("{field}.schema_version is required")),
    }
}

fn require_equal(errors: &mut Vec<String>, actual: &str, expected: &str, field: &str) {
    if actual != expected {
        errors.push(format!("{field} must be {expected}, got {actual}"));
    }
}

fn require_non_empty(errors: &mut Vec<String>, value: &str, field: &str) {
    if value.is_empty() {
        errors.push(format!("{field} must be a non-empty string"));
    }
}

fn require_argv(errors: &mut Vec<String>, argv: &[String], field: &str) {
    if argv.is_empty() || argv.iter().any(|item| item.is_empty()) {
        errors.push(format!("{field} must be a non-empty argv array"));
    }
}

fn require_sha256(errors: &mut Vec<String>, value: &str, field: &str) {
    let is_sha256 = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !is_sha256 {
        errors.push(format!("{field} must be a lowercase SHA-256 hex digest"));
    }
}

fn require_value_str_equal(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    expected: &str,
    field: &str,
) {
    match value.get(key).and_then(Value::as_str) {
        Some(actual) => require_equal(errors, actual, expected, field),
        None => errors.push(format!("{field} is required")),
    }
}

fn require_value_u64_equal(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    expected: u64,
    field: &str,
) {
    match value.get(key).and_then(Value::as_u64) {
        Some(actual) if actual == expected => {}
        Some(actual) => errors.push(format!("{field} must be {expected}, got {actual}")),
        None => errors.push(format!("{field} is required")),
    }
}
