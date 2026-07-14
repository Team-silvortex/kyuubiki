use super::{MATRIX_PATH, TENSOR_PATH, TOPOLOGY_PATH, object_entries, string_array, string_field};
use serde_json::Value;

pub(super) fn render_markdown(report: &Value) -> String {
    let mut lines = vec![
        "# Module Function Coverage Tensor".to_string(),
        String::new(),
        format!("- Source: `{TENSOR_PATH}`"),
        format!("- Topology: `{TOPOLOGY_PATH}`"),
        format!("- Matrix: `{MATRIX_PATH}`"),
        "- Axes: `module x function_paradigm x evidence_depth`".to_string(),
        format!(
            "- Modules: `{}`",
            report
                .pointer("/axes/modules")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        ),
        format!(
            "- Paradigms: `{}`",
            report
                .pointer("/axes/paradigms")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        ),
        format!(
            "- Depth axes: `{}`",
            string_array(report.pointer("/axes").unwrap_or(&Value::Null), "depth").join("`, `")
        ),
        format!(
            "- Blocking gaps: `{}`",
            report
                .get("blocking_gap_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        String::new(),
        "## Module Summary".to_string(),
        String::new(),
        "| Module | Layer | OK | Weak | Weak Evidence | Watch | Planned | Required Gap | Missing | N/A |".to_string(),
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |".to_string(),
    ];
    for module_id in axis_values(report, "modules") {
        if let Some(summary) = report.pointer(&format!("/module_summary/{module_id}")) {
            let counts = summary.get("counts").unwrap_or(&Value::Null);
            lines.push(format!(
                "| `{module_id}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
                string_field(summary, "layer").unwrap_or_default(),
                count(counts, "ok"),
                count(counts, "weak"),
                count(counts, "weak_evidence"),
                count(counts, "watch"),
                count(counts, "planned"),
                count(counts, "required_gap"),
                count(counts, "missing"),
                count(counts, "not_applicable")
            ));
        }
    }
    render_paradigm_summary(report, &mut lines);
    render_contract_evidence(report, &mut lines);
    render_gaps(report, &mut lines);
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_paradigm_summary(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Paradigm Summary".to_string(),
        String::new(),
    ]);
    lines.push(
        "| Paradigm | OK | Weak | Weak Evidence | Watch | Planned | Required Gap | Missing | N/A |"
            .to_string(),
    );
    lines.push("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |".to_string());
    for paradigm in axis_values(report, "paradigms") {
        if let Some(counts) = report.pointer(&format!("/paradigm_summary/{paradigm}")) {
            lines.push(format!(
                "| `{paradigm}` | {} | {} | {} | {} | {} | {} | {} | {} |",
                count(counts, "ok"),
                count(counts, "weak"),
                count(counts, "weak_evidence"),
                count(counts, "watch"),
                count(counts, "planned"),
                count(counts, "required_gap"),
                count(counts, "missing"),
                count(counts, "not_applicable")
            ));
        }
    }
}

fn render_contract_evidence(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Contract Evidence".to_string(),
        String::new(),
    ]);
    let entries = object_entries(report, "paradigm_contract_evidence");
    if entries.is_empty() {
        lines.push("No contract evidence.".to_string());
        return;
    }
    lines.push("| Paradigm | Evidence | Files | Required Text |".to_string());
    lines.push("| --- | --- | --- | --- |".to_string());
    for (paradigm, list) in entries {
        for entry in list.as_array().into_iter().flatten() {
            lines.push(format!(
                "| `{paradigm}` | `{}` | {} | {} |",
                string_field(entry, "id").unwrap_or_default(),
                string_array(entry, "files")
                    .iter()
                    .map(|file| format!("`{file}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                string_array(entry, "required_text")
                    .iter()
                    .map(|text| format!("`{text}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
}

fn render_gaps(report: &Value, lines: &mut Vec<String>) {
    lines.extend([String::new(), "## Gaps".to_string(), String::new()]);
    let gaps = report
        .get("gaps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if gaps.is_empty() {
        lines.push("No gaps.".to_string());
        return;
    }
    lines.push(
        "| Gap | Module | Paradigm | Status | Required | Benchmark Lanes | Security Lanes |"
            .to_string(),
    );
    lines.push("| --- | --- | --- | --- | --- | --- | --- |".to_string());
    for gap in gaps {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
            string_field(&gap, "gap").unwrap_or_default(),
            string_field(&gap, "module_id").unwrap_or_default(),
            string_field(&gap, "paradigm").unwrap_or_default(),
            string_field(&gap, "status").unwrap_or_default(),
            gap.get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            joined_or_dash(&string_array(&gap, "benchmark_lanes")),
            joined_or_dash(&string_array(&gap, "security_lanes"))
        ));
    }
}

fn count(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn axis_values(report: &Value, key: &str) -> Vec<String> {
    string_array(report.pointer("/axes").unwrap_or(&Value::Null), key)
}

fn joined_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}
