use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

type RunnerResult<T> = Result<T, String>;

const CONFIG_PATH: &str = "config/architecture/usability-release-gate.json";
const CONFIG_SCHEMA: &str = "kyuubiki.usability-release-gate/v1";
const CAPABILITY_SCHEMA: &str = "kyuubiki.desktop-capability-closure/v1";
const REPORT_SCHEMA: &str = "kyuubiki.usability-readiness-report/v1";
const DEFAULT_OUT: &str = "tmp/usability-readiness-report.json";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GateConfig {
    schema_version: String,
    baseline_release: String,
    target_release: String,
    policy: Policy,
    capability_contract: String,
    source_guards: Vec<SourceGuard>,
    journeys: Vec<Journey>,
}

#[derive(Deserialize)]
struct Policy {
    all_blocking_journeys_must_pass: bool,
    planned_or_static_only_is_not_release_evidence: bool,
    production_runtime_must_be_native: bool,
    gate_scope: String,
    release_claim_allowed: bool,
    closed_release_subtiers: Vec<String>,
    unclosed_release_tiers: Vec<String>,
    open_release_subtiers: Vec<String>,
}

#[derive(Deserialize)]
struct SourceGuard {
    id: String,
    paths: Vec<String>,
    forbidden_tokens: Vec<String>,
}

#[derive(Deserialize)]
struct Journey {
    id: String,
    title: String,
    blocking: bool,
    capabilities: Vec<String>,
    probes: Vec<Vec<String>>,
}

#[derive(Clone, Serialize)]
struct ProbeResult {
    command: Vec<String>,
    status: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    output: String,
}

#[derive(Serialize)]
struct JourneyResult {
    id: String,
    title: String,
    blocking: bool,
    status: String,
    capabilities: Vec<String>,
    operational_probe_count: usize,
    probes: Vec<ProbeResult>,
}

#[derive(Serialize)]
struct ReadinessReport {
    schema_version: &'static str,
    generated_at_unix_ms: u128,
    baseline_release: String,
    target_release: String,
    gate_scope: String,
    release_claim_allowed: bool,
    closed_release_subtiers: Vec<String>,
    unclosed_release_tiers: Vec<String>,
    open_release_subtiers: Vec<String>,
    executed: bool,
    status: String,
    summary: Summary,
    journeys: Vec<JourneyResult>,
}

#[derive(Serialize)]
struct Summary {
    journey_count: usize,
    blocking_count: usize,
    passed_count: usize,
    failed_count: usize,
    planned_count: usize,
    unique_probe_count: usize,
    unique_operational_probe_count: usize,
    open_release_subtier_count: usize,
}

#[derive(Default)]
struct Options {
    execute: bool,
    self_test: bool,
    help: bool,
    out: Option<String>,
}

pub(crate) fn run_check_usability_release_gate(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-usability-release-gate [--execute] [--out <path>]"
        );
        return Ok(0);
    }
    if options.self_test {
        run_self_test()?;
        println!("usability release gate self-test passed");
        return Ok(0);
    }

    let config: GateConfig = read_json(root, CONFIG_PATH)?;
    validate_config(root, &config)?;
    let report = build_report(&config, options.execute)?;
    validate_report(&config, &report)?;
    let output_path = options
        .out
        .as_deref()
        .or(options.execute.then_some(DEFAULT_OUT));
    if let Some(out) = output_path {
        write_json(root, out, &report)?;
        println!("usability readiness report written: {out}");
    }
    println!(
        "usability release gate {}: {}/{} journey(s) passed, executed={}",
        report.status, report.summary.passed_count, report.summary.journey_count, report.executed
    );
    Ok(if report.status == "fail" { 1 } else { 0 })
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--execute" => options.execute = true,
            "--self-test" => options.self_test = true,
            "--out" => {
                options.out = Some(
                    iter.next()
                        .ok_or_else(|| "--out requires a path".to_string())?
                        .to_string_lossy()
                        .to_string(),
                );
            }
            "--help" | "-h" => {
                options.help = true;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn validate_config(root: &Path, config: &GateConfig) -> RunnerResult<()> {
    if config.schema_version != CONFIG_SCHEMA {
        return Err(format!("schema_version must be {CONFIG_SCHEMA}"));
    }
    let expected_baseline = baseline_release_for(VERSION)?;
    if config.baseline_release != expected_baseline || config.target_release != "daji 3.0.0" {
        return Err(format!(
            "usability gate must describe the {expected_baseline} to daji 3.0.0 line"
        ));
    }
    if !config.policy.all_blocking_journeys_must_pass
        || !config.policy.planned_or_static_only_is_not_release_evidence
        || !config.policy.production_runtime_must_be_native
    {
        return Err("all 3.0 usability policies must remain enabled".to_string());
    }
    if config.policy.gate_scope.trim().is_empty() {
        return Err("usability gate scope must be explicit".to_string());
    }
    validate_release_tier_policy(&config.policy)?;
    if config.policy.closed_release_subtiers.is_empty()
        || config
            .policy
            .closed_release_subtiers
            .iter()
            .any(|tier| tier.trim().is_empty())
    {
        return Err("closed release subtiers must be explicit".to_string());
    }

    let capability_contract: serde_json::Value = read_json(root, &config.capability_contract)?;
    if capability_contract
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some(CAPABILITY_SCHEMA)
    {
        return Err(format!(
            "{} must use {CAPABILITY_SCHEMA}",
            config.capability_contract
        ));
    }
    let known_capabilities = capability_contract
        .get("capabilities")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("id").and_then(serde_json::Value::as_str))
        .collect::<BTreeSet<_>>();

    let mut ids = BTreeSet::new();
    if config.journeys.is_empty() || !config.journeys.iter().all(|journey| journey.blocking) {
        return Err("3.0 gate requires at least one journey and all journeys blocking".to_string());
    }
    for journey in &config.journeys {
        if journey.id.trim().is_empty() || !ids.insert(journey.id.as_str()) {
            return Err(format!("missing or duplicate journey id {}", journey.id));
        }
        if journey.title.trim().is_empty()
            || journey.capabilities.is_empty()
            || journey.probes.is_empty()
        {
            return Err(format!("journey {} is incomplete", journey.id));
        }
        for capability in &journey.capabilities {
            if !known_capabilities.contains(capability.as_str()) {
                return Err(format!(
                    "journey {} references unknown capability {capability}",
                    journey.id
                ));
            }
        }
        for probe in &journey.probes {
            if probe.is_empty() || probe[0].trim().is_empty() {
                return Err(format!("journey {} contains an empty probe", journey.id));
            }
            validate_retained_probe(root, &journey.id, probe)?;
        }
        if config.policy.planned_or_static_only_is_not_release_evidence
            && !journey
                .probes
                .iter()
                .any(|probe| is_operational_probe(probe))
        {
            return Err(format!(
                "journey {} requires at least one operational probe",
                journey.id
            ));
        }
    }
    validate_source_guards(root, &config.source_guards)
}

fn validate_release_tier_policy(policy: &Policy) -> RunnerResult<()> {
    let unclosed = policy
        .unclosed_release_tiers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let closed = policy
        .closed_release_subtiers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let open = policy
        .open_release_subtiers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if unclosed.len() != policy.unclosed_release_tiers.len()
        || closed.len() != policy.closed_release_subtiers.len()
        || open.len() != policy.open_release_subtiers.len()
    {
        return Err("release tier ids must be unique".to_string());
    }
    if policy.release_claim_allowed != (unclosed.is_empty() && open.is_empty()) {
        return Err(
            "release_claim_allowed must be true only when no release tiers or subtiers remain open"
                .to_string(),
        );
    }
    if !closed.is_disjoint(&open) {
        return Err("release subtiers cannot be both closed and open".to_string());
    }
    let mut represented_parents = BTreeSet::new();
    for subtier in &open {
        let (parent, child) = subtier
            .split_once('/')
            .ok_or_else(|| format!("open release subtier must use parent/child: {subtier}"))?;
        if parent.is_empty() || child.is_empty() || !unclosed.contains(parent) {
            return Err(format!(
                "open release subtier {subtier} must belong to an unclosed parent tier"
            ));
        }
        represented_parents.insert(parent);
    }
    if represented_parents != unclosed {
        return Err(
            "every unclosed release tier must expose at least one open subtier".to_string(),
        );
    }
    Ok(())
}

fn baseline_release_for(version: &str) -> RunnerResult<String> {
    let (minor_line, patch) = version
        .rsplit_once('.')
        .ok_or_else(|| format!("package version must use major.minor.patch: {version}"))?;
    if patch.is_empty()
        || minor_line.split('.').count() != 2
        || minor_line.split('.').any(|part| part.is_empty())
    {
        return Err(format!(
            "package version must use major.minor.patch: {version}"
        ));
    }
    Ok(format!("moxi {minor_line}.x"))
}

fn validate_retained_probe(root: &Path, journey_id: &str, probe: &[String]) -> RunnerResult<()> {
    if probe.first().map(String::as_str) != Some("desktop-packaged-smoke") {
        return Ok(());
    }
    let Some(index) = probe
        .iter()
        .position(|argument| argument == "--verify-report")
    else {
        return Ok(());
    };
    let relative = probe
        .get(index + 1)
        .ok_or_else(|| format!("journey {journey_id} --verify-report requires a path"))?;
    let path = repo_path(root, relative)?;
    crate::packaged_desktop_smoke::verify_retained_report(&path).map_err(|error| {
        format!("journey {journey_id} retained packaged desktop evidence is invalid: {error}")
    })
}

fn is_operational_probe(probe: &[String]) -> bool {
    let Some(command) = probe.first().map(String::as_str) else {
        return false;
    };
    let has = |argument: &str| probe.iter().any(|entry| entry == argument);
    match command {
        "check-desktop-usability-journeys" => has("--execute"),
        "check-operator-validation" => has("--execute") && has("--out"),
        "build-material-research-bundle"
        | "check-installer-recovery-fault-injection"
        | "check-orchestra-recovery-fault-injection"
        | "check-runtime-recovery-fault-injection" => has("--out"),
        "check-agent-update-operational-qualification"
        | "check-desktop-ui-validation"
        | "check-fleet-scheduling-operational-qualification"
        | "check-fleet-update-operational-qualification"
        | "check-installed-runtime-operational-qualification"
        | "check-runtime-payload-operational-qualification"
        | "desktop-packaged-smoke" => has("--verify-report"),
        _ => false,
    }
}

fn validate_source_guards(root: &Path, guards: &[SourceGuard]) -> RunnerResult<()> {
    if guards.is_empty() {
        return Err("usability gate requires source guards".to_string());
    }
    for guard in guards {
        if guard.id.trim().is_empty() || guard.paths.is_empty() || guard.forbidden_tokens.is_empty()
        {
            return Err("source guard must declare id, paths, and forbidden tokens".to_string());
        }
        for relative in &guard.paths {
            let source = fs::read_to_string(repo_path(root, relative)?)
                .map_err(|error| format!("failed to read {relative}: {error}"))?;
            for token in &guard.forbidden_tokens {
                if source.contains(token) {
                    return Err(format!(
                        "source guard {} rejected {relative}: {token}",
                        guard.id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn build_report(config: &GateConfig, execute: bool) -> RunnerResult<ReadinessReport> {
    let mut cache = BTreeMap::<String, ProbeResult>::new();
    if execute {
        for probe in config.journeys.iter().flat_map(|journey| &journey.probes) {
            let key = probe.join("\u{1f}");
            if let std::collections::btree_map::Entry::Vacant(e) = cache.entry(key) {
                e.insert(execute_probe(probe)?);
            }
        }
    }

    let journeys = config
        .journeys
        .iter()
        .map(|journey| {
            let operational_probe_count = journey
                .probes
                .iter()
                .filter(|probe| is_operational_probe(probe))
                .count();
            let probes = journey
                .probes
                .iter()
                .map(|probe| {
                    cache
                        .get(&probe.join("\u{1f}"))
                        .cloned()
                        .unwrap_or_else(|| ProbeResult {
                            command: probe.clone(),
                            status: "planned".to_string(),
                            exit_code: None,
                            elapsed_ms: 0,
                            output: String::new(),
                        })
                })
                .collect::<Vec<_>>();
            let status = if !execute {
                "planned"
            } else if probes.iter().all(|probe| probe.status == "pass") {
                "pass"
            } else {
                "fail"
            };
            JourneyResult {
                id: journey.id.clone(),
                title: journey.title.clone(),
                blocking: journey.blocking,
                status: status.to_string(),
                capabilities: journey.capabilities.clone(),
                operational_probe_count,
                probes,
            }
        })
        .collect::<Vec<_>>();
    let passed_count = count_status(&journeys, "pass");
    let failed_count = count_status(&journeys, "fail");
    let planned_count = count_status(&journeys, "planned");
    let status = if !execute {
        "planned"
    } else if failed_count == 0 {
        if config.policy.release_claim_allowed {
            "pass"
        } else {
            "baseline_pass"
        }
    } else {
        "fail"
    };
    Ok(ReadinessReport {
        schema_version: REPORT_SCHEMA,
        generated_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock before epoch: {error}"))?
            .as_millis(),
        baseline_release: config.baseline_release.clone(),
        target_release: config.target_release.clone(),
        gate_scope: config.policy.gate_scope.clone(),
        release_claim_allowed: config.policy.release_claim_allowed,
        closed_release_subtiers: config.policy.closed_release_subtiers.clone(),
        unclosed_release_tiers: config.policy.unclosed_release_tiers.clone(),
        open_release_subtiers: config.policy.open_release_subtiers.clone(),
        executed: execute,
        status: status.to_string(),
        summary: Summary {
            journey_count: journeys.len(),
            blocking_count: journeys.iter().filter(|journey| journey.blocking).count(),
            passed_count,
            failed_count,
            planned_count,
            unique_probe_count: if execute {
                cache.len()
            } else {
                config
                    .journeys
                    .iter()
                    .flat_map(|journey| &journey.probes)
                    .map(|probe| probe.join("\u{1f}"))
                    .collect::<BTreeSet<_>>()
                    .len()
            },
            unique_operational_probe_count: config
                .journeys
                .iter()
                .flat_map(|journey| &journey.probes)
                .filter(|probe| is_operational_probe(probe))
                .map(|probe| probe.join("\u{1f}"))
                .collect::<BTreeSet<_>>()
                .len(),
            open_release_subtier_count: config.policy.open_release_subtiers.len(),
        },
        journeys,
    })
}

fn execute_probe(args: &[String]) -> RunnerResult<ProbeResult> {
    let executable =
        std::env::current_exe().map_err(|error| format!("failed to resolve runner: {error}"))?;
    let started = Instant::now();
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute {}: {error}", args.join(" ")))?;
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(ProbeResult {
        command: args.to_vec(),
        status: if output.status.success() {
            "pass"
        } else {
            "fail"
        }
        .to_string(),
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        output: truncate(&rendered, 4000),
    })
}

fn validate_report(config: &GateConfig, report: &ReadinessReport) -> RunnerResult<()> {
    if report.journeys.len() != config.journeys.len()
        || report.summary.journey_count != config.journeys.len()
    {
        return Err("usability report journey count mismatch".to_string());
    }
    if report.executed && report.summary.planned_count != 0 {
        return Err("executed usability report cannot contain planned journeys".to_string());
    }
    if !report.executed && report.status != "planned" {
        return Err("static usability report must remain planned".to_string());
    }
    if report.status == "pass" && !report.release_claim_allowed {
        return Err("release pass cannot be emitted while release claims are blocked".to_string());
    }
    if config.policy.planned_or_static_only_is_not_release_evidence
        && report
            .journeys
            .iter()
            .any(|journey| journey.operational_probe_count == 0)
    {
        return Err("usability report contains a static-only journey".to_string());
    }
    Ok(())
}

fn count_status(journeys: &[JourneyResult], status: &str) -> usize {
    journeys
        .iter()
        .filter(|journey| journey.status == status)
        .count()
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn repo_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || relative.is_empty()
        || path.components().any(|part| part.as_os_str() == "..")
    {
        return Err(format!("path escapes repository: {relative}"));
    }
    Ok(root.join(path))
}

fn read_json<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> RunnerResult<T> {
    let text = fs::read_to_string(repo_path(root, relative)?)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative}: invalid JSON: {error}"))
}

fn write_json(root: &Path, relative: &str, report: &ReadinessReport) -> RunnerResult<()> {
    let path = repo_path(root, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to serialize usability report: {error}"))?;
    fs::write(&path, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_self_test() -> RunnerResult<()> {
    let planned = JourneyResult {
        id: "test".to_string(),
        title: "Test".to_string(),
        blocking: true,
        status: "planned".to_string(),
        capabilities: vec!["test.capability".to_string()],
        operational_probe_count: 1,
        probes: Vec::new(),
    };
    if count_status(&[planned], "planned") != 1 {
        return Err("planned journey counting failed".to_string());
    }
    if truncate("abcdef", 3) != "abc" {
        return Err("probe output truncation failed".to_string());
    }
    if !is_operational_probe(&[
        "check-desktop-usability-journeys".to_string(),
        "--execute".to_string(),
    ]) || is_operational_probe(&["check-desktop-usability-journeys".to_string()])
    {
        return Err("operational probe classification failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        JourneyResult, Policy, baseline_release_for, count_status, is_operational_probe, truncate,
        validate_release_tier_policy, validate_retained_probe,
    };

    fn release_policy(
        release_claim_allowed: bool,
        closed: &[&str],
        unclosed: &[&str],
        open: &[&str],
    ) -> Policy {
        Policy {
            all_blocking_journeys_must_pass: true,
            planned_or_static_only_is_not_release_evidence: true,
            production_runtime_must_be_native: true,
            gate_scope: "test".to_string(),
            release_claim_allowed,
            closed_release_subtiers: closed.iter().map(|value| value.to_string()).collect(),
            unclosed_release_tiers: unclosed.iter().map(|value| value.to_string()).collect(),
            open_release_subtiers: open.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn counts_planned_journeys() {
        let planned = JourneyResult {
            id: "test".to_string(),
            title: "Test".to_string(),
            blocking: true,
            status: "planned".to_string(),
            capabilities: Vec::new(),
            operational_probe_count: 1,
            probes: Vec::new(),
        };
        assert_eq!(count_status(&[planned], "planned"), 1);
    }

    #[test]
    fn truncates_probe_output_by_character() {
        assert_eq!(truncate("abcdef", 3), "abc");
        assert_eq!(truncate("可用性", 2), "可用");
    }

    #[test]
    fn distinguishes_operational_from_static_probes() {
        assert!(is_operational_probe(&[
            "check-operator-validation".to_string(),
            "--execute".to_string(),
            "--out".to_string(),
            "tmp/report.json".to_string(),
        ]));
        assert!(!is_operational_probe(&[
            "check-operator-validation".to_string(),
            "--profile".to_string(),
            "line-field-closed-form".to_string(),
        ]));
    }

    #[test]
    fn rejects_retained_desktop_probe_without_report_path() {
        let error = validate_retained_probe(
            Path::new("/repo"),
            "create-open-project",
            &[
                "desktop-packaged-smoke".to_string(),
                "--verify-report".to_string(),
            ],
        )
        .expect_err("missing retained report path should fail");
        assert!(error.contains("--verify-report requires a path"));
    }

    #[test]
    fn derives_current_minor_release_baseline() {
        assert_eq!(
            baseline_release_for("2.17.0").expect("valid version"),
            "moxi 2.17.x"
        );
        assert!(baseline_release_for("2.17").is_err());
    }

    #[test]
    fn accepts_open_subtiers_that_cover_every_unclosed_parent() {
        let policy = release_policy(
            false,
            &["desktop/macos"],
            &["desktop", "recovery"],
            &["desktop/windows", "recovery/power-loss"],
        );
        validate_release_tier_policy(&policy).expect("well-formed policy should pass");
    }

    #[test]
    fn rejects_open_subtiers_without_an_unclosed_parent() {
        let policy = release_policy(false, &[], &["desktop"], &["recovery/power-loss"]);
        let error = validate_release_tier_policy(&policy)
            .expect_err("orphaned open subtier should fail validation");
        assert!(error.contains("must belong to an unclosed parent tier"));
    }

    #[test]
    fn rejects_unclosed_parents_without_a_concrete_open_subtier() {
        let policy = release_policy(false, &[], &["desktop", "recovery"], &["desktop/windows"]);
        let error = validate_release_tier_policy(&policy)
            .expect_err("every unclosed parent should expose concrete work");
        assert!(error.contains("every unclosed release tier"));
    }

    #[test]
    fn rejects_subtiers_marked_open_and_closed() {
        let policy = release_policy(
            false,
            &["desktop/windows"],
            &["desktop"],
            &["desktop/windows"],
        );
        let error = validate_release_tier_policy(&policy)
            .expect_err("a subtier cannot be open and closed simultaneously");
        assert!(error.contains("both closed and open"));
    }
}
