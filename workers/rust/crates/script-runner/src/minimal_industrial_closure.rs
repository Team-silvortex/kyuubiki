use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const MANIFEST_PATH: &str = "docs/minimal-industrial-closure.manifest.json";
const MARKDOWN_PATH: &str = "docs/minimal-industrial-closure.md";
const DOCS_DIR: &str = "docs";
const SCHEMA_VERSION: &str = "kyuubiki.minimal-industrial-closure/v1";
const RELEASE_LINE: &str = "1.15.x -> 1.20.x";
const EXPECTED_GATE_COUNT: usize = 8;
const STATES: &[&str] = &["present", "partial", "missing", "blocked"];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_validate_minimal_industrial_closure(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner validate-minimal-industrial-closure");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("validate-minimal-industrial-closure does not accept arguments".to_string());
    }
    let manifest = read_json(root, MANIFEST_PATH)?;
    let markdown = read_text(root, MARKDOWN_PATH)?;
    let issues = closure_issues(root, &manifest, &markdown);
    if !issues.is_empty() {
        eprintln!("minimal industrial closure validation failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }
    let gates = array(&manifest, "gates");
    let evidence_count = gates
        .iter()
        .map(|gate| array(gate, "evidence_docs").len())
        .sum::<usize>();
    println!(
        "minimal industrial closure manifest ok: {} gates, {} evidence links",
        gates.len(),
        evidence_count
    );
    Ok(0)
}

fn closure_issues(root: &Path, manifest: &Value, markdown: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if field(manifest, "schema_version") != SCHEMA_VERSION {
        issues.push(format!("{MANIFEST_PATH}: unexpected schema_version"));
    }
    if field(manifest, "release_line") != RELEASE_LINE {
        issues.push(format!(
            "{MANIFEST_PATH}: release_line must stay on the 1.15.x -> 1.20.x bridge"
        ));
    }
    if string_array(manifest, "state_values") != STATES {
        issues.push(format!("{MANIFEST_PATH}: state_values drifted"));
    }
    let gates = array(manifest, "gates");
    if gates.len() != EXPECTED_GATE_COUNT {
        issues.push(format!(
            "{MANIFEST_PATH}: expected {EXPECTED_GATE_COUNT} minimal industrial closure gates"
        ));
    }
    if !markdown.contains("minimal-industrial-closure.manifest.json") {
        issues.push(format!(
            "{MARKDOWN_PATH}: missing paired manifest reference"
        ));
    }
    let exit_statement = field(manifest, "exit_statement");
    if !normalize(markdown).contains(&normalize(exit_statement)) {
        issues.push(format!("{MARKDOWN_PATH}: missing manifest exit statement"));
    }
    check_gates(root, markdown, &gates, &mut issues);
    issues
}

fn check_gates(root: &Path, markdown: &str, gates: &[&Value], issues: &mut Vec<String>) {
    let mut gate_ids = BTreeSet::new();
    for (index, gate) in gates.iter().enumerate() {
        let fallback_id = (index + 1).to_string();
        let id = field(gate, "id");
        let label_id = if id.is_empty() { &fallback_id } else { id };
        let label = format!("{MANIFEST_PATH}: gate {label_id}");
        if id.is_empty() || !gate_ids.insert(id.to_string()) {
            issues.push(format!("{label}: missing or duplicate id"));
        }
        require_text(
            field(gate, "title"),
            &format!("{label}: missing title"),
            issues,
        );
        require_text(
            field(gate, "minimum_state"),
            &format!("{label}: missing minimum_state"),
            issues,
        );
        if !STATES.contains(&field(gate, "minimum_state")) {
            issues.push(format!(
                "{label}: unknown minimum_state {}",
                field(gate, "minimum_state")
            ));
        }
        for key in ["required", "next_closure_work", "evidence_docs"] {
            if array(gate, key).is_empty() {
                issues.push(format!("{label}: missing {key} items"));
            }
        }
        let title = field(gate, "title");
        if !markdown.contains(&format!("## {}. {title}", index + 1)) {
            issues.push(format!("{MARKDOWN_PATH}: missing heading for {title}"));
        }
        for evidence_doc in string_array(gate, "evidence_docs") {
            if !root.join(DOCS_DIR).join(&evidence_doc).exists() {
                issues.push(format!("{label}: missing evidence doc {evidence_doc}"));
            }
            if !markdown.contains(&format!("]({evidence_doc})")) {
                issues.push(format!(
                    "{MARKDOWN_PATH}: missing evidence link {evidence_doc}"
                ));
            }
        }
    }
}

fn require_text(value: &str, message: &str, issues: &mut Vec<String>) {
    if value.trim().is_empty() {
        issues.push(message.to_string());
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    array(value, key)
        .into_iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{closure_issues, normalize};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn normalize_collapses_markdown_whitespace() {
        assert_eq!(
            normalize("A\n  bounded\tFEM workflow"),
            "A bounded FEM workflow"
        );
    }

    #[test]
    fn rejects_state_value_drift() {
        let manifest = json!({
            "schema_version": "kyuubiki.minimal-industrial-closure/v1",
            "release_line": "1.15.x -> 1.20.x",
            "state_values": ["present"],
            "exit_statement": "Ship one bounded loop.",
            "gates": []
        });
        let issues = closure_issues(
            Path::new("."),
            &manifest,
            "minimal-industrial-closure.manifest.json\nShip one bounded loop.",
        );
        assert!(issues.iter().any(|issue| issue.contains("state_values")));
    }
}
