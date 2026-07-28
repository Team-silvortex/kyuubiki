use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const MANIFEST_PATH: &str = "docs/moxi-handoff.manifest.json";
const MARKDOWN_PATH: &str = "docs/moxi-handoff.md";
const SCHEMA_VERSION: &str = "kyuubiki.moxi-handoff/v1";
const FROM_LINE: &str = "moxi 2.0.0";
const TO_LINE: &str = "moxi 2.x";
const EXPECTED_GATE_COUNT: usize = 7;
const GATE_STATES: &[&str] = &["ready", "active", "watch", "defer_to_2x"];

pub(crate) fn run_check_moxi_handoff(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner check-moxi-handoff");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-moxi-handoff does not accept arguments".to_string());
    }

    let manifest = read_json(root, MANIFEST_PATH)?;
    let markdown = read_text(root, MARKDOWN_PATH)?;
    let issues = handoff_issues(root, &manifest, &markdown);
    if !issues.is_empty() {
        eprintln!("moxi handoff validation failed:");
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
        "moxi handoff manifest ok: {} gates, {} evidence links",
        gates.len(),
        evidence_count
    );
    Ok(0)
}

fn handoff_issues(root: &Path, manifest: &Value, markdown: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if field(manifest, "schema_version") != SCHEMA_VERSION {
        issues.push(format!("{MANIFEST_PATH}: unexpected schema_version"));
    }
    if field(manifest, "from_line") != FROM_LINE {
        issues.push(format!("{MANIFEST_PATH}: from_line must stay {FROM_LINE}"));
    }
    if field(manifest, "to_line") != TO_LINE {
        issues.push(format!("{MANIFEST_PATH}: to_line must stay {TO_LINE}"));
    }
    if string_array(manifest, "allowed_gate_states") != GATE_STATES {
        issues.push(format!("{MANIFEST_PATH}: allowed_gate_states drifted"));
    }
    if !normalize(markdown).contains(&normalize(field(manifest, "handoff_statement"))) {
        issues.push(format!(
            "{MARKDOWN_PATH}: missing manifest handoff statement"
        ));
    }
    if !markdown.contains("moxi-handoff.manifest.json") {
        issues.push(format!(
            "{MARKDOWN_PATH}: missing paired manifest reference"
        ));
    }

    let gates = array(manifest, "gates");
    if gates.len() != EXPECTED_GATE_COUNT {
        issues.push(format!(
            "{MANIFEST_PATH}: expected {EXPECTED_GATE_COUNT} gates"
        ));
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
        for key in ["title", "state", "handoff_question"] {
            if field(gate, key).trim().is_empty() {
                issues.push(format!("{label}: missing {key}"));
            }
        }
        if !GATE_STATES.contains(&field(gate, "state")) {
            issues.push(format!(
                "{label}: unsupported state {}",
                field(gate, "state")
            ));
        }
        for key in ["must_close", "evidence_docs"] {
            if array(gate, key).is_empty() {
                issues.push(format!("{label}: missing {key} items"));
            }
        }
        let title = field(gate, "title");
        if !markdown.contains(&format!("### {}. {title}", index + 1)) {
            issues.push(format!("{MARKDOWN_PATH}: missing heading for {title}"));
        }
        for item in string_array(gate, "must_close") {
            if !markdown.contains(item) {
                issues.push(format!(
                    "{MARKDOWN_PATH}: missing must_close item for {label_id}"
                ));
            }
        }
        for evidence_doc in string_array(gate, "evidence_docs") {
            if !root.join("docs").join(evidence_doc).exists() {
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

fn read_json(root: &Path, relative: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative)?;
    serde_json::from_str(&text).map_err(|error| format!("failed to parse {relative}: {error}"))
}

fn read_text(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn string_array<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    array(value, key)
        .into_iter()
        .filter_map(Value::as_str)
        .collect()
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{MANIFEST_PATH, MARKDOWN_PATH, handoff_issues, read_json, read_text};
    use std::path::PathBuf;

    #[test]
    fn retained_moxi_handoff_is_valid() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let manifest = read_json(&root, MANIFEST_PATH).expect("handoff manifest should load");
        let markdown = read_text(&root, MARKDOWN_PATH).expect("handoff markdown should load");
        let issues = handoff_issues(&root, &manifest, &markdown);
        assert!(issues.is_empty(), "{}", issues.join("\n"));
    }
}
