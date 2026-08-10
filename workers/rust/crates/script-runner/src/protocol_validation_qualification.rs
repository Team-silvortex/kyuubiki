use kyuubiki_protocol::{
    JobStatus, OperatorTaskDigestError, OperatorTaskSummaryErrorCode, ProgressEvent, RPC_VERSION,
    RpcEnvelopeErrorCode, RpcError, RpcMethod, RpcProgress, RpcProtocolDescriptor, RpcRequest,
    RpcResponse, preview_operator_task_execution, summarize_operator_task_execution_checked,
    validate_rpc_progress_envelope, validate_rpc_request_envelope, validate_rpc_response_envelope,
    verify_operator_task_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/protocol-validation-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.protocol-validation-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.protocol-validation-qualification-report/v1";
const DEFAULT_OUT: &str = "tmp/protocol-validation-qualification-report.json";

#[derive(Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    minimum_protocol_tests: usize,
    minimum_rpc_methods: usize,
    minimum_task_ir_count: usize,
    task_ir_examples: Vec<String>,
    required_authoring_modes: Vec<String>,
    required_tests: Vec<String>,
    fuzz_profiles: Vec<FuzzProfileContract>,
}

#[derive(Deserialize)]
struct FuzzProfileContract {
    id: String,
    source: String,
    constant: String,
    cases: usize,
    test: String,
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    protocol_tests: ProtocolTestReport,
    fuzz_profiles: Vec<FuzzProfileReport>,
    task_ir: TaskIrReport,
    rpc: RpcReport,
    contract_checks: Vec<CommandReport>,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct ProtocolTestReport {
    command: Vec<String>,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    passed: usize,
    failed: usize,
    ignored: usize,
    measured: usize,
    filtered_out: usize,
    required_tests: Vec<CheckResult>,
    output_excerpt: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct CheckResult {
    id: String,
    passed: bool,
}

#[derive(Deserialize, Serialize)]
struct FuzzProfileReport {
    id: String,
    cases: usize,
    test: String,
    passed: bool,
}

#[derive(Deserialize, Serialize)]
struct TaskIrReport {
    task_count: usize,
    verified_digest_count: usize,
    execution_summary_count: usize,
    tamper_rejection_count: usize,
    authoring_modes: Vec<String>,
    structured_rejection_codes: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct RpcReport {
    advertised_method_count: usize,
    round_trip_count: usize,
    unique_wire_method_count: usize,
    unknown_method_rejected: bool,
    boundary_rejection_codes: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct CommandReport {
    command: Vec<String>,
    status: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    output: String,
}

#[derive(Default)]
struct TestSummary {
    passed: usize,
    failed: usize,
    ignored: usize,
    measured: usize,
    filtered_out: usize,
}

#[derive(Default)]
struct Options {
    out: Option<String>,
    verify_report: Option<String>,
    self_test: bool,
}

pub(crate) fn run_check_protocol_validation_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args)?;
    if options.self_test {
        run_self_test()?;
        println!("protocol validation qualification self-test passed");
        return Ok(0);
    }
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("protocol validation qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("protocol validation qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "protocol validation qualified: {} tests, {} RPC methods, {} TaskIR examples",
        report.protocol_tests.passed, report.rpc.advertised_method_count, report.task_ir.task_count
    );
    println!("protocol validation qualification report written: {out}");
    Ok(0)
}

fn parse_options(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => options.out = Some(required_path(&mut iter, "--out")?),
            "--verify-report" => {
                options.verify_report = Some(required_path(&mut iter, "--verify-report")?)
            }
            "--self-test" => options.self_test = true,
            other => return Err(format!("unknown protocol qualification argument: {other}")),
        }
    }
    if options.out.is_some() && options.verify_report.is_some() {
        return Err("--out and --verify-report cannot be combined".to_string());
    }
    Ok(options)
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("protocol qualification schemas are invalid".to_string());
    }
    if contract.minimum_protocol_tests < 50
        || contract.minimum_rpc_methods < 20
        || contract.minimum_task_ir_count < 3
    {
        return Err("protocol qualification thresholds are too weak".to_string());
    }
    require_unique_nonempty(&contract.task_ir_examples, "TaskIR example")?;
    require_unique_nonempty(&contract.required_authoring_modes, "authoring mode")?;
    require_unique_nonempty(&contract.required_tests, "required test")?;
    for path in &contract.task_ir_examples {
        let _: Value = read_json(root, path)?;
    }
    let required_tests = contract.required_tests.iter().collect::<BTreeSet<_>>();
    let mut fuzz_ids = BTreeSet::new();
    for profile in &contract.fuzz_profiles {
        if profile.id.is_empty()
            || profile.constant.is_empty()
            || profile.cases == 0
            || !fuzz_ids.insert(profile.id.as_str())
            || !required_tests.contains(&profile.test)
        {
            return Err(format!("invalid fuzz profile {}", profile.id));
        }
        let source = fs::read_to_string(repo_path(root, &profile.source)?)
            .map_err(|error| format!("failed to read {}: {error}", profile.source))?;
        let declaration = format!("const {}: usize = {};", profile.constant, profile.cases);
        if !source.contains(&declaration) {
            return Err(format!("fuzz profile {} source count drifted", profile.id));
        }
    }
    if contract
        .fuzz_profiles
        .iter()
        .map(|profile| profile.cases)
        .sum::<usize>()
        < 1_000
    {
        return Err("protocol qualification requires at least 1000 fuzz cases".to_string());
    }
    Ok(())
}

fn require_unique_nonempty(values: &[String], label: &str) -> RunnerResult<()> {
    let unique = values
        .iter()
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if unique.len() != values.len() || values.is_empty() {
        return Err(format!("{label} values must be non-empty and unique"));
    }
    Ok(())
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let protocol_tests = run_protocol_tests(root, contract)?;
    let fuzz_profiles = contract
        .fuzz_profiles
        .iter()
        .map(|profile| FuzzProfileReport {
            id: profile.id.clone(),
            cases: profile.cases,
            test: profile.test.clone(),
            passed: protocol_tests
                .required_tests
                .iter()
                .any(|test| test.id == profile.test && test.passed),
        })
        .collect::<Vec<_>>();
    let task_ir = qualify_task_ir(root, contract)?;
    let rpc = qualify_rpc()?;
    let contract_checks = vec![
        run_runner_command(&["check-operator-task-ir-contract"])?,
        run_runner_command(&["check-operator-task-ir-contract", "--self-test"])?,
    ];
    let passed = protocol_tests.exit_code == Some(0)
        && protocol_tests.failed == 0
        && protocol_tests.passed >= contract.minimum_protocol_tests
        && protocol_tests.required_tests.iter().all(|test| test.passed)
        && fuzz_profiles.iter().all(|profile| profile.passed)
        && task_ir.task_count >= contract.minimum_task_ir_count
        && rpc.advertised_method_count >= contract.minimum_rpc_methods
        && contract_checks.iter().all(|check| check.status == "pass");
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock before epoch: {error}"))?
            .as_millis(),
        contract_path: CONTRACT_PATH.to_string(),
        status: if passed { "pass" } else { "fail" }.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        protocol_tests,
        fuzz_profiles,
        task_ir,
        rpc,
        contract_checks,
    })
}

fn run_protocol_tests(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<ProtocolTestReport> {
    let command = [
        "cargo",
        "test",
        "-p",
        "kyuubiki-protocol",
        "--",
        "--test-threads=1",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root.join("workers/rust"))
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute protocol tests: {error}"))?;
    let rendered = portable_output(root, &output);
    let summary = parse_test_summary(&rendered);
    let required_tests = contract
        .required_tests
        .iter()
        .map(|id| CheckResult {
            id: id.clone(),
            passed: test_passed(&rendered, id),
        })
        .collect();
    Ok(ProtocolTestReport {
        command,
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        passed: summary.passed,
        failed: summary.failed,
        ignored: summary.ignored,
        measured: summary.measured,
        filtered_out: summary.filtered_out,
        required_tests,
        output_excerpt: rendered.chars().take(32_000).collect(),
    })
}

fn parse_test_summary(output: &str) -> TestSummary {
    output
        .lines()
        .filter_map(parse_test_summary_line)
        .max_by_key(|summary| summary.passed)
        .unwrap_or_default()
}

fn parse_test_summary_line(line: &str) -> Option<TestSummary> {
    let fields = line.trim().strip_prefix("test result: ok. ")?;
    let mut summary = TestSummary::default();
    for field in fields.split(';') {
        let mut parts = field.trim().split_whitespace();
        let Some(value) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
            continue;
        };
        match parts.next() {
            Some("passed") => summary.passed = value,
            Some("failed") => summary.failed = value,
            Some("ignored") => summary.ignored = value,
            Some("measured") => summary.measured = value,
            Some("filtered") if parts.next() == Some("out") => summary.filtered_out = value,
            _ => {}
        }
    }
    Some(summary)
}

fn test_passed(output: &str, id: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim() == format!("test {id} ... ok"))
}

fn qualify_task_ir(root: &Path, contract: &QualificationContract) -> RunnerResult<TaskIrReport> {
    let mut tasks = Vec::new();
    for path in &contract.task_ir_examples {
        collect_task_ir(&read_json::<Value>(root, path)?, &mut tasks);
    }
    let mut verified_digest_count = 0;
    let mut execution_summary_count = 0;
    let mut tamper_rejection_count = 0;
    let mut authoring_modes = BTreeSet::new();
    for task in &tasks {
        verify_operator_task_digest(task)
            .map_err(|error| format!("TaskIR digest verification failed: {error:?}"))?;
        verified_digest_count += 1;
        summarize_operator_task_execution_checked(task)
            .map_err(|error| format!("TaskIR summary failed: {}", error.message))?;
        preview_operator_task_execution(task)
            .map_err(|error| format!("TaskIR preview failed: {}", error.message))?;
        execution_summary_count += 1;
        if let Some(mode) = task
            .pointer("/descriptor_authoring/mode")
            .and_then(Value::as_str)
        {
            authoring_modes.insert(mode.to_string());
        }
        let mut tampered = task.clone();
        tampered["config"]["_qualification_tamper"] = json!(true);
        if matches!(
            verify_operator_task_digest(&tampered),
            Err(OperatorTaskDigestError::Mismatch { .. })
        ) {
            tamper_rejection_count += 1;
        }
    }
    let fixture = tasks
        .first()
        .ok_or_else(|| "protocol qualification found no TaskIR examples".to_string())?;
    let structured_rejection_codes = structured_task_ir_rejections(fixture)?;
    Ok(TaskIrReport {
        task_count: tasks.len(),
        verified_digest_count,
        execution_summary_count,
        tamper_rejection_count,
        authoring_modes: authoring_modes.into_iter().collect(),
        structured_rejection_codes,
    })
}

fn collect_task_ir(value: &Value, tasks: &mut Vec<Value>) {
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_some_and(|schema| schema == "kyuubiki.operator-task-ir/v1")
    {
        tasks.push(value.clone());
    }
    match value {
        Value::Array(items) => items.iter().for_each(|item| collect_task_ir(item, tasks)),
        Value::Object(object) => object
            .values()
            .for_each(|item| collect_task_ir(item, tasks)),
        _ => {}
    }
}

fn structured_task_ir_rejections(task: &Value) -> RunnerResult<Vec<String>> {
    let mut cases = Vec::new();
    let mut mirror = task.clone();
    mirror["runtime_hints"]["operator_kind"] = json!("solver");
    cases.push(summary_rejection_code(&mirror)?);
    let mut abi = task.clone();
    abi["execution_program"]["abi"]["kind"] = json!("solver_rpc");
    cases.push(summary_rejection_code(&abi)?);
    let mut program = task.clone();
    program["execution_program"]["program_id"] = json!("transform.wrong");
    cases.push(summary_rejection_code(&program)?);
    let mut entrypoint = task.clone();
    entrypoint["execution_program"]["entrypoint"]["name"] = json!("transform.wrong");
    cases.push(summary_rejection_code(&entrypoint)?);
    cases.sort();
    cases.dedup();
    Ok(cases)
}

fn summary_rejection_code(task: &Value) -> RunnerResult<String> {
    let error = summarize_operator_task_execution_checked(task)
        .expect_err("qualification mutation must be rejected");
    Ok(match error.code {
        OperatorTaskSummaryErrorCode::Invalid => "invalid",
        OperatorTaskSummaryErrorCode::MissingField => "missing_field",
        OperatorTaskSummaryErrorCode::MirrorMismatch => "mirror_mismatch",
        OperatorTaskSummaryErrorCode::ExecutionAbiMismatch => "execution_abi_mismatch",
        OperatorTaskSummaryErrorCode::ProgramMismatch => "program_mismatch",
        OperatorTaskSummaryErrorCode::EntrypointMismatch => "entrypoint_mismatch",
    }
    .to_string())
}

fn qualify_rpc() -> RunnerResult<RpcReport> {
    let methods = RpcProtocolDescriptor::solver_agent_default().methods;
    let mut wire_methods = BTreeSet::new();
    let mut round_trip_count = 0;
    for (index, method) in methods.iter().enumerate() {
        let wire = serde_json::to_value(method)
            .map_err(|error| format!("failed to encode RPC method: {error}"))?
            .as_str()
            .ok_or_else(|| "RPC method wire value must be a string".to_string())?
            .to_string();
        wire_methods.insert(wire);
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: format!("qualification-{index}"),
            method: method.clone(),
            params: json!({}),
        };
        validate_rpc_request_envelope(&request)
            .map_err(|error| format!("advertised RPC request rejected: {}", error.message))?;
        let encoded = serde_json::to_vec(&request)
            .map_err(|error| format!("failed to encode RPC request: {error}"))?;
        let decoded: RpcRequest = serde_json::from_slice(&encoded)
            .map_err(|error| format!("failed to decode RPC request: {error}"))?;
        if decoded != request {
            return Err("RPC request round trip changed the envelope".to_string());
        }
        round_trip_count += 1;
    }
    let unknown_method_rejected = serde_json::from_value::<RpcRequest>(json!({
        "rpc_version": RPC_VERSION,
        "id": "qualification-unknown",
        "method": "solve_not_real",
        "params": {}
    }))
    .is_err();
    Ok(RpcReport {
        advertised_method_count: methods.len(),
        round_trip_count,
        unique_wire_method_count: wire_methods.len(),
        unknown_method_rejected,
        boundary_rejection_codes: rpc_boundary_rejection_codes()?,
    })
}

fn rpc_boundary_rejection_codes() -> RunnerResult<Vec<String>> {
    let mut request = RpcRequest {
        rpc_version: RPC_VERSION + 1,
        id: "qualification-boundary".to_string(),
        method: RpcMethod::Ping,
        params: json!({}),
    };
    let mut codes = vec![request_error_code(&request)?];
    request.rpc_version = RPC_VERSION;
    request.id.clear();
    codes.push(request_error_code(&request)?);

    let mut response = RpcResponse::success("qualification-boundary", json!({}));
    response.error = Some(RpcError {
        code: "mixed".to_string(),
        message: "mixed state".to_string(),
        details: None,
    });
    codes.push(
        validate_rpc_response_envelope(&response)
            .expect_err("mixed response state must be rejected")
            .code
            .as_str()
            .to_string(),
    );
    let progress_event = ProgressEvent::new("job-boundary", JobStatus::Solving, 0.5);
    let mut progress = RpcProgress::new("qualification-boundary", progress_event);
    progress.event = "unknown".to_string();
    codes.push(
        validate_rpc_progress_envelope(&progress)
            .expect_err("unknown progress event must be rejected")
            .code
            .as_str()
            .to_string(),
    );
    codes.sort();
    codes.dedup();
    Ok(codes)
}

fn request_error_code(request: &RpcRequest) -> RunnerResult<String> {
    Ok(validate_rpc_request_envelope(request)
        .expect_err("invalid request must be rejected")
        .code
        .as_str()
        .to_string())
}

fn run_runner_command(args: &[&str]) -> RunnerResult<CommandReport> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to resolve script runner: {error}"))?;
    let started = Instant::now();
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute {}: {error}", args.join(" ")))?;
    let rendered = combined_output(&output);
    Ok(CommandReport {
        command: args.iter().map(|arg| (*arg).to_string()).collect(),
        status: if output.status.success() {
            "pass"
        } else {
            "fail"
        }
        .to_string(),
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        output: rendered.chars().take(8_000).collect(),
    })
}

fn combined_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn portable_output(root: &Path, output: &std::process::Output) -> String {
    portable_text(root, combined_output(output))
}

fn portable_text(root: &Path, rendered: String) -> String {
    let root = root.to_string_lossy();
    if root.is_empty() {
        rendered
    } else {
        rendered.replace(root.as_ref(), "@repo")
    }
}

fn validate_report(
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.status != "pass"
        || report.generated_at_unix_ms == 0
    {
        return Err("protocol qualification report header is invalid".to_string());
    }
    if report.protocol_tests.exit_code != Some(0)
        || report.protocol_tests.passed < contract.minimum_protocol_tests
        || report.protocol_tests.failed != 0
        || report.protocol_tests.ignored != 0
        || report.protocol_tests.measured != 0
        || report.protocol_tests.required_tests.len() != contract.required_tests.len()
        || report
            .protocol_tests
            .required_tests
            .iter()
            .any(|test| !test.passed)
    {
        return Err("protocol test suite does not meet qualification".to_string());
    }
    for required in &contract.required_tests {
        if !report
            .protocol_tests
            .required_tests
            .iter()
            .any(|test| test.id == *required && test.passed)
            || !report.protocol_tests.output_excerpt.contains(required)
        {
            return Err(format!("protocol report misses required test {required}"));
        }
    }
    if report.fuzz_profiles.len() != contract.fuzz_profiles.len()
        || report.fuzz_profiles.iter().any(|profile| !profile.passed)
        || report
            .fuzz_profiles
            .iter()
            .map(|profile| profile.cases)
            .sum::<usize>()
            != contract
                .fuzz_profiles
                .iter()
                .map(|profile| profile.cases)
                .sum::<usize>()
    {
        return Err("protocol fuzz profile evidence is incomplete".to_string());
    }
    if report.task_ir.task_count < contract.minimum_task_ir_count
        || report.task_ir.verified_digest_count != report.task_ir.task_count
        || report.task_ir.execution_summary_count != report.task_ir.task_count
        || report.task_ir.tamper_rejection_count != report.task_ir.task_count
        || !contract
            .required_authoring_modes
            .iter()
            .all(|mode| report.task_ir.authoring_modes.contains(mode))
        || report.task_ir.structured_rejection_codes
            != [
                "entrypoint_mismatch",
                "execution_abi_mismatch",
                "mirror_mismatch",
                "program_mismatch",
            ]
    {
        return Err("TaskIR qualification evidence is incomplete".to_string());
    }
    if report.rpc.advertised_method_count < contract.minimum_rpc_methods
        || report.rpc.round_trip_count != report.rpc.advertised_method_count
        || report.rpc.unique_wire_method_count != report.rpc.advertised_method_count
        || !report.rpc.unknown_method_rejected
        || report.rpc.boundary_rejection_codes
            != [
                RpcEnvelopeErrorCode::InvalidProgressEvent.as_str(),
                RpcEnvelopeErrorCode::InvalidRequestId.as_str(),
                RpcEnvelopeErrorCode::InvalidResponseState.as_str(),
                RpcEnvelopeErrorCode::InvalidVersion.as_str(),
            ]
    {
        return Err("RPC qualification evidence is incomplete".to_string());
    }
    if report.contract_checks.len() != 2
        || report
            .contract_checks
            .iter()
            .any(|check| check.status != "pass")
        || !report.contract_checks.iter().any(|check| {
            check.command == ["check-operator-task-ir-contract"]
                && check.output.contains("operator task IR example contracts")
        })
        || !report.contract_checks.iter().any(|check| {
            check.command == ["check-operator-task-ir-contract", "--self-test"]
                && check.output.contains("self-test passed")
        })
    {
        return Err("TaskIR contract command evidence is incomplete".to_string());
    }
    Ok(())
}

fn repo_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!("path escapes repository: {relative}"));
    }
    Ok(root.join(path))
}

fn read_json<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> RunnerResult<T> {
    let text = fs::read_to_string(repo_path(root, relative)?)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON {relative}: {error}"))
}

fn write_json(root: &Path, relative: &str, report: &QualificationReport) -> RunnerResult<()> {
    let path = repo_path(root, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to encode protocol report: {error}"))?;
    fs::write(&path, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_self_test() -> RunnerResult<()> {
    let output = "running 94 tests\n\
test tests::rpc_fuzz::smoke ... ok\n\
test result: ok. 94 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s\n\
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n";
    let summary = parse_test_summary(output);
    if summary.passed != 94 || summary.failed != 0 || !test_passed(output, "tests::rpc_fuzz::smoke")
    {
        return Err("protocol test output parser self-test failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_protocol_test_output() {
        super::run_self_test().unwrap();
    }

    #[test]
    fn removes_repository_paths_from_retained_output() {
        let output = super::portable_text(
            std::path::Path::new("/private/repo"),
            "Compiling (/private/repo/workers/rust)".to_string(),
        );
        assert_eq!(output, "Compiling (@repo/workers/rust)");
    }
}
