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
    let schema_version = required_string(task, &["schema_version"])?;
    if schema_version != OPERATOR_TASK_IR_SCHEMA {
        return Err(format!(
            "operator task schema_version must be {OPERATOR_TASK_IR_SCHEMA}"
        ));
    }

    let task_digest = compute_operator_task_digest(task)?;
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

    if program_id != operator_id || program_kind != operator_kind {
        return Err("operator task execution program does not match operator".to_string());
    }

    validate_execution_abi(
        &program_kind,
        &runtime_protocol,
        &abi_kind,
        &entrypoint_kind,
    )?;

    if program_kind != "solver" && entrypoint_name != operator_id {
        return Err("operator task entrypoint does not match operator id".to_string());
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
        package_ref: optional_string(task, &["execution_program", "package_ref"]),
        package_version: optional_string(task, &["execution_program", "package_version"]),
        authority_mode: optional_string(task, &["runtime_hints", "authority_mode"]),
        execution_mode: optional_string(task, &["runtime_hints", "execution_mode"]),
        cache_scope: optional_string(task, &["runtime_hints", "cache_scope"]),
        agent_fetchable: optional_bool(task, &["runtime_hints", "agent_fetchable"]),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorTaskDigestError {
    Missing,
    InvalidTask(String),
    Mismatch { expected: String, actual: String },
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
) -> Result<(), String> {
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
        return Err(format!(
            "{} has inconsistent runtime protocol, abi, or entrypoint",
            expected.3
        ));
    }

    Ok(())
}

fn required_string<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, String> {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("operator task missing {}", path.join(".")))
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
