use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_ROOT: &str = "tmp/benchmark-profile";
const DEFAULT_COVERAGE_TARGETS: &str = "config/benchmark-profile-coverage.json";

type RunnerResult<T> = Result<T, String>;

mod markdown;

pub(crate) fn run_build_benchmark_profile_index(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let coverage_targets = read_coverage_targets(&options.coverage_targets)?;
    let discovery = discover_runs(&options.root)?;
    let gate = evaluate_gate(&discovery.runs, &discovery.skipped, &coverage_targets);
    let payload = json!({
        "schema_version": "kyuubiki.benchmark-profile-index/v1",
        "root": display_path(repo_root, &options.root),
        "coverage_targets_manifest": display_path(repo_root, &options.coverage_targets),
        "generated_at_unix_s": unix_seconds_now(),
        "gate": gate,
        "coverage_summaries": coverage_summaries(&discovery.runs, &coverage_targets),
        "matrix_summaries": matrix_summaries(&discovery.runs),
        "skipped_runs": discovery.skipped,
        "retained_runs": discovery.runs,
    });
    fs::create_dir_all(&options.root)
        .map_err(|error| format!("failed to create {}: {error}", options.root.display()))?;
    write_json(&options.root.join("index.json"), &payload)?;
    fs::write(
        options.root.join("README.md"),
        markdown::render_readme(
            &display_path(repo_root, &options.root),
            &display_path(repo_root, &options.coverage_targets),
            &payload,
        ),
    )
    .map_err(|error| format!("failed to write README.md: {error}"))?;
    println!("{}", display_path(repo_root, &options.root));
    Ok(0)
}

struct Options {
    coverage_targets: PathBuf,
    root: PathBuf,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        coverage_targets: repo_root.join(DEFAULT_COVERAGE_TARGETS),
        root: repo_root.join(DEFAULT_ROOT),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-benchmark-profile-index [--root tmp/benchmark-profile] [--coverage-targets config/benchmark-profile-coverage.json]"
                );
                return Ok(options);
            }
            "--root" => options.root = path_arg(repo_root, &mut iter, "--root")?,
            "--coverage-targets" => {
                options.coverage_targets = path_arg(repo_root, &mut iter, "--coverage-targets")?
            }
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    Ok(options)
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<PathBuf> {
    let value = iter
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))?;
    Ok(if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    })
}

#[derive(Clone, Debug)]
struct CoverageTarget {
    expected_cases: Vec<String>,
    matrix: String,
    profile: String,
}

fn read_coverage_targets(path: &Path) -> RunnerResult<Vec<CoverageTarget>> {
    let payload = read_json(path)?;
    let targets = payload
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "coverage targets {} must define a non-empty targets array",
                path.display()
            )
        })?;
    if targets.is_empty() {
        return Err(format!(
            "coverage targets {} must define a non-empty targets array",
            path.display()
        ));
    }
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| validate_coverage_target(path, target, index))
        .collect()
}

fn validate_coverage_target(
    path: &Path,
    target: &Value,
    index: usize,
) -> RunnerResult<CoverageTarget> {
    let prefix = format!("coverage target {index} in {}", path.display());
    let matrix = non_empty_string(target, "matrix")
        .ok_or_else(|| format!("{prefix} must define a non-empty matrix"))?;
    let profile = non_empty_string(target, "profile")
        .ok_or_else(|| format!("{prefix} must define a non-empty profile"))?;
    let expected = target
        .get("expected_cases")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{prefix} must define a non-empty expected_cases array"))?;
    if expected.is_empty() {
        return Err(format!(
            "{prefix} must define a non-empty expected_cases array"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut expected_cases = Vec::new();
    for (case_index, value) in expected.iter().enumerate() {
        let case = value
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| {
                format!("{prefix} expected_cases[{case_index}] must be a non-empty string")
            })?;
        if !seen.insert(case.to_string()) {
            return Err(format!("{prefix} has duplicate expected_cases entries"));
        }
        expected_cases.push(case.to_string());
    }
    Ok(CoverageTarget {
        expected_cases,
        matrix,
        profile,
    })
}

struct Discovery {
    runs: Vec<Value>,
    skipped: Vec<Value>,
}

fn discover_runs(root: &Path) -> RunnerResult<Discovery> {
    let mut runs = Vec::new();
    let mut skipped = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(Discovery { runs, skipped });
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", root.display()))?;
        let run_dir = entry.path();
        if !run_dir.is_dir() {
            continue;
        }
        let summary_path = run_dir.join("summary.json");
        if !summary_path.exists() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        match read_json(&summary_path) {
            Ok(summary) => runs.push(run_row(root, &slug, &run_dir, &summary_path, &summary)?),
            Err(error) => skipped.push(
                json!({ "slug": slug, "reason": format!("failed to parse summary.json: {error}") }),
            ),
        }
    }
    runs.sort_by(|left, right| {
        number_field(right, "generated_at_unix_s")
            .total_cmp(&number_field(left, "generated_at_unix_s"))
    });
    skipped.sort_by(|left, right| string_field(left, "slug").cmp(&string_field(right, "slug")));
    Ok(Discovery { runs, skipped })
}

fn run_row(
    root: &Path,
    slug: &str,
    run_dir: &Path,
    summary_path: &Path,
    summary: &Value,
) -> RunnerResult<Value> {
    let modified = summary_path
        .metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    Ok(json!({
        "slug": slug,
        "generated_at_unix_s": modified,
        "profile": string_field(summary, "profile").if_empty("unknown"),
        "matrix": string_field(summary, "matrix").if_empty("unknown"),
        "case_count": summary.get("case_count").cloned().unwrap_or_else(|| json!(0)),
        "case_ids": summary.get("case_ids").and_then(Value::as_array).map(|items| {
            items.iter().filter_map(Value::as_str).map(Value::from).collect::<Vec<_>>()
        }).unwrap_or_default(),
        "total_median_ms": summary.get("total_median_ms").cloned().unwrap_or_else(|| json!(0)),
        "peak_rss_mib": summary.get("peak_rss_mib").cloned().unwrap_or_else(|| json!(0)),
        "slowest_case": string_field(summary, "slowest_case").if_empty("--"),
        "files": {
            "summary_json": display_path(root, summary_path),
            "readme_md": display_path(root, &run_dir.join("README.md")),
        },
    }))
}

trait EmptyFallback {
    fn if_empty(self, fallback: &str) -> String;
}

impl EmptyFallback for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn evaluate_gate(runs: &[Value], skipped: &[Value], coverage_targets: &[CoverageTarget]) -> Value {
    let mut reasons = skipped
        .iter()
        .map(|item| {
            format!(
                "skipped run {}: {}",
                string_field(item, "slug"),
                string_field(item, "reason")
            )
        })
        .collect::<Vec<_>>();
    for entry in coverage_summaries(runs, coverage_targets) {
        let missing = number_field(&entry, "missing_case_count") as usize;
        let covered = number_field(&entry, "covered_case_count") as usize;
        if missing > 0 && covered > 0 {
            let missing_cases = array_strings(&entry, "missing_cases").join(", ");
            reasons.push(format!(
                "coverage {}/{} missing {missing} case(s): {missing_cases}",
                string_field(&entry, "matrix"),
                string_field(&entry, "profile")
            ));
        }
    }
    if runs.is_empty() {
        reasons.push("no retained benchmark profile runs".to_string());
        return json!({ "status": "warn", "reasons": reasons });
    }
    let latest = &runs[0];
    if latest
        .get("case_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        <= 0
    {
        reasons.push(format!(
            "latest run {} has no benchmark cases",
            string_field(latest, "slug")
        ));
    }
    if number_field(latest, "total_median_ms") <= 0.0 {
        reasons.push(format!(
            "latest run {} has invalid total median time",
            string_field(latest, "slug")
        ));
    }
    if number_field(latest, "peak_rss_mib") <= 0.0 {
        reasons.push(format!(
            "latest run {} has invalid peak RSS",
            string_field(latest, "slug")
        ));
    }
    json!({ "status": if reasons.is_empty() { "pass" } else { "warn" }, "reasons": reasons })
}

fn matrix_summaries(runs: &[Value]) -> Vec<Value> {
    let mut groups = BTreeMap::<String, MatrixSummary>::new();
    for run in runs {
        groups
            .entry(string_field(run, "matrix"))
            .or_insert_with(|| MatrixSummary::from_run(run))
            .add(run);
    }
    groups
        .into_values()
        .map(MatrixSummary::into_value)
        .collect()
}

struct MatrixSummary {
    case_count: f64,
    matrix: String,
    peak_rss_mib: f64,
    run_count: f64,
    slowest_case: String,
    slowest_case_median_ms: f64,
    total_median_ms: f64,
}

impl MatrixSummary {
    fn from_run(run: &Value) -> Self {
        Self {
            case_count: 0.0,
            matrix: string_field(run, "matrix"),
            peak_rss_mib: 0.0,
            run_count: 0.0,
            slowest_case: "--".to_string(),
            slowest_case_median_ms: 0.0,
            total_median_ms: 0.0,
        }
    }

    fn add(&mut self, run: &Value) {
        self.run_count += 1.0;
        self.case_count += number_field(run, "case_count");
        self.total_median_ms += number_field(run, "total_median_ms");
        self.peak_rss_mib = self.peak_rss_mib.max(number_field(run, "peak_rss_mib"));
        if number_field(run, "total_median_ms") > self.slowest_case_median_ms {
            self.slowest_case = string_field(run, "slowest_case");
            self.slowest_case_median_ms = number_field(run, "total_median_ms");
        }
    }

    fn into_value(self) -> Value {
        json!({
            "matrix": self.matrix,
            "run_count": self.run_count as u64,
            "case_count": self.case_count as u64,
            "total_median_ms": self.total_median_ms,
            "peak_rss_mib": self.peak_rss_mib,
            "slowest_case": self.slowest_case,
            "slowest_case_median_ms": self.slowest_case_median_ms,
        })
    }
}

fn coverage_summaries(runs: &[Value], coverage_targets: &[CoverageTarget]) -> Vec<Value> {
    coverage_targets
        .iter()
        .map(|target| {
            let observed = runs
                .iter()
                .filter(|run| {
                    string_field(run, "matrix") == target.matrix
                        && string_field(run, "profile") == target.profile
                })
                .flat_map(observed_case_ids)
                .collect::<BTreeSet<_>>();
            let covered_cases = target
                .expected_cases
                .iter()
                .filter(|case| observed.contains(*case))
                .cloned()
                .collect::<Vec<_>>();
            let missing_cases = target
                .expected_cases
                .iter()
                .filter(|case| !observed.contains(*case))
                .cloned()
                .collect::<Vec<_>>();
            json!({
                "matrix": target.matrix,
                "profile": target.profile,
                "expected_case_count": target.expected_cases.len(),
                "covered_case_count": covered_cases.len(),
                "missing_case_count": missing_cases.len(),
                "covered_cases": covered_cases,
                "missing_cases": missing_cases,
            })
        })
        .collect()
}

fn observed_case_ids(run: &Value) -> Vec<String> {
    let case_ids = array_strings(run, "case_ids");
    if case_ids.is_empty() {
        vec![normalize_case_id(&string_field(run, "slowest_case"))]
    } else {
        case_ids
            .iter()
            .map(|case| normalize_case_id(case))
            .collect()
    }
}

fn normalize_case_id(case_id: &str) -> String {
    case_id.split('#').next().unwrap_or(case_id).to_string()
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|error| format!("failed to encode {}: {error}", path.display()))?
        ),
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn non_empty_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
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

#[cfg(test)]
#[path = "benchmark_profile_index/tests.rs"]
mod tests;

fn unix_seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
