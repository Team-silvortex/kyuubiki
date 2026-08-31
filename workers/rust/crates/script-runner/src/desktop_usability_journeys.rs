use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const GATE_PATH: &str = "config/architecture/usability-release-gate.json";
const GATE_SCHEMA: &str = "kyuubiki.usability-release-gate/v1";
const CAPABILITY_SCHEMA: &str = "kyuubiki.desktop-capability-closure/v1";

const NATIVE_PROBES: &[&str] = &[
    "build-material-research-bundle",
    "check-agent-rolling-replacement-operational-qualification",
    "check-agent-update-operational-qualification",
    "check-component-integrity-protocol",
    "check-desktop-bundle-update-operational-qualification",
    "check-desktop-usability-journeys",
    "check-desktop-ui-validation",
    "check-gui-runtime-capability-contract",
    "check-fleet-scheduling-operational-qualification",
    "check-fleet-update-operational-qualification",
    "check-install-update-disk-hygiene",
    "check-installed-runtime-operational-qualification",
    "check-installer-recovery-fault-injection",
    "check-material-exploration-chain-contract",
    "check-material-research-bundle",
    "check-operator-task-ir-contract",
    "check-operator-validation",
    "check-orchestra-long-workflow-takeover-operational-qualification",
    "check-orchestra-network-partition-operational-qualification",
    "check-orchestra-recovery-fault-injection",
    "check-runtime-recovery-fault-injection",
    "check-runtime-payload-operational-qualification",
    "check-ui-automation-contract",
    "check-workflow-dataset-contract",
    "desktop-packaged-smoke",
    "validate-language-packs",
];

const REQUIRED_CHAINS: &[(&str, &[&str])] = &[
    (
        "create-open-project",
        &[
            "hub.project.create",
            "hub.project.inspect",
            "hub.project.validate",
            "hub.project.open-workbench",
        ],
    ),
    ("compose-workflow", &["workbench.analysis.open-local"]),
    (
        "execute-observe",
        &[
            "hub.runtime.start-local",
            "workbench.runtime.start-local",
            "workbench.runtime.inspect",
        ],
    ),
    (
        "diagnose-recover",
        &[
            "hub.environment.validate",
            "installer.integrity.inspect",
            "installer.integrity.repair",
            "workbench.runtime.inspect",
        ],
    ),
];

#[derive(Clone, Deserialize)]
struct GateConfig {
    schema_version: String,
    capability_contract: String,
    journeys: Vec<Journey>,
}

#[derive(Clone, Deserialize)]
struct Journey {
    id: String,
    title: String,
    blocking: bool,
    capabilities: Vec<String>,
    probes: Vec<Vec<String>>,
}

#[derive(Clone, Deserialize)]
struct CapabilityContract {
    schema_version: String,
    capabilities: Vec<Capability>,
}

#[derive(Clone, Deserialize)]
struct Capability {
    id: String,
    app: String,
    ui_action: String,
    route_file: String,
    route_tokens: Vec<String>,
    native_command: String,
    native_action: Option<String>,
    native_file: String,
    native_tokens: Vec<String>,
    automation_action: Option<String>,
}

#[derive(Default)]
struct Options {
    self_test: bool,
    execute: bool,
    journey: Option<String>,
}

pub(crate) fn run_check_desktop_usability_journeys(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    if options.self_test {
        run_self_test(root)?;
        println!("desktop usability journey check self-test passed");
        return Ok(0);
    }

    let issues = check_journeys(root, options.journey.as_deref())?;
    if let Some(issue) = issues.first() {
        eprintln!("desktop usability journey check failed: {issue}");
        return Ok(1);
    }
    if options.execute {
        execute_journeys(options.journey.as_deref())?;
    }

    let mut suffix = options
        .journey
        .as_ref()
        .map(|id| format!(" for {id}"))
        .unwrap_or_default();
    if options.execute {
        suffix.push_str(" with native execution");
    }
    println!("desktop usability journey check passed{suffix}");
    Ok(0)
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--self-test" => options.self_test = true,
            "--execute" => options.execute = true,
            "--journey" => {
                options.journey = Some(
                    iter.next()
                        .ok_or_else(|| "--journey requires an id".to_string())?
                        .to_string_lossy()
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn check_journeys(root: &Path, selected: Option<&str>) -> RunnerResult<Vec<String>> {
    let config: GateConfig = read_json(root, GATE_PATH)?;
    let contract: CapabilityContract = read_json(root, &config.capability_contract)?;
    let mut issues = validate_gate_shape(&config, &contract);
    let capabilities = contract
        .capabilities
        .iter()
        .map(|capability| (capability.id.as_str(), capability))
        .collect::<BTreeMap<_, _>>();

    let selected_journeys = config
        .journeys
        .iter()
        .filter(|journey| selected.is_none_or(|id| journey.id == id))
        .collect::<Vec<_>>();
    if selected.is_some() && selected_journeys.is_empty() {
        issues.push(format!(
            "{GATE_PATH}: unknown journey {}",
            selected.unwrap_or_default()
        ));
    }

    for journey in selected_journeys {
        validate_journey(root, journey, &capabilities, &mut issues);
    }
    validate_required_chains(&config, selected, &mut issues);
    Ok(issues)
}

fn validate_gate_shape(config: &GateConfig, contract: &CapabilityContract) -> Vec<String> {
    let mut issues = Vec::new();
    if config.schema_version != GATE_SCHEMA {
        issues.push(format!("{GATE_PATH}: schema_version must be {GATE_SCHEMA}"));
    }
    if contract.schema_version != CAPABILITY_SCHEMA {
        issues.push(format!(
            "{}: schema_version must be {CAPABILITY_SCHEMA}",
            config.capability_contract
        ));
    }
    let mut ids = BTreeSet::new();
    for journey in &config.journeys {
        if journey.id.trim().is_empty() || !ids.insert(journey.id.as_str()) {
            issues.push(format!("{GATE_PATH}: missing or duplicate journey id"));
        }
        if journey.title.trim().is_empty() || !journey.blocking {
            issues.push(format!(
                "{}: journey {} must be titled and blocking",
                GATE_PATH, journey.id
            ));
        }
    }
    issues
}

fn validate_journey(
    root: &Path,
    journey: &Journey,
    capabilities: &BTreeMap<&str, &Capability>,
    issues: &mut Vec<String>,
) {
    if journey.capabilities.is_empty() {
        issues.push(format!("journey {} declares no capabilities", journey.id));
    }
    for capability_id in &journey.capabilities {
        match capabilities.get(capability_id.as_str()) {
            Some(capability) => validate_capability_closure(root, capability, issues),
            None => issues.push(format!(
                "journey {} references unknown capability {capability_id}",
                journey.id
            )),
        }
    }

    if journey.probes.is_empty() {
        issues.push(format!("journey {} declares no probes", journey.id));
    }
    for probe in &journey.probes {
        validate_probe(&journey.id, probe, issues);
    }
}

fn validate_capability_closure(root: &Path, capability: &Capability, issues: &mut Vec<String>) {
    for (field, value) in [
        ("app", &capability.app),
        ("ui_action", &capability.ui_action),
        ("route_file", &capability.route_file),
        ("native_command", &capability.native_command),
        ("native_file", &capability.native_file),
    ] {
        if value.trim().is_empty() {
            issues.push(format!("capability {} misses {field}", capability.id));
        }
    }
    if capability.native_command == "guarded_mutation_action"
        && capability
            .native_action
            .as_deref()
            .is_none_or(str::is_empty)
    {
        issues.push(format!(
            "capability {} uses guarded_mutation_action without native_action",
            capability.id
        ));
    }
    if capability.app == "hub-gui"
        && capability
            .automation_action
            .as_deref()
            .is_none_or(str::is_empty)
    {
        issues.push(format!(
            "capability {} must expose a Hub Pwdt automation action",
            capability.id
        ));
    }
    validate_file_tokens(
        root,
        &capability.route_file,
        &capability.route_tokens,
        issues,
    );
    validate_file_tokens(
        root,
        &capability.native_file,
        &capability.native_tokens,
        issues,
    );
}

fn validate_file_tokens(root: &Path, relative: &str, tokens: &[String], issues: &mut Vec<String>) {
    if tokens.is_empty() {
        issues.push(format!("{relative}: expected at least one closure token"));
        return;
    }
    let text = match fs::read_to_string(root.join(relative)) {
        Ok(text) => text,
        Err(error) => {
            issues.push(format!("failed to read {relative}: {error}"));
            return;
        }
    };
    for token in tokens {
        if !text.contains(token) {
            issues.push(format!("{relative}: missing token {token}"));
        }
    }
}

fn validate_probe(journey_id: &str, probe: &[String], issues: &mut Vec<String>) {
    let Some(command) = probe.first() else {
        issues.push(format!("journey {journey_id} has an empty probe"));
        return;
    };
    if command.contains("node")
        || command.ends_with("-node-test")
        || matches!(command.as_str(), "node" | "npm" | "pnpm" | "yarn")
    {
        issues.push(format!(
            "journey {journey_id} uses non-native probe {command}"
        ));
    }
    if !NATIVE_PROBES.contains(&command.as_str()) {
        issues.push(format!(
            "journey {journey_id} probe {command} is not in the native usability allowlist"
        ));
    }
}

fn validate_required_chains(config: &GateConfig, selected: Option<&str>, issues: &mut Vec<String>) {
    for (journey_id, required) in REQUIRED_CHAINS {
        if selected.is_some_and(|selected_id| selected_id != *journey_id) {
            continue;
        }
        let Some(journey) = config
            .journeys
            .iter()
            .find(|journey| journey.id == *journey_id)
        else {
            issues.push(format!(
                "{GATE_PATH}: missing required journey {journey_id}"
            ));
            continue;
        };
        for capability in *required {
            if !journey
                .capabilities
                .iter()
                .any(|declared| declared == capability)
            {
                issues.push(format!(
                    "journey {journey_id} misses required capability {capability}"
                ));
            }
        }
        if !journey.probes.iter().any(|probe| {
            probe
                .first()
                .is_some_and(|cmd| cmd == "check-desktop-usability-journeys")
        }) {
            issues.push(format!(
                "journey {journey_id} must include check-desktop-usability-journeys"
            ));
        }
        if *journey_id == "create-open-project"
            && !journey.probes.iter().any(|probe| {
                probe.first().is_some_and(|cmd| {
                    cmd == "check-desktop-usability-journeys"
                        && probe.iter().any(|argument| argument == "--execute")
                })
            })
        {
            issues.push(
                "journey create-open-project must execute its native project round trip"
                    .to_string(),
            );
        }
        if *journey_id == "diagnose-recover" {
            for probe_id in [
                "check-runtime-recovery-fault-injection",
                "check-orchestra-recovery-fault-injection",
                "check-orchestra-long-workflow-takeover-operational-qualification",
                "check-orchestra-network-partition-operational-qualification",
                "check-installer-recovery-fault-injection",
            ] {
                if !journey
                    .probes
                    .iter()
                    .any(|probe| probe.first().is_some_and(|cmd| cmd == probe_id))
                {
                    issues.push(format!("journey diagnose-recover must execute {probe_id}"));
                }
            }
        }
        if *journey_id == "execute-observe"
            && !journey.probes.iter().any(|probe| {
                probe
                    .first()
                    .is_some_and(|cmd| cmd == "check-installed-runtime-operational-qualification")
            })
        {
            issues.push(
                "journey execute-observe must verify installed Runtime operational evidence"
                    .to_string(),
            );
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> RunnerResult<T> {
    let path = repo_path(root, relative)?;
    let text =
        fs::read_to_string(&path).map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative}: invalid JSON: {error}"))
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

fn execute_journeys(selected: Option<&str>) -> RunnerResult<()> {
    if selected.is_none_or(|id| id == "create-open-project") {
        execute_project_bundle_roundtrip()?;
    }
    Ok(())
}

fn execute_project_bundle_roundtrip() -> RunnerResult<()> {
    let fixture_root = std::env::temp_dir().join(format!(
        "kyuubiki-usability-project-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("failed to build journey timestamp: {error}"))?
            .as_nanos()
    ));
    let bundle_path = fixture_root.join("Usability Roundtrip.kyuubiki");
    let result = (|| {
        let path = bundle_path
            .to_str()
            .ok_or_else(|| "project journey fixture path is not UTF-8".to_string())?;
        let created = kyuubiki_project_bundle::create_project_bundle(path)?;
        let created: serde_json::Value = serde_json::from_str(&created)
            .map_err(|error| format!("invalid project create report: {error}"))?;
        if created.get("created").and_then(serde_json::Value::as_bool) != Some(true)
            || !bundle_path.is_file()
        {
            return Err("project journey did not create a bundle".to_string());
        }

        let inspected = kyuubiki_project_bundle::inspect_project_bundle(path)?;
        let inspected: serde_json::Value = serde_json::from_str(&inspected)
            .map_err(|error| format!("invalid project inspect report: {error}"))?;
        if inspected.get("schema").and_then(serde_json::Value::as_str)
            != Some("kyuubiki.project/v2")
            || inspected
                .get("project_name")
                .and_then(serde_json::Value::as_str)
                != Some("Usability Roundtrip")
        {
            return Err("project journey inspect report lost bundle identity".to_string());
        }

        let validation = kyuubiki_project_bundle::validate_project_bundle(path)?;
        if !kyuubiki_project_bundle::validation_passed(&validation)? {
            return Err(format!("project journey validation failed: {validation}"));
        }
        Ok(())
    })();
    let cleanup = fs::remove_dir_all(&fixture_root);
    if let Err(error) = cleanup
        && fixture_root.exists()
    {
        return Err(format!(
            "failed to clean project journey fixture {}: {error}",
            fixture_root.display()
        ));
    }
    result
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut config: GateConfig = read_json(root, GATE_PATH)?;
    let contract: CapabilityContract = read_json(root, &config.capability_contract)?;
    if validate_gate_shape(&config, &contract)
        .into_iter()
        .any(|issue| issue.contains("schema_version"))
    {
        return Err("self-test expected repository usability schemas to load".to_string());
    }

    if let Some(journey) = config
        .journeys
        .iter_mut()
        .find(|journey| journey.id == "create-open-project" && !journey.probes.is_empty())
    {
        journey.probes[0] = vec!["integration-desktop-gui-node-test".to_string()];
    }
    let capabilities = contract
        .capabilities
        .iter()
        .map(|capability| (capability.id.as_str(), capability))
        .collect::<BTreeMap<_, _>>();
    let mut issues = Vec::new();
    validate_journey(
        root,
        config
            .journeys
            .iter()
            .find(|journey| journey.id == "create-open-project")
            .ok_or_else(|| "self-test missing create-open-project journey".to_string())?,
        &capabilities,
        &mut issues,
    );
    if !issues
        .iter()
        .any(|issue| issue.contains("non-native probe"))
    {
        return Err("self-test expected non-native probe rejection".to_string());
    }

    let mut static_only: GateConfig = read_json(root, GATE_PATH)?;
    let journey = static_only
        .journeys
        .iter_mut()
        .find(|journey| journey.id == "create-open-project")
        .ok_or_else(|| "self-test missing create-open-project journey".to_string())?;
    let probe = journey
        .probes
        .iter_mut()
        .find(|probe| {
            probe
                .first()
                .is_some_and(|command| command == "check-desktop-usability-journeys")
        })
        .ok_or_else(|| "self-test missing native project journey probe".to_string())?;
    probe.retain(|argument| argument != "--execute");
    let mut issues = Vec::new();
    validate_required_chains(&static_only, Some("create-open-project"), &mut issues);
    if !issues
        .iter()
        .any(|issue| issue.contains("must execute its native project round trip"))
    {
        return Err("self-test expected static-only project journey rejection".to_string());
    }

    let mut missing_runtime: GateConfig = read_json(root, GATE_PATH)?;
    let journey = missing_runtime
        .journeys
        .iter_mut()
        .find(|journey| journey.id == "execute-observe")
        .ok_or_else(|| "self-test missing execute-observe journey".to_string())?;
    journey.probes.retain(|probe| {
        probe
            .first()
            .is_none_or(|command| command != "check-installed-runtime-operational-qualification")
    });
    let mut issues = Vec::new();
    validate_required_chains(&missing_runtime, Some("execute-observe"), &mut issues);
    if !issues
        .iter()
        .any(|issue| issue.contains("installed Runtime operational evidence"))
    {
        return Err("self-test expected installed Runtime probe rejection".to_string());
    }
    Ok(())
}
