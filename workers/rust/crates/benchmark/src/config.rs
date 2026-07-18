use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BenchmarkConfig {
    pub(crate) repeat: usize,
    pub(crate) case_filter: Option<String>,
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
    pub(crate) fn from_env() -> Result<Self, String> {
        let mut config = Self {
            repeat: 10,
            case_filter: None,
            matrix: "core".to_string(),
            format: OutputFormat::Table,
            profile: BenchmarkProfile::TenK,
            baseline_out: None,
            baseline_compare: None,
            compare_report_out: None,
            solver_preconditioner: "jacobi".to_string(),
            progress: false,
            dry_run_shapes: false,
            fail_on_median_regression_pct: None,
            fail_on_rss_regression_pct: None,
            min_baseline_median_ms: 5.0,
        };

        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--repeat" => {
                    if let Some(value) = args.next() {
                        config.repeat = value.parse().unwrap_or(config.repeat).max(1);
                    }
                }
                "--case" => {
                    if let Some(value) = args.next() {
                        config.case_filter = Some(value.clone());
                    }
                }
                "--matrix" => {
                    if let Some(value) = args.next() {
                        config.matrix = value.clone();
                    }
                }
                "--format" => {
                    if let Some(value) = args.next() {
                        config.format = match value.as_str() {
                            "json" => OutputFormat::Json,
                            _ => OutputFormat::Table,
                        };
                    }
                }
                "--profile" => {
                    if let Some(value) = args.next() {
                        config.profile = parse_profile(value)?;
                    }
                }
                "--baseline-out" => {
                    if let Some(value) = args.next() {
                        config.baseline_out = Some(value.clone());
                    }
                }
                "--baseline-compare" => {
                    if let Some(value) = args.next() {
                        config.baseline_compare = Some(value.clone());
                    }
                }
                "--compare-report-out" => {
                    if let Some(value) = args.next() {
                        config.compare_report_out = Some(value.clone());
                    }
                }
                "--solver-preconditioner" => {
                    if let Some(value) = args.next() {
                        config.solver_preconditioner = value.clone();
                    }
                }
                "--progress" => {
                    config.progress = true;
                }
                "--dry-run-shapes" => {
                    config.dry_run_shapes = true;
                }
                "--fail-on-median-regression-pct" => {
                    if let Some(value) = args.next() {
                        config.fail_on_median_regression_pct = value.parse().ok();
                    }
                }
                "--fail-on-rss-regression-pct" => {
                    if let Some(value) = args.next() {
                        config.fail_on_rss_regression_pct = value.parse().ok();
                    }
                }
                "--min-baseline-median-ms" => {
                    if let Some(value) = args.next() {
                        config.min_baseline_median_ms =
                            value.parse().unwrap_or(config.min_baseline_median_ms);
                    }
                }
                _ => {}
            }
        }

        validate_solver_preconditioner(&config.solver_preconditioner)?;
        Ok(config)
    }
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
    use super::{BenchmarkProfile, parse_profile, validate_solver_preconditioner};

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
