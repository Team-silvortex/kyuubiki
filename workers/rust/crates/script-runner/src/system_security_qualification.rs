use crate::qualification_support::{
    generated_at_unix_ms, parse_options, portable_output, read_json, repo_path, write_json_compact,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/system-security-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.system-security-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.system-security-qualification-report/v1";
const REPORT_SCHEMA_PATH: &str = "schemas/system-security-qualification-report.schema.json";
const TOPOLOGY_PATH: &str = "config/architecture/module-topology.json";
const DEFAULT_OUT: &str = "tmp/system-security-qualification-report.json";
const OUTPUT_EXCERPT_LIMIT: usize = 32_000;

#[derive(Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    rounds: usize,
    required_modules: Vec<String>,
    required_lanes: Vec<String>,
    checks: Vec<CheckSpec>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CheckSpec {
    id: String,
    program: String,
    cwd: String,
    args: Vec<String>,
    coordinates: Vec<SecurityCoordinate>,
    required_output: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct SecurityCoordinate {
    module_id: String,
    lane: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    rounds: usize,
    checks: Vec<CheckReport>,
    modules: Vec<ModuleReport>,
    lanes: Vec<LaneReport>,
    summary: Summary,
}

#[derive(Clone, Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct CheckReport {
    id: String,
    program: String,
    cwd: String,
    args: Vec<String>,
    coordinates: Vec<SecurityCoordinate>,
    rounds: Vec<RoundReport>,
    repeatable: bool,
}

#[derive(Clone, Deserialize, Serialize)]
struct RoundReport {
    round: usize,
    status: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    output_sha256: String,
    assertions: Vec<AssertionReport>,
    output_excerpt: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct AssertionReport {
    text: String,
    passed: bool,
}

#[derive(Clone, Deserialize, Serialize)]
struct ModuleReport {
    module_id: String,
    status: String,
    covered_lanes: Vec<String>,
    check_ids: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct LaneReport {
    lane: String,
    status: String,
    covered_modules: Vec<String>,
    check_ids: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Summary {
    check_count: usize,
    round_count: usize,
    passed_round_count: usize,
    module_count: usize,
    lane_count: usize,
    coordinate_count: usize,
    failed_check_ids: Vec<String>,
}

pub(crate) fn run_check_system_security_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "system security qualification")?;
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    let topology: Value = read_json(root, TOPOLOGY_PATH)?;
    validate_contract(root, &contract, &topology)?;
    if options.self_test {
        run_self_test(&contract, &topology)?;
        println!("system security qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &topology, &report)?;
        println!("system security qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract, &topology)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json_compact(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &topology, &report) {
        eprintln!("system security qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "system security qualified: {} checks, {} rounds, {} module/lane coordinates",
        report.summary.check_count, report.rounds, report.summary.coordinate_count
    );
    println!("system security qualification report written: {out}");
    Ok(0)
}

fn validate_contract(
    root: &Path,
    contract: &QualificationContract,
    topology: &Value,
) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("system security qualification schemas are invalid".to_string());
    }
    let report_schema: Value = read_json(root, REPORT_SCHEMA_PATH)?;
    if report_schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("system security report schema const drifted".to_string());
    }
    if !(2..=4).contains(&contract.rounds) {
        return Err("system security qualification requires 2 to 4 rounds".to_string());
    }
    require_unique_nonempty(&contract.required_modules, "required module")?;
    require_unique_nonempty(&contract.required_lanes, "required security lane")?;
    if contract.required_modules.len() < 8 || contract.required_lanes.len() < 7 {
        return Err("system security qualification scope is too small".to_string());
    }
    if contract.checks.len() < 10 {
        return Err("system security qualification requires at least 10 checks".to_string());
    }

    let module_lanes = topology_module_lanes(topology)?;
    let required_modules = contract.required_modules.iter().collect::<BTreeSet<_>>();
    let required_lanes = contract.required_lanes.iter().collect::<BTreeSet<_>>();
    let mut check_ids = BTreeSet::new();
    let mut covered = BTreeSet::new();
    for check in &contract.checks {
        if check.id.is_empty() || !check_ids.insert(check.id.as_str()) {
            return Err(format!(
                "invalid or duplicate security check id {}",
                check.id
            ));
        }
        if !matches!(check.program.as_str(), "self" | "cargo" | "mix" | "python3") {
            return Err(format!(
                "security check {} uses an unsupported program",
                check.id
            ));
        }
        if check.args.is_empty()
            || check.required_output.is_empty()
            || check.coordinates.is_empty()
            || check.args.iter().any(|value| value.is_empty())
        {
            return Err(format!("security check {} is incomplete", check.id));
        }
        if check.program == "self"
            && check
                .args
                .first()
                .is_some_and(|value| value == "check-system-security-qualification")
        {
            return Err("system security qualification cannot recursively execute itself".into());
        }
        let cwd = repo_path(root, &check.cwd)?;
        if !cwd.is_dir() {
            return Err(format!("security check {} cwd does not exist", check.id));
        }
        let mut local_coordinates = BTreeSet::new();
        for coordinate in &check.coordinates {
            if !required_modules.contains(&coordinate.module_id)
                || !required_lanes.contains(&coordinate.lane)
                || !local_coordinates.insert(coordinate.clone())
            {
                return Err(format!(
                    "security check {} has an invalid coordinate",
                    check.id
                ));
            }
            let valid = module_lanes
                .get(&coordinate.module_id)
                .is_some_and(|lanes| lanes.contains(&coordinate.lane));
            if !valid {
                return Err(format!(
                    "security check {} maps topology-invalid coordinate {}/{}",
                    check.id, coordinate.module_id, coordinate.lane
                ));
            }
            covered.insert(coordinate.clone());
        }
    }
    let expected = expected_coordinates(contract, &module_lanes)?;
    let missing = expected.difference(&covered).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "system security contract misses coordinates: {missing:?}"
        ));
    }
    Ok(())
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
    topology: &Value,
) -> RunnerResult<QualificationReport> {
    let mut checks = Vec::new();
    for check in &contract.checks {
        let mut rounds = Vec::new();
        for round in 1..=contract.rounds {
            rounds.push(run_check_round(root, check, round)?);
        }
        let repeatable = rounds.iter().all(|receipt| receipt.status == "pass");
        checks.push(CheckReport {
            id: check.id.clone(),
            program: check.program.clone(),
            cwd: check.cwd.clone(),
            args: check.args.clone(),
            coordinates: check.coordinates.clone(),
            rounds,
            repeatable,
        });
    }
    build_report(contract, topology, checks, generated_at_unix_ms()?)
}

fn run_check_round(root: &Path, check: &CheckSpec, round: usize) -> RunnerResult<RoundReport> {
    let mut command = if check.program == "self" {
        Command::new(
            std::env::current_exe()
                .map_err(|error| format!("failed to resolve script runner: {error}"))?,
        )
    } else {
        Command::new(&check.program)
    };
    let started = Instant::now();
    let output = command
        .args(&check.args)
        .current_dir(repo_path(root, &check.cwd)?)
        .env("NO_COLOR", "1")
        .env("MIX_ENV", "test")
        .output()
        .map_err(|error| format!("failed to execute security check {}: {error}", check.id))?;
    let rendered = portable_output(root, &output);
    if rendered.chars().count() > OUTPUT_EXCERPT_LIMIT {
        return Err(format!(
            "security check {} output exceeds the retained evidence budget",
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
    let passed = output.status.success() && assertions.iter().all(|assertion| assertion.passed);
    Ok(RoundReport {
        round,
        status: if passed { "pass" } else { "fail" }.to_string(),
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        output_sha256: output_digest(&rendered),
        assertions,
        output_excerpt: rendered,
    })
}

fn build_report(
    contract: &QualificationContract,
    topology: &Value,
    checks: Vec<CheckReport>,
    generated_at_unix_ms: u128,
) -> RunnerResult<QualificationReport> {
    let module_lanes = topology_module_lanes(topology)?;
    let modules = contract
        .required_modules
        .iter()
        .map(|module_id| {
            let covered_lanes = module_lanes
                .get(module_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|lane| contract.required_lanes.contains(lane))
                .collect::<Vec<_>>();
            let check_ids = checks_for_module(&checks, module_id);
            let passed = covered_lanes.iter().all(|lane| {
                checks.iter().any(|check| {
                    check.repeatable
                        && check.coordinates.iter().any(|coordinate| {
                            coordinate.module_id == *module_id && coordinate.lane == *lane
                        })
                })
            });
            ModuleReport {
                module_id: module_id.clone(),
                status: if passed { "pass" } else { "fail" }.to_string(),
                covered_lanes,
                check_ids,
            }
        })
        .collect::<Vec<_>>();
    let lanes = contract
        .required_lanes
        .iter()
        .map(|lane| {
            let covered_modules = contract
                .required_modules
                .iter()
                .filter(|module_id| {
                    module_lanes
                        .get(*module_id)
                        .is_some_and(|lanes| lanes.contains(lane))
                })
                .cloned()
                .collect::<Vec<_>>();
            let check_ids = checks_for_lane(&checks, lane);
            let passed = covered_modules.iter().all(|module_id| {
                checks.iter().any(|check| {
                    check.repeatable
                        && check.coordinates.iter().any(|coordinate| {
                            coordinate.module_id == *module_id && coordinate.lane == *lane
                        })
                })
            });
            LaneReport {
                lane: lane.clone(),
                status: if passed { "pass" } else { "fail" }.to_string(),
                covered_modules,
                check_ids,
            }
        })
        .collect::<Vec<_>>();
    let failed_check_ids = checks
        .iter()
        .filter(|check| !check.repeatable)
        .map(|check| check.id.clone())
        .collect::<Vec<_>>();
    let passed_round_count = checks
        .iter()
        .flat_map(|check| &check.rounds)
        .filter(|round| round.status == "pass")
        .count();
    let coordinate_count = expected_coordinates(contract, &module_lanes)?.len();
    let status = if failed_check_ids.is_empty()
        && modules.iter().all(|module| module.status == "pass")
        && lanes.iter().all(|lane| lane.status == "pass")
    {
        "pass"
    } else {
        "fail"
    };
    Ok(QualificationReport {
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
            check_count: checks.len(),
            round_count: checks.len() * contract.rounds,
            passed_round_count,
            module_count: modules.len(),
            lane_count: lanes.len(),
            coordinate_count,
            failed_check_ids,
        },
        checks,
        modules,
        lanes,
    })
}

fn validate_report(
    contract: &QualificationContract,
    topology: &Value,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
        || report.rounds != contract.rounds
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
    {
        return Err("system security qualification report header is invalid".to_string());
    }
    if report.status != "pass" {
        return Err(format!(
            "system security checks failed: {}",
            report.summary.failed_check_ids.join(", ")
        ));
    }
    if report.checks.len() != contract.checks.len() {
        return Err("system security qualification check count drifted".to_string());
    }
    for spec in &contract.checks {
        let receipt = report
            .checks
            .iter()
            .find(|check| check.id == spec.id)
            .ok_or_else(|| format!("security report misses check {}", spec.id))?;
        if receipt.program != spec.program
            || receipt.cwd != spec.cwd
            || receipt.args != spec.args
            || receipt.coordinates != spec.coordinates
            || !receipt.repeatable
            || receipt.rounds.len() != contract.rounds
        {
            return Err(format!("security report check {} drifted", spec.id));
        }
        for (index, round) in receipt.rounds.iter().enumerate() {
            if round.round != index + 1
                || round.status != "pass"
                || round.exit_code != Some(0)
                || round.output_sha256 != output_digest(&round.output_excerpt)
                || round.assertions.len() != spec.required_output.len()
                || round.assertions.iter().any(|assertion| !assertion.passed)
            {
                return Err(format!("security report check {} round failed", spec.id));
            }
            for required in &spec.required_output {
                if !round
                    .assertions
                    .iter()
                    .any(|assertion| assertion.text == *required && assertion.passed)
                    || !round.output_excerpt.contains(required)
                {
                    return Err(format!(
                        "security report check {} misses assertion",
                        spec.id
                    ));
                }
            }
        }
    }
    let module_lanes = topology_module_lanes(topology)?;
    if report.modules.len() != contract.required_modules.len()
        || report.lanes.len() != contract.required_lanes.len()
        || report.modules.iter().any(|module| module.status != "pass")
        || report.lanes.iter().any(|lane| lane.status != "pass")
    {
        return Err("system security qualification coverage receipts failed".to_string());
    }
    let expected_rounds = contract.checks.len() * contract.rounds;
    let expected_coordinates = expected_coordinates(contract, &module_lanes)?.len();
    if report.summary.check_count != contract.checks.len()
        || report.summary.round_count != expected_rounds
        || report.summary.passed_round_count != expected_rounds
        || report.summary.module_count != contract.required_modules.len()
        || report.summary.lane_count != contract.required_lanes.len()
        || report.summary.coordinate_count != expected_coordinates
        || !report.summary.failed_check_ids.is_empty()
    {
        return Err("system security qualification summary failed".to_string());
    }
    Ok(())
}

fn topology_module_lanes(topology: &Value) -> RunnerResult<BTreeMap<String, Vec<String>>> {
    let modules = topology
        .get("modules")
        .and_then(Value::as_array)
        .ok_or_else(|| "module topology modules are invalid".to_string())?;
    let mut result = BTreeMap::new();
    for module in modules {
        let Some(id) = module.get("id").and_then(Value::as_str) else {
            continue;
        };
        let lanes = module
            .get("security_lanes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        result.insert(id.to_string(), lanes);
    }
    Ok(result)
}

fn expected_coordinates(
    contract: &QualificationContract,
    module_lanes: &BTreeMap<String, Vec<String>>,
) -> RunnerResult<BTreeSet<SecurityCoordinate>> {
    let required_lanes = contract.required_lanes.iter().collect::<BTreeSet<_>>();
    let mut coordinates = BTreeSet::new();
    for module_id in &contract.required_modules {
        let lanes = module_lanes
            .get(module_id)
            .ok_or_else(|| format!("required security module {module_id} is unknown"))?;
        if lanes.is_empty() {
            return Err(format!("required security module {module_id} has no lanes"));
        }
        for lane in lanes {
            if !required_lanes.contains(lane) {
                return Err(format!("required security lane {lane} is not declared"));
            }
            coordinates.insert(SecurityCoordinate {
                module_id: module_id.clone(),
                lane: lane.clone(),
            });
        }
    }
    Ok(coordinates)
}

fn checks_for_module(checks: &[CheckReport], module_id: &str) -> Vec<String> {
    checks
        .iter()
        .filter(|check| {
            check
                .coordinates
                .iter()
                .any(|coordinate| coordinate.module_id == module_id)
        })
        .map(|check| check.id.clone())
        .collect()
}

fn checks_for_lane(checks: &[CheckReport], lane: &str) -> Vec<String> {
    checks
        .iter()
        .filter(|check| {
            check
                .coordinates
                .iter()
                .any(|coordinate| coordinate.lane == lane)
        })
        .map(|check| check.id.clone())
        .collect()
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

fn run_self_test(contract: &QualificationContract, topology: &Value) -> RunnerResult<()> {
    let checks = contract
        .checks
        .iter()
        .map(|spec| CheckReport {
            id: spec.id.clone(),
            program: spec.program.clone(),
            cwd: spec.cwd.clone(),
            args: spec.args.clone(),
            coordinates: spec.coordinates.clone(),
            rounds: (1..=contract.rounds)
                .map(|round| {
                    let output_excerpt = spec.required_output.join("\n");
                    RoundReport {
                        round,
                        status: "pass".to_string(),
                        exit_code: Some(0),
                        elapsed_ms: 1,
                        output_sha256: output_digest(&output_excerpt),
                        assertions: spec
                            .required_output
                            .iter()
                            .map(|text| AssertionReport {
                                text: text.clone(),
                                passed: true,
                            })
                            .collect(),
                        output_excerpt,
                    }
                })
                .collect(),
            repeatable: true,
        })
        .collect();
    let report = build_report(contract, topology, checks, 1)?;
    validate_report(contract, topology, &report)?;
    let mut failed = report.clone();
    failed.checks[0].rounds[0].assertions[0].passed = false;
    if validate_report(contract, topology, &failed).is_ok() {
        return Err("system security self-test accepted a failed assertion".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::require_unique_nonempty;

    #[test]
    fn unique_contract_values_are_fail_closed() {
        assert!(require_unique_nonempty(&["a".into(), "b".into()], "value").is_ok());
        assert!(require_unique_nonempty(&["a".into(), "a".into()], "value").is_err());
        assert!(require_unique_nonempty(&[], "value").is_err());
    }
}
