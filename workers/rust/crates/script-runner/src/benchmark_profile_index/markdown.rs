use serde_json::Value;

pub(super) fn render_readme(root_label: &str, coverage_label: &str, payload: &Value) -> String {
    let runs = array_field(payload, "retained_runs");
    let skipped = array_field(payload, "skipped_runs");
    let gate = payload.get("gate").unwrap_or(&Value::Null);
    let mut lines = vec![
        "# Benchmark Profile Runs".to_string(),
        String::new(),
        format!("- Root: `{root_label}`"),
        format!("- Coverage targets: `{coverage_label}`"),
        format!("- Indexed runs: `{}`", runs.len()),
        format!("- Skipped runs: `{}`", skipped.len()),
        format!("- Gate status: `{}`", string_field(gate, "status")),
        String::new(),
    ];
    let reasons = array_strings(gate, "reasons");
    for reason in &reasons {
        lines.push(format!("- Gate reason: {reason}"));
    }
    if !reasons.is_empty() {
        lines.push(String::new());
    }
    if runs.is_empty() {
        lines.push("No benchmark profile runs were found.".to_string());
        lines.push(String::new());
        return trim_join(lines);
    }
    render_matrix_table(&mut lines, payload);
    render_coverage_table(&mut lines, payload);
    render_runs_table(&mut lines, runs);
    render_skipped_table(&mut lines, skipped);
    trim_join(lines)
}

fn render_matrix_table(lines: &mut Vec<String>, payload: &Value) {
    lines.extend([
        "## Matrix summaries".to_string(),
        String::new(),
        "| Matrix | Runs | Cases | Total median ms | Peak RSS MiB | Slowest case |".to_string(),
        "| --- | ---: | ---: | ---: | ---: | --- |".to_string(),
    ]);
    for entry in array_field(payload, "matrix_summaries") {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{:.3}` | `{:.1}` | `{}` |",
            string_field(entry, "matrix"),
            number_field(entry, "run_count") as u64,
            number_field(entry, "case_count") as u64,
            number_field(entry, "total_median_ms"),
            number_field(entry, "peak_rss_mib"),
            string_field(entry, "slowest_case"),
        ));
    }
    lines.push(String::new());
}

fn render_coverage_table(lines: &mut Vec<String>, payload: &Value) {
    lines.extend([
        "## Coverage summaries".to_string(),
        String::new(),
        "| Matrix | Profile | Covered | Missing | Missing cases |".to_string(),
        "| --- | --- | ---: | ---: | --- |".to_string(),
    ]);
    for entry in array_field(payload, "coverage_summaries") {
        let missing = array_strings(entry, "missing_cases")
            .iter()
            .map(|case| format!("`{case}`"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "| `{}` | `{}` | `{}/{}` | `{}` | {} |",
            string_field(entry, "matrix"),
            string_field(entry, "profile"),
            number_field(entry, "covered_case_count") as u64,
            number_field(entry, "expected_case_count") as u64,
            number_field(entry, "missing_case_count") as u64,
            if missing.is_empty() {
                "--".to_string()
            } else {
                missing
            },
        ));
    }
    lines.push(String::new());
}

fn render_runs_table(lines: &mut Vec<String>, runs: &[Value]) {
    lines.extend([
        "## Runs".to_string(),
        String::new(),
        "| Slug | Profile | Matrix | Cases | Total median ms | Peak RSS MiB | Slowest case |"
            .to_string(),
        "| --- | --- | --- | ---: | ---: | ---: | --- |".to_string(),
    ]);
    for run in runs {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.1}` | `{}` |",
            string_field(run, "slug"),
            string_field(run, "profile"),
            string_field(run, "matrix"),
            number_field(run, "case_count") as u64,
            number_field(run, "total_median_ms"),
            number_field(run, "peak_rss_mib"),
            string_field(run, "slowest_case"),
        ));
    }
    lines.push(String::new());
}

fn render_skipped_table(lines: &mut Vec<String>, skipped: &[Value]) {
    if skipped.is_empty() {
        return;
    }
    lines.extend([
        "## Skipped runs".to_string(),
        String::new(),
        "| Slug | Reason |".to_string(),
        "| --- | --- |".to_string(),
    ]);
    for entry in skipped {
        lines.push(format!(
            "| `{}` | {} |",
            string_field(entry, "slug"),
            string_field(entry, "reason")
        ));
    }
    lines.push(String::new());
}

fn trim_join(mut lines: Vec<String>) -> String {
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    format!("{}\n", lines.join("\n"))
}

fn array_field<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn number_field(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn array_strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}
