use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

use crate::{SdkError, SdkResult};

pub const MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle/v1";

const POSTURE: &str = "screening_research_bundle";
const EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
const NEXT_ROUND_EXECUTION_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round-execution/v1";
const CHAIN_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-chain/v1";
const AUTHORITY_TRACE_SCHEMA_VERSION: &str = "kyuubiki.research-execution-authority-trace/v1";
const EXECUTION_AUTHORITY_SCHEMA_VERSION: &str = "kyuubiki.execution-authority/v1";
const RESEARCH_EVIDENCE_SCHEMA_VERSION: &str = "kyuubiki.material-research-evidence/v1";

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
    pub research_evidence: Value,
    pub validation_evidence: Value,
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
    pub material_card_ref_count: usize,
    pub material_card_refs: Vec<Value>,
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
    validate_execution_trace(&mut errors, &bundle.execution_trace);
    validate_research_evidence(&mut errors, bundle);
    crate::material_research_bundle_validation::validate_validation_evidence(&mut errors, bundle);
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
    crate::material_research_bundle_validation::validate_material_card_refs(&mut errors, bundle);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SdkError::Validation { errors })
    }
}

fn validate_execution_trace(errors: &mut Vec<String>, trace: &Value) {
    for duration in [
        "initial_duration_ms",
        "plan_next_duration_ms",
        "run_next_duration_ms",
        "chain_next_duration_ms",
    ] {
        require_u64(
            errors,
            trace.get(duration),
            &format!("execution_trace.{duration}"),
        );
    }
    let Some(authority) = trace.get("authority") else {
        errors.push("execution_trace.authority is required".to_string());
        return;
    };
    if authority.get("schema_version").and_then(Value::as_str)
        != Some(AUTHORITY_TRACE_SCHEMA_VERSION)
    {
        errors.push("execution_trace.authority.schema_version is invalid".to_string());
    }
    for assertion in ["all_real_solver", "no_mock_execution", "no_fallback"] {
        if authority
            .pointer(&format!("/assertions/{assertion}"))
            .and_then(Value::as_bool)
            != Some(true)
        {
            errors.push(format!(
                "execution_trace.authority.assertions.{assertion} must be true"
            ));
        }
    }
    for field in ["initial", "next"] {
        match authority.get(field) {
            Some(value) => validate_real_solver_authority(
                errors,
                value,
                &format!("execution_trace.authority.{field}"),
            ),
            None => errors.push(format!("execution_trace.authority.{field} is required")),
        }
    }
    match authority.get("chain").and_then(Value::as_array) {
        Some(chain) if !chain.is_empty() => {
            for (index, value) in chain.iter().enumerate() {
                validate_real_solver_authority(
                    errors,
                    value,
                    &format!("execution_trace.authority.chain[{index}]"),
                );
            }
        }
        _ => errors.push("execution_trace.authority.chain must be non-empty".to_string()),
    }
}

fn validate_real_solver_authority(errors: &mut Vec<String>, authority: &Value, field: &str) {
    require_value_str(errors, authority, "schema_version", field);
    require_value_str(errors, authority, "executor_id", field);
    require_value_str(errors, authority, "runtime", field);
    require_value_str(errors, authority, "result_origin", field);
    require_value_str(errors, authority, "evidence_statement", field);
    require_value_str_const(
        errors,
        authority,
        "schema_version",
        EXECUTION_AUTHORITY_SCHEMA_VERSION,
        field,
    );
    require_value_str_const(errors, authority, "execution_class", "real_solver", field);
    require_value_bool(errors, authority, "mock_execution", false, field);
    require_value_bool(errors, authority, "fallback_used", false, field);
    require_value_bool(errors, authority, "production_eligible", true, field);
}

fn validate_research_evidence(errors: &mut Vec<String>, bundle: &MaterialResearchBundle) {
    let evidence = &bundle.research_evidence;
    require_value_str_const(
        errors,
        evidence,
        "schema_version",
        RESEARCH_EVIDENCE_SCHEMA_VERSION,
        "research_evidence",
    );
    let ranked = require_string_array(
        errors,
        evidence.get("ranked_candidate_ids"),
        "research_evidence.ranked_candidate_ids",
        false,
    );
    require_unique(errors, &ranked, "research_evidence.ranked_candidate_ids");
    if let Some(candidate_count) = require_positive_u64(
        errors,
        evidence.get("candidate_count"),
        "research_evidence.candidate_count",
    ) && candidate_count != ranked.len() as u64
    {
        errors.push("research_evidence.candidate_count must match ranked_candidate_ids".into());
    }
    let winner = require_value_str(errors, evidence, "winner_candidate_id", "research_evidence");
    if let Some(winner) = winner.as_deref() {
        if winner != bundle.summary.winner_candidate_id {
            errors.push(
                "research_evidence.winner_candidate_id must match summary.winner_candidate_id"
                    .into(),
            );
        }
        if !ranked.iter().any(|candidate| candidate == winner) {
            errors.push("research_evidence winner must be present in ranked candidates".into());
        }
    }
    let focus = require_string_array(
        errors,
        evidence.get("focus_candidate_ids"),
        "research_evidence.focus_candidate_ids",
        false,
    );
    require_unique(errors, &focus, "research_evidence.focus_candidate_ids");
    for candidate in focus {
        if !ranked.contains(&candidate) {
            errors.push(format!(
                "research_evidence.focus_candidate_ids contains unknown candidate {candidate:?}"
            ));
        }
    }
    let metrics = require_string_array(
        errors,
        evidence.get("primary_metric_ids"),
        "research_evidence.primary_metric_ids",
        false,
    );
    require_unique(errors, &metrics, "research_evidence.primary_metric_ids");
    if let Some(count) = require_positive_u64(
        errors,
        evidence.get("metric_objective_count"),
        "research_evidence.metric_objective_count",
    ) && count != metrics.len() as u64
    {
        errors
            .push("research_evidence.metric_objective_count must match primary_metric_ids".into());
    }
    require_string_array(
        errors,
        evidence.get("violated_quality_gate_ids"),
        "research_evidence.violated_quality_gate_ids",
        true,
    );
    require_value_str_equal(
        errors,
        evidence,
        "quality_gate_decision",
        &bundle.summary.reliability_decision,
        "research_evidence.quality_gate_decision",
    );
    require_value_str_equal(
        errors,
        evidence,
        "plan_decision",
        &bundle.summary.next_round_decision,
        "research_evidence.plan_decision",
    );
    require_value_str(
        errors,
        evidence,
        "final_winner_candidate_id",
        "research_evidence",
    );
    if let Some(expected) = bundle.summary.runnable_next_step_count {
        require_value_u64_equal(
            errors,
            evidence,
            "plan_step_count",
            expected as u64,
            "research_evidence.plan_step_count",
        );
    } else {
        require_u64(
            errors,
            evidence.get("plan_step_count"),
            "research_evidence.plan_step_count",
        );
    }
    if let Some(expected) = bundle.summary.chain_round_count {
        require_value_u64_equal(
            errors,
            evidence,
            "chain_round_count",
            expected as u64,
            "research_evidence.chain_round_count",
        );
    } else {
        require_positive_u64(
            errors,
            evidence.get("chain_round_count"),
            "research_evidence.chain_round_count",
        );
    }
    let trace_count = require_positive_u64(
        errors,
        evidence.get("chain_trace_round_count"),
        "research_evidence.chain_trace_round_count",
    );
    if let (Some(expected), Some(trace)) = (
        trace_count,
        bundle
            .chain
            .get("optimization_trace")
            .and_then(Value::as_array),
    ) && expected != trace.len() as u64
    {
        errors.push(
            "research_evidence.chain_trace_round_count must match chain.optimization_trace".into(),
        );
    }
    if let Some(final_winner) = bundle
        .chain
        .get("final_winner_candidate_id")
        .and_then(Value::as_str)
    {
        require_value_str_equal(
            errors,
            evidence,
            "final_winner_candidate_id",
            final_winner,
            "research_evidence.final_winner_candidate_id",
        );
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

fn require_value_str(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    field: &str,
) -> Option<String> {
    match value.get(key).and_then(Value::as_str) {
        Some(actual) if !actual.is_empty() => Some(actual.to_string()),
        _ => {
            errors.push(format!("{field}.{key} must be a non-empty string"));
            None
        }
    }
}

fn require_value_str_const(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    expected: &str,
    field: &str,
) {
    match value.get(key).and_then(Value::as_str) {
        Some(actual) if actual == expected => {}
        Some(actual) => errors.push(format!("{field}.{key} must be {expected}, got {actual}")),
        None => errors.push(format!("{field}.{key} is required")),
    }
}

fn require_value_bool(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    expected: bool,
    field: &str,
) {
    if value.get(key).and_then(Value::as_bool) != Some(expected) {
        errors.push(format!("{field}.{key} must be {expected}"));
    }
}

fn require_u64(errors: &mut Vec<String>, value: Option<&Value>, field: &str) -> Option<u64> {
    match value.and_then(Value::as_u64) {
        Some(actual) => Some(actual),
        None => {
            errors.push(format!("{field} must be a non-negative integer"));
            None
        }
    }
}

fn require_positive_u64(
    errors: &mut Vec<String>,
    value: Option<&Value>,
    field: &str,
) -> Option<u64> {
    match require_u64(errors, value, field) {
        Some(actual) if actual > 0 => Some(actual),
        Some(_) => {
            errors.push(format!("{field} must be positive"));
            None
        }
        None => None,
    }
}

fn require_string_array(
    errors: &mut Vec<String>,
    value: Option<&Value>,
    field: &str,
    allow_empty: bool,
) -> Vec<String> {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{field} must be an array"));
        return Vec::new();
    };
    if !allow_empty && items.is_empty() {
        errors.push(format!("{field} must be non-empty"));
    }
    let mut output = Vec::with_capacity(items.len());
    for item in items {
        match item.as_str() {
            Some(text) if !text.is_empty() => output.push(text.to_string()),
            _ => errors.push(format!("{field} must contain only non-empty strings")),
        }
    }
    output
}

fn require_unique(errors: &mut Vec<String>, values: &[String], field: &str) {
    let unique = values.iter().collect::<HashSet<_>>();
    if unique.len() != values.len() {
        errors.push(format!("{field} must not contain duplicates"));
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
