use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle/v1";

const MATERIAL_EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
const NEXT_ROUND_EXECUTION_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round-execution/v1";
const MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-chain/v1";
const SCREENING_RESEARCH_BUNDLE_POSTURE: &str = "screening_research_bundle";

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
    pub material_card_ref_count: usize,
    pub material_card_refs: Vec<MaterialResearchBundleMaterialCardRef>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialResearchBundleMaterialCardRef {
    pub material_card_id: String,
    pub schema_version: String,
    pub candidate_id: String,
    pub confidence: String,
    pub unit_system: String,
    pub parameter_scope: String,
    #[serde(default)]
    pub source: String,
}

pub fn validate_material_research_bundle(bundle: &MaterialResearchBundle) -> Result<(), String> {
    require_equal(
        &bundle.schema_version,
        MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION,
        "schema_version",
    )?;
    require_equal(
        &bundle.posture,
        SCREENING_RESEARCH_BUNDLE_POSTURE,
        "posture",
    )?;
    require_non_empty(&bundle.bundle_id, "bundle_id")?;
    require_non_empty(&bundle.generated_at_utc, "generated_at_utc")?;
    require_non_empty(&bundle.study, "study")?;
    validate_reproducibility(&bundle.reproducibility)?;
    validate_checksums(&bundle.artifact_checksums)?;
    require_artifact_schema(
        &bundle.initial_exploration,
        MATERIAL_EXPLORATION_SCHEMA_VERSION,
        "initial_exploration",
    )?;
    require_artifact_schema(
        &bundle.next_round_execution_plan,
        NEXT_ROUND_EXECUTION_SCHEMA_VERSION,
        "next_round_execution_plan",
    )?;
    require_artifact_schema(
        &bundle.next_exploration,
        MATERIAL_EXPLORATION_SCHEMA_VERSION,
        "next_exploration",
    )?;
    require_artifact_schema(
        &bundle.chain,
        MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION,
        "chain",
    )?;
    validate_summary_artifact_consistency(bundle)?;
    require_non_empty(
        &bundle.summary.winner_candidate_id,
        "summary.winner_candidate_id",
    )?;
    require_non_empty(
        &bundle.summary.reliability_decision,
        "summary.reliability_decision",
    )?;
    validate_material_card_refs(&bundle.summary)?;
    require_non_empty(
        &bundle.summary.next_round_decision,
        "summary.next_round_decision",
    )?;
    require_non_empty(
        &bundle.summary.chain_stop_reason,
        "summary.chain_stop_reason",
    )?;
    Ok(())
}

fn validate_material_card_refs(summary: &MaterialResearchBundleSummary) -> Result<(), String> {
    if summary.material_card_ref_count == 0 {
        return Err("summary.material_card_ref_count must be at least 1".to_string());
    }
    if summary.material_card_refs.len() != summary.material_card_ref_count {
        return Err(
            "summary.material_card_refs length must match material_card_ref_count".to_string(),
        );
    }
    for (index, reference) in summary.material_card_refs.iter().enumerate() {
        let label = format!("summary.material_card_refs[{index}]");
        require_non_empty(
            &reference.material_card_id,
            &format!("{label}.material_card_id"),
        )?;
        require_equal(
            &reference.schema_version,
            "kyuubiki.material-card/v1",
            &format!("{label}.schema_version"),
        )?;
        require_non_empty(&reference.candidate_id, &format!("{label}.candidate_id"))?;
        require_non_empty(&reference.confidence, &format!("{label}.confidence"))?;
        require_non_empty(&reference.unit_system, &format!("{label}.unit_system"))?;
        require_non_empty(
            &reference.parameter_scope,
            &format!("{label}.parameter_scope"),
        )?;
    }
    Ok(())
}

fn validate_summary_artifact_consistency(bundle: &MaterialResearchBundle) -> Result<(), String> {
    require_value_str_equal(
        &bundle.next_round_execution_plan,
        "decision",
        &bundle.summary.next_round_decision,
        "next_round_execution_plan.decision",
    )?;
    if let Some(expected) = bundle.summary.runnable_next_step_count {
        require_value_u64_equal(
            &bundle.next_round_execution_plan,
            "runnable_step_count",
            expected as u64,
            "next_round_execution_plan.runnable_step_count",
        )?;
    }
    if let Some(expected) = bundle.summary.next_iteration {
        require_value_u64_equal(
            &bundle.next_round_execution_plan,
            "iteration",
            expected as u64,
            "next_round_execution_plan.iteration",
        )?;
        require_value_u64_equal(
            &bundle.next_exploration,
            "iteration",
            expected as u64,
            "next_exploration.iteration",
        )?;
    }
    require_value_str_equal(
        &bundle.chain,
        "stop_reason",
        &bundle.summary.chain_stop_reason,
        "chain.stop_reason",
    )
}

fn validate_reproducibility(
    reproducibility: &MaterialResearchBundleReproducibility,
) -> Result<(), String> {
    require_non_empty(&reproducibility.workspace, "reproducibility.workspace")?;
    require_argv(
        &reproducibility.initial_command,
        "reproducibility.initial_command",
    )?;
    require_argv(
        &reproducibility.plan_next_command_template,
        "reproducibility.plan_next_command_template",
    )?;
    require_argv(
        &reproducibility.run_next_command_template,
        "reproducibility.run_next_command_template",
    )?;
    require_argv(
        &reproducibility.chain_next_command_template,
        "reproducibility.chain_next_command_template",
    )
}

fn validate_checksums(checksums: &MaterialResearchBundleArtifactChecksums) -> Result<(), String> {
    require_sha256(
        &checksums.initial_exploration_sha256,
        "artifact_checksums.initial_exploration_sha256",
    )?;
    require_sha256(
        &checksums.next_round_execution_plan_sha256,
        "artifact_checksums.next_round_execution_plan_sha256",
    )?;
    require_sha256(
        &checksums.next_exploration_sha256,
        "artifact_checksums.next_exploration_sha256",
    )?;
    require_sha256(&checksums.chain_sha256, "artifact_checksums.chain_sha256")
}

fn require_artifact_schema(value: &Value, expected: &str, field: &str) -> Result<(), String> {
    let actual = value
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field}.schema_version is required"))?;
    require_equal(actual, expected, &format!("{field}.schema_version"))
}

fn require_equal(actual: &str, expected: &str, field: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{field} must be {expected}, got {actual}"))
    }
}

fn require_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty() {
        Err(format!("{field} must be a non-empty string"))
    } else {
        Ok(())
    }
}

fn require_argv(argv: &[String], field: &str) -> Result<(), String> {
    if argv.is_empty() {
        return Err(format!("{field} must be a non-empty argv array"));
    }
    for (index, value) in argv.iter().enumerate() {
        if value.is_empty() {
            return Err(format!("{field}[{index}] must be a non-empty string"));
        }
    }
    Ok(())
}

fn require_sha256(value: &str, field: &str) -> Result<(), String> {
    let is_sha256 = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if is_sha256 {
        Ok(())
    } else {
        Err(format!("{field} must be a lowercase SHA-256 hex digest"))
    }
}

fn require_value_str_equal(
    value: &Value,
    key: &str,
    expected: &str,
    field: &str,
) -> Result<(), String> {
    let actual = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field} is required"))?;
    require_equal(actual, expected, field)
}

fn require_value_u64_equal(
    value: &Value,
    key: &str,
    expected: u64,
    field: &str,
) -> Result<(), String> {
    let actual = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{field} is required"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{field} must be {expected}, got {actual}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_shared_material_research_bundle_fixture() {
        let bundle: MaterialResearchBundle = serde_json::from_str(include_str!(
            "../../../../../schemas/examples.material-research-bundle.json"
        ))
        .expect("fixture should decode");

        validate_material_research_bundle(&bundle).expect("fixture should validate");

        assert_eq!(
            bundle.schema_version,
            MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION
        );
        assert_eq!(bundle.study, "heat-spreader");
        assert_eq!(
            bundle.summary.winner_candidate_id,
            "pyrolytic_graphite_in_plane"
        );
    }

    #[test]
    fn rejects_bad_retained_artifact_schema() {
        let mut bundle: MaterialResearchBundle = serde_json::from_str(include_str!(
            "../../../../../schemas/examples.material-research-bundle.json"
        ))
        .expect("fixture should decode");
        bundle.chain["schema_version"] = Value::String("wrong".to_string());

        let error =
            validate_material_research_bundle(&bundle).expect_err("bad chain schema should fail");

        assert!(error.contains("chain.schema_version"));
    }

    #[test]
    fn rejects_bad_checksum_shape() {
        let mut bundle: MaterialResearchBundle = serde_json::from_str(include_str!(
            "../../../../../schemas/examples.material-research-bundle.json"
        ))
        .expect("fixture should decode");
        bundle.artifact_checksums.chain_sha256 = "not-a-digest".to_string();

        let error =
            validate_material_research_bundle(&bundle).expect_err("bad checksum should fail");

        assert!(error.contains("chain_sha256"));
    }

    #[test]
    fn rejects_bundle_summary_plan_decision_mismatch() {
        let mut bundle: MaterialResearchBundle = serde_json::from_str(include_str!(
            "../../../../../schemas/examples.material-research-bundle.json"
        ))
        .expect("fixture should decode");
        bundle.next_round_execution_plan["decision"] =
            Value::String("repair_validation".to_string());

        let error = validate_material_research_bundle(&bundle)
            .expect_err("summary and plan decision mismatch should fail");

        assert!(error.contains("next_round_execution_plan.decision"));
    }
}
