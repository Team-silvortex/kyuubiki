use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::{
    ModelCollaborationSession, ModelHeadlessPlan, ModelWorkflowProposal, SdkError, SdkResult,
    build_model_headless_plan,
};

pub const MODEL_RESEARCH_BOOTSTRAP_SCHEMA_VERSION: &str = "kyuubiki.model-research-bootstrap/v1";
pub const MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION: &str =
    "kyuubiki.model-research-readiness-report/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResearchSdk {
    Rust,
    Python,
    Elixir,
}

impl ModelResearchSdk {
    fn key(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Elixir => "elixir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResearchSelectedSurface {
    pub collaboration_path: String,
    pub preflight_path: String,
    pub execution_path: String,
    pub approval_path: String,
    pub frontier_path: String,
    pub validation_path: String,
    pub request: String,
    pub inspect: String,
    pub bootstrap_plan: String,
    pub normalize: String,
    pub plan: String,
    pub executor: String,
    pub dispatcher: String,
    pub approval_verifier: String,
    pub plan_digest: String,
    pub approval_request: String,
    pub frontier_start: String,
    pub frontier_advance: String,
    pub frontier_digest: String,
    pub frontier_validator: String,
    pub frontier_digest_verifier: String,
    pub result_validator: String,
    pub receipt_verifier: String,
    pub frontier_verifier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResearchReadinessReport {
    pub schema_version: String,
    pub selected_sdk: ModelResearchSdk,
    pub ready_for_planning: bool,
    pub execution_authority: String,
    pub version_line: String,
    pub entrypoint: String,
    pub workflow_id: String,
    pub selected_surface: Option<ModelResearchSelectedSurface>,
    pub required_resources: Vec<String>,
    pub missing_resources: Vec<String>,
    pub blockers: Vec<String>,
    pub hard_rules: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub completion_contract: Option<Value>,
}

pub fn inspect_model_research_bootstrap<F>(
    bootstrap: &Value,
    sdk: ModelResearchSdk,
    resource_exists: F,
) -> ModelResearchReadinessReport
where
    F: Fn(&str) -> bool,
{
    let mut blockers = Vec::new();
    let Some(root) = bootstrap.as_object() else {
        return empty_report(sdk, "bootstrap must be a JSON object");
    };

    require_exact(
        root,
        "schema_version",
        MODEL_RESEARCH_BOOTSTRAP_SCHEMA_VERSION,
        &mut blockers,
    );
    let version_line = text(root, "version_line").unwrap_or_else(|| "unknown".into());
    let entrypoint = text(root, "entrypoint").unwrap_or_else(|| "unknown".into());
    let first_research = object(root, "first_research", &mut blockers);
    let workflow_id = first_research
        .and_then(|value| text(value, "workflow_id"))
        .unwrap_or_else(|| "unknown".into());
    if first_research
        .and_then(|value| text(value, "reliability_posture"))
        .as_deref()
        != Some("screening_only")
    {
        blockers.push("first_research.reliability_posture must be screening_only".into());
    }

    let hard_rules = string_list(root.get("hard_rules"));
    if hard_rules.len() < 8 {
        blockers.push("hard_rules must contain at least 8 non-empty rules".into());
    }
    let stop_conditions = string_list(root.get("stop_conditions"));
    if stop_conditions.len() < 4 {
        blockers.push("stop_conditions must contain at least 4 non-empty rules".into());
    }
    let completion_contract = validate_completion_contract(root, &mut blockers);
    if root
        .get("research_protocol")
        .and_then(Value::as_array)
        .is_none_or(|stages| stages.len() < 6)
    {
        blockers.push("research_protocol must contain at least 6 stages".into());
    }

    let mut resources = BTreeSet::new();
    add_path(&entrypoint, "entrypoint", &mut resources, &mut blockers);
    add_document_paths(root, &mut resources, &mut blockers);
    let selected_surface = build_selected_surface(root, sdk, &mut resources, &mut blockers);
    add_execution_resources(root, &mut resources, &mut blockers);
    add_first_research_resources(first_research, &mut resources, &mut blockers);
    add_preflight_resources(root, &mut resources, &mut blockers);

    let required_resources = resources.into_iter().collect::<Vec<_>>();
    let missing_resources = required_resources
        .iter()
        .filter(|path| !resource_exists(path))
        .cloned()
        .collect::<Vec<_>>();
    blockers.extend(
        missing_resources
            .iter()
            .map(|path| format!("missing required resource: {path}")),
    );
    blockers.sort();
    blockers.dedup();

    ModelResearchReadinessReport {
        schema_version: MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION.into(),
        selected_sdk: sdk,
        ready_for_planning: blockers.is_empty() && selected_surface.is_some(),
        execution_authority: "none_preflight_only".into(),
        version_line,
        entrypoint,
        workflow_id,
        selected_surface,
        required_resources,
        missing_resources,
        blockers,
        hard_rules,
        stop_conditions,
        completion_contract,
    }
}

pub fn build_bootstrapped_model_headless_plan(
    readiness: &ModelResearchReadinessReport,
    session: &ModelCollaborationSession,
    proposal: &ModelWorkflowProposal,
) -> SdkResult<ModelHeadlessPlan> {
    validate_readiness_for_plan(readiness)?;
    if session.workflow_id != readiness.workflow_id {
        return bootstrap_error(
            "collaboration session workflow_id does not match readiness report",
        );
    }
    let plan = build_model_headless_plan(session, proposal)?;
    if !plan.ok {
        return Err(SdkError::Validation {
            errors: plan
                .issues
                .iter()
                .map(|issue| format!("bootstrapped plan: {issue}"))
                .collect(),
        });
    }
    Ok(plan)
}

fn validate_readiness_for_plan(readiness: &ModelResearchReadinessReport) -> SdkResult<()> {
    let surface_valid = readiness.selected_surface.as_ref().is_some_and(|surface| {
        surface.preflight_path == "sdks/rust/src/model_research_bootstrap.rs"
            && surface.inspect == "inspect_model_research_bootstrap"
            && surface.bootstrap_plan == "build_bootstrapped_model_headless_plan"
    });
    if readiness.schema_version != MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION
        || readiness.selected_sdk != ModelResearchSdk::Rust
        || !readiness.ready_for_planning
        || readiness.execution_authority != "none_preflight_only"
        || !readiness.missing_resources.is_empty()
        || !readiness.blockers.is_empty()
        || readiness.hard_rules.len() < 8
        || readiness.stop_conditions.len() < 4
        || readiness.completion_contract.is_none()
        || !surface_valid
    {
        return bootstrap_error("readiness report is not valid for Rust planning");
    }
    Ok(())
}

fn bootstrap_error<T>(message: impl Into<String>) -> SdkResult<T> {
    Err(SdkError::Validation {
        errors: vec![message.into()],
    })
}

fn build_selected_surface(
    root: &Map<String, Value>,
    sdk: ModelResearchSdk,
    resources: &mut BTreeSet<String>,
    blockers: &mut Vec<String>,
) -> Option<ModelResearchSelectedSurface> {
    let key = sdk.key();
    let collaboration = root
        .get("sdk_surfaces")
        .and_then(Value::as_object)
        .and_then(|surfaces| surfaces.get(key))
        .and_then(Value::as_object);
    let execution_root = root.get("execution_contract").and_then(Value::as_object);
    if execution_root
        .and_then(|value| text(value, "approval_authority"))
        .as_deref()
        != Some("caller_only")
    {
        blockers.push("execution_contract.approval_authority must be caller_only".into());
    }
    let execution = execution_root
        .and_then(|value| value.get("surfaces"))
        .and_then(Value::as_object)
        .and_then(|surfaces| surfaces.get(key))
        .and_then(Value::as_object);
    let preflight = root
        .get("preflight")
        .and_then(Value::as_object)
        .and_then(|value| value.get("surfaces"))
        .and_then(Value::as_object)
        .and_then(|surfaces| surfaces.get(key))
        .and_then(Value::as_object);
    let (Some(collaboration), Some(preflight), Some(execution)) =
        (collaboration, preflight, execution)
    else {
        blockers.push(format!("selected SDK surface is missing: {key}"));
        return None;
    };

    let fields = [
        field(
            collaboration,
            "path",
            &format!("sdk_surfaces.{key}.path"),
            blockers,
        ),
        field(
            preflight,
            "path",
            &format!("preflight.surfaces.{key}.path"),
            blockers,
        ),
        field(
            execution,
            "path",
            &format!("execution_contract.surfaces.{key}.path"),
            blockers,
        ),
        field(
            execution,
            "approval_path",
            &format!("execution_contract.surfaces.{key}.approval_path"),
            blockers,
        ),
        field(
            execution,
            "frontier_path",
            &format!("execution_contract.surfaces.{key}.frontier_path"),
            blockers,
        ),
        field(
            execution,
            "validation_path",
            &format!("execution_contract.surfaces.{key}.validation_path"),
            blockers,
        ),
        field(
            collaboration,
            "request",
            &format!("sdk_surfaces.{key}.request"),
            blockers,
        ),
        field(
            preflight,
            "inspect",
            &format!("preflight.surfaces.{key}.inspect"),
            blockers,
        ),
        field(
            preflight,
            "build_plan",
            &format!("preflight.surfaces.{key}.build_plan"),
            blockers,
        ),
        field(
            collaboration,
            "normalize",
            &format!("sdk_surfaces.{key}.normalize"),
            blockers,
        ),
        field(
            collaboration,
            "plan",
            &format!("sdk_surfaces.{key}.plan"),
            blockers,
        ),
        field(
            execution,
            "executor",
            &format!("execution_contract.surfaces.{key}.executor"),
            blockers,
        ),
        field(
            execution,
            "dispatcher",
            &format!("execution_contract.surfaces.{key}.dispatcher"),
            blockers,
        ),
        field(
            execution,
            "approval_verifier",
            &format!("execution_contract.surfaces.{key}.approval_verifier"),
            blockers,
        ),
        field(
            execution,
            "plan_digest",
            &format!("execution_contract.surfaces.{key}.plan_digest"),
            blockers,
        ),
        field(
            execution,
            "approval_request",
            &format!("execution_contract.surfaces.{key}.approval_request"),
            blockers,
        ),
        field(
            execution,
            "frontier_start",
            &format!("execution_contract.surfaces.{key}.frontier_start"),
            blockers,
        ),
        field(
            execution,
            "frontier_advance",
            &format!("execution_contract.surfaces.{key}.frontier_advance"),
            blockers,
        ),
        field(
            execution,
            "frontier_digest",
            &format!("execution_contract.surfaces.{key}.frontier_digest"),
            blockers,
        ),
        field(
            execution,
            "frontier_validator",
            &format!("execution_contract.surfaces.{key}.frontier_validator"),
            blockers,
        ),
        field(
            execution,
            "frontier_digest_verifier",
            &format!("execution_contract.surfaces.{key}.frontier_digest_verifier"),
            blockers,
        ),
        field(
            execution,
            "result_validator",
            &format!("execution_contract.surfaces.{key}.result_validator"),
            blockers,
        ),
        field(
            execution,
            "receipt_verifier",
            &format!("execution_contract.surfaces.{key}.receipt_verifier"),
            blockers,
        ),
        field(
            execution,
            "frontier_verifier",
            &format!("execution_contract.surfaces.{key}.frontier_verifier"),
            blockers,
        ),
    ];
    let values = fields.into_iter().collect::<Option<Vec<_>>>()?;
    for (index, path) in values.iter().take(6).enumerate() {
        add_path(
            path,
            &format!("selected_surface.path[{index}]"),
            resources,
            blockers,
        );
    }
    Some(ModelResearchSelectedSurface {
        collaboration_path: values[0].clone(),
        preflight_path: values[1].clone(),
        execution_path: values[2].clone(),
        approval_path: values[3].clone(),
        frontier_path: values[4].clone(),
        validation_path: values[5].clone(),
        request: values[6].clone(),
        inspect: values[7].clone(),
        bootstrap_plan: values[8].clone(),
        normalize: values[9].clone(),
        plan: values[10].clone(),
        executor: values[11].clone(),
        dispatcher: values[12].clone(),
        approval_verifier: values[13].clone(),
        plan_digest: values[14].clone(),
        approval_request: values[15].clone(),
        frontier_start: values[16].clone(),
        frontier_advance: values[17].clone(),
        frontier_digest: values[18].clone(),
        frontier_validator: values[19].clone(),
        frontier_digest_verifier: values[20].clone(),
        result_validator: values[21].clone(),
        receipt_verifier: values[22].clone(),
        frontier_verifier: values[23].clone(),
    })
}

fn add_document_paths(
    root: &Map<String, Value>,
    resources: &mut BTreeSet<String>,
    blockers: &mut Vec<String>,
) {
    let Some(documents) = root.get("required_documents").and_then(Value::as_array) else {
        blockers.push("required_documents must be an array".into());
        return;
    };
    if documents.len() < 4 {
        blockers.push("required_documents must contain at least 4 entries".into());
    }
    for (index, document) in documents.iter().enumerate() {
        let path = document.get("path").and_then(Value::as_str).unwrap_or("");
        add_path(
            path,
            &format!("required_documents[{index}].path"),
            resources,
            blockers,
        );
    }
}

fn add_execution_resources(
    root: &Map<String, Value>,
    resources: &mut BTreeSet<String>,
    blockers: &mut Vec<String>,
) {
    let Some(execution) = root.get("execution_contract").and_then(Value::as_object) else {
        blockers.push("execution_contract must be a JSON object".into());
        return;
    };
    for key in [
        "approval_request_schema",
        "approval_request_fixture",
        "approval_schema",
        "approval_fixture",
        "receipt_schema",
        "frontier_schema",
        "frontier_fixture",
        "validation_report_schema",
        "validation_report_fixture",
    ] {
        let path = execution.get(key).and_then(Value::as_str).unwrap_or("");
        add_path(
            path,
            &format!("execution_contract.{key}"),
            resources,
            blockers,
        );
    }
}

fn add_first_research_resources(
    first: Option<&Map<String, Value>>,
    resources: &mut BTreeSet<String>,
    blockers: &mut Vec<String>,
) {
    let Some(first) = first else { return };
    for key in [
        "session_fixture",
        "proposal_fixture",
        "catalog_request_fixture",
    ] {
        let path = first.get(key).and_then(Value::as_str).unwrap_or("");
        add_path(path, &format!("first_research.{key}"), resources, blockers);
    }
}

fn add_preflight_resources(
    root: &Map<String, Value>,
    resources: &mut BTreeSet<String>,
    blockers: &mut Vec<String>,
) {
    let Some(preflight) = root.get("preflight").and_then(Value::as_object) else {
        blockers.push("preflight must be a JSON object".into());
        return;
    };
    if preflight.get("execution_authority").and_then(Value::as_str) != Some("none_preflight_only") {
        blockers.push("preflight.execution_authority must be none_preflight_only".into());
    }
    for key in ["report_schema", "report_fixture"] {
        let path = preflight.get(key).and_then(Value::as_str).unwrap_or("");
        add_path(path, &format!("preflight.{key}"), resources, blockers);
    }
}

fn add_path(path: &str, label: &str, resources: &mut BTreeSet<String>, blockers: &mut Vec<String>) {
    if safe_repo_path(path) {
        resources.insert(path.to_string());
    } else {
        blockers.push(format!("{label} must be a safe project-relative path"));
    }
}

fn safe_repo_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.split('/').any(|part| part == ".." || part.is_empty())
}

fn object<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    blockers: &mut Vec<String>,
) -> Option<&'a Map<String, Value>> {
    let value = root.get(key).and_then(Value::as_object);
    if value.is_none() {
        blockers.push(format!("{key} must be a JSON object"));
    }
    value
}

fn field(
    root: &Map<String, Value>,
    key: &str,
    label: &str,
    blockers: &mut Vec<String>,
) -> Option<String> {
    let value = text(root, key);
    if value.is_none() {
        blockers.push(format!("{label} must be a non-empty string"));
    }
    value
}

fn text(root: &Map<String, Value>, key: &str) -> Option<String> {
    root.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn validate_completion_contract(
    root: &Map<String, Value>,
    blockers: &mut Vec<String>,
) -> Option<Value> {
    let Some(completion) = root.get("completion_contract").and_then(Value::as_object) else {
        blockers.push("completion_contract must be a JSON object".into());
        return None;
    };
    for (key, minimum) in [
        ("required_artifacts", 3),
        ("required_claims", 3),
        ("forbidden_claims", 2),
    ] {
        if string_list(completion.get(key)).len() < minimum {
            blockers.push(format!(
                "completion_contract.{key} must contain at least {minimum} entries"
            ));
        }
    }
    Some(Value::Object(completion.clone()))
}

fn require_exact(root: &Map<String, Value>, key: &str, expected: &str, blockers: &mut Vec<String>) {
    if root.get(key).and_then(Value::as_str) != Some(expected) {
        blockers.push(format!("{key} must be {expected}"));
    }
}

fn empty_report(sdk: ModelResearchSdk, blocker: &str) -> ModelResearchReadinessReport {
    ModelResearchReadinessReport {
        schema_version: MODEL_RESEARCH_READINESS_REPORT_SCHEMA_VERSION.into(),
        selected_sdk: sdk,
        ready_for_planning: false,
        execution_authority: "none_preflight_only".into(),
        version_line: "unknown".into(),
        entrypoint: "unknown".into(),
        workflow_id: "unknown".into(),
        selected_surface: None,
        required_resources: Vec::new(),
        missing_resources: Vec::new(),
        blockers: vec![blocker.into()],
        hard_rules: Vec::new(),
        stop_conditions: Vec::new(),
        completion_contract: None,
    }
}
