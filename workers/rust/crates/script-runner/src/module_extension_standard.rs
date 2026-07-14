use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const STANDARD_PATH: &str = "config/architecture/module-extension-standard.json";
const SCHEMA_PATH: &str = "schemas/module-extension-standard.schema.json";
const SCHEMA_VERSION: &str = "kyuubiki.module-extension-standard/v1";
const REQUIRED_TYPES: &[&str] = &[
    "module",
    "function_paradigm",
    "service_surface",
    "evidence_lane",
    "contract_family",
];

pub(crate) fn run_check_module_extension_standard(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("module extension standard self-test passed");
        return Ok(0);
    }
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-module-extension-standard [--self-test]");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-module-extension-standard only accepts --self-test".to_string());
    }

    let standard = read_json(root, STANDARD_PATH)?;
    validate_standard(root, &standard)?;
    println!("module extension standard passed");
    Ok(0)
}

fn validate_standard(root: &Path, standard: &Value) -> RunnerResult<()> {
    require_string_eq(
        standard.get("$schema"),
        "../../schemas/module-extension-standard.schema.json",
        "standard must point at module-extension-standard schema",
    )?;
    require_string_eq(
        standard.get("schema_version"),
        SCHEMA_VERSION,
        &format!("schema_version must be {SCHEMA_VERSION}"),
    )?;
    assert_repo_file(root, SCHEMA_PATH)?;

    let source_of_truth = standard
        .get("source_of_truth")
        .and_then(Value::as_object)
        .ok_or_else(|| "source_of_truth must be an object".to_string())?;
    for file in source_of_truth.values().filter_map(Value::as_str) {
        assert_repo_file(root, file)?;
    }

    let evidence_rule = standard
        .pointer("/evidence_rules/required_cell_without_evidence")
        .and_then(Value::as_str);
    if evidence_rule != Some("weak_evidence") {
        return Err(
            "extension standard must preserve weak_evidence for empty required coverage"
                .to_string(),
        );
    }

    let mut seen_types = BTreeSet::new();
    for entry in value_array(standard, "extension_types")? {
        let id = required_str(entry, "id", "extension type")?;
        if !REQUIRED_TYPES.contains(&id) {
            return Err(format!("unknown extension type: {id}"));
        }
        if !seen_types.insert(id.to_string()) {
            return Err(format!("duplicate extension type: {id}"));
        }
        for file in entry
            .get("required_files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            assert_repo_file(root, file)?;
        }
        if entry
            .get("steps")
            .and_then(Value::as_array)
            .is_none_or(|steps| steps.len() < 3)
        {
            return Err(format!("{id} must have at least three onboarding steps"));
        }
    }
    for required_type in REQUIRED_TYPES {
        if !seen_types.contains(*required_type) {
            return Err(format!("missing extension type: {required_type}"));
        }
    }

    let make_targets = list_make_targets(root)?;
    for gate in value_array(standard, "gates")? {
        let id = required_str(gate, "id", "gate")?;
        let command = required_str(gate, "command", "gate")?;
        let Some(target) = command.strip_prefix("make ") else {
            return Err(format!("gate {id} must use a make target command"));
        };
        if target.contains(' ') || target.is_empty() {
            return Err(format!("gate {id} must use a make target command"));
        }
        if !make_targets.contains(target) {
            return Err(format!("gate {id} points at unknown make target {target}"));
        }
    }

    let docs_path = source_of_truth
        .get("docs")
        .and_then(Value::as_str)
        .ok_or_else(|| "source_of_truth.docs must be set".to_string())?;
    let docs_text = read_text(root, docs_path)?;
    for anchor in [
        "Adding A Module",
        "Adding A Function Paradigm",
        "Adding A Service Surface",
        "weak_evidence",
    ] {
        if !docs_text.contains(anchor) {
            return Err(format!("extension standard docs missing {anchor}"));
        }
    }

    Ok(())
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let fixture = serde_json::json!({
        "$schema": "../../schemas/module-extension-standard.schema.json",
        "schema_version": SCHEMA_VERSION,
        "source_of_truth": {
            "topology": "config/architecture/module-topology.json",
            "matrix": "config/architecture/module-function-coverage-matrix.json",
            "tensor": "config/architecture/module-function-coverage-tensor.json",
            "docs": "docs/architecture-extension-standard.md"
        },
        "evidence_rules": { "required_cell_without_evidence": "weak_evidence" },
        "extension_types": REQUIRED_TYPES.iter().map(|id| serde_json::json!({
            "id": id,
            "required_files": ["config/architecture/module-topology.json"],
            "steps": ["first required step", "second required step", "third required step"]
        })).collect::<Vec<_>>(),
        "gates": [{ "id": "topology", "command": "make check-module-topology" }]
    });
    validate_standard(root, &fixture)?;
    let mut broken = fixture;
    broken["evidence_rules"]["required_cell_without_evidence"] = Value::String("ok".to_string());
    if validate_standard(root, &broken).is_ok() {
        return Err("self-test expected weak_evidence drift to fail".to_string());
    }
    Ok(())
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = repo_path(root, relative_path)?;
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<std::path::PathBuf> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.split('/').any(|part| part == "..")
    {
        return Err(format!("invalid repository-relative path: {relative_path}"));
    }
    Ok(root.join(relative_path))
}

fn assert_repo_file(root: &Path, relative_path: &str) -> RunnerResult<()> {
    let path = repo_path(root, relative_path)?;
    if !path.is_file() {
        return Err(format!("missing required path: {relative_path}"));
    }
    Ok(())
}

fn list_make_targets(root: &Path) -> RunnerResult<BTreeSet<String>> {
    let mut targets = BTreeSet::new();
    for file in [
        "Makefile",
        "make/checks.mk",
        "make/help.mk",
        "make/tests.mk",
        "make/benchmarks.mk",
    ] {
        if !root.join(file).is_file() {
            continue;
        }
        for line in read_text(root, file)?.lines() {
            if let Some((target, _)) = line.split_once(':') {
                if !target.is_empty()
                    && target
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
                {
                    targets.insert(target.to_string());
                }
            }
        }
    }
    Ok(targets)
}

fn value_array<'a>(value: &'a Value, key: &str) -> RunnerResult<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))
}

fn required_str<'a>(value: &'a Value, key: &str, context: &str) -> RunnerResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{context} missing {key}"))
}

fn require_string_eq(value: Option<&Value>, expected: &str, message: &str) -> RunnerResult<()> {
    if value.and_then(Value::as_str) != Some(expected) {
        return Err(message.to_string());
    }
    Ok(())
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::list_make_targets;
    use std::path::Path;

    #[test]
    fn make_target_parser_finds_known_targets() {
        let targets = list_make_targets(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../..")
                .as_path(),
        )
        .expect("make targets should parse");

        assert!(targets.contains("check-module-topology"));
        assert!(targets.contains("audit-rust-lines"));
    }
}
