use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/operator-task-ir.schema.json";
const GOLDEN_MANIFEST_PATH: &str = "schemas/operator-task-ir-golden-manifest.json";
const GOLDEN_MANIFEST_SCHEMA: &str = "kyuubiki.operator-task-ir-golden-manifest/v1";
const EXAMPLE_PATHS: &[&str] = &[
    "schemas/examples.operator-task-ir.json",
    "schemas/examples.operator-task-ir-float.json",
    "schemas/examples.operator-task-ir-elixir.json",
    "schemas/examples.operator-task-batch.json",
];
const REQUIRED_AUTHORING_MODES: &[&str] = &["rust_native", "elixir_control_plane"];
const REQUIRED_DIGEST_FIELDS: &[&str] = &[
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

pub(crate) fn run_check_operator_task_ir_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("operator task IR contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-operator-task-ir-contract only accepts --self-test".to_string());
    }
    match check_contracts(root)? {
        CheckOutcome::Ok(task_count) => {
            println!("Validated {task_count} operator task IR example contracts.");
            Ok(0)
        }
        CheckOutcome::Issue(issue) => {
            eprintln!("operator task IR contract check failed: {issue}");
            Ok(1)
        }
    }
}

enum CheckOutcome {
    Ok(usize),
    Issue(String),
}

fn check_contracts(root: &Path) -> RunnerResult<CheckOutcome> {
    let schema = read_json(root, SCHEMA_PATH)?;
    let constraints = match parse_constraints(&schema) {
        Ok(constraints) => constraints,
        Err(issue) => return Ok(CheckOutcome::Issue(issue)),
    };
    let mut task_count = 0usize;
    let mut authoring_modes = Vec::new();
    let mut tasks_by_path = Map::new();
    for example_path in EXAMPLE_PATHS {
        let example = read_json(root, example_path)?;
        let mut tasks = Vec::new();
        collect_tasks(&example, &mut tasks);
        if tasks.is_empty() {
            return Ok(CheckOutcome::Issue(format!(
                "{example_path}: no TaskIR examples found"
            )));
        }
        for (index, task) in tasks.iter().enumerate() {
            let context = format!("{example_path}#task-{}", index + 1);
            for validator in [
                validate_mirror_constraints,
                validate_digest_field_coverage,
                validate_descriptor_digest,
                validate_task_digest,
            ] {
                if let Err(issue) = validator(task, &constraints, &context) {
                    return Ok(CheckOutcome::Issue(issue));
                }
            }
            if let Some(mode) = task
                .pointer("/descriptor_authoring/mode")
                .and_then(Value::as_str)
            {
                if !authoring_modes.iter().any(|found| found == mode) {
                    authoring_modes.push(mode.to_string());
                }
            }
            task_count += 1;
        }
        tasks_by_path.insert(
            (*example_path).to_string(),
            Value::Array(tasks.into_iter().cloned().collect()),
        );
    }
    for mode in REQUIRED_AUTHORING_MODES {
        if !authoring_modes.iter().any(|found| found == mode) {
            return Ok(CheckOutcome::Issue(format!(
                "TaskIR examples must include descriptor_authoring.mode={mode}"
            )));
        }
    }
    if let Err(issue) = validate_golden_manifest(root, &tasks_by_path) {
        return Ok(CheckOutcome::Issue(issue));
    }
    Ok(CheckOutcome::Ok(task_count))
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let task = serde_json::json!({
        "operator": { "kind": "transform" },
        "execution_program": { "program_kind": "solver" }
    });
    let constraint = MirrorConstraint {
        source: "/operator/kind".to_string(),
        mirror: "/execution_program/program_kind".to_string(),
        reason: "self-test".to_string(),
    };
    if validate_mirror_constraints(&task, &[constraint], "self").is_ok() {
        return Err("self-test did not reject a mirror mismatch".to_string());
    }
    run_digest_self_test(root)?;
    run_descriptor_digest_self_test(root)
}

fn run_digest_self_test(root: &Path) -> RunnerResult<()> {
    let mut task = read_json(root, EXAMPLE_PATHS[0])?;
    if let Some(integrity) = task.get_mut("integrity").and_then(Value::as_object_mut) {
        integrity.insert("task_digest".to_string(), Value::from("0".repeat(64)));
    }
    if validate_task_digest(&task, &[], "self-digest").is_ok() {
        return Err("self-test did not reject a digest mismatch".to_string());
    }
    Ok(())
}

fn run_descriptor_digest_self_test(root: &Path) -> RunnerResult<()> {
    let mut task = read_json(root, EXAMPLE_PATHS[0])?;
    if let Some(integrity) = task.get_mut("integrity").and_then(Value::as_object_mut) {
        integrity.insert("descriptor_digest".to_string(), Value::from("0".repeat(64)));
    }
    if validate_descriptor_digest(&task, &[], "self-descriptor-digest").is_ok() {
        return Err("self-test did not reject a descriptor digest mismatch".to_string());
    }
    Ok(())
}

#[derive(Clone)]
struct MirrorConstraint {
    source: String,
    mirror: String,
    reason: String,
}

fn parse_constraints(schema: &Value) -> Result<Vec<MirrorConstraint>, String> {
    let Some(constraints) = schema
        .get("x-kyuubiki-mirror_constraints")
        .and_then(Value::as_array)
    else {
        return Err(format!(
            "{SCHEMA_PATH}: missing x-kyuubiki-mirror_constraints"
        ));
    };
    if constraints.is_empty() {
        return Err(format!(
            "{SCHEMA_PATH}: missing x-kyuubiki-mirror_constraints"
        ));
    }
    let mut parsed = Vec::new();
    for (index, constraint) in constraints.iter().enumerate() {
        let read_field = |field: &str| -> Result<String, String> {
            let Some(value) = constraint.get(field).and_then(Value::as_str) else {
                return Err(format!(
                    "{SCHEMA_PATH}: mirror constraint {index} missing {field}"
                ));
            };
            if value.is_empty() {
                return Err(format!(
                    "{SCHEMA_PATH}: mirror constraint {index} missing {field}"
                ));
            }
            Ok(value.to_string())
        };
        parsed.push(MirrorConstraint {
            source: read_field("source")?,
            mirror: read_field("mirror")?,
            reason: read_field("reason")?,
        });
    }
    Ok(parsed)
}

fn collect_tasks<'a>(value: &'a Value, tasks: &mut Vec<&'a Value>) {
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_some_and(|version| version == "kyuubiki.operator-task-ir/v1")
    {
        tasks.push(value);
    }
    match value {
        Value::Array(items) => {
            for item in items {
                collect_tasks(item, tasks);
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                collect_tasks(child, tasks);
            }
        }
        _ => {}
    }
}

fn validate_mirror_constraints(
    task: &Value,
    constraints: &[MirrorConstraint],
    context: &str,
) -> Result<(), String> {
    for constraint in constraints {
        let source = pointer_get(task, &constraint.source);
        let mirror = pointer_get(task, &constraint.mirror);
        if source.is_none() || mirror.is_none() {
            continue;
        }
        if source != mirror {
            return Err(format!(
                "{context}: {} must mirror {} ({})",
                constraint.mirror, constraint.source, constraint.reason
            ));
        }
    }
    Ok(())
}

fn validate_digest_field_coverage(
    task: &Value,
    _constraints: &[MirrorConstraint],
    context: &str,
) -> Result<(), String> {
    let Some(fields) = task
        .pointer("/integrity/task_digest_fields")
        .and_then(Value::as_array)
    else {
        return Err(format!(
            "{context}: integrity.task_digest_fields must be an array"
        ));
    };
    let actual = fields.iter().filter_map(Value::as_str).collect::<Vec<_>>();
    if actual.join("\n") != REQUIRED_DIGEST_FIELDS.join("\n") {
        return Err(format!(
            "{context}: integrity.task_digest_fields must match the canonical field order"
        ));
    }
    Ok(())
}

fn validate_descriptor_digest(
    task: &Value,
    _constraints: &[MirrorConstraint],
    context: &str,
) -> Result<(), String> {
    let expected = task
        .pointer("/integrity/descriptor_digest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if expected.is_empty() {
        return Err(format!(
            "{context}: integrity.descriptor_digest must be a non-empty string"
        ));
    }
    let actual = compute_descriptor_digest(task)?;
    if expected != actual {
        return Err(format!(
            "{context}: integrity.descriptor_digest mismatch; expected {expected}, computed {actual}"
        ));
    }
    Ok(())
}

fn validate_task_digest(
    task: &Value,
    _constraints: &[MirrorConstraint],
    context: &str,
) -> Result<(), String> {
    let expected = task
        .pointer("/integrity/task_digest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if expected.is_empty() {
        return Err(format!(
            "{context}: integrity.task_digest must be a non-empty string"
        ));
    }
    let actual = compute_task_digest(task);
    if expected != actual {
        return Err(format!(
            "{context}: integrity.task_digest mismatch; expected {expected}, computed {actual}"
        ));
    }
    Ok(())
}

fn validate_golden_manifest(root: &Path, tasks_by_path: &Map<String, Value>) -> Result<(), String> {
    let manifest = read_json(root, GOLDEN_MANIFEST_PATH)?;
    if manifest
        .get("schema_version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != GOLDEN_MANIFEST_SCHEMA
    {
        return Err(format!(
            "{GOLDEN_MANIFEST_PATH}: schema_version must be {GOLDEN_MANIFEST_SCHEMA}"
        ));
    }
    if manifest
        .get("line")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "moxi 2.x"
    {
        return Err(format!("{GOLDEN_MANIFEST_PATH}: line must be moxi 2.x"));
    }
    let Some(examples) = manifest.get("examples").and_then(Value::as_array) else {
        return Err(format!(
            "{GOLDEN_MANIFEST_PATH}: examples must be a non-empty array"
        ));
    };
    if examples.is_empty() {
        return Err(format!(
            "{GOLDEN_MANIFEST_PATH}: examples must be a non-empty array"
        ));
    }
    let mut manifest_paths = Vec::new();
    for (index, entry) in examples.iter().enumerate() {
        let context = format!("{GOLDEN_MANIFEST_PATH}:examples[{index}]");
        let example_path = entry
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if example_path.is_empty() {
            return Err(format!("{context}: path must be a non-empty string"));
        }
        manifest_paths.push(example_path.to_string());
        let Some(tasks) = tasks_by_path.get(example_path).and_then(Value::as_array) else {
            return Err(format!(
                "{context}: no collected TaskIR examples for {example_path}"
            ));
        };
        validate_manifest_values(&context, tasks, "/task_id", entry, "task_ids")?;
        validate_manifest_values(
            &context,
            tasks,
            "/descriptor_authoring/mode",
            entry,
            "descriptor_authoring_modes",
        )?;
        validate_manifest_values(&context, tasks, "/operator/kind", entry, "operator_kinds")?;
        validate_manifest_values(
            &context,
            tasks,
            "/execution_program/program_kind",
            entry,
            "program_kinds",
        )?;
        validate_manifest_values(
            &context,
            tasks,
            "/runtime_hints/execution_mode",
            entry,
            "execution_modes",
        )?;
    }
    for example_path in EXAMPLE_PATHS {
        if !manifest_paths.iter().any(|path| path == example_path) {
            return Err(format!(
                "{GOLDEN_MANIFEST_PATH}: missing manifest entry for {example_path}"
            ));
        }
    }
    Ok(())
}

fn validate_manifest_values(
    context: &str,
    tasks: &[Value],
    pointer: &str,
    entry: &Value,
    field: &str,
) -> Result<(), String> {
    let Some(expected) = entry.get(field).and_then(Value::as_array) else {
        return Err(format!(
            "{context}: manifest field {field} must be a non-empty array"
        ));
    };
    if expected.is_empty() {
        return Err(format!(
            "{context}: manifest field {field} must be a non-empty array"
        ));
    }
    let actual = tasks
        .iter()
        .filter_map(|task| pointer_get(task, pointer).and_then(Value::as_str))
        .collect::<Vec<_>>();
    let missing = expected
        .iter()
        .filter_map(Value::as_str)
        .filter(|expected_value| {
            !actual
                .iter()
                .any(|actual_value| actual_value == expected_value)
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "{context}: missing manifest {field}: {}",
            missing.join(", ")
        ));
    }
    Ok(())
}

fn compute_task_digest(task: &Value) -> String {
    let mut payload = Map::new();
    for field in REQUIRED_DIGEST_FIELDS {
        if let Some(value) = task.get(*field) {
            payload.insert(field.to_string(), value.clone());
        }
    }
    sha256_canonical(&Value::Object(payload))
}

fn compute_descriptor_digest(task: &Value) -> Result<String, String> {
    let Some(operator) = task.get("operator") else {
        return Err("operator task descriptor digest requires an operator object".to_string());
    };
    if !operator.is_object() || operator.is_array() {
        return Err("operator task descriptor digest requires an operator object".to_string());
    }
    Ok(sha256_canonical(operator))
}

fn sha256_canonical(value: &Value) -> String {
    let canonical = canonical_json(value);
    let digest = Sha256::digest(canonical.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let body = items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{body}]")
        }
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let body = keys
                .into_iter()
                .map(|key| {
                    let encoded_key = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".into());
                    format!("{encoded_key}:{}", canonical_json(&map[key]))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
        Value::Number(number) => canonical_number(number),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

fn canonical_number(number: &serde_json::Number) -> String {
    if let Some(value) = number.as_i64() {
        return value.to_string();
    }
    if let Some(value) = number.as_u64() {
        return value.to_string();
    }
    let value = number.as_f64().unwrap_or(0.0);
    if !value.is_finite() {
        return "null".to_string();
    }
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }
    let mut encoded = format!("{value:.15}");
    while encoded.ends_with('0') {
        encoded.pop();
    }
    if encoded.ends_with('.') {
        encoded.push('0');
    }
    encoded
}

fn pointer_get<'a>(value: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(value);
    }
    pointer
        .split('/')
        .skip(1)
        .try_fold(value, |current, segment| {
            let key = segment.replace("~1", "/").replace("~0", "~");
            current.get(key)
        })
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{canonical_json, pointer_get, sha256_canonical};
    use serde_json::json;

    #[test]
    fn pointer_get_unescapes_tokens() {
        let value = json!({ "a/b": { "~": 3 } });
        assert_eq!(pointer_get(&value, "/a~1b/~"), Some(&json!(3)));
    }

    #[test]
    fn canonical_json_sorts_object_keys_and_formats_fractional_numbers() {
        let value = json!({ "b": 2, "a": 1.25 });
        assert_eq!(canonical_json(&value), "{\"a\":1.25,\"b\":2}");
    }

    #[test]
    fn canonical_digest_is_stable() {
        assert_eq!(
            sha256_canonical(&json!({"b":2,"a":1})),
            "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
        );
    }
}
