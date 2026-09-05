use serde::{Deserialize, Serialize};

pub(crate) const HELP_TEXT: &str = r#"Kyuubiki benchmark runner

Usage:
  kyuubiki-benchmark [options]

Options:
  --matrix <name>                       Benchmark matrix (default: core)
  --profile <profile>                   medium|large|v2|10k|15k|20k|100k|200k|300k|400k|500k|1m
  --case <substring>                    Run matching cases only
  --case-exact <id>                     Run one exact case ID
  --case-isolation <mode>               process|in-process (default: process)
  --repeat <count>                      Positive execution count (default: 10)
  --format <table|json>                 Report format (default: table)
  --solver-preconditioner <name>        jacobi|sgs|ic0|auto|all|compare (default: auto)
  --baseline-out <path>                 Write a baseline report
  --baseline-compare <path>             Compare against a baseline report
  --compare-report-out <path>           Write the comparison report
  --fail-on-median-regression-pct <n>   Reject median-time regression above n
  --fail-on-rss-regression-pct <n>      Reject RSS regression above n
  --min-baseline-median-ms <n>          Minimum baseline duration for timing gates
  --progress                            Stream case progress
  --dry-run-shapes                      Report workload shapes without execution
  -h, --help                            Show this help and exit
"#;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BenchmarkConfig {
    pub(crate) repeat: usize,
    pub(crate) case_filter: Option<String>,
    pub(crate) exact_case_id: Option<String>,
    pub(crate) case_isolation: CaseIsolation,
    pub(crate) matrix: String,
    pub(crate) format: OutputFormat,
    pub(crate) profile: BenchmarkProfile,
    pub(crate) baseline_out: Option<String>,
    pub(crate) baseline_compare: Option<String>,
    pub(crate) compare_report_out: Option<String>,
    pub(crate) solver_preconditioner: String,
    pub(crate) progress: bool,
    pub(crate) dry_run_shapes: bool,
    pub(crate) fail_on_median_regression_pct: Option<f64>,
    pub(crate) fail_on_rss_regression_pct: Option<f64>,
    pub(crate) min_baseline_median_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaseIsolation {
    Process,
    InProcess,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BenchmarkCommand {
    Run(Box<BenchmarkConfig>),
    Help,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            repeat: 10,
            case_filter: None,
            exact_case_id: None,
            case_isolation: CaseIsolation::Process,
            matrix: "core".to_string(),
            format: OutputFormat::Table,
            profile: BenchmarkProfile::TenK,
            baseline_out: None,
            baseline_compare: None,
            compare_report_out: None,
            solver_preconditioner: "auto".to_string(),
            progress: false,
            dry_run_shapes: false,
            fail_on_median_regression_pct: None,
            fail_on_rss_regression_pct: None,
            min_baseline_median_ms: 5.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BenchmarkProfile {
    Medium,
    Large,
    V2,
    TenK,
    FifteenK,
    TwentyK,
    HundredK,
    TwoHundredK,
    ThreeHundredK,
    FourHundredK,
    FiveHundredK,
    OneMillion,
}

impl BenchmarkProfile {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Medium => "medium",
            Self::Large => "large",
            Self::V2 => "v2",
            Self::TenK => "10k",
            Self::FifteenK => "15k",
            Self::TwentyK => "20k",
            Self::HundredK => "100k",
            Self::TwoHundredK => "200k",
            Self::ThreeHundredK => "300k",
            Self::FourHundredK => "400k",
            Self::FiveHundredK => "500k",
            Self::OneMillion => "1m",
        }
    }
}

impl BenchmarkConfig {
    pub(crate) fn from_env() -> Result<BenchmarkCommand, String> {
        Self::from_args(std::env::args().skip(1))
    }

    pub(crate) fn from_args(
        args: impl IntoIterator<Item = String>,
    ) -> Result<BenchmarkCommand, String> {
        let mut config = Self::default();

        let args = args.into_iter().collect::<Vec<_>>();
        if args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
        {
            return Ok(BenchmarkCommand::Help);
        }
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--repeat" => {
                    let value = required_value(&mut args, "--repeat")?;
                    config.repeat = parse_positive_usize(&value, "--repeat")?;
                }
                "--case" => {
                    config.case_filter = Some(required_value(&mut args, "--case")?);
                }
                "--case-exact" => {
                    config.exact_case_id = Some(required_value(&mut args, "--case-exact")?);
                }
                "--case-isolation" => {
                    config.case_isolation =
                        parse_case_isolation(&required_value(&mut args, "--case-isolation")?)?;
                }
                "--matrix" => {
                    config.matrix = required_value(&mut args, "--matrix")?;
                }
                "--format" => {
                    config.format = match required_value(&mut args, "--format")?.as_str() {
                        "table" => OutputFormat::Table,
                        "json" => OutputFormat::Json,
                        value => return Err(format!("unsupported benchmark format: {value}")),
                    };
                }
                "--profile" => {
                    config.profile = parse_profile(&required_value(&mut args, "--profile")?)?;
                }
                "--baseline-out" => {
                    config.baseline_out = Some(required_value(&mut args, "--baseline-out")?);
                }
                "--baseline-compare" => {
                    config.baseline_compare =
                        Some(required_value(&mut args, "--baseline-compare")?);
                }
                "--compare-report-out" => {
                    config.compare_report_out =
                        Some(required_value(&mut args, "--compare-report-out")?);
                }
                "--solver-preconditioner" => {
                    config.solver_preconditioner =
                        required_value(&mut args, "--solver-preconditioner")?;
                }
                "--progress" => {
                    config.progress = true;
                }
                "--dry-run-shapes" => {
                    config.dry_run_shapes = true;
                }
                "--fail-on-median-regression-pct" => {
                    let value = required_value(&mut args, "--fail-on-median-regression-pct")?;
                    config.fail_on_median_regression_pct = Some(parse_nonnegative_f64(
                        &value,
                        "--fail-on-median-regression-pct",
                    )?);
                }
                "--fail-on-rss-regression-pct" => {
                    let value = required_value(&mut args, "--fail-on-rss-regression-pct")?;
                    config.fail_on_rss_regression_pct = Some(parse_nonnegative_f64(
                        &value,
                        "--fail-on-rss-regression-pct",
                    )?);
                }
                "--min-baseline-median-ms" => {
                    let value = required_value(&mut args, "--min-baseline-median-ms")?;
                    config.min_baseline_median_ms =
                        parse_nonnegative_f64(&value, "--min-baseline-median-ms")?;
                }
                other => return Err(format!("unknown benchmark argument: {other}")),
            }
        }

        validate_solver_preconditioner(&config.solver_preconditioner)?;
        validate_comparison_options(&config)?;
        if config.case_filter.is_some() && config.exact_case_id.is_some() {
            return Err("--case and --case-exact cannot be combined".to_string());
        }
        Ok(BenchmarkCommand::Run(Box::new(config)))
    }
}

fn validate_comparison_options(config: &BenchmarkConfig) -> Result<(), String> {
    let comparison_output_requested = config.compare_report_out.is_some()
        || config.fail_on_median_regression_pct.is_some()
        || config.fail_on_rss_regression_pct.is_some();
    if comparison_output_requested && config.baseline_compare.is_none() {
        return Err(
            "comparison reports and regression gates require --baseline-compare".to_string(),
        );
    }

    let report_option_requested = config.baseline_out.is_some()
        || config.baseline_compare.is_some()
        || config.compare_report_out.is_some()
        || config.fail_on_median_regression_pct.is_some()
        || config.fail_on_rss_regression_pct.is_some();
    if config.dry_run_shapes && report_option_requested {
        return Err(
            "--dry-run-shapes cannot be combined with benchmark report options".to_string(),
        );
    }

    Ok(())
}

fn required_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .filter(|value| !value.is_empty() && !value.starts_with("--"))
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_positive_usize(value: &str, option: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} requires a positive integer, received '{value}'"))
}

fn parse_nonnegative_f64(value: &str, option: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| {
            format!("{option} requires a finite non-negative number, received '{value}'")
        })
}

fn parse_profile(value: &str) -> Result<BenchmarkProfile, String> {
    match value {
        "medium" => Ok(BenchmarkProfile::Medium),
        "large" => Ok(BenchmarkProfile::Large),
        "v2" => Ok(BenchmarkProfile::V2),
        "10k" => Ok(BenchmarkProfile::TenK),
        "15k" => Ok(BenchmarkProfile::FifteenK),
        "20k" => Ok(BenchmarkProfile::TwentyK),
        "100k" => Ok(BenchmarkProfile::HundredK),
        "200k" => Ok(BenchmarkProfile::TwoHundredK),
        "300k" => Ok(BenchmarkProfile::ThreeHundredK),
        "400k" => Ok(BenchmarkProfile::FourHundredK),
        "500k" => Ok(BenchmarkProfile::FiveHundredK),
        "1m" | "1000k" | "one_million" => Ok(BenchmarkProfile::OneMillion),
        other => Err(format!("unsupported benchmark profile: {other}")),
    }
}

fn parse_case_isolation(value: &str) -> Result<CaseIsolation, String> {
    match value {
        "process" => Ok(CaseIsolation::Process),
        "in-process" => Ok(CaseIsolation::InProcess),
        other => Err(format!("unsupported case isolation mode: {other}")),
    }
}

fn validate_solver_preconditioner(value: &str) -> Result<(), String> {
    match value {
        "jacobi"
        | "sgs"
        | "symmetric-gauss-seidel"
        | "ic0"
        | "incomplete-cholesky"
        | "auto"
        | "all"
        | "compare" => Ok(()),
        other => Err(format!("unsupported solver preconditioner: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkCommand, BenchmarkConfig, BenchmarkProfile, CaseIsolation, OutputFormat,
        parse_profile, validate_solver_preconditioner,
    };

    fn parse(args: &[&str]) -> Result<BenchmarkCommand, String> {
        BenchmarkConfig::from_args(args.iter().map(|value| value.to_string()))
    }

    #[test]
    fn help_short_circuits_without_starting_a_benchmark() {
        assert_eq!(parse(&["--help"]).unwrap(), BenchmarkCommand::Help);
        assert_eq!(parse(&["--unknown", "-h"]).unwrap(), BenchmarkCommand::Help);
    }

    #[test]
    fn parses_explicit_values_without_silent_fallback() {
        let BenchmarkCommand::Run(config) =
            parse(&["--repeat", "3", "--format", "json", "--profile", "1m"]).unwrap()
        else {
            panic!("expected runnable benchmark config");
        };
        assert_eq!(config.repeat, 3);
        assert_eq!(config.format, OutputFormat::Json);
        assert_eq!(config.profile, BenchmarkProfile::OneMillion);
        assert_eq!(config.case_isolation, CaseIsolation::Process);
    }

    #[test]
    fn rejects_unknown_missing_and_invalid_arguments() {
        assert_eq!(
            parse(&["--unknown"]).unwrap_err(),
            "unknown benchmark argument: --unknown"
        );
        assert_eq!(
            parse(&["--repeat"]).unwrap_err(),
            "--repeat requires a value"
        );
        assert_eq!(
            parse(&["--repeat", "0"]).unwrap_err(),
            "--repeat requires a positive integer, received '0'"
        );
        assert_eq!(
            parse(&["--format", "yaml"]).unwrap_err(),
            "unsupported benchmark format: yaml"
        );
        assert!(parse(&["--fail-on-rss-regression-pct", "NaN"]).is_err());
        assert!(parse(&["--case-isolation", "thread"]).is_err());
    }

    #[test]
    fn parses_case_isolation_and_rejects_ambiguous_filters() {
        let BenchmarkCommand::Run(config) = parse(&[
            "--case-exact",
            "frame-2d-100k",
            "--case-isolation",
            "in-process",
        ])
        .unwrap() else {
            panic!("expected runnable benchmark config");
        };
        assert_eq!(config.exact_case_id.as_deref(), Some("frame-2d-100k"));
        assert_eq!(config.case_isolation, CaseIsolation::InProcess);
        assert!(parse(&["--case", "frame", "--case-exact", "frame-2d-100k"]).is_err());
    }

    #[test]
    fn comparison_outputs_and_gates_require_a_baseline() {
        for args in [
            &["--compare-report-out", "comparison.md"][..],
            &["--fail-on-median-regression-pct", "5"][..],
            &["--fail-on-rss-regression-pct", "5"][..],
        ] {
            assert!(parse(args).unwrap_err().contains("--baseline-compare"));
        }
        assert!(parse(&["--baseline-compare", "baseline.json"]).is_ok());
    }

    #[test]
    fn dry_run_rejects_ignored_report_options() {
        let error = parse(&["--dry-run-shapes", "--baseline-out", "baseline.json"]).unwrap_err();

        assert!(error.contains("cannot be combined"));
    }

    #[test]
    fn rejects_unknown_profiles_without_panicking() {
        assert_eq!(
            parse_profile("invalid").unwrap_err(),
            "unsupported benchmark profile: invalid"
        );
        assert_eq!(parse_profile("1m").unwrap(), BenchmarkProfile::OneMillion);
    }

    #[test]
    fn rejects_unknown_preconditioners_without_silent_fallback() {
        assert_eq!(
            validate_solver_preconditioner("not-a-preconditioner").unwrap_err(),
            "unsupported solver preconditioner: not-a-preconditioner"
        );
        assert!(validate_solver_preconditioner("symmetric-gauss-seidel").is_ok());
        assert!(validate_solver_preconditioner("auto").is_ok());
    }
}
