use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const MANIFEST_PATH: &str = "docs/commercial-readiness-2.0.manifest.json";
const MARKDOWN_PATH: &str = "docs/commercial-readiness-2.0.md";
const DOCS_DIR: &str = "docs";
const SCHEMA_VERSION: &str = "kyuubiki.commercial-readiness/v1";
const RELEASE_TARGET: &str = "2.0";
const EXPECTED_GATE_COUNT: usize = 8;
const CLASSIFICATIONS: &[&str] = &[
    "required",
    "acceptable limitation",
    "defer to 2.x",
    "blocker",
];
const GATE_STATES: &[&str] = &["ready", "partial", "watch", "blocked"];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_validate_commercial_readiness(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner validate-commercial-readiness");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("validate-commercial-readiness does not accept arguments".to_string());
    }
    let manifest = read_json(root, MANIFEST_PATH)?;
    let markdown = read_text(root, MARKDOWN_PATH)?;
    let issues = readiness_issues(root, &manifest, &markdown);
    if !issues.is_empty() {
        eprintln!("commercial readiness validation failed:");
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
        "commercial readiness manifest ok: {} gates, {} evidence links",
        gates.len(),
        evidence_count
    );
    Ok(0)
}

fn readiness_issues(root: &Path, manifest: &Value, markdown: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if field(manifest, "schema_version") != SCHEMA_VERSION {
        issues.push(format!("{MANIFEST_PATH}: unexpected schema_version"));
    }
    if field(manifest, "release_target") != RELEASE_TARGET {
        issues.push(format!(
            "{MANIFEST_PATH}: release_target must be {RELEASE_TARGET}"
        ));
    }
    if string_array(manifest, "classification_values") != CLASSIFICATIONS {
        issues.push(format!("{MANIFEST_PATH}: classification_values drifted"));
    }
    let gates = array(manifest, "gates");
    if gates.len() != EXPECTED_GATE_COUNT {
        issues.push(format!(
            "{MANIFEST_PATH}: expected {EXPECTED_GATE_COUNT} commercial readiness gates"
        ));
    }
    if !markdown.contains("commercial-readiness-2.0.manifest.json") {
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
            field(gate, "readiness_state"),
            &format!("{label}: missing readiness_state"),
            issues,
        );
        if !GATE_STATES.contains(&field(gate, "readiness_state")) {
            issues.push(format!(
                "{label}: unknown readiness_state {}",
                field(gate, "readiness_state")
            ));
        }
        for key in [
            "next_1x_focus",
            "required",
            "acceptable_limitations",
            "blockers",
            "evidence_docs",
        ] {
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
    use super::{normalize, readiness_issues};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn normalize_collapses_markdown_whitespace() {
        assert_eq!(normalize("A\n  bounded\tworkflow"), "A bounded workflow");
    }

    #[test]
    fn rejects_schema_drift_before_gate_walk() {
        let manifest = json!({
            "schema_version": "wrong",
            "release_target": "2.0",
            "classification_values": ["required", "acceptable limitation", "defer to 2.x", "blocker"],
            "exit_statement": "Ship honestly.",
            "gates": []
        });
        let issues = readiness_issues(
            Path::new("."),
            &manifest,
            "commercial-readiness-2.0.manifest.json\nShip honestly.",
        );
        assert!(issues.iter().any(|issue| issue.contains("schema_version")));
    }
}
