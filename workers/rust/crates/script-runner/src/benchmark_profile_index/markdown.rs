use serde_json::Value;

pub(super) fn render_readme(root_label: &str, coverage_label: &str, payload: &Value) -> String {
    let runs = array_field(payload, "retained_runs");
    let failures = array_field(payload, "failed_runs");
    let skipped = array_field(payload, "skipped_runs");
    let gate = payload.get("gate").unwrap_or(&Value::Null);
    let mut lines = vec![
        "# Benchmark Profile Runs".to_string(),
        String::new(),
        format!("- Root: `{root_label}`"),
        format!("- Coverage targets: `{coverage_label}`"),
        format!("- Indexed runs: `{}`", runs.len()),
        format!("- Failed runs: `{}`", failures.len()),
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
    render_profile_coverage_table(&mut lines, payload);
    render_coverage_table(&mut lines, payload);
    render_scale_limitations_table(&mut lines, payload);
    render_solver_strategy_table(&mut lines, payload);
    render_runs_table(&mut lines, runs);
    render_failures_table(&mut lines, failures);
    render_skipped_table(&mut lines, skipped);
    trim_join(lines)
}

fn render_scale_limitations_table(lines: &mut Vec<String>, payload: &Value) {
    let limitations = array_field(payload, "coverage_summaries")
        .iter()
        .flat_map(|entry| {
            array_field(entry, "below_scale_threshold_details")
                .iter()
                .filter_map(|detail| {
                    let reason = string_field(detail, "reason");
                    (!reason.is_empty()).then(|| {
                        (
                            string_field(entry, "matrix"),
                            string_field(entry, "profile"),
                            string_field(detail, "id"),
                            reason,
                            string_field(detail, "remediation"),
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if limitations.is_empty() {
        return;
    }
    lines.extend([
        "## Scale limitations".to_string(),
        String::new(),
        "| Matrix | Profile | Case | Current limitation | Planned remediation |".to_string(),
        "| --- | --- | --- | --- | --- |".to_string(),
    ]);
    for (matrix, profile, case, reason, remediation) in limitations {
        lines.push(format!(
            "| `{matrix}` | `{profile}` | `{case}` | {reason} | {} |",
            if remediation.is_empty() {
                "--"
            } else {
                &remediation
            }
        ));
    }
    lines.push(String::new());
}

fn render_profile_coverage_table(lines: &mut Vec<String>, payload: &Value) {
    let summaries = array_field(payload, "profile_coverage_summaries");
    if summaries.is_empty() {
        return;
    }
    lines.extend([
        "## Profile coverage summaries".to_string(),
        String::new(),
        "| Profile | Covered | Scale-qualified | Below threshold | Missing |".to_string(),
        "| --- | ---: | ---: | ---: | ---: |".to_string(),
    ]);
    for entry in summaries {
        let threshold = entry
            .get("scale_qualified_node_threshold")
            .and_then(Value::as_u64);
        lines.push(format!(
            "| `{}` | `{}/{}` | {} | `{}` | `{}` |",
            string_field(entry, "profile"),
            number_field(entry, "covered_case_count") as u64,
            number_field(entry, "expected_case_count") as u64,
            threshold.map_or_else(
                || "--".to_string(),
                |value| {
                    format!(
                        "{}/{} (>= {value} nodes)",
                        number_field(entry, "scale_qualified_covered_case_count") as u64,
                        number_field(entry, "expected_case_count") as u64,
                    )
                }
            ),
            number_field(entry, "below_scale_threshold_case_count") as u64,
            number_field(entry, "missing_case_count") as u64,
        ));
    }
    lines.push(String::new());
}

fn render_solver_strategy_table(lines: &mut Vec<String>, payload: &Value) {
    let summaries = array_field(payload, "solver_strategy_summaries");
    if summaries.is_empty() {
        return;
    }
    lines.extend([
        "## Solver strategy summaries".to_string(),
        String::new(),
        "| Matrix | Profile | Case | Strategy | Reason | Iterations | Latest median ms | Peak RSS MiB |"
            .to_string(),
        "| --- | --- | --- | --- | --- | ---: | ---: | ---: |".to_string(),
    ]);
    for summary in summaries {
        for strategy in array_field(summary, "strategies") {
            lines.push(format!(
                "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.1}` |",
                string_field(summary, "matrix"),
                string_field(summary, "profile"),
                string_field(summary, "case_id"),
                string_field(strategy, "preconditioner"),
                string_field(strategy, "solver_preconditioner_reason"),
                number_field(strategy, "solver_iterations") as u64,
                number_field(strategy, "median_ms"),
                number_field(strategy, "peak_rss_mib"),
            ));
        }
    }
    lines.push(String::new());
}

fn render_failures_table(lines: &mut Vec<String>, failures: &[Value]) {
    if failures.is_empty() {
        return;
    }
    lines.extend([
        "## Failed runs".to_string(),
        String::new(),
        "| Slug | Profile | Matrix | Case | Phase | Kind | Exit | Resolved | Host |".to_string(),
        "| --- | --- | --- | --- | --- | --- | ---: | --- | --- |".to_string(),
    ]);
    for failure in failures {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
            string_field(failure, "slug"),
            string_field(failure, "profile"),
            string_field(failure, "matrix"),
            string_field(failure, "case"),
            string_field(failure, "phase"),
            string_field(failure, "failure_kind"),
            number_field(failure, "exit_code") as u64,
            if failure
                .get("resolved_by_success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                format!("yes: {}", string_field(failure, "resolved_by_slug"))
            } else {
                "no".to_string()
            },
            string_field(failure, "remote_host"),
        ));
    }
    lines.push(String::new());
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
        "| Matrix | Profile | Covered | Scale-qualified | Below threshold | Missing | Missing cases |"
            .to_string(),
        "| --- | --- | ---: | ---: | ---: | ---: | --- |".to_string(),
    ]);
    for entry in array_field(payload, "coverage_summaries") {
        let missing = array_strings(entry, "missing_cases")
            .iter()
            .map(|case| format!("`{case}`"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "| `{}` | `{}` | `{}/{}` | `{}/{}` | `{}` | `{}` | {} |",
            string_field(entry, "matrix"),
            string_field(entry, "profile"),
            number_field(entry, "covered_case_count") as u64,
            number_field(entry, "expected_case_count") as u64,
            number_field(entry, "scale_qualified_covered_case_count") as u64,
            if entry
                .get("scale_qualified_node_threshold")
                .is_some_and(|value| !value.is_null())
            {
                number_field(entry, "expected_case_count") as u64
            } else {
                0
            },
            number_field(entry, "below_scale_threshold_case_count") as u64,
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
        "| Slug | Profile | Matrix | Solver | Cases | Total median ms | Peak RSS MiB | Slowest case |"
            .to_string(),
        "| --- | --- | --- | --- | ---: | ---: | ---: | --- |".to_string(),
    ]);
    for run in runs {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.1}` | `{}` |",
            string_field(run, "slug"),
            string_field(run, "profile"),
            string_field(run, "matrix"),
            array_strings(run, "solver_preconditioners").join(", "),
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
