use crate::{
    OperatorSdkReadinessIssue, OperatorSdkReadinessReport, OperatorSdkReadinessSeverity,
    operator_descriptor_readiness,
};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorValidationStatus, WORKFLOW_DATASET_DATA_CLASSES,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const OPERATOR_MODEL_DRAFT_SCHEMA_VERSION: &str = "kyuubiki.operator-model-draft/v1";
pub const OPERATOR_MODEL_AUTHORING_MANIFEST_SCHEMA_VERSION: &str =
    "kyuubiki.operator-model-authoring-manifest/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorModelAuthoringPolicy {
    #[serde(default = "default_max_ports")]
    pub max_ports_per_direction: usize,
    #[serde(default = "default_max_tags")]
    pub max_capability_tags: usize,
    #[serde(default = "default_max_algorithm_steps")]
    pub max_algorithm_steps: usize,
    #[serde(default)]
    pub allow_side_effects: bool,
    #[serde(default)]
    pub allow_unverified_validation: bool,
}

impl Default for OperatorModelAuthoringPolicy {
    fn default() -> Self {
        Self {
            max_ports_per_direction: default_max_ports(),
            max_capability_tags: default_max_tags(),
            max_algorithm_steps: default_max_algorithm_steps(),
            allow_side_effects: false,
            allow_unverified_validation: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorModelHandlerTrait {
    JsonOperator,
    OperatorHandler,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorModelImplementationDraft {
    pub input_type: String,
    pub operator_type: String,
    pub handler_trait: OperatorModelHandlerTrait,
    #[serde(default)]
    pub algorithm_steps: Vec<String>,
    #[serde(default)]
    pub deterministic: bool,
    #[serde(default)]
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorModelDraft {
    pub schema_version: String,
    pub descriptor: OperatorDescriptor,
    pub input_json_schema: Value,
    pub output_json_schema: Value,
    pub implementation: OperatorModelImplementationDraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorModelAuthoringManifest {
    pub schema_version: &'static str,
    pub sdk_api_version: &'static str,
    pub language: &'static str,
    pub accepted_kinds: &'static [&'static str],
    pub accepted_handler_traits: &'static [&'static str],
    pub workflow_dataset_data_classes: &'static [&'static str],
    pub required_workflow: &'static [&'static str],
    pub hard_boundaries: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperatorModelDraftReport {
    pub schema_version: &'static str,
    pub ok: bool,
    pub issue_count: usize,
    pub issues: Vec<OperatorSdkReadinessIssue>,
    pub descriptor_readiness: OperatorSdkReadinessReport,
}

pub fn operator_model_authoring_manifest() -> OperatorModelAuthoringManifest {
    OperatorModelAuthoringManifest {
        schema_version: OPERATOR_MODEL_AUTHORING_MANIFEST_SCHEMA_VERSION,
        sdk_api_version: crate::OPERATOR_SDK_API_VERSION,
        language: "rust",
        accepted_kinds: &[
            "solver",
            "transform",
            "extract",
            "export",
            "workflow_bridge",
        ],
        accepted_handler_traits: &["json_operator", "operator_handler"],
        workflow_dataset_data_classes: WORKFLOW_DATASET_DATA_CLASSES,
        required_workflow: &[
            "draft_descriptor_and_json_schemas",
            "validate_operator_model_draft",
            "implement_rust_handler",
            "run_operator_descriptor_readiness",
            "package_manifest_preflight",
            "dynamic_smoke_before_activation",
        ],
        hard_boundaries: &[
            "model_draft_is_not_executable_code",
            "model_draft_cannot_activate_dynamic_libraries",
            "host_owns_package_admission_and_loading",
            "qualification_requires_independent_evidence",
        ],
    }
}

pub fn validate_operator_model_draft(
    draft: &OperatorModelDraft,
    policy: &OperatorModelAuthoringPolicy,
) -> OperatorModelDraftReport {
    let descriptor_readiness = operator_descriptor_readiness(&draft.descriptor);
    let mut issues = descriptor_readiness.issues.clone();

    if draft.schema_version != OPERATOR_MODEL_DRAFT_SCHEMA_VERSION {
        push_error(
            &mut issues,
            "model_draft_schema_mismatch",
            "schema_version",
            format!(
                "expected {} but found {}",
                OPERATOR_MODEL_DRAFT_SCHEMA_VERSION, draft.schema_version
            ),
        );
    }
    check_json_schema(&mut issues, &draft.input_json_schema, "input_json_schema");
    check_json_schema(&mut issues, &draft.output_json_schema, "output_json_schema");
    check_nonempty(
        &mut issues,
        &draft.implementation.input_type,
        "implementation.input_type",
    );
    check_nonempty(
        &mut issues,
        &draft.implementation.operator_type,
        "implementation.operator_type",
    );
    if draft.implementation.algorithm_steps.is_empty() {
        push_error(
            &mut issues,
            "model_algorithm_steps_empty",
            "implementation.algorithm_steps",
            "model draft must describe at least one bounded algorithm step".to_string(),
        );
    }
    if draft.implementation.algorithm_steps.len() > policy.max_algorithm_steps {
        push_error(
            &mut issues,
            "model_algorithm_steps_limit",
            "implementation.algorithm_steps",
            format!(
                "algorithm step count {} exceeds policy limit {}",
                draft.implementation.algorithm_steps.len(),
                policy.max_algorithm_steps
            ),
        );
    }
    if draft.descriptor.inputs.len() > policy.max_ports_per_direction
        || draft.descriptor.outputs.len() > policy.max_ports_per_direction
    {
        push_error(
            &mut issues,
            "model_port_limit",
            format!("descriptor.{}", draft.descriptor.id),
            format!(
                "input/output port count must not exceed {} per direction",
                policy.max_ports_per_direction
            ),
        );
    }
    if draft.descriptor.capability_tags.len() > policy.max_capability_tags {
        push_error(
            &mut issues,
            "model_capability_tag_limit",
            format!("descriptor.{}", draft.descriptor.id),
            format!(
                "capability tag count {} exceeds policy limit {}",
                draft.descriptor.capability_tags.len(),
                policy.max_capability_tags
            ),
        );
    }
    if !policy.allow_side_effects && !draft.implementation.side_effects.is_empty() {
        push_error(
            &mut issues,
            "model_side_effects_blocked",
            "implementation.side_effects",
            "model-authored operator drafts must be side-effect free under current policy"
                .to_string(),
        );
    }
    if !policy.allow_unverified_validation
        && draft.descriptor.validation.baseline_status == OperatorValidationStatus::Unverified
    {
        push_error(
            &mut issues,
            "model_unverified_validation_blocked",
            format!("descriptor.{}.validation", draft.descriptor.id),
            "model-authored drafts must declare at least partial validation evidence".to_string(),
        );
    }
    check_kind_contract(&mut issues, &draft.descriptor);

    let ok = issues
        .iter()
        .all(|issue| issue.severity != OperatorSdkReadinessSeverity::Error);
    OperatorModelDraftReport {
        schema_version: OPERATOR_MODEL_DRAFT_SCHEMA_VERSION,
        ok,
        issue_count: issues.len(),
        issues,
        descriptor_readiness,
    }
}

fn check_json_schema(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    schema: &Value,
    subject: &'static str,
) {
    if !schema.is_object() {
        push_error(
            issues,
            "model_json_schema_not_object",
            subject,
            "JSON Schema must be an object".to_string(),
        );
        return;
    }
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        push_error(
            issues,
            "model_json_schema_root_type",
            subject,
            "operator input and output JSON Schema roots must use type object".to_string(),
        );
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        push_warning(
            issues,
            "model_json_schema_open_properties",
            subject,
            "set additionalProperties to false before package qualification".to_string(),
        );
    }
}

fn check_kind_contract(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    descriptor: &OperatorDescriptor,
) {
    if descriptor.kind == OperatorKind::Solver
        && !descriptor.capability_tags.iter().any(|tag| tag == "solver")
    {
        push_warning(
            issues,
            "model_solver_tag_missing",
            format!("descriptor.{}", descriptor.id),
            "solver drafts should include the stable solver capability tag".to_string(),
        );
    }
}

fn check_nonempty(issues: &mut Vec<OperatorSdkReadinessIssue>, value: &str, subject: &'static str) {
    if value.trim().is_empty() {
        push_error(
            issues,
            "model_required_field_empty",
            subject,
            format!("{subject} must not be empty"),
        );
    }
}

fn push_error(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    code: &'static str,
    subject: impl Into<String>,
    message: String,
) {
    issues.push(OperatorSdkReadinessIssue {
        severity: OperatorSdkReadinessSeverity::Error,
        code,
        subject: subject.into(),
        message,
    });
}

fn push_warning(
    issues: &mut Vec<OperatorSdkReadinessIssue>,
    code: &'static str,
    subject: impl Into<String>,
    message: String,
) {
    issues.push(OperatorSdkReadinessIssue {
        severity: OperatorSdkReadinessSeverity::Warning,
        code,
        subject: subject.into(),
        message,
    });
}

fn default_max_ports() -> usize {
    32
}

fn default_max_tags() -> usize {
    32
}

fn default_max_algorithm_steps() -> usize {
    64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> OperatorModelDraft {
        serde_json::from_str(include_str!(
            "../../../../../schemas/examples.operator-model-draft.json"
        ))
        .expect("operator model draft fixture")
    }

    #[test]
    fn model_authoring_manifest_keeps_model_out_of_runtime_authority() {
        let manifest = operator_model_authoring_manifest();
        assert_eq!(manifest.language, "rust");
        assert!(
            manifest
                .hard_boundaries
                .contains(&"model_draft_cannot_activate_dynamic_libraries")
        );
        serde_json::to_value(manifest).expect("authoring manifest serializes");
    }

    #[test]
    fn repository_model_draft_passes_authoring_preflight() {
        let report =
            validate_operator_model_draft(&fixture(), &OperatorModelAuthoringPolicy::default());
        assert!(report.ok, "{:?}", report.issues);
    }

    #[test]
    fn model_draft_blocks_side_effects_and_open_schema_root() {
        let mut draft = fixture();
        draft.implementation.side_effects = vec!["write arbitrary host files".to_string()];
        draft.input_json_schema = serde_json::json!({ "type": "string" });
        let report =
            validate_operator_model_draft(&draft, &OperatorModelAuthoringPolicy::default());
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "model_side_effects_blocked")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "model_json_schema_root_type")
        );
    }
}
