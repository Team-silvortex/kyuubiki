use crate::RunnerResult;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str = "tmp/operator-package-dynamic-smoke.json";
const SCHEMA_PATH: &str = "schemas/operator-package-dynamic-smoke.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.operator-package-dynamic-smoke.json";
const SCHEMAS_README_PATH: &str = "schemas/README.md";
const SCHEMA_VERSION: &str = "kyuubiki.operator-package-dynamic-smoke/v1";
const REQUIRED_STAGES: &[&str] = &[
    "template_tests",
    "strict_preflight",
    "template_cdylib_build",
    "engine_dynamic_host_load",
];

pub(crate) fn run_check_operator_package_dynamic_smoke(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(root, args)?;
    if options.self_test {
        run_self_test(root)?;
        println!("operator package dynamic smoke check self-test passed");
        return Ok(0);
    }
    let (absolute_input, relative_input) = repo_local_input(root, &options.input)?;
    if !absolute_input.exists() {
        eprintln!(
            "operator package dynamic smoke check failed: input does not exist: {relative_input}"
        );
        return Ok(1);
    }
    if let Some(issue) = check_schema_and_example(root)? {
        eprintln!("operator package dynamic smoke check failed: {issue}");
        return Ok(1);
    }
    let report = read_json_path(&absolute_input, &relative_input)?;
    let errors = dynamic_smoke_errors(root, &report, &relative_input);
    if let Some(issue) = errors.first() {
        eprintln!("operator package dynamic smoke check failed: {issue}");
        return Ok(1);
    }
    println!("operator package dynamic smoke check passed: {relative_input}");
    Ok(0)
}

pub(crate) fn run_check_operator_package_dynamic_smoke_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("operator package dynamic smoke check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err(
            "check-operator-package-dynamic-smoke-contract only accepts --self-test".to_string(),
        );
    }
    if let Some(issue) = check_schema_and_example(root)? {
        eprintln!("operator package dynamic smoke check failed: {issue}");
        return Ok(1);
    }
    println!("operator package dynamic smoke check passed: {EXAMPLE_PATH}");
    Ok(0)
}

#[derive(Debug, Clone)]
struct CheckOptions {
    input: PathBuf,
    self_test: bool,
}

fn parse_args(root: &Path, args: Vec<OsString>) -> RunnerResult<CheckOptions> {
    let mut input = PathBuf::from(DEFAULT_INPUT);
    let mut self_test = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--self-test" => self_test = true,
            "--in" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --in".to_string());
                };
                input = PathBuf::from(value);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let input = if input.is_absolute() {
        input
    } else {
        root.join(input)
    };
    Ok(CheckOptions { input, self_test })
}

fn repo_local_input(root: &Path, input: &Path) -> RunnerResult<(PathBuf, String)> {
    let absolute = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| "--in must stay inside the repository".to_string())?
        .to_string_lossy()
        .to_string();
    if relative.starts_with("..") || Path::new(&relative).is_absolute() {
        return Err("--in must stay inside the repository".to_string());
    }
    Ok((absolute, relative))
}

fn check_schema_and_example(root: &Path) -> RunnerResult<Option<String>> {
    let schema = read_repo_json(root, SCHEMA_PATH)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(SCHEMA_VERSION)
    {
        return Ok(Some(format!(
            "{SCHEMA_PATH}: schema_version const must match {SCHEMA_VERSION}"
        )));
    }
    let stage_ids = schema
        .pointer("/properties/stages/prefixItems")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let def_name = item.get("$ref")?.as_str()?.strip_prefix("#/$defs/")?;
                    schema
                        .pointer(&format!("/$defs/{def_name}/allOf/1/properties/id/const"))
                        .and_then(Value::as_str)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if stage_ids.join("\n") != REQUIRED_STAGES.join("\n") {
        return Ok(Some(format!(
            "{SCHEMA_PATH}: stage prefixItems must match canonical stage order"
        )));
    }
    for required_property in ["description", "cwd", "command"] {
        let required = schema
            .pointer("/$defs/passingStage/required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        if !required.contains(&required_property) {
            return Ok(Some(format!(
                "{SCHEMA_PATH}: passingStage must require {required_property}"
            )));
        }
    }
    let example = read_repo_json(root, EXAMPLE_PATH)?;
    if let Some(issue) = dynamic_smoke_errors(root, &example, EXAMPLE_PATH)
        .into_iter()
        .next()
    {
        return Ok(Some(issue));
    }
    let readme = read_repo_text(root, SCHEMAS_README_PATH)?;
    for expected in [
        "operator-package-dynamic-smoke.schema.json",
        "examples.operator-package-dynamic-smoke.json",
    ] {
        if !readme.contains(expected) {
            return Ok(Some(format!("{SCHEMAS_README_PATH}: missing {expected}")));
        }
    }
    Ok(None)
}

fn dynamic_smoke_errors(root: &Path, report: &Value, context: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if field(report, "schema_version") != SCHEMA_VERSION {
        errors.push(format!("{context}: unexpected schema_version"));
    }
    if report.get("ok").and_then(Value::as_bool) != Some(true) {
        errors.push(format!("{context}: ok must be true"));
    }
    for field_name in [
        "generated_at",
        "package_id",
        "host_version",
        "sdk_api_version",
    ] {
        require_string(
            report.get(field_name),
            &format!("{context}.{field_name}"),
            &mut errors,
        );
    }
    match report.get("operator_ids").and_then(Value::as_array) {
        Some(ids) if !ids.is_empty() => {
            for (index, operator_id) in ids.iter().enumerate() {
                require_string(
                    Some(operator_id),
                    &format!("{context}.operator_ids[{index}]"),
                    &mut errors,
                );
            }
        }
        _ => errors.push(format!("{context}.operator_ids must be a non-empty array")),
    }
    for field_name in [
        "template_manifest",
        "package_manifest",
        "preflight_report",
        "dynamic_library",
    ] {
        require_repo_path(
            root,
            report.get(field_name),
            &format!("{context}.{field_name}"),
            &mut errors,
        );
    }
    let Some(stages) = report.get("stages").and_then(Value::as_array) else {
        errors.push(format!("{context}.stages must be an array"));
        return errors;
    };
    let actual_stages = stages
        .iter()
        .map(|stage| field(stage, "id"))
        .collect::<Vec<_>>();
    if actual_stages.join("\n") != REQUIRED_STAGES.join("\n") {
        errors.push(format!(
            "{context}.stages must match the canonical stage order"
        ));
    }
    for (index, stage) in stages.iter().enumerate() {
        require_string(
            stage.get("description"),
            &format!("{context}.stages[{index}].description"),
            &mut errors,
        );
        require_repo_path(
            root,
            stage.get("cwd"),
            &format!("{context}.stages[{index}].cwd"),
            &mut errors,
        );
        require_portable_command(
            root,
            stage.get("command"),
            &format!("{context}.stages[{index}].command"),
            &mut errors,
        );
        if stage.get("ok").and_then(Value::as_bool) != Some(true)
            || stage.get("status").and_then(Value::as_i64) != Some(0)
        {
            errors.push(format!("{context}.stages[{index}] must pass with status 0"));
        }
    }
    errors
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let stage_fixture = |id: &str| {
        json!({
            "id": id,
            "description": format!("{id} diagnostic stage"),
            "cwd": ".",
            "command": ["echo", id],
            "status": 0,
            "ok": true
        })
    };
    let sample = json!({
        "schema_version": SCHEMA_VERSION,
        "generated_at": "2026-07-12T00:00:00Z",
        "ok": true,
        "package_id": "operator.template.summary",
        "operator_ids": ["extract.template_summary"],
        "host_version": "1.20.0",
        "sdk_api_version": "kyuubiki.operator-sdk/v1",
        "template_manifest": "workers/rust/templates/operator-crate-template/Cargo.toml",
        "package_manifest": "workers/rust/templates/operator-crate-template/kyuubiki-operator.json",
        "preflight_report": "tmp/operator-package-dynamic-preflight.json",
        "dynamic_library": "workers/rust/templates/operator-crate-template/target/debug/libkyuubiki_operator_template.dylib",
        "stages": REQUIRED_STAGES.iter().map(|id| stage_fixture(id)).collect::<Vec<_>>()
    });
    if let Some(issue) = dynamic_smoke_errors(root, &sample, "self-test").first() {
        return Err(format!("self-test fixture unexpectedly failed: {issue}"));
    }
    if let Some(issue) = check_schema_and_example(root)? {
        return Err(issue);
    }
    expect_error(root, mutate_reversed_stages(&sample), "stage order")?;
    expect_error(root, mutate_failed_stage(&sample), "status 0")?;
    expect_error(root, mutate_missing_command(&sample), "command")?;
    expect_error(
        root,
        mutate_absolute_command(root, &sample),
        "absolute paths",
    )?;
    expect_error(
        root,
        mutate_absolute_cwd(root, &sample),
        "repo-relative path",
    )
}

fn expect_error(root: &Path, sample: Value, expected: &str) -> RunnerResult<()> {
    if !dynamic_smoke_errors(root, &sample, "self-test")
        .iter()
        .any(|error| error.contains(expected))
    {
        return Err(format!("self-test expected {expected} to fail"));
    }
    Ok(())
}

fn mutate_reversed_stages(sample: &Value) -> Value {
    let mut broken = sample.clone();
    if let Some(stages) = broken.get_mut("stages").and_then(Value::as_array_mut) {
        stages.reverse();
    }
    broken
}

fn mutate_failed_stage(sample: &Value) -> Value {
    mutate_stage(sample, 1, |stage| {
        stage["status"] = Value::from(1);
        stage["ok"] = Value::from(false);
    })
}

fn mutate_missing_command(sample: &Value) -> Value {
    mutate_stage(sample, 2, |stage| {
        stage["command"] = Value::Array(Vec::new());
    })
}

fn mutate_absolute_command(root: &Path, sample: &Value) -> Value {
    mutate_stage(sample, 0, |stage| {
        stage["command"] = json!(["cargo", root.join("Cargo.toml").to_string_lossy()]);
    })
}

fn mutate_absolute_cwd(root: &Path, sample: &Value) -> Value {
    mutate_stage(sample, 0, |stage| {
        stage["cwd"] = Value::from(root.to_string_lossy().to_string());
    })
}

fn mutate_stage(sample: &Value, index: usize, mutate: impl FnOnce(&mut Value)) -> Value {
    let mut broken = sample.clone();
    if let Some(stage) = broken
        .get_mut("stages")
        .and_then(Value::as_array_mut)
        .and_then(|stages| stages.get_mut(index))
    {
        mutate(stage);
    }
    broken
}

fn require_string(value: Option<&Value>, context: &str, errors: &mut Vec<String>) {
    if value.and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push(format!("{context} must be a non-empty string"));
    }
}

fn require_repo_path(root: &Path, value: Option<&Value>, context: &str, errors: &mut Vec<String>) {
    require_string(value, context, errors);
    let Some(path) = value.and_then(Value::as_str) else {
        return;
    };
    if Path::new(path).is_absolute() || path.contains(&root.to_string_lossy().to_string()) {
        errors.push(format!("{context} must be a repo-relative path"));
        return;
    }
    let absolute = root.join(path);
    if absolute
        .strip_prefix(root)
        .map(|relative| relative.starts_with("..") || relative.is_absolute())
        .unwrap_or(true)
    {
        errors.push(format!("{context} must stay inside the repository"));
    }
}

fn require_portable_command(
    root: &Path,
    value: Option<&Value>,
    context: &str,
    errors: &mut Vec<String>,
) {
    let Some(command) = value.and_then(Value::as_array) else {
        errors.push(format!("{context} must be a non-empty string array"));
        return;
    };
    if command.is_empty() {
        errors.push(format!("{context} must be a non-empty string array"));
        return;
    }
    for (index, item) in command.iter().enumerate() {
        require_string(Some(item), &format!("{context}[{index}]"), errors);
        if let Some(text) = item.as_str() {
            if Path::new(text).is_absolute() || text.contains(&root.to_string_lossy().to_string()) {
                errors.push(format!(
                    "{context}[{index}] must not contain local absolute paths"
                ));
            }
        }
    }
}

fn read_repo_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    read_json_path(&root.join(relative_path), relative_path)
}

fn read_json_path(path: &Path, label: &str) -> RunnerResult<Value> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label}: invalid json: {error}"))
}

fn read_repo_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{REQUIRED_STAGES, SCHEMA_VERSION, dynamic_smoke_errors};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn stage_order_is_canonical() {
        assert_eq!(REQUIRED_STAGES[0], "template_tests");
        assert_eq!(REQUIRED_STAGES[3], "engine_dynamic_host_load");
    }

    #[test]
    fn rejects_bad_schema_version() {
        let report = json!({ "schema_version": "wrong", "stages": [] });
        let errors = dynamic_smoke_errors(Path::new("."), &report, "self");
        assert!(errors.iter().any(|error| error.contains("schema_version")));
    }

    #[test]
    fn schema_version_is_stable() {
        assert_eq!(SCHEMA_VERSION, "kyuubiki.operator-package-dynamic-smoke/v1");
    }
}
