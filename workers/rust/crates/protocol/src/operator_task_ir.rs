use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const OPERATOR_TASK_IR_SCHEMA: &str = "kyuubiki.operator-task-ir/v1";

pub const OPERATOR_TASK_DIGEST_FIELDS: &[&str] = &[
    "schema_version",
    "task_id",
    "operator",
    "descriptor_authoring",
    "node",
    "input_artifact",
    "config",
    "execution_program",
    "dataset_contract",
    "orchestration_context",
    "runtime_hints",
];

pub fn operator_task_digest_fields() -> &'static [&'static str] {
    OPERATOR_TASK_DIGEST_FIELDS
}

pub fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => canonical_object_json(object),
        Value::Array(values) => {
            let parts = values.iter().map(canonical_json).collect::<Vec<_>>();
            format!("[{}]", parts.join(","))
        }
        Value::Number(number) => canonical_number_json(number),
        _ => serde_json::to_string(value).expect("json scalar should encode"),
    }
}

pub fn compute_operator_task_digest(task: &Value) -> Result<String, String> {
    let object = task
        .as_object()
        .ok_or_else(|| "operator task ir must be a json object".to_string())?;

    let mut digest_payload = Map::new();
    for field in OPERATOR_TASK_DIGEST_FIELDS {
        if let Some(value) = object.get(*field) {
            digest_payload.insert((*field).to_string(), value.clone());
        }
    }

    Ok(sha256_hex(
        canonical_json(&Value::Object(digest_payload)).as_bytes(),
    ))
}

pub fn verify_operator_task_digest(task: &Value) -> Result<(), OperatorTaskDigestError> {
    let expected = task
        .get("integrity")
        .and_then(Value::as_object)
        .and_then(|integrity| integrity.get("task_digest"))
        .and_then(Value::as_str)
        .ok_or(OperatorTaskDigestError::Missing)?;

    let actual =
        compute_operator_task_digest(task).map_err(OperatorTaskDigestError::InvalidTask)?;

    if expected == actual {
        Ok(())
    } else {
        Err(OperatorTaskDigestError::Mismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

pub fn summarize_operator_task_execution(
    task: &Value,
) -> Result<OperatorTaskExecutionSummary, String> {
    summarize_operator_task_execution_checked(task).map_err(|error| error.message)
}

pub fn summarize_operator_task_execution_checked(
    task: &Value,
) -> Result<OperatorTaskExecutionSummary, OperatorTaskSummaryError> {
    let schema_version = required_string(task, &["schema_version"])?;
    if schema_version != OPERATOR_TASK_IR_SCHEMA {
        return Err(OperatorTaskSummaryError::invalid(format!(
            "operator task schema_version must be {OPERATOR_TASK_IR_SCHEMA}",
        )));
    }

    let task_digest =
        compute_operator_task_digest(task).map_err(OperatorTaskSummaryError::invalid)?;
    let operator_id = required_string(task, &["operator", "id"])?.to_string();
    let operator_kind = required_string(task, &["operator", "kind"])?.to_string();
    let program_id = required_string(task, &["execution_program", "program_id"])?.to_string();
    let program_kind = required_string(task, &["execution_program", "program_kind"])?.to_string();
    let runtime_protocol =
        required_string(task, &["execution_program", "runtime_protocol"])?.to_string();
    let abi_kind = required_string(task, &["execution_program", "abi", "kind"])?.to_string();
    let entrypoint_kind =
        required_string(task, &["execution_program", "entrypoint", "kind"])?.to_string();
    let entrypoint_name =
        required_string(task, &["execution_program", "entrypoint", "name"])?.to_string();
    let package_ref = optional_string(task, &["execution_program", "package_ref"]);
    let package_version = optional_string(task, &["execution_program", "package_version"]);
    let authority_mode = optional_string(task, &["runtime_hints", "authority_mode"]);
    let execution_mode = optional_string(task, &["runtime_hints", "execution_mode"]);
    let cache_scope = optional_string(task, &["runtime_hints", "cache_scope"]);
    let agent_fetchable = optional_bool(task, &["runtime_hints", "agent_fetchable"]);

    if program_id != operator_id || program_kind != operator_kind {
        return Err(OperatorTaskSummaryError::new(
            OperatorTaskSummaryErrorCode::ProgramMismatch,
            "operator task execution program does not match operator",
        ));
    }

    validate_mirror_field(
        "execution_program.entrypoint.operator_kind",
        optional_string(task, &["execution_program", "entrypoint", "operator_kind"]).as_deref(),
        "operator.kind",
        &operator_kind,
    )?;
    validate_mirror_field(
        "runtime_hints.operator_kind",
        optional_string(task, &["runtime_hints", "operator_kind"]).as_deref(),
        "operator.kind",
        &operator_kind,
    )?;
    validate_optional_mirror_field(
        "operator.execution.package_ref",
        optional_string(task, &["operator", "execution", "package_ref"]).as_deref(),
        "execution_program.package_ref",
        package_ref.as_deref(),
    )?;
    validate_optional_mirror_field(
        "runtime_hints.package_ref",
        optional_string(task, &["runtime_hints", "package_ref"]).as_deref(),
        "execution_program.package_ref",
        package_ref.as_deref(),
    )?;
    validate_optional_mirror_field(
        "runtime_hints.package_version",
        optional_string(task, &["runtime_hints", "package_version"]).as_deref(),
        "execution_program.package_version",
        package_version.as_deref(),
    )?;

    validate_execution_abi(
        &program_kind,
        &runtime_protocol,
        &abi_kind,
        &entrypoint_kind,
    )?;

    if program_kind != "solver" && entrypoint_name != operator_id {
        return Err(OperatorTaskSummaryError::new(
            OperatorTaskSummaryErrorCode::EntrypointMismatch,
            "operator task entrypoint does not match operator id",
        ));
    }

    Ok(OperatorTaskExecutionSummary {
        task_digest,
        task_id: required_string(task, &["task_id"])?.to_string(),
        operator_id,
        operator_kind,
        program_id,
        program_kind,
        runtime_protocol,
        abi_kind,
        entrypoint_kind,
        entrypoint_name,
        package_ref,
        package_version,
        authority_mode,
        execution_mode,
        cache_scope,
        agent_fetchable,
    })
}

pub fn preview_operator_task_execution(
    task: &Value,
) -> Result<OperatorTaskExecutionPreview, OperatorTaskSummaryError> {
    let summary = summarize_operator_task_execution_checked(task)?;
    let package_fetch_required = task_requires_package_fetch(&summary);
    let package_readiness_gate = package_readiness_gate(&summary, package_fetch_required);
    let result_serialization =
        optional_string(task, &["execution_program", "abi", "output_encoding"])
            .unwrap_or_else(|| "json".to_string());
    let dispatch_route = dispatch_route(&summary, package_fetch_required);
    let offline_runnable = task_is_offline_runnable(&summary, package_fetch_required);
    let dispatch_warnings = dispatch_warnings(&summary, package_fetch_required, offline_runnable);

    Ok(OperatorTaskExecutionPreview {
        task_digest: summary.task_digest,
        task_id: summary.task_id,
        operator_id: summary.operator_id,
        operator_kind: summary.operator_kind,
        dispatch_route,
        package_ref: summary.package_ref,
        package_version: summary.package_version,
        package_fetch_required,
        package_readiness_gate,
        result_serialization,
        authority_mode: summary.authority_mode,
        execution_mode: summary.execution_mode,
        cache_scope: summary.cache_scope,
        agent_fetchable: summary.agent_fetchable,
        offline_runnable,
        dispatch_warnings,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorTaskDigestError {
    Missing,
    InvalidTask(String),
    Mismatch { expected: String, actual: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorTaskSummaryErrorCode {
    Invalid,
    MissingField,
    MirrorMismatch,
    ExecutionAbiMismatch,
    ProgramMismatch,
    EntrypointMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorTaskSummaryError {
    pub code: OperatorTaskSummaryErrorCode,
    pub message: String,
}

impl OperatorTaskSummaryError {
    fn new(code: OperatorTaskSummaryErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new(OperatorTaskSummaryErrorCode::Invalid, message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorTaskExecutionSummary {
    pub task_digest: String,
    pub task_id: String,
    pub operator_id: String,
    pub operator_kind: String,
    pub program_id: String,
    pub program_kind: String,
    pub runtime_protocol: String,
    pub abi_kind: String,
    pub entrypoint_kind: String,
    pub entrypoint_name: String,
    pub package_ref: Option<String>,
    pub package_version: Option<String>,
    pub authority_mode: Option<String>,
    pub execution_mode: Option<String>,
    pub cache_scope: Option<String>,
    pub agent_fetchable: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorTaskExecutionPreview {
    pub task_digest: String,
    pub task_id: String,
    pub operator_id: String,
    pub operator_kind: String,
    pub dispatch_route: String,
    pub package_ref: Option<String>,
    pub package_version: Option<String>,
    pub package_fetch_required: bool,
    pub package_readiness_gate: String,
    pub result_serialization: String,
    pub authority_mode: Option<String>,
    pub execution_mode: Option<String>,
    pub cache_scope: Option<String>,
    pub agent_fetchable: Option<bool>,
    pub offline_runnable: bool,
    pub dispatch_warnings: Vec<String>,
}

fn canonical_object_json(object: &Map<String, Value>) -> String {
    let mut keys = object.keys().collect::<Vec<_>>();
    keys.sort();

    let parts = keys
        .into_iter()
        .map(|key| {
            let encoded_key = serde_json::to_string(key).expect("json object key should encode");
            let encoded_value = canonical_json(&object[key]);
            format!("{encoded_key}:{encoded_value}")
        })
        .collect::<Vec<_>>();

    format!("{{{}}}", parts.join(","))
}

fn canonical_number_json(number: &serde_json::Number) -> String {
    if let Some(value) = number.as_i64() {
        return value.to_string();
    }
    if let Some(value) = number.as_u64() {
        return value.to_string();
    }
    let value = number.as_f64().expect("json number should be finite");
    let mut encoded = format!("{value:.15}");
    while encoded.ends_with('0') {
        encoded.pop();
    }
    if encoded.ends_with('.') {
        encoded.push('0');
    }
    encoded
}

fn validate_execution_abi(
    program_kind: &str,
    runtime_protocol: &str,
    abi_kind: &str,
    entrypoint_kind: &str,
) -> Result<(), OperatorTaskSummaryError> {
    let expected = if program_kind == "solver" {
        (
            "kyuubiki.solver-rpc/v1",
            "solver_rpc",
            "solver_method",
            "solver execution program",
        )
    } else {
        (
            "kyuubiki.operator-execution/v1",
            "operator_task",
            "operator_id",
            "operator execution program",
        )
    };

    if runtime_protocol != expected.0 || abi_kind != expected.1 || entrypoint_kind != expected.2 {
        return Err(OperatorTaskSummaryError::new(
            OperatorTaskSummaryErrorCode::ExecutionAbiMismatch,
            format!(
                "{} has inconsistent runtime protocol, abi, or entrypoint",
                expected.3
            ),
        ));
    }

    Ok(())
}

fn task_requires_package_fetch(summary: &OperatorTaskExecutionSummary) -> bool {
    summary.execution_mode.as_deref() == Some("orchestra_fetch")
        || summary.authority_mode.as_deref() == Some("central_operator_library")
        || summary.agent_fetchable == Some(true)
        || summary
            .package_ref
            .as_deref()
            .is_some_and(|package_ref| package_ref.starts_with("orchestra://"))
}

fn package_readiness_gate(
    summary: &OperatorTaskExecutionSummary,
    package_fetch_required: bool,
) -> String {
    if package_fetch_required {
        return "central_package_readiness".to_string();
    }
    if summary.package_ref.is_some() {
        return "local_package_readiness".to_string();
    }
    "built_in_operator_descriptor".to_string()
}

fn dispatch_route(summary: &OperatorTaskExecutionSummary, package_fetch_required: bool) -> String {
    if summary.operator_kind == "solver" {
        return "solver_rpc".to_string();
    }
    if package_fetch_required {
        return "fetch_package_then_operator_task".to_string();
    }
    "local_operator_task".to_string()
}

fn task_is_offline_runnable(
    summary: &OperatorTaskExecutionSummary,
    package_fetch_required: bool,
) -> bool {
    !package_fetch_required
        && !matches!(
            summary.authority_mode.as_deref(),
            Some("central_operator_library")
        )
        && !matches!(summary.execution_mode.as_deref(), Some("orchestra_fetch"))
}

fn dispatch_warnings(
    summary: &OperatorTaskExecutionSummary,
    package_fetch_required: bool,
    offline_runnable: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if package_fetch_required && summary.package_ref.is_none() {
        warnings.push(
            "package fetch is required but execution_program.package_ref is missing".to_string(),
        );
    }
    if !offline_runnable && summary.cache_scope.is_none() {
        warnings.push(
            "remote or centralized execution should declare runtime_hints.cache_scope".to_string(),
        );
    }
    warnings
}

fn validate_mirror_field(
    mirror_name: &str,
    mirror_value: Option<&str>,
    source_name: &str,
    source_value: &str,
) -> Result<(), OperatorTaskSummaryError> {
    if let Some(value) = mirror_value
        && value != source_value
    {
        return Err(OperatorTaskSummaryError::new(
            OperatorTaskSummaryErrorCode::MirrorMismatch,
            format!("operator task {mirror_name} must match {source_name}"),
        ));
    }

    Ok(())
}

fn validate_optional_mirror_field(
    mirror_name: &str,
    mirror_value: Option<&str>,
    source_name: &str,
    source_value: Option<&str>,
) -> Result<(), OperatorTaskSummaryError> {
    if let (Some(mirror), Some(source)) = (mirror_value, source_value)
        && mirror != source
    {
        return Err(OperatorTaskSummaryError::new(
            OperatorTaskSummaryErrorCode::MirrorMismatch,
            format!("operator task {mirror_name} must match {source_name}"),
        ));
    }

    Ok(())
}

fn required_string<'a>(
    value: &'a Value,
    path: &[&str],
) -> Result<&'a str, OperatorTaskSummaryError> {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            OperatorTaskSummaryError::new(
                OperatorTaskSummaryErrorCode::MissingField,
                format!("operator task missing {}", path.join(".")),
            )
        })
}

fn optional_string(value: &Value, path: &[&str]) -> Option<String> {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn optional_bool(value: &Value, path: &[&str]) -> Option<bool> {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_bool)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
