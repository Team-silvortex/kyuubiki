use super::{MATRIX_PATH, TENSOR_PATH, TOPOLOGY_PATH, object_entries, string_array, string_field};
use serde_json::Value;

pub(super) fn render_markdown(report: &Value) -> String {
    let mut lines = vec![
        "# Module Function Coverage Tensor".to_string(),
        String::new(),
        format!("- Source: `{TENSOR_PATH}`"),
        format!("- Topology: `{TOPOLOGY_PATH}`"),
        format!("- Matrix: `{MATRIX_PATH}`"),
        format!(
            "- Evidence includes: `{}`",
            joined_or_dash(&string_array(report, "evidence_includes"))
        ),
        format!(
            "- Current checkpoint: `{}`",
            report
                .pointer("/release_readiness/current_checkpoint")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Release target: `{}`",
            report
                .pointer("/release_readiness/target_release")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Release readiness: `{}` (claim allowed: `{}`)",
            report
                .pointer("/release_readiness/status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            report
                .pointer("/release_readiness/release_claim_allowed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        ),
        "- Axes: `module x function_paradigm x scoped_evidence_depth`".to_string(),
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
        format!(
            "- Maturity gaps: `{}`",
            report
                .get("maturity_gap_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Thin evidence points: `{}`",
            report
                .get("thin_evidence_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Evidence grade mode: `{}`",
            report
                .pointer("/evidence_grade_calibration/gate_mode")
                .and_then(Value::as_str)
                .unwrap_or("advisory")
        ),
        format!(
            "- Evidence grade gaps: `{}`",
            report
                .get("evidence_grade_gap_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Average required-cell evidence score: `{:.1}`",
            report
                .pointer("/evidence_grade_calibration/average_evidence_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Required cells meeting target: `{:.1}%`",
            report
                .pointer("/evidence_grade_calibration/target_met_percent")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Evidence progress toward configured targets: `{:.1}%`",
            report
                .pointer("/evidence_grade_calibration/proof_completion_percent")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
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
    render_evidence_grades(report, &mut lines);
    render_release_readiness(report, &mut lines);
    render_contract_evidence(report, &mut lines);
    render_thin_points(report, &mut lines);
    render_gaps(report, &mut lines);
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_release_readiness(report: &Value, lines: &mut Vec<String>) {
    let readiness = report.get("release_readiness").unwrap_or(&Value::Null);
    lines.extend([
        String::new(),
        "## Daji Release Calibration".to_string(),
        String::new(),
        format!(
            "The `{}` gate remains `{}` until `{}`. Structural success does not grant a release claim.",
            string_field(readiness, "target_release").unwrap_or("release"),
            string_field(readiness, "gate_mode").unwrap_or("advisory"),
            string_field(readiness, "enforce_from").unwrap_or("the configured enforcement point")
        ),
        String::new(),
        "| Criterion | Met | Expected | Actual |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
    ]);
    for criterion in readiness
        .get("criteria")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` |",
            string_field(criterion, "id").unwrap_or_default(),
            criterion
                .get("met")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            json_scalar(criterion.get("expected")),
            json_scalar(criterion.get("actual"))
        ));
    }

    lines.extend([
        String::new(),
        "### Release Criticality".to_string(),
        String::new(),
        "| Criticality | Required Cells | Target Met | Gaps | Target Met % |".to_string(),
        "| --- | ---: | ---: | ---: | ---: |".to_string(),
    ]);
    for criticality in ["p0", "p1", "p2"] {
        let summary = readiness
            .pointer(&format!("/criticality_summary/{criticality}"))
            .unwrap_or(&Value::Null);
        lines.push(format!(
            "| `{criticality}` | {} | {} | {} | {:.1} |",
            count(summary, "required_cell_count"),
            count(summary, "target_met_count"),
            count(summary, "gap_count"),
            summary
                .get("target_met_percent")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ));
    }

    lines.extend([
        String::new(),
        "### P0 Blocking Coordinates".to_string(),
        String::new(),
    ]);
    let blockers = readiness
        .get("blocking_coordinates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if blockers.is_empty() {
        lines.push("No P0 coordinate gaps.".to_string());
    } else {
        lines.push("| Module | Paradigm | Achieved | Target | Next Action |".to_string());
        lines.push("| --- | --- | --- | --- | --- |".to_string());
        for blocker in blockers {
            lines.push(format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` |",
                string_field(&blocker, "module_id").unwrap_or_default(),
                string_field(&blocker, "paradigm").unwrap_or_default(),
                string_field(&blocker, "achieved_grade").unwrap_or_default(),
                string_field(&blocker, "target_grade").unwrap_or_default(),
                string_field(&blocker, "recommended_action").unwrap_or_default()
            ));
        }
    }

    lines.extend([
        String::new(),
        "### External Release Gates".to_string(),
        String::new(),
    ]);
    let gates = readiness
        .get("external_gates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if gates.is_empty() {
        lines.push("No external release gates.".to_string());
        return;
    }
    lines.push("| Gate | Criticality | Met | Expected | Actual | Next Action |".to_string());
    lines.push("| --- | --- | --- | --- | --- | --- |".to_string());
    for gate in gates {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
            string_field(&gate, "id").unwrap_or_default(),
            string_field(&gate, "criticality").unwrap_or_default(),
            gate.get("met").and_then(Value::as_bool).unwrap_or(false),
            json_scalar(gate.get("expected")),
            json_scalar(gate.get("actual")),
            string_field(&gate, "recommended_action").unwrap_or_default()
        ));
    }
}

fn render_evidence_grades(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Evidence Grade Calibration".to_string(),
        String::new(),
        "Structural `ok` remains the hard gate. Evidence-grade targets are an independent hardening queue.".to_string(),
        String::new(),
        "| Grade | Score | Required Cells Achieved | Required Cell Targets |".to_string(),
        "| --- | ---: | ---: | ---: |".to_string(),
    ]);
    for level in report
        .pointer("/evidence_grade_policy/levels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = string_field(level, "id").unwrap_or_default();
        lines.push(format!(
            "| `{id}` | {} | {} | {} |",
            level.get("score").and_then(Value::as_u64).unwrap_or(0),
            report
                .pointer(&format!(
                    "/evidence_grade_calibration/achieved_summary/{id}"
                ))
                .and_then(Value::as_u64)
                .unwrap_or(0),
            report
                .pointer(&format!("/evidence_grade_calibration/target_summary/{id}"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ));
    }

    lines.extend([
        String::new(),
        "### Weakest Coordinates".to_string(),
        String::new(),
    ]);
    let points = report
        .pointer("/evidence_grade_calibration/weakest_points")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if points.is_empty() {
        lines.push("No evidence-grade gaps.".to_string());
        return;
    }
    lines.push(
        "| Priority | Module | Paradigm | Achieved | Target | Steps | Next | Recommended Action |"
            .to_string(),
    );
    lines.push("| ---: | --- | --- | --- | --- | ---: | --- | --- |".to_string());
    for point in points {
        lines.push(format!(
            "| {} | `{}` | `{}` | `{}` | `{}` | {} | `{}` | `{}` |",
            point
                .get("priority_score")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            string_field(&point, "module_id").unwrap_or_default(),
            string_field(&point, "paradigm").unwrap_or_default(),
            string_field(&point, "achieved_grade").unwrap_or_default(),
            string_field(&point, "target_grade").unwrap_or_default(),
            point.get("gap_steps").and_then(Value::as_u64).unwrap_or(0),
            string_field(&point, "next_grade").unwrap_or_default(),
            string_field(&point, "recommended_action").unwrap_or_default()
        ));
    }
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
    lines.push("| Paradigm | Evidence | Modules | Files | Required Text |".to_string());
    lines.push("| --- | --- | --- | --- | --- |".to_string());
    for (paradigm, list) in entries {
        for entry in list.as_array().into_iter().flatten() {
            lines.push(format!(
                "| `{paradigm}` | `{}` | {} | {} | {} |",
                string_field(entry, "id").unwrap_or_default(),
                joined_or_dash(&string_array(entry, "modules")),
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

fn render_thin_points(report: &Value, lines: &mut Vec<String>) {
    lines.extend([
        String::new(),
        "## Thin Evidence Points".to_string(),
        String::new(),
    ]);
    let points = report
        .get("thin_points")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if points.is_empty() {
        lines.push("No thin evidence points.".to_string());
        return;
    }
    lines.push(
        "| Maturity | Module | Paradigm | Missing Dimensions | Present Dimensions | Contract Evidence | Test Commands |"
            .to_string(),
    );
    lines.push("| --- | --- | --- | --- | --- | ---: | ---: |".to_string());
    for point in points {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} |",
            string_field(&point, "maturity").unwrap_or_default(),
            string_field(&point, "module_id").unwrap_or_default(),
            string_field(&point, "paradigm").unwrap_or_default(),
            joined_or_dash(&string_array(&point, "missing_dimensions")),
            joined_or_dash(&string_array(&point, "present_dimensions")),
            point
                .get("contract_evidence_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            point
                .get("test_command_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
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

fn json_scalar(value: Option<&Value>) -> String {
    value
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "null".to_string())
}
