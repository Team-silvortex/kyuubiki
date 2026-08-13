use crate::qualification_support::{
    generated_at_unix_ms, parse_options, portable_output, read_json, repo_path, write_json_compact,
};
use kyuubiki_operator_sdk::{
    OperatorModelAuthoringPolicy, OperatorModelDraft, validate_operator_model_draft,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/contracts-validation-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.contracts-validation-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.contracts-validation-qualification-report/v1";
const REPORT_SCHEMA_PATH: &str = "schemas/contracts-validation-qualification-report.schema.json";
const MODEL_DRAFT_PATH: &str = "schemas/examples.operator-model-draft.json";
const DEFAULT_OUT: &str = "tmp/contracts-validation-qualification-report.json";
const OUTPUT_EXCERPT_LIMIT: usize = 8_192;

#[derive(Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    rounds: usize,
    required_families: Vec<String>,
    checks: Vec<CheckSpec>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CheckSpec {
    id: String,
    family: String,
    expected_outcome: String,
    fault_boundary: String,
    program: String,
    cwd: String,
    args: Vec<String>,
    required_output: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    rounds: usize,
    families: Vec<FamilyReport>,
    checks: Vec<CheckReport>,
    summary: Summary,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct FamilyReport {
    id: String,
    status: String,
    check_ids: Vec<String>,
    acceptance_count: usize,
    rejection_count: usize,
}

#[derive(Deserialize, Serialize)]
struct CheckReport {
    id: String,
    family: String,
    expected_outcome: String,
    fault_boundary: String,
    program: String,
    cwd: String,
    args: Vec<String>,
    rounds: Vec<RoundReport>,
    repeatable: bool,
    stable_output: bool,
}

#[derive(Deserialize, Serialize)]
struct RoundReport {
    round: usize,
    status: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    output_sha256: String,
    assertions: Vec<AssertionReport>,
    output_excerpt: String,
}

#[derive(Deserialize, Serialize)]
struct AssertionReport {
    text: String,
    passed: bool,
}

#[derive(Deserialize, Serialize)]
struct Summary {
    family_count: usize,
    check_count: usize,
    round_count: usize,
    passed_round_count: usize,
    acceptance_check_count: usize,
    rejection_check_count: usize,
    stable_check_count: usize,
    failed_check_ids: Vec<String>,
}

pub(crate) fn run_check_contracts_validation_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "contracts validation qualification")?;
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;

    if options.self_test {
        run_self_test(root, &contract)?;
        println!("contracts validation qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("contracts validation qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json_compact(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("contracts validation qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "contracts validation qualified: {} families, {} checks, {} rejection boundaries",
        report.summary.family_count,
        report.summary.check_count,
        report.summary.rejection_check_count
    );
    println!("contracts validation qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("contracts validation qualification schemas are invalid".to_string());
    }
    let report_schema: Value = read_json(root, REPORT_SCHEMA_PATH)?;
    if report_schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("contracts validation report schema const drifted".to_string());
    }
    if !(2..=3).contains(&contract.rounds) {
        return Err("contracts validation qualification requires 2 or 3 rounds".to_string());
    }
    require_unique_nonempty(&contract.required_families, "required contract family")?;
    if contract.required_families.len() < 6 || contract.checks.len() < 12 {
        return Err("contracts validation qualification scope is too small".to_string());
    }

    let families = contract.required_families.iter().collect::<BTreeSet<_>>();
    let mut check_ids = BTreeSet::new();
    let mut covered_families = BTreeSet::new();
    let mut rejection_boundaries = BTreeSet::new();
    let mut acceptance_count = 0usize;
    let mut rejection_count = 0usize;
    for check in &contract.checks {
        if check.id.is_empty() || !check_ids.insert(check.id.as_str()) {
            return Err(format!(
                "invalid or duplicate qualification check {}",
                check.id
            ));
        }
        if !families.contains(&check.family) {
            return Err(format!(
                "check {} uses an undeclared contract family",
                check.id
            ));
        }
        if check.fault_boundary.is_empty()
            || check.required_output.is_empty()
            || check.required_output.iter().any(String::is_empty)
            || check.args.is_empty()
            || check.args.iter().any(String::is_empty)
        {
            return Err(format!("qualification check {} is incomplete", check.id));
        }
        match check.expected_outcome.as_str() {
            "acceptance" => acceptance_count += 1,
            "rejection" => {
                rejection_count += 1;
                if !rejection_boundaries.insert(check.fault_boundary.as_str()) {
                    return Err(format!(
                        "qualification check {} duplicates a rejection boundary",
                        check.id
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "check {} has an invalid expected outcome",
                    check.id
                ));
            }
        }
        match check.program.as_str() {
            "self" => {
                if check
                    .args
                    .first()
                    .is_some_and(|arg| arg == "check-contracts-validation-qualification")
                {
                    return Err("contracts qualification cannot recursively execute itself".into());
                }
            }
            "internal" => validate_internal_boundary(&check.args)?,
            _ => return Err(format!("check {} uses an unsupported program", check.id)),
        }
        if !repo_path(root, &check.cwd)?.is_dir() {
            return Err(format!(
                "qualification check {} cwd does not exist",
                check.id
            ));
        }
        covered_families.insert(check.family.as_str());
    }
    if covered_families.len() != families.len() {
        return Err("not every required contract family has a qualification check".to_string());
    }
    if acceptance_count < 6 || rejection_count < 5 {
        return Err("qualification requires at least 6 acceptance and 5 rejection checks".into());
    }
    Ok(())
}

fn validate_internal_boundary(args: &[String]) -> RunnerResult<()> {
    let Some(boundary) = args.first().filter(|_| args.len() == 1) else {
        return Err("internal qualification checks require exactly one boundary".to_string());
    };
    if !matches!(
        boundary.as_str(),
        "operator-model-valid"
            | "operator-model-schema-mismatch"
            | "operator-model-side-effects"
            | "repository-path-escape"
    ) {
        return Err(format!(
            "unknown internal qualification boundary: {boundary}"
        ));
    }
    Ok(())
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let mut checks = Vec::new();
    for check in &contract.checks {
        let mut rounds = Vec::new();
        for round in 1..=contract.rounds {
            rounds.push(run_check_round(root, check, round)?);
        }
        let repeatable = rounds.iter().all(|receipt| receipt.status == "pass");
        let stable_output = rounds
            .iter()
            .map(|receipt| receipt.output_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == 1;
        checks.push(CheckReport {
            id: check.id.clone(),
            family: check.family.clone(),
            expected_outcome: check.expected_outcome.clone(),
            fault_boundary: check.fault_boundary.clone(),
            program: check.program.clone(),
            cwd: check.cwd.clone(),
            args: check.args.clone(),
            rounds,
            repeatable,
            stable_output,
        });
    }
    Ok(build_report(contract, checks, generated_at_unix_ms()?))
}

fn run_check_round(root: &Path, check: &CheckSpec, round: usize) -> RunnerResult<RoundReport> {
    let started = Instant::now();
    let (exit_code, rendered) = if check.program == "internal" {
        match run_internal_boundary(root, &check.args[0]) {
            Ok(output) => (Some(0), format!("{output}\n")),
            Err(error) => (Some(1), format!("{error}\n")),
        }
    } else {
        let output = Command::new(
            std::env::current_exe()
                .map_err(|error| format!("failed to resolve script runner: {error}"))?,
        )
        .args(&check.args)
        .current_dir(repo_path(root, &check.cwd)?)
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute contract check {}: {error}", check.id))?;
        (output.status.code(), portable_output(root, &output))
    };
    if rendered.chars().count() > OUTPUT_EXCERPT_LIMIT {
        return Err(format!(
            "contract check {} output exceeds the retained evidence budget",
            check.id
        ));
    }
    let assertions = check
        .required_output
        .iter()
        .map(|text| AssertionReport {
            text: text.clone(),
            passed: rendered.contains(text),
        })
        .collect::<Vec<_>>();
    let passed = exit_code == Some(0) && assertions.iter().all(|assertion| assertion.passed);
    Ok(RoundReport {
        round,
        status: if passed { "pass" } else { "fail" }.to_string(),
        exit_code,
        elapsed_ms: started.elapsed().as_millis(),
        output_sha256: output_digest(&rendered),
        assertions,
        output_excerpt: rendered,
    })
}

fn run_internal_boundary(root: &Path, boundary: &str) -> RunnerResult<String> {
    match boundary {
        "operator-model-valid" => {
            let draft: OperatorModelDraft = read_json(root, MODEL_DRAFT_PATH)?;
            let report =
                validate_operator_model_draft(&draft, &OperatorModelAuthoringPolicy::default());
            if !report.ok {
                return Err("repository operator model draft was rejected".to_string());
            }
            Ok("operator model draft accepted".to_string())
        }
        "operator-model-schema-mismatch" => {
            let mut draft: OperatorModelDraft = read_json(root, MODEL_DRAFT_PATH)?;
            draft.schema_version = "kyuubiki.operator-model-draft/v0".to_string();
            require_model_rejection(&draft, &["model_draft_schema_mismatch"])?;
            Ok("operator model schema mismatch rejected".to_string())
        }
        "operator-model-side-effects" => {
            let mut draft: OperatorModelDraft = read_json(root, MODEL_DRAFT_PATH)?;
            draft.implementation.side_effects = vec!["write arbitrary host files".to_string()];
            draft.input_json_schema = json!({ "type": "string" });
            require_model_rejection(
                &draft,
                &["model_side_effects_blocked", "model_json_schema_root_type"],
            )?;
            Ok("operator model side effects and open schema root rejected".to_string())
        }
        "repository-path-escape" => {
            if repo_path(root, "../outside-contract").is_ok() {
                return Err("repository path traversal was accepted".to_string());
            }
            Ok("repository path traversal rejected".to_string())
        }
        _ => Err(format!(
            "unknown internal qualification boundary: {boundary}"
        )),
    }
}

fn require_model_rejection(draft: &OperatorModelDraft, codes: &[&str]) -> RunnerResult<()> {
    let report = validate_operator_model_draft(draft, &OperatorModelAuthoringPolicy::default());
    if report.ok {
        return Err("invalid operator model draft was accepted".to_string());
    }
    for code in codes {
        if !report.issues.iter().any(|issue| issue.code == *code) {
            return Err(format!("operator model rejection misses issue code {code}"));
        }
    }
    Ok(())
}

fn build_report(
    contract: &QualificationContract,
    checks: Vec<CheckReport>,
    generated_at_unix_ms: u128,
) -> QualificationReport {
    let families = contract
        .required_families
        .iter()
        .map(|family| {
            let family_checks = checks
                .iter()
                .filter(|check| check.family == *family)
                .collect::<Vec<_>>();
            FamilyReport {
                id: family.clone(),
                status: if family_checks
                    .iter()
                    .all(|check| check.repeatable && check.stable_output)
                {
                    "pass"
                } else {
                    "fail"
                }
                .to_string(),
                check_ids: family_checks.iter().map(|check| check.id.clone()).collect(),
                acceptance_count: family_checks
                    .iter()
                    .filter(|check| check.expected_outcome == "acceptance")
                    .count(),
                rejection_count: family_checks
                    .iter()
                    .filter(|check| check.expected_outcome == "rejection")
                    .count(),
            }
        })
        .collect::<Vec<_>>();
    let failed_check_ids = checks
        .iter()
        .filter(|check| !check.repeatable || !check.stable_output)
        .map(|check| check.id.clone())
        .collect::<Vec<_>>();
    let passed_round_count = checks
        .iter()
        .flat_map(|check| &check.rounds)
        .filter(|round| round.status == "pass")
        .count();
    let status =
        if failed_check_ids.is_empty() && families.iter().all(|family| family.status == "pass") {
            "pass"
        } else {
            "fail"
        };
    QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms,
        contract_path: CONTRACT_PATH.to_string(),
        status: status.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        rounds: contract.rounds,
        summary: Summary {
            family_count: families.len(),
            check_count: checks.len(),
            round_count: checks.len() * contract.rounds,
            passed_round_count,
            acceptance_check_count: checks
                .iter()
                .filter(|check| check.expected_outcome == "acceptance")
                .count(),
            rejection_check_count: checks
                .iter()
                .filter(|check| check.expected_outcome == "rejection")
                .count(),
            stable_check_count: checks.iter().filter(|check| check.stable_output).count(),
            failed_check_ids,
        },
        families,
        checks,
    }
}

fn validate_report(
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
        || report.status != "pass"
        || report.rounds != contract.rounds
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
    {
        return Err("contracts validation qualification report header is invalid".to_string());
    }
    if report.checks.len() != contract.checks.len()
        || report.families.len() != contract.required_families.len()
    {
        return Err("contracts validation qualification report scope drifted".to_string());
    }
    for spec in &contract.checks {
        let check = report
            .checks
            .iter()
            .find(|check| check.id == spec.id)
            .ok_or_else(|| format!("qualification report misses check {}", spec.id))?;
        if check.family != spec.family
            || check.expected_outcome != spec.expected_outcome
            || check.fault_boundary != spec.fault_boundary
            || check.program != spec.program
            || check.cwd != spec.cwd
            || check.args != spec.args
            || !check.repeatable
            || !check.stable_output
            || check.rounds.len() != contract.rounds
        {
            return Err(format!("qualification report check {} drifted", spec.id));
        }
        for (index, round) in check.rounds.iter().enumerate() {
            if round.round != index + 1
                || round.status != "pass"
                || round.exit_code != Some(0)
                || round.output_sha256 != output_digest(&round.output_excerpt)
                || round.assertions.len() != spec.required_output.len()
                || round.assertions.iter().any(|assertion| !assertion.passed)
            {
                return Err(format!(
                    "qualification report check {} round failed",
                    spec.id
                ));
            }
            for required in &spec.required_output {
                if !round.output_excerpt.contains(required)
                    || !round
                        .assertions
                        .iter()
                        .any(|assertion| assertion.text == *required && assertion.passed)
                {
                    return Err(format!("qualification check {} misses assertion", spec.id));
                }
            }
        }
    }
    for family in &contract.required_families {
        let receipt = report
            .families
            .iter()
            .find(|receipt| receipt.id == *family)
            .ok_or_else(|| format!("qualification report misses family {family}"))?;
        let expected = contract
            .checks
            .iter()
            .filter(|check| check.family == *family)
            .collect::<Vec<_>>();
        if receipt.status != "pass"
            || receipt.check_ids
                != expected
                    .iter()
                    .map(|check| check.id.clone())
                    .collect::<Vec<_>>()
            || receipt.acceptance_count
                != expected
                    .iter()
                    .filter(|check| check.expected_outcome == "acceptance")
                    .count()
            || receipt.rejection_count
                != expected
                    .iter()
                    .filter(|check| check.expected_outcome == "rejection")
                    .count()
        {
            return Err(format!("qualification family {family} receipt drifted"));
        }
    }
    let expected_rounds = contract.checks.len() * contract.rounds;
    if report.summary.family_count != contract.required_families.len()
        || report.summary.check_count != contract.checks.len()
        || report.summary.round_count != expected_rounds
        || report.summary.passed_round_count != expected_rounds
        || report.summary.acceptance_check_count < 6
        || report.summary.rejection_check_count < 5
        || report.summary.stable_check_count != contract.checks.len()
        || !report.summary.failed_check_ids.is_empty()
    {
        return Err("contracts validation qualification summary failed".to_string());
    }
    Ok(())
}

fn run_self_test(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    let mut weak = contract.clone();
    weak.rounds = 1;
    if validate_contract(root, &weak).is_ok() {
        return Err("self-test accepted a single-round qualification".to_string());
    }
    let mut recursive = contract.clone();
    recursive.checks[0].args = vec!["check-contracts-validation-qualification".to_string()];
    if validate_contract(root, &recursive).is_ok() {
        return Err("self-test accepted recursive qualification execution".to_string());
    }
    run_internal_boundary(root, "operator-model-schema-mismatch")?;
    run_internal_boundary(root, "repository-path-escape")?;
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

fn output_digest(output: &str) -> String {
    format!("{:x}", Sha256::digest(output.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{output_digest, validate_internal_boundary};

    #[test]
    fn output_digest_is_stable() {
        assert_eq!(output_digest("contract"), output_digest("contract"));
    }

    #[test]
    fn internal_boundaries_are_closed() {
        assert!(validate_internal_boundary(&["repository-path-escape".to_string()]).is_ok());
        assert!(validate_internal_boundary(&["unknown".to_string()]).is_err());
    }
}
