use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str = "tmp/remote-material-research/summary.json";
const DEFAULT_MAX_STAGE_SHARE_PCT: f64 = 105.0;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_remote_material_stage_health(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    let result = if options.self_test {
        run_self_test()
    } else {
        let summary = read_repo_json(root, &options.input)?;
        check_summary(&summary, &options)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("remote material stage health failed: {issue}");
            Ok(1)
        }
    }
}

struct Options {
    input: String,
    max_stage_share_pct: f64,
    self_test: bool,
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        input: DEFAULT_INPUT.to_string(),
        max_stage_share_pct: DEFAULT_MAX_STAGE_SHARE_PCT,
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-remote-material-stage-health [--self-test] [--in tmp/remote-material-research/summary.json] [--max-stage-share-pct 105]"
                );
                return Ok(options);
            }
            "--self-test" => options.self_test = true,
            "--in" => options.input = required_value(&mut iter, "--in")?,
            "--max-stage-share-pct" => {
                options.max_stage_share_pct =
                    parse_finite(required_value(&mut iter, "--max-stage-share-pct")?)?;
            }
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    option_issues(&options)
        .into_iter()
        .next()
        .map_or(Ok(options), Err)
}

fn option_issues(options: &Options) -> Vec<String> {
    if !options.max_stage_share_pct.is_finite() || options.max_stage_share_pct <= 0.0 {
        vec!["--max-stage-share-pct must be a finite positive number".to_string()]
    } else {
        Vec::new()
    }
}

fn run_self_test() -> RunnerResult<String> {
    let options = Options {
        input: DEFAULT_INPUT.to_string(),
        max_stage_share_pct: DEFAULT_MAX_STAGE_SHARE_PCT,
        self_test: true,
    };
    assert_no_issues(&fake_summary(99.0, 10.0, "solve_spd_matvec"), &options)?;
    expect_issue(
        &fake_summary(106.0, 10.0, "solve_spd_matvec"),
        &options,
        "stage share",
    )?;
    expect_issue(
        &fake_summary(f64::NAN, 10.0, "solve_spd_matvec"),
        &options,
        "stage_share_pct",
    )?;
    expect_issue(
        &fake_summary(99.0, -1.0, "solve_spd_matvec"),
        &options,
        "elapsed_ms",
    )?;
    expect_issue(&fake_summary(99.0, 10.0, ""), &options, "stage")?;
    expect_issue(
        &with_empty_array(
            fake_summary(99.0, 10.0, "solve_spd_matvec"),
            "latest_stage_summary",
        ),
        &options,
        "latest_stage_summary",
    )?;
    expect_issue(
        &with_empty_array(
            fake_summary(99.0, 10.0, "solve_spd_matvec"),
            "latest_optimization_targets",
        ),
        &options,
        "latest_optimization_targets",
    )?;
    expect_issue(
        &with_empty_array(
            fake_summary(99.0, 10.0, "solve_spd_matvec"),
            "latest_preconditioner_economics",
        ),
        &options,
        "latest_preconditioner_economics",
    )?;
    expect_issue(
        &with_empty_array(
            fake_summary(99.0, 10.0, "solve_spd_matvec"),
            "latest_solver_tuning_notes",
        ),
        &options,
        "latest_solver_tuning_notes",
    )?;
    let mut bad_matvec = fake_summary(99.0, 10.0, "solve_spd_matvec");
    if let Some(row) = bad_matvec.pointer_mut("/latest_sparse_matvec_throughput/0/stage") {
        *row = Value::String("solve_spd_dot".to_string());
    }
    expect_issue(&bad_matvec, &options, "non-matvec")?;
    expect_issue(
        &json!({ "latest_stage_hotspots": [] }),
        &options,
        "no latest",
    )?;
    let bad_options = Options {
        max_stage_share_pct: 0.0,
        ..options
    };
    if !option_issues(&bad_options)
        .first()
        .is_some_and(|issue| issue.contains("positive"))
    {
        return Err("self-test did not reject invalid options".to_string());
    }
    Ok("remote material stage health self-test passed".to_string())
}

fn check_summary(summary: &Value, options: &Options) -> RunnerResult<String> {
    let issues = stage_health_issues(summary, options);
    if !issues.is_empty() {
        return Err(format!("\n- {}", issues.join("\n- ")));
    }
    let hotspot_count = array(summary, "latest_stage_hotspots").len();
    Ok(format!(
        "remote material stage health ok: {hotspot_count} hotspots, max share {}%",
        options.max_stage_share_pct
    ))
}

fn stage_health_issues(summary: &Value, options: &Options) -> Vec<String> {
    let hotspots = array(summary, "latest_stage_hotspots");
    if hotspots.is_empty() {
        return vec!["summary has no latest_stage_hotspots".to_string()];
    }
    let mut issues = Vec::new();
    issues.extend(optimization_target_issues(array(
        summary,
        "latest_optimization_targets",
    )));
    issues.extend(preconditioner_economics_issues(array(
        summary,
        "latest_preconditioner_economics",
    )));
    issues.extend(solver_tuning_note_issues(array(
        summary,
        "latest_solver_tuning_notes",
    )));
    issues.extend(sparse_matvec_throughput_issues(array(
        summary,
        "latest_sparse_matvec_throughput",
    )));
    issues.extend(stage_summary_issues(array(summary, "latest_stage_summary")));
    for item in hotspots {
        let context = stage_context(item, "case_id");
        if field(item, "stage").is_empty() {
            issues.push(format!("{context}: missing non-empty stage"));
        }
        if !finite_non_negative(item, "elapsed_ms") {
            issues.push(format!("{context}: missing finite non-negative elapsed_ms"));
        }
        match finite(item, "stage_share_pct") {
            Some(value) if value > options.max_stage_share_pct => issues.push(format!(
                "{context}: stage share {value:.2}% > {}%",
                options.max_stage_share_pct
            )),
            Some(_) => {}
            None => issues.push(format!("{context}: missing finite stage_share_pct")),
        }
    }
    issues
}

fn optimization_target_issues(rows: Vec<&Value>) -> Vec<String> {
    if rows.is_empty() {
        return vec!["summary has no latest_optimization_targets".to_string()];
    }
    let mut issues = Vec::new();
    for item in rows {
        let context = stage_context(item, "stage");
        require_text(
            item,
            "stage",
            "missing non-empty target stage",
            &context,
            &mut issues,
        );
        require_text(
            item,
            "focus",
            "missing non-empty target focus",
            &context,
            &mut issues,
        );
        require_positive(
            item,
            "case_count",
            "missing positive target case_count",
            &context,
            &mut issues,
        );
        require_non_negative(
            item,
            "elapsed_ms_total",
            "missing finite non-negative target elapsed_ms_total",
            &context,
            &mut issues,
        );
        require_non_negative(
            item,
            "priority_score",
            "missing finite non-negative priority_score",
            &context,
            &mut issues,
        );
        require_optional_non_negative(
            item,
            "ms_per_million_non_zero_visits",
            &context,
            &mut issues,
        );
    }
    issues
}

fn preconditioner_economics_issues(rows: Vec<&Value>) -> Vec<String> {
    if rows.is_empty() {
        return vec!["summary has no latest_preconditioner_economics".to_string()];
    }
    let mut issues = Vec::new();
    for item in rows {
        let context = format!(
            "{}/{}/{}",
            field(item, "matrix"),
            field(item, "profile"),
            field(item, "base_case_id")
        );
        require_text(
            item,
            "winner_preconditioner",
            "missing winner_preconditioner",
            &context,
            &mut issues,
        );
        require_text(
            item,
            "slowest_preconditioner",
            "missing slowest_preconditioner",
            &context,
            &mut issues,
        );
        for field_name in [
            "elapsed_saved_ms",
            "iterations_saved",
            "winner_speedup_ratio",
        ] {
            if finite(item, field_name).is_none() {
                issues.push(format!("{context}: missing finite {field_name}"));
            }
        }
        for field_name in [
            "extra_pre_ms_per_iteration_saved",
            "gross_non_preconditioner_saved_ms",
            "ms_saved_per_iteration_saved",
            "preconditioner_extra_ms",
            "winner_preconditioner_ms",
            "slowest_preconditioner_ms",
            "winner_pre_ms_per_iteration",
            "slowest_pre_ms_per_iteration",
            "winner_matvec_ms_per_iteration",
            "slowest_matvec_ms_per_iteration",
        ] {
            if item.get(field_name).is_some_and(|value| !value.is_null())
                && finite(item, field_name).is_none()
            {
                issues.push(format!("{context}: invalid {field_name}"));
            }
        }
    }
    issues
}

fn solver_tuning_note_issues(rows: Vec<&Value>) -> Vec<String> {
    if rows.is_empty() {
        return vec!["summary has no latest_solver_tuning_notes".to_string()];
    }
    let mut issues = Vec::new();
    for item in rows {
        let context = format!(
            "{}/{}/{}",
            field(item, "matrix"),
            field(item, "profile"),
            field(item, "case_id")
        );
        require_text(item, "focus", "missing tuning focus", &context, &mut issues);
        require_text(
            item,
            "reason",
            "missing tuning reason",
            &context,
            &mut issues,
        );
        for field_name in [
            "winner_pre_ms_per_iteration",
            "winner_matvec_ms_per_iteration",
        ] {
            if finite(item, field_name).is_none() {
                issues.push(format!("{context}: missing finite {field_name}"));
            }
        }
    }
    issues
}

fn sparse_matvec_throughput_issues(rows: Vec<&Value>) -> Vec<String> {
    let mut issues = Vec::new();
    for item in rows {
        let context = stage_context(item, "stage");
        if field(item, "stage") != "solve_spd_matvec" {
            issues.push(format!(
                "{context}: sparse matvec throughput row has non-matvec stage"
            ));
        }
        require_positive(
            item,
            "non_zero_visit_count",
            "missing positive non_zero_visit_count",
            &context,
            &mut issues,
        );
        require_non_negative(
            item,
            "ms_per_million_non_zero_visits",
            "missing finite non-negative ms_per_million_non_zero_visits",
            &context,
            &mut issues,
        );
    }
    issues
}

fn stage_summary_issues(rows: Vec<&Value>) -> Vec<String> {
    if rows.is_empty() {
        return vec!["summary has no latest_stage_summary".to_string()];
    }
    let mut issues = Vec::new();
    for item in rows {
        let context = stage_context(item, "stage");
        require_text(
            item,
            "stage",
            "missing non-empty summary stage",
            &context,
            &mut issues,
        );
        require_positive(
            item,
            "case_count",
            "missing positive summary case_count",
            &context,
            &mut issues,
        );
        require_non_negative(
            item,
            "elapsed_ms_total",
            "missing finite non-negative elapsed_ms_total",
            &context,
            &mut issues,
        );
        require_non_negative(
            item,
            "max_elapsed_ms",
            "missing finite non-negative max_elapsed_ms",
            &context,
            &mut issues,
        );
        require_optional_non_negative(
            item,
            "ms_per_million_non_zero_visits",
            &context,
            &mut issues,
        );
    }
    issues
}

fn fake_summary(stage_share_pct: f64, elapsed_ms: f64, stage: &str) -> Value {
    json!({
        "latest_stage_hotspots": [{
            "case_id": "panel-10k", "elapsed_ms": elapsed_ms, "matrix": "mechanical-core",
            "profile": "ten_k", "stage": stage, "stage_share_pct": stage_share_pct
        }],
        "latest_optimization_targets": [{
            "case_count": 1, "elapsed_ms_total": elapsed_ms, "focus": "sparse-matvec-throughput",
            "matrix": "mechanical-core", "priority_score": elapsed_ms, "profile": "ten_k", "stage": stage
        }],
        "latest_preconditioner_economics": [{
            "base_case_id": "panel-10k", "elapsed_saved_ms": 1, "extra_pre_ms_per_iteration_saved": 1,
            "gross_non_preconditioner_saved_ms": 2, "iterations_saved": 1, "matrix": "mechanical-core",
            "ms_saved_per_iteration_saved": 1, "preconditioner_extra_ms": 1, "profile": "ten_k",
            "slowest_preconditioner": "jacobi", "slowest_matvec_ms_per_iteration": 1,
            "slowest_preconditioner_ms": 1, "slowest_pre_ms_per_iteration": 1,
            "winner_matvec_ms_per_iteration": 1, "winner_preconditioner": "symmetric-gauss-seidel",
            "winner_preconditioner_ms": 2, "winner_pre_ms_per_iteration": 2, "winner_speedup_ratio": 1.2
        }],
        "latest_solver_tuning_notes": [{
            "case_id": "panel-10k", "focus": "sgs-sweep-cost", "matrix": "mechanical-core",
            "profile": "ten_k", "reason": "winner preconditioner cost per iteration is above matvec cost per iteration",
            "winner_matvec_ms_per_iteration": 1, "winner_pre_ms_per_iteration": 2
        }],
        "latest_sparse_matvec_throughput": [{
            "case_count": 1, "elapsed_ms_total": elapsed_ms, "matrix": "mechanical-core",
            "ms_per_million_non_zero_visits": 1, "non_zero_elapsed_ms_total": elapsed_ms,
            "non_zero_visit_count": 1_000_000, "profile": "ten_k", "stage": stage
        }],
        "latest_stage_summary": [{
            "case_count": 1, "elapsed_ms_total": elapsed_ms, "matrix": "mechanical-core",
            "max_elapsed_ms": elapsed_ms, "profile": "ten_k", "stage": stage
        }],
    })
}

fn with_empty_array(mut value: Value, key: &str) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_string(), Value::Array(Vec::new()));
    }
    value
}

fn assert_no_issues(summary: &Value, options: &Options) -> RunnerResult<()> {
    let issues = stage_health_issues(summary, options);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "self-test expected no issues, got {}",
            issues.join("; ")
        ))
    }
}

fn expect_issue(summary: &Value, options: &Options, needle: &str) -> RunnerResult<()> {
    if stage_health_issues(summary, options)
        .first()
        .is_some_and(|issue| issue.contains(needle))
    {
        Ok(())
    } else {
        Err(format!(
            "self-test did not produce issue containing {needle:?}"
        ))
    }
}

fn read_repo_json(root: &Path, input: &str) -> RunnerResult<Value> {
    let (absolute, relative) = repo_local_path(root, input, "--in")?;
    if !absolute.exists() {
        return Err(format!("input does not exist: {relative}"));
    }
    let text = fs::read_to_string(&absolute)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("failed to parse {relative}: {error}"))
}

fn repo_local_path(root: &Path, path: &str, label: &str) -> RunnerResult<(PathBuf, String)> {
    let absolute = root.join(path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{label} must stay inside the repository"))?
        .to_string_lossy()
        .replace('\\', "/");
    if relative.starts_with("..") || Path::new(&relative).is_absolute() {
        return Err(format!("{label} must stay inside the repository"));
    }
    Ok((absolute, relative))
}

fn required_value(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))
}

fn parse_finite(value: String) -> RunnerResult<f64> {
    value
        .parse::<f64>()
        .map_err(|_| "expected a finite number".to_string())
}

fn require_text(item: &Value, key: &str, message: &str, context: &str, issues: &mut Vec<String>) {
    if field(item, key).is_empty() {
        issues.push(format!("{context}: {message}"));
    }
}

fn require_positive(
    item: &Value,
    key: &str,
    message: &str,
    context: &str,
    issues: &mut Vec<String>,
) {
    if finite(item, key).is_none_or(|value| value <= 0.0) {
        issues.push(format!("{context}: {message}"));
    }
}

fn require_non_negative(
    item: &Value,
    key: &str,
    message: &str,
    context: &str,
    issues: &mut Vec<String>,
) {
    if !finite_non_negative(item, key) {
        issues.push(format!("{context}: {message}"));
    }
}

fn require_optional_non_negative(item: &Value, key: &str, context: &str, issues: &mut Vec<String>) {
    if item.get(key).is_some_and(|value| !value.is_null()) && !finite_non_negative(item, key) {
        issues.push(format!("{context}: invalid {key}"));
    }
}

fn stage_context(item: &Value, id_key: &str) -> String {
    format!(
        "{}/{}/{}",
        field(item, "matrix"),
        field(item, "profile"),
        field(item, id_key)
    )
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn finite_non_negative(value: &Value, key: &str) -> bool {
    finite(value, key).is_some_and(|value| value >= 0.0)
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests;
