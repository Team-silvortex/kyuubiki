use crate::qualification_support::{
    generated_at_unix_ms, parse_options, portable_output, read_json, repo_path, write_json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/headless-sdk-validation-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.headless-sdk-validation-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.headless-sdk-validation-qualification-report/v1";
const DEFAULT_OUT: &str = "tmp/headless-sdk-validation-qualification-report.json";

#[derive(Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    minimum_total_tests: usize,
    shared_fixtures: Vec<String>,
    language_suites: Vec<LanguageSuiteContract>,
    parity_cases: Vec<ParityCaseContract>,
}

#[derive(Deserialize)]
struct LanguageSuiteContract {
    id: String,
    minimum_tests: usize,
    source_root: String,
    source_extension: String,
    required_tests: Vec<String>,
}

#[derive(Deserialize)]
struct ParityCaseContract {
    id: String,
    python: String,
    elixir: String,
    rust: String,
}

impl ParityCaseContract {
    fn test_for(&self, language: &str) -> Option<&str> {
        match language {
            "python" => Some(&self.python),
            "elixir" => Some(&self.elixir),
            "rust" => Some(&self.rust),
            _ => None,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    total_passed: usize,
    suites: Vec<SuiteReport>,
    parity_cases: Vec<ParityCaseReport>,
    shared_fixtures: Vec<FixtureReport>,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct SuiteReport {
    id: String,
    command: Vec<String>,
    working_directory: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    passed: usize,
    failed: usize,
    skipped: usize,
    required_tests: Vec<CheckResult>,
    output_excerpt: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct CheckResult {
    id: String,
    passed: bool,
}

#[derive(Deserialize, Serialize)]
struct ParityCaseReport {
    id: String,
    passed: bool,
    languages: Vec<ParityLanguageReport>,
}

#[derive(Deserialize, Serialize)]
struct ParityLanguageReport {
    id: String,
    test: String,
    passed: bool,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
struct FixtureReport {
    path: String,
    sha256: String,
}

#[derive(Default)]
struct SuiteSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
}

struct SuiteCommand {
    command: Vec<String>,
    working_directory: String,
    env: Vec<(String, String)>,
}

pub(crate) fn run_check_headless_sdk_validation(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "Headless SDK validation qualification")?;
    if options.self_test {
        run_self_test()?;
        println!("Headless SDK validation qualification self-test passed");
        return Ok(0);
    }
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(root, &contract, &report)?;
        println!("Headless SDK validation qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, out, &report)?;
    if let Err(error) = validate_report(root, &contract, &report) {
        eprintln!("Headless SDK validation qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "Headless SDK validation qualified: {} tests across {} languages, {} parity cases",
        report.total_passed,
        report.suites.len(),
        report.parity_cases.len()
    );
    println!("Headless SDK validation qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("Headless SDK validation schemas are invalid".to_string());
    }
    if contract.minimum_total_tests < 250 || contract.parity_cases.len() < 10 {
        return Err("Headless SDK validation thresholds are too weak".to_string());
    }
    require_unique_nonempty(&contract.shared_fixtures, "shared fixture")?;
    for fixture in &contract.shared_fixtures {
        let _: Value = read_json(root, fixture)?;
    }

    let expected = BTreeMap::from([
        ("python", ("sdks/python/tests", "py")),
        ("elixir", ("sdks/elixir/test", "exs")),
        ("rust", ("sdks/rust/tests", "rs")),
    ]);
    let mut sources = BTreeMap::new();
    let mut language_ids = BTreeSet::new();
    for suite in &contract.language_suites {
        if !language_ids.insert(suite.id.as_str())
            || suite.minimum_tests < 50
            || expected.get(suite.id.as_str())
                != Some(&(suite.source_root.as_str(), suite.source_extension.as_str()))
        {
            return Err(format!("invalid Headless SDK suite {}", suite.id));
        }
        require_unique_nonempty(&suite.required_tests, "required SDK test")?;
        let source = read_source_tree(root, &suite.source_root, &suite.source_extension)?;
        for test in &suite.required_tests {
            if !source.contains(test) {
                return Err(format!("{} SDK required test drifted: {test}", suite.id));
            }
        }
        sources.insert(suite.id.as_str(), source);
    }
    if language_ids != expected.keys().copied().collect()
        || contract
            .language_suites
            .iter()
            .map(|suite| suite.minimum_tests)
            .sum::<usize>()
            < contract.minimum_total_tests
    {
        return Err("Headless SDK validation must cover all official languages".to_string());
    }

    let mut parity_ids = BTreeSet::new();
    for case in &contract.parity_cases {
        if case.id.is_empty() || !parity_ids.insert(case.id.as_str()) {
            return Err(format!("invalid Headless SDK parity case {}", case.id));
        }
        for language in expected.keys() {
            let test = case
                .test_for(language)
                .filter(|test| !test.is_empty())
                .ok_or_else(|| format!("parity case {} misses {language}", case.id))?;
            if !sources
                .get(language)
                .is_some_and(|source| source.contains(test))
            {
                return Err(format!(
                    "parity case {} drifted for {language}: {test}",
                    case.id
                ));
            }
        }
    }
    Ok(())
}

fn require_unique_nonempty(values: &[String], label: &str) -> RunnerResult<()> {
    let unique = values
        .iter()
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if values.is_empty() || unique.len() != values.len() {
        return Err(format!("{label} values must be non-empty and unique"));
    }
    Ok(())
}

fn read_source_tree(root: &Path, relative: &str, extension: &str) -> RunnerResult<String> {
    let mut files = Vec::new();
    collect_source_files(&repo_path(root, relative)?, extension, &mut files)?;
    files.sort();
    let mut combined = String::new();
    for path in files {
        combined.push_str(
            &fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
        );
        combined.push('\n');
    }
    Ok(combined)
}

fn collect_source_files(
    path: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> RunnerResult<()> {
    for entry in
        fs::read_dir(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?
    {
        let path = entry
            .map_err(|error| format!("failed to read source entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_source_files(&path, extension, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            files.push(path);
        }
    }
    Ok(())
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let mut suites = Vec::new();
    for suite in &contract.language_suites {
        suites.push(run_suite(root, suite)?);
    }
    let parity_cases = contract
        .parity_cases
        .iter()
        .map(|case| parity_report(case, &suites))
        .collect::<RunnerResult<Vec<_>>>()?;
    let shared_fixtures = contract
        .shared_fixtures
        .iter()
        .map(|path| fixture_report(root, path))
        .collect::<RunnerResult<Vec<_>>>()?;
    let total_passed = suites.iter().map(|suite| suite.passed).sum();
    let passed = total_passed >= contract.minimum_total_tests
        && suites.iter().all(|suite| {
            suite.exit_code == Some(0)
                && suite.failed == 0
                && suite.skipped == 0
                && suite.required_tests.iter().all(|test| test.passed)
        })
        && parity_cases.iter().all(|case| case.passed);
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        contract_path: CONTRACT_PATH.to_string(),
        status: if passed { "pass" } else { "fail" }.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        total_passed,
        suites,
        parity_cases,
        shared_fixtures,
    })
}

fn run_suite(root: &Path, contract: &LanguageSuiteContract) -> RunnerResult<SuiteReport> {
    let spec = suite_command(&contract.id)?;
    let started = Instant::now();
    let output = Command::new(&spec.command[0])
        .args(&spec.command[1..])
        .current_dir(root.join(&spec.working_directory))
        .envs(spec.env.iter().map(|(key, value)| (key, value)))
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute {} SDK tests: {error}", contract.id))?;
    let rendered = portable_output(root, &output);
    let summary = parse_suite_summary(&contract.id, &rendered)?;
    let suite_passed = output.status.success();
    let required_tests = contract
        .required_tests
        .iter()
        .map(|id| CheckResult {
            id: id.clone(),
            passed: suite_passed && rendered.contains(id),
        })
        .collect();
    Ok(SuiteReport {
        id: contract.id.clone(),
        command: spec.command,
        working_directory: spec.working_directory,
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        passed: summary.passed,
        failed: summary.failed,
        skipped: summary.skipped,
        required_tests,
        output_excerpt: rendered.chars().take(96_000).collect(),
    })
}

fn suite_command(language: &str) -> RunnerResult<SuiteCommand> {
    let strings = |values: &[&str]| values.iter().map(|value| (*value).to_string()).collect();
    match language {
        "python" => Ok(SuiteCommand {
            command: strings(&[
                "python3",
                "-m",
                "unittest",
                "discover",
                "-v",
                "-s",
                "sdks/python/tests",
            ]),
            working_directory: ".".to_string(),
            env: vec![
                ("PYTHONPATH".to_string(), "sdks/python".to_string()),
                ("PYTHONDONTWRITEBYTECODE".to_string(), "1".to_string()),
            ],
        }),
        "elixir" => Ok(SuiteCommand {
            command: strings(&["mix", "test", "--trace"]),
            working_directory: "sdks/elixir".to_string(),
            env: vec![("MIX_ENV".to_string(), "test".to_string())],
        }),
        "rust" => Ok(SuiteCommand {
            command: strings(&[
                "cargo",
                "test",
                "--manifest-path",
                "sdks/rust/Cargo.toml",
                "--",
                "--test-threads=1",
            ]),
            working_directory: ".".to_string(),
            env: vec![("CARGO_TERM_COLOR".to_string(), "never".to_string())],
        }),
        _ => Err(format!("unsupported Headless SDK language: {language}")),
    }
}

fn parse_suite_summary(language: &str, output: &str) -> RunnerResult<SuiteSummary> {
    match language {
        "python" => parse_python_summary(output),
        "elixir" => parse_elixir_summary(output),
        "rust" => parse_rust_summary(output),
        _ => Err(format!("unsupported Headless SDK language: {language}")),
    }
}

fn parse_python_summary(output: &str) -> RunnerResult<SuiteSummary> {
    let total = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Ran "))
        .filter_map(|line| line.split_whitespace().next()?.parse::<usize>().ok())
        .max()
        .ok_or_else(|| "Python SDK test summary is missing".to_string())?;
    let failed = named_count(output, "failures=") + named_count(output, "errors=");
    let skipped = named_count(output, "skipped=");
    Ok(SuiteSummary {
        passed: total.saturating_sub(failed + skipped),
        failed,
        skipped,
    })
}

fn parse_elixir_summary(output: &str) -> RunnerResult<SuiteSummary> {
    let line = output
        .lines()
        .rev()
        .find(|line| line.contains(" tests") && line.contains(" failures"))
        .ok_or_else(|| "Elixir SDK test summary is missing".to_string())?;
    let total = count_before_word(line, "tests")?;
    let failed = count_before_word(line, "failures")?;
    let skipped = count_before_word(line, "excluded").unwrap_or(0);
    Ok(SuiteSummary {
        passed: total.saturating_sub(failed + skipped),
        failed,
        skipped,
    })
}

fn parse_rust_summary(output: &str) -> RunnerResult<SuiteSummary> {
    let summaries = output
        .lines()
        .filter_map(parse_rust_summary_line)
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        return Err("Rust SDK test summary is missing".to_string());
    }
    Ok(SuiteSummary {
        passed: summaries.iter().map(|summary| summary.passed).sum(),
        failed: summaries.iter().map(|summary| summary.failed).sum(),
        skipped: summaries.iter().map(|summary| summary.skipped).sum(),
    })
}

fn parse_rust_summary_line(line: &str) -> Option<SuiteSummary> {
    let fields = line
        .trim()
        .strip_prefix("test result: ")?
        .split_once(". ")?
        .1;
    let mut summary = SuiteSummary::default();
    for field in fields.split(';') {
        let mut parts = field.trim().split_whitespace();
        let Some(value) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
            continue;
        };
        match parts.next() {
            Some("passed") => summary.passed = value,
            Some("failed") => summary.failed = value,
            Some("ignored") => summary.skipped = value,
            _ => {}
        }
    }
    Some(summary)
}

fn named_count(output: &str, name: &str) -> usize {
    output
        .match_indices(name)
        .filter_map(|(index, _)| {
            output[index + name.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<usize>()
                .ok()
        })
        .max()
        .unwrap_or(0)
}

fn count_before_word(line: &str, word: &str) -> RunnerResult<usize> {
    let tokens = line
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    tokens
        .windows(2)
        .find(|pair| pair[1] == word)
        .and_then(|pair| pair[0].parse::<usize>().ok())
        .ok_or_else(|| format!("test summary misses {word} count"))
}

fn parity_report(
    contract: &ParityCaseContract,
    suites: &[SuiteReport],
) -> RunnerResult<ParityCaseReport> {
    let mut languages = Vec::new();
    for language in ["python", "elixir", "rust"] {
        let test = contract
            .test_for(language)
            .ok_or_else(|| format!("parity case {} misses {language}", contract.id))?;
        let suite = suites
            .iter()
            .find(|suite| suite.id == language)
            .ok_or_else(|| format!("qualification report misses {language} suite"))?;
        languages.push(ParityLanguageReport {
            id: language.to_string(),
            test: test.to_string(),
            passed: suite.exit_code == Some(0) && suite.output_excerpt.contains(test),
        });
    }
    Ok(ParityCaseReport {
        id: contract.id.clone(),
        passed: languages.iter().all(|language| language.passed),
        languages,
    })
}

fn fixture_report(root: &Path, relative: &str) -> RunnerResult<FixtureReport> {
    let bytes = fs::read(repo_path(root, relative)?)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    Ok(FixtureReport {
        path: relative.to_string(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}

fn validate_report(
    root: &Path,
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.status != "pass"
        || report.generated_at_unix_ms == 0
    {
        return Err("Headless SDK qualification report header is invalid".to_string());
    }
    if report.suites.len() != contract.language_suites.len()
        || report.total_passed
            != report
                .suites
                .iter()
                .map(|suite| suite.passed)
                .sum::<usize>()
        || report.total_passed < contract.minimum_total_tests
    {
        return Err("Headless SDK qualification totals are invalid".to_string());
    }
    for required in &contract.language_suites {
        let suite = report
            .suites
            .iter()
            .find(|suite| suite.id == required.id)
            .ok_or_else(|| format!("report misses {} SDK suite", required.id))?;
        let expected = suite_command(&required.id)?;
        if suite.command != expected.command
            || suite.working_directory != expected.working_directory
            || suite.exit_code != Some(0)
            || suite.passed < required.minimum_tests
            || suite.failed != 0
            || suite.skipped != 0
            || suite.required_tests.len() != required.required_tests.len()
        {
            return Err(format!("{} SDK suite does not qualify", required.id));
        }
        for test in &required.required_tests {
            if !suite
                .required_tests
                .iter()
                .any(|result| result.id == *test && result.passed)
                || !suite.output_excerpt.contains(test)
            {
                return Err(format!("{} SDK report misses {test}", required.id));
            }
        }
    }
    if report.parity_cases.len() != contract.parity_cases.len() {
        return Err("Headless SDK parity case count drifted".to_string());
    }
    for required in &contract.parity_cases {
        let case = report
            .parity_cases
            .iter()
            .find(|case| case.id == required.id)
            .ok_or_else(|| format!("report misses parity case {}", required.id))?;
        if !case.passed || case.languages.len() != 3 {
            return Err(format!("parity case {} did not qualify", required.id));
        }
        for language in ["python", "elixir", "rust"] {
            let expected = required.test_for(language).unwrap_or_default();
            if !case
                .languages
                .iter()
                .any(|result| result.id == language && result.test == expected && result.passed)
            {
                return Err(format!("parity case {} misses {language}", required.id));
            }
        }
    }
    let expected_fixtures = contract
        .shared_fixtures
        .iter()
        .map(|path| fixture_report(root, path))
        .collect::<RunnerResult<Vec<_>>>()?;
    if report.shared_fixtures != expected_fixtures {
        return Err("Headless SDK shared fixture digests drifted".to_string());
    }
    Ok(())
}

fn run_self_test() -> RunnerResult<()> {
    let python = parse_python_summary("Ran 95 tests in 1.0s\n\nOK\n")?;
    let elixir = parse_elixir_summary("89 tests, 0 failures\n")?;
    let rust = parse_rust_summary(
        "test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\
test result: ok. 70 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n",
    )?;
    if (python.passed, python.failed, python.skipped) != (95, 0, 0)
        || (elixir.passed, elixir.failed, elixir.skipped) != (89, 0, 0)
        || (rust.passed, rust.failed, rust.skipped) != (79, 0, 0)
    {
        return Err("Headless SDK test summary parser self-test failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_all_official_sdk_test_summaries() {
        super::run_self_test().unwrap();
    }
}
