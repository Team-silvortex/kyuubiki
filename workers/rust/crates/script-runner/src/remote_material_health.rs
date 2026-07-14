use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str = "tmp/remote-material-research/summary.json";
const DEFAULT_MIN_ITERATION_REDUCTION_PCT: f64 = 10.0;
const DEFAULT_MIN_SPEEDUP_RATIO: f64 = 1.05;
const DEFAULT_WINNER: &str = "symmetric-gauss-seidel";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_remote_material_preconditioner_health(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_preconditioner_args(args)?;
    let result = if options.self_test {
        run_preconditioner_self_test()
    } else {
        let summary = read_repo_json(root, &options.input)?;
        check_preconditioner_summary(&summary, &options)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("remote material preconditioner health failed: {issue}");
            Ok(1)
        }
    }
}

struct PreconditionerOptions {
    input: String,
    min_iteration_reduction_pct: f64,
    min_speedup_ratio: f64,
    self_test: bool,
    winner: String,
}

fn parse_preconditioner_args(args: Vec<OsString>) -> RunnerResult<PreconditionerOptions> {
    let mut options = PreconditionerOptions {
        input: DEFAULT_INPUT.to_string(),
        min_iteration_reduction_pct: DEFAULT_MIN_ITERATION_REDUCTION_PCT,
        min_speedup_ratio: DEFAULT_MIN_SPEEDUP_RATIO,
        self_test: false,
        winner: DEFAULT_WINNER.to_string(),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-remote-material-preconditioner-health [--self-test] [--in tmp/remote-material-research/summary.json] [--min-speedup-ratio 1.05] [--min-iteration-reduction-pct 10] [--winner symmetric-gauss-seidel]"
                );
                return Ok(options);
            }
            "--self-test" => options.self_test = true,
            "--in" => options.input = required_value(&mut iter, "--in")?,
            "--min-speedup-ratio" => {
                options.min_speedup_ratio =
                    parse_finite(required_value(&mut iter, "--min-speedup-ratio")?)?;
            }
            "--min-iteration-reduction-pct" => {
                options.min_iteration_reduction_pct =
                    parse_finite(required_value(&mut iter, "--min-iteration-reduction-pct")?)?;
            }
            "--winner" => options.winner = required_value(&mut iter, "--winner")?,
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    option_issues(&options)
        .into_iter()
        .next()
        .map_or(Ok(options), Err)
}

fn option_issues(options: &PreconditionerOptions) -> Vec<String> {
    let mut issues = Vec::new();
    if !options.min_speedup_ratio.is_finite() || options.min_speedup_ratio < 0.0 {
        issues.push("--min-speedup-ratio must be a finite non-negative number".to_string());
    }
    if !options.min_iteration_reduction_pct.is_finite() || options.min_iteration_reduction_pct < 0.0
    {
        issues
            .push("--min-iteration-reduction-pct must be a finite non-negative number".to_string());
    }
    if options.winner.trim().is_empty() {
        issues.push("--winner must be non-empty".to_string());
    }
    issues
}

fn run_preconditioner_self_test() -> RunnerResult<String> {
    let options = PreconditionerOptions {
        input: DEFAULT_INPUT.to_string(),
        min_iteration_reduction_pct: 10.0,
        min_speedup_ratio: 1.05,
        self_test: true,
        winner: DEFAULT_WINNER.to_string(),
    };
    assert_no_issues(&fake_summary(1.2, 25.0, DEFAULT_WINNER), &options)?;
    expect_issue(
        &fake_summary(1.01, 25.0, DEFAULT_WINNER),
        &options,
        "speedup",
    )?;
    expect_issue(
        &fake_summary(1.2, 5.0, DEFAULT_WINNER),
        &options,
        "iteration reduction",
    )?;
    expect_issue(&fake_summary(1.2, 25.0, "jacobi"), &options, "winner")?;
    expect_issue(
        &json!({ "latest_preconditioner_comparisons": [] }),
        &options,
        "no latest",
    )?;
    let bad_options = PreconditionerOptions {
        min_speedup_ratio: f64::NAN,
        ..options
    };
    if !option_issues(&bad_options)
        .first()
        .is_some_and(|issue| issue.contains("finite"))
    {
        return Err("self-test did not reject invalid options".to_string());
    }
    if parse_json_text("{", "broken.json").is_ok() {
        return Err("self-test did not reject invalid JSON".to_string());
    }
    Ok("remote material preconditioner health self-test passed".to_string())
}

fn assert_no_issues(summary: &Value, options: &PreconditionerOptions) -> RunnerResult<()> {
    let issues = preconditioner_health_issues(summary, options);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "self-test expected no issues, got {}",
            issues.join("; ")
        ))
    }
}

fn expect_issue(
    summary: &Value,
    options: &PreconditionerOptions,
    needle: &str,
) -> RunnerResult<()> {
    if preconditioner_health_issues(summary, options)
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

fn check_preconditioner_summary(
    summary: &Value,
    options: &PreconditionerOptions,
) -> RunnerResult<String> {
    let issues = preconditioner_health_issues(summary, options);
    if !issues.is_empty() {
        return Err(format!("\n- {}", issues.join("\n- ")));
    }
    let comparison_count = array(summary, "latest_preconditioner_comparisons").len();
    Ok(format!(
        "remote material preconditioner health ok: {comparison_count} comparisons, min speedup {}x",
        options.min_speedup_ratio
    ))
}

fn preconditioner_health_issues(summary: &Value, options: &PreconditionerOptions) -> Vec<String> {
    let comparisons = array(summary, "latest_preconditioner_comparisons");
    if comparisons.is_empty() {
        return vec!["summary has no latest_preconditioner_comparisons".to_string()];
    }
    let mut issues = Vec::new();
    for item in comparisons {
        let context = format!(
            "{}/{}/{}",
            field(item, "matrix"),
            field(item, "profile"),
            field(item, "base_case_id")
        );
        if field(item, "winner_preconditioner") != options.winner {
            issues.push(format!(
                "{context}: winner {} != {}",
                field(item, "winner_preconditioner"),
                options.winner
            ));
        }
        match finite(item, "winner_speedup_ratio") {
            Some(value) if value < options.min_speedup_ratio => issues.push(format!(
                "{context}: speedup {value:.3} < {}",
                options.min_speedup_ratio
            )),
            Some(_) => {}
            None => issues.push(format!("{context}: missing finite winner_speedup_ratio")),
        }
        match finite(item, "winner_iteration_reduction_pct") {
            Some(value) if value < options.min_iteration_reduction_pct => issues.push(format!(
                "{context}: iteration reduction {value:.2}% < {}%",
                options.min_iteration_reduction_pct
            )),
            Some(_) => {}
            None => issues.push(format!(
                "{context}: missing finite winner_iteration_reduction_pct"
            )),
        }
    }
    issues
}

fn fake_summary(speedup: f64, iteration_reduction_pct: f64, winner: &str) -> Value {
    json!({
        "latest_preconditioner_comparisons": [{
            "base_case_id": "panel-10k",
            "matrix": "mechanical-core",
            "profile": "ten_k",
            "winner_iteration_reduction_pct": iteration_reduction_pct,
            "winner_preconditioner": winner,
            "winner_speedup_ratio": speedup,
        }],
    })
}

fn read_repo_json(root: &Path, input: &str) -> RunnerResult<Value> {
    let (absolute, relative) = repo_local_path(root, input, "--in")?;
    if !absolute.exists() {
        return Err(format!("input does not exist: {relative}"));
    }
    let text = fs::read_to_string(&absolute)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    parse_json_text(&text, &relative)
}

fn parse_json_text(text: &str, context: &str) -> RunnerResult<Value> {
    serde_json::from_str(text).map_err(|error| format!("failed to parse {context}: {error}"))
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

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_WINNER, PreconditionerOptions, fake_summary, option_issues,
        preconditioner_health_issues,
    };

    #[test]
    fn preconditioner_health_accepts_good_summary() {
        let options = PreconditionerOptions {
            input: "tmp/x.json".to_string(),
            min_iteration_reduction_pct: 10.0,
            min_speedup_ratio: 1.05,
            self_test: false,
            winner: DEFAULT_WINNER.to_string(),
        };
        assert!(
            preconditioner_health_issues(&fake_summary(1.2, 25.0, DEFAULT_WINNER), &options)
                .is_empty()
        );
        assert!(
            preconditioner_health_issues(&fake_summary(1.01, 25.0, DEFAULT_WINNER), &options)[0]
                .contains("speedup")
        );
    }

    #[test]
    fn preconditioner_options_reject_bad_thresholds() {
        let options = PreconditionerOptions {
            input: "tmp/x.json".to_string(),
            min_iteration_reduction_pct: 10.0,
            min_speedup_ratio: f64::NAN,
            self_test: false,
            winner: DEFAULT_WINNER.to_string(),
        };
        assert!(option_issues(&options)[0].contains("finite"));
    }
}
