use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const MANIFEST_PATH: &str = "docs/material-score-contract.manifest.json";
const MARKDOWN_PATH: &str = "docs/material-score-contract.md";
const SCHEMA_VERSION: &str = "kyuubiki.material-score-contract/v1";
const OPERATOR_ID: &str = "transform.score_material_candidates";
const REQUIRED_ARRAY_KEYS: &[&str] = &[
    "runtime_paths",
    "test_paths",
    "required_result_fields",
    "ranking_fields",
    "policy_fields",
    "range_fields",
    "stable_error_codes",
];

pub(crate) fn run_validate_material_score_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner validate-material-score-contract");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err(
            "validate-material-score-contract does not accept positional arguments".to_string(),
        );
    }

    let manifest = read_json(root, MANIFEST_PATH)?;
    let markdown = read_text(root, MARKDOWN_PATH)?;
    let issues = validate(root, &manifest, &markdown)?;
    if !issues.is_empty() {
        eprintln!("material score contract validation failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }

    println!(
        "material score contract ok: {} result fields, {} errors",
        string_array(&manifest, "required_result_fields").len(),
        string_array(&manifest, "stable_error_codes").len()
    );
    Ok(0)
}

fn validate(root: &Path, manifest: &Value, markdown: &str) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    if manifest.get("schema_version").and_then(Value::as_str) != Some(SCHEMA_VERSION) {
        issues.push(format!("{MANIFEST_PATH}: unexpected schema_version"));
    }
    if manifest.get("operator_id").and_then(Value::as_str) != Some(OPERATOR_ID) {
        issues.push(format!("{MANIFEST_PATH}: unexpected operator_id"));
    }
    for key in REQUIRED_ARRAY_KEYS {
        if string_array(manifest, key).is_empty() {
            issues.push(format!("{MANIFEST_PATH}: missing {key}"));
        }
    }

    for relative_path in string_array(manifest, "runtime_paths")
        .into_iter()
        .chain(string_array(manifest, "test_paths"))
    {
        if !repo_file_exists(root, &relative_path)? {
            issues.push(format!(
                "{MANIFEST_PATH}: missing referenced path {relative_path}"
            ));
        }
    }

    let runtime_sources = read_referenced_sources(root, &string_array(manifest, "runtime_paths"))?;
    let test_sources = read_referenced_sources(root, &string_array(manifest, "test_paths"))?;

    for token in markdown_tokens(manifest) {
        if !markdown.contains(&token) {
            issues.push(format!("{MARKDOWN_PATH}: missing contract token {token}"));
        }
    }
    for token in source_tokens(manifest, true) {
        if !runtime_sources.contains(&token) {
            issues.push(format!("runtime sources: missing contract token {token}"));
        }
    }
    for token in source_tokens(manifest, false) {
        if !test_sources.contains(&token) {
            issues.push(format!("test sources: missing contract token {token}"));
        }
    }

    Ok(issues)
}

fn markdown_tokens(manifest: &Value) -> Vec<String> {
    let mut tokens = vec![OPERATOR_ID.to_string()];
    for key in [
        "required_result_fields",
        "ranking_fields",
        "policy_fields",
        "range_fields",
        "stable_error_codes",
    ] {
        tokens.extend(string_array(manifest, key));
    }
    tokens
}

fn source_tokens(manifest: &Value, include_ranges: bool) -> Vec<String> {
    let mut tokens = Vec::new();
    for key in [
        "required_result_fields",
        "policy_fields",
        "stable_error_codes",
    ] {
        tokens.extend(string_array(manifest, key));
    }
    if include_ranges {
        tokens.extend(string_array(manifest, "range_fields"));
    }
    tokens
}

fn string_array(manifest: &Value, key: &str) -> Vec<String> {
    manifest
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn read_referenced_sources(root: &Path, relative_paths: &[String]) -> RunnerResult<String> {
    let mut sources = String::new();
    for relative_path in relative_paths {
        let path = repo_path(root, relative_path)?;
        if path.is_file() {
            sources.push_str(
                &fs::read_to_string(&path)
                    .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
            );
            sources.push('\n');
        }
    }
    Ok(sources)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = repo_path(root, relative_path)?;
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn repo_file_exists(root: &Path, relative_path: &str) -> RunnerResult<bool> {
    Ok(repo_path(root, relative_path)?.is_file())
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

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::{markdown_tokens, source_tokens};
    use serde_json::json;

    #[test]
    fn token_sets_follow_material_score_contract_lanes() {
        let manifest = json!({
            "required_result_fields": ["result"],
            "ranking_fields": ["ranking"],
            "policy_fields": ["policy"],
            "range_fields": ["min", "max"],
            "stable_error_codes": ["missing"]
        });

        assert!(markdown_tokens(&manifest).contains(&"ranking".to_string()));
        assert!(source_tokens(&manifest, true).contains(&"min".to_string()));
        assert!(!source_tokens(&manifest, false).contains(&"max".to_string()));
    }
}
