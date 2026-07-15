use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const SURFACE_PATH: &str = "config/architecture/verification-evidence-surface.json";
const MATRIX_PATH: &str = "config/architecture/module-function-coverage-matrix.json";
const SCHEMA_VERSION: &str = "kyuubiki.verification-evidence-surface/v1";
const MODULE_ID: &str = "verification-evidence";
const COVERED_PARADIGMS: &[&str] = &[
    "runtime_api",
    "solver_execution",
    "workflow_composition",
    "deployment_update",
    "sdk_headless",
    "persistence_provenance",
];

pub(crate) fn run_check_verification_evidence_surface(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-verification-evidence-surface");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-verification-evidence-surface does not accept arguments".to_string());
    }
    let surface = read_json(root, SURFACE_PATH)?;
    assert_surface(root, &surface)?;
    assert_matrix_alignment(root, &surface)?;
    println!(
        "verification evidence surface passed: {} covered evidence paradigm(s)",
        COVERED_PARADIGMS.len()
    );
    Ok(0)
}

fn assert_surface(root: &Path, surface: &Value) -> RunnerResult<()> {
    if string_field(surface, "schema_version") != Some(SCHEMA_VERSION) {
        return Err(format!("schema_version must be {SCHEMA_VERSION}"));
    }
    if string_field(surface, "module_id") != Some(MODULE_ID) {
        return Err(format!("module_id must be {MODULE_ID}"));
    }
    assert_path_exists(
        root,
        string_field(surface, "matrix").unwrap_or_default(),
        "matrix",
    )?;
    assert_path_exists(
        root,
        string_field(surface, "tensor").unwrap_or_default(),
        "tensor",
    )?;

    for command in surface
        .pointer("/runtime_api/stable_commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        assert_command(root, command)?;
    }
    for artifact in surface
        .pointer("/runtime_api/generated_artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !artifact.starts_with("tmp/") {
            return Err(format!(
                "generated artifact must live under tmp/: {artifact}"
            ));
        }
    }

    assert_includes(
        &string_array_at(surface, "/runtime_api/stable_commands"),
        "./scripts/kyuubiki build-central-readiness-report",
        "stable command",
    )?;
    assert_includes(
        &string_array_at(surface, "/runtime_api/stable_commands"),
        "./scripts/kyuubiki check-central-readiness-report",
        "stable command",
    )?;
    assert_includes(
        &string_array_at(surface, "/runtime_api/generated_artifacts"),
        "tmp/central-readiness-report.json",
        "generated artifact",
    )?;
    assert_includes(
        &string_array_at(surface, "/runtime_api/generated_artifacts"),
        "tmp/central-readiness-report.md",
        "generated artifact",
    )?;

    for paradigm in COVERED_PARADIGMS {
        let Some(block) = surface.get(*paradigm) else {
            return Err(format!("missing {paradigm} evidence block"));
        };
        if *paradigm == "runtime_api" {
            continue;
        }
        let sources = string_array(block, "evidence_sources");
        if sources.is_empty() {
            return Err(format!("{paradigm} must list evidence_sources"));
        }
        for source in sources {
            assert_path_exists(root, &source, "evidence source")?;
        }
    }
    Ok(())
}

fn assert_command(root: &Path, command: &str) -> RunnerResult<()> {
    let mut parts = command.split_whitespace();
    let binary = parts.next().unwrap_or_default();
    if binary != "./scripts/kyuubiki" {
        return Err(format!(
            "runtime command must use native wrapper: {command}"
        ));
    }
    let command_name = parts.next().unwrap_or_default();
    if command_name.is_empty() {
        return Err(format!(
            "runtime command missing runner subcommand: {command}"
        ));
    }
    assert_path_exists(root, "scripts/kyuubiki", "runtime command wrapper")
}

fn assert_matrix_alignment(root: &Path, surface: &Value) -> RunnerResult<()> {
    let matrix_path = string_field(surface, "matrix").unwrap_or(MATRIX_PATH);
    let matrix = read_json(root, matrix_path)?;
    let Some(row) = matrix.pointer(&format!("/cells/{MODULE_ID}")) else {
        return Err(format!("missing matrix row for {MODULE_ID}"));
    };
    for paradigm in COVERED_PARADIGMS {
        if string_field(row, paradigm) != Some("covered") {
            return Err(format!("{MODULE_ID}/{paradigm} must be covered"));
        }
    }
    Ok(())
}

fn assert_includes(values: &[String], expected: &str, label: &str) -> RunnerResult<()> {
    if !values.iter().any(|value| value == expected) {
        return Err(format!("{label} missing {expected}"));
    }
    Ok(())
}

fn assert_path_exists(root: &Path, relative_path: &str, label: &str) -> RunnerResult<()> {
    if relative_path.is_empty() {
        return Err(format!("{label} path does not exist: {relative_path}"));
    }
    let path = repo_path(root, relative_path)?;
    if !path.exists() {
        return Err(format!("{label} path does not exist: {relative_path}"));
    }
    Ok(())
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    if relative_path.starts_with('/')
        || relative_path.is_empty()
        || relative_path.split('/').any(|part| part == "..")
    {
        return Err(format!("path escapes repository: {relative_path}"));
    }
    Ok(root.join(relative_path))
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let path = repo_path(root, relative_path)?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn string_array_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::{assert_includes, repo_path};
    use std::path::Path;

    #[test]
    fn repo_path_rejects_escape() {
        assert!(repo_path(Path::new("."), "../outside").is_err());
        assert!(repo_path(Path::new("."), "/tmp/outside").is_err());
    }

    #[test]
    fn includes_reports_missing_label() {
        let error = assert_includes(&["a".to_string()], "b", "stable command").unwrap_err();
        assert!(error.contains("stable command missing b"));
    }
}
