use crate::Platform;
use crate::agent_lifecycle_control::AgentLifecycleClient;
use crate::agent_replacement::{AgentReplacementReceipt, replace_agent_with_drain};
use crate::agent_rolling_process::{
    ManagedQualificationAgent, SolverProbeResult, prepare_binary_copy,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const AGENT_ROLLING_QUALIFICATION_SCHEMA_VERSION: &str =
    "kyuubiki.agent-rolling-replacement-qualification/v1";
pub(crate) const AGENT_ROLLING_QUALIFICATION_JOURNEY: &str =
    "installer-managed-two-agent-live-rolling-replacement";
pub(crate) const AGENT_ROLLING_REQUIRED_CHECKS: &[&str] = &[
    "initial_two_agents_accepting",
    "initial_fleet_executable",
    "first_agent_quiesced",
    "second_agent_served_during_first_replacement",
    "first_agent_instance_replaced",
    "second_agent_quiesced",
    "first_agent_served_during_second_replacement",
    "second_agent_instance_replaced",
    "final_fleet_executable",
    "binary_payload_changed",
    "work_root_cleaned",
];

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRollingQualificationReport {
    pub schema_version: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub first_binary_sha256: String,
    pub second_binary_sha256: String,
    pub agent_count: usize,
    pub initial_instances: Vec<AgentRollingInstanceObservation>,
    pub replacements: Vec<AgentReplacementReceipt>,
    pub final_instances: Vec<AgentRollingInstanceObservation>,
    pub execution_probes: Vec<AgentRollingExecutionProbe>,
    pub checks: Vec<AgentRollingQualificationCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRollingInstanceObservation {
    pub node_id: String,
    pub process_instance_id: String,
    pub binary_sha256: String,
    pub accepting_new_work: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRollingExecutionProbe {
    pub phase: String,
    pub node_id: String,
    pub success: bool,
    pub max_stress: f64,
    pub tip_displacement: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRollingQualificationCheck {
    pub id: String,
    pub ok: bool,
}

pub fn run_agent_rolling_qualification(
    first_binary: &Path,
    second_binary: &Path,
    work_root: &Path,
    first_version: &str,
    second_version: &str,
) -> Result<AgentRollingQualificationReport, String> {
    validate_version_pair(first_version, second_version)?;
    let mut managed_root = QualificationRoot::prepare(work_root)?;
    let first_copy = work_root.join(binary_name("agent-first"));
    let second_copy = work_root.join(binary_name("agent-second"));
    let first_digest = prepare_binary_copy(first_binary, &first_copy)?;
    let second_digest = prepare_binary_copy(second_binary, &second_copy)?;
    if first_digest == second_digest {
        return Err("rolling qualification binaries must have different content".to_string());
    }

    let first = RefCell::new(ManagedQualificationAgent::new(
        work_root,
        "agent-01",
        first_copy.clone(),
    )?);
    let second = RefCell::new(ManagedQualificationAgent::new(
        work_root,
        "agent-02",
        first_copy.clone(),
    )?);
    first.borrow_mut().start()?;
    second.borrow_mut().start()?;
    let first_control =
        AgentLifecycleClient::new(first.borrow().address(), Duration::from_secs(15))
            .map_err(|error| error.to_string())?;
    let second_control =
        AgentLifecycleClient::new(second.borrow().address(), Duration::from_secs(15))
            .map_err(|error| error.to_string())?;

    let initial_first = first_control
        .describe()
        .map_err(|error| error.to_string())?;
    let initial_second = second_control
        .describe()
        .map_err(|error| error.to_string())?;
    let initial_instances = vec![
        instance_observation("agent-01", &initial_first, &first_digest),
        instance_observation("agent-02", &initial_second, &first_digest),
    ];
    let probes = RefCell::new(vec![
        execution_probe(
            "initial",
            "agent-01",
            first.borrow().solve_bar("rolling-initial-01")?,
        ),
        execution_probe(
            "initial",
            "agent-02",
            second.borrow().solve_bar("rolling-initial-02")?,
        ),
    ]);
    let controller_id = "installer-rolling-qualification";

    let first_target_binary = second_copy.clone();
    let first_compensation_binary = first_copy.clone();
    let first_receipt = replace_agent_with_drain(
        &first_control,
        "agent-01",
        controller_id,
        "rolling qualification replacement",
        || {
            let mut target = first.borrow_mut();
            target.stop()?;
            probes.borrow_mut().push(execution_probe(
                "during-agent-01-replacement",
                "agent-02",
                second.borrow().solve_bar("rolling-continuity-agent-02")?,
            ));
            target.replace_binary(first_target_binary)
        },
        || first.borrow_mut().restore_binary(first_compensation_binary),
    )
    .map_err(|error| error.to_string())?;

    let second_target_binary = second_copy.clone();
    let second_compensation_binary = first_copy.clone();
    let second_receipt = replace_agent_with_drain(
        &second_control,
        "agent-02",
        controller_id,
        "rolling qualification replacement",
        || {
            let mut target = second.borrow_mut();
            target.stop()?;
            probes.borrow_mut().push(execution_probe(
                "during-agent-02-replacement",
                "agent-01",
                first.borrow().solve_bar("rolling-continuity-agent-01")?,
            ));
            target.replace_binary(second_target_binary)
        },
        || {
            second
                .borrow_mut()
                .restore_binary(second_compensation_binary)
        },
    )
    .map_err(|error| error.to_string())?;
    let replacements = vec![first_receipt, second_receipt];

    probes.borrow_mut().extend([
        execution_probe(
            "final",
            "agent-01",
            first.borrow().solve_bar("rolling-final-01")?,
        ),
        execution_probe(
            "final",
            "agent-02",
            second.borrow().solve_bar("rolling-final-02")?,
        ),
    ]);
    let final_first = first_control
        .describe()
        .map_err(|error| error.to_string())?;
    let final_second = second_control
        .describe()
        .map_err(|error| error.to_string())?;
    let final_instances = vec![
        instance_observation("agent-01", &final_first, &second_digest),
        instance_observation("agent-02", &final_second, &second_digest),
    ];
    let execution_probes = probes.into_inner();

    drop(first);
    drop(second);
    managed_root.cleanup()?;
    let checks = qualification_checks(
        &initial_instances,
        &replacements,
        &final_instances,
        &execution_probes,
        &first_digest,
        &second_digest,
        !work_root.exists(),
    );
    if checks.iter().any(|check| !check.ok) {
        return Err("Agent rolling replacement qualification checks failed".to_string());
    }
    let report = AgentRollingQualificationReport {
        schema_version: AGENT_ROLLING_QUALIFICATION_SCHEMA_VERSION.to_string(),
        status: "pass".to_string(),
        journey: AGENT_ROLLING_QUALIFICATION_JOURNEY.to_string(),
        execution_host_role: qualification_host_role(Platform::current()),
        platform: Platform::current().as_str().to_string(),
        first_version: first_version.to_string(),
        second_version: second_version.to_string(),
        first_binary_sha256: first_digest,
        second_binary_sha256: second_digest,
        agent_count: 2,
        initial_instances,
        replacements,
        final_instances,
        execution_probes,
        checks,
    };
    crate::agent_rolling_qualification_validation::validate_agent_rolling_qualification_report(
        &report,
    )
    .map_err(|errors| {
        format!(
            "rolling qualification report is invalid: {}",
            errors.join("; ")
        )
    })?;
    Ok(report)
}

pub fn write_agent_rolling_qualification_report(
    report: &AgentRollingQualificationReport,
    path: &Path,
) -> Result<(), String> {
    crate::agent_rolling_qualification_validation::validate_agent_rolling_qualification_report(
        report,
    )
    .map_err(|errors| {
        format!(
            "rolling qualification report is invalid: {}",
            errors.join("; ")
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn qualification_checks(
    initial: &[AgentRollingInstanceObservation],
    replacements: &[AgentReplacementReceipt],
    final_instances: &[AgentRollingInstanceObservation],
    probes: &[AgentRollingExecutionProbe],
    first_digest: &str,
    second_digest: &str,
    work_root_cleaned: bool,
) -> Vec<AgentRollingQualificationCheck> {
    let probe_ok = |phase: &str, node_id: &str| {
        probes
            .iter()
            .any(|probe| probe.phase == phase && probe.node_id == node_id && valid_probe(probe))
    };
    let receipt = |node_id: &str| {
        replacements
            .iter()
            .find(|receipt| receipt.node_id == node_id)
    };
    vec![
        check(
            "initial_two_agents_accepting",
            initial.len() == 2 && initial.iter().all(|item| item.accepting_new_work),
        ),
        check(
            "initial_fleet_executable",
            probe_ok("initial", "agent-01") && probe_ok("initial", "agent-02"),
        ),
        check(
            "first_agent_quiesced",
            receipt("agent-01").is_some_and(|item| item.quiescent_observed),
        ),
        check(
            "second_agent_served_during_first_replacement",
            probe_ok("during-agent-01-replacement", "agent-02"),
        ),
        check(
            "first_agent_instance_replaced",
            receipt("agent-01").is_some_and(|item| item.replacement_verified),
        ),
        check(
            "second_agent_quiesced",
            receipt("agent-02").is_some_and(|item| item.quiescent_observed),
        ),
        check(
            "first_agent_served_during_second_replacement",
            probe_ok("during-agent-02-replacement", "agent-01"),
        ),
        check(
            "second_agent_instance_replaced",
            receipt("agent-02").is_some_and(|item| item.replacement_verified),
        ),
        check(
            "final_fleet_executable",
            final_instances.len() == 2
                && final_instances.iter().all(|item| item.accepting_new_work)
                && probe_ok("final", "agent-01")
                && probe_ok("final", "agent-02"),
        ),
        check("binary_payload_changed", first_digest != second_digest),
        check("work_root_cleaned", work_root_cleaned),
    ]
}

fn execution_probe(
    phase: &str,
    node_id: &str,
    result: SolverProbeResult,
) -> AgentRollingExecutionProbe {
    AgentRollingExecutionProbe {
        phase: phase.to_string(),
        node_id: node_id.to_string(),
        success: (result.max_stress - 10.0).abs() <= 1.0e-9
            && (result.tip_displacement - 0.01).abs() <= 1.0e-12,
        max_stress: result.max_stress,
        tip_displacement: result.tip_displacement,
    }
}

fn valid_probe(probe: &AgentRollingExecutionProbe) -> bool {
    probe.success
        && (probe.max_stress - 10.0).abs() <= 1.0e-9
        && (probe.tip_displacement - 0.01).abs() <= 1.0e-12
}

fn instance_observation(
    node_id: &str,
    lifecycle: &kyuubiki_protocol::AgentLifecycleDescriptor,
    digest: &str,
) -> AgentRollingInstanceObservation {
    AgentRollingInstanceObservation {
        node_id: node_id.to_string(),
        process_instance_id: lifecycle.process_instance_id.clone(),
        binary_sha256: digest.to_string(),
        accepting_new_work: lifecycle.accepting_new_work,
    }
}

fn check(id: &str, ok: bool) -> AgentRollingQualificationCheck {
    AgentRollingQualificationCheck {
        id: id.to_string(),
        ok,
    }
}

fn validate_version_pair(first: &str, second: &str) -> Result<(), String> {
    if first == second || !valid_version(first) || !valid_version(second) {
        return Err("rolling qualification versions must be distinct portable identifiers".into());
    }
    Ok(())
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn qualification_host_role(platform: Platform) -> String {
    let transport = if std::env::var_os("SSH_CONNECTION").is_some() {
        "remote"
    } else {
        "local"
    };
    format!("{transport}-{}-qualification-host", platform.as_str())
}

fn binary_name(stem: &str) -> PathBuf {
    if Platform::current() == Platform::Windows {
        PathBuf::from(format!("bin/{stem}.exe"))
    } else {
        PathBuf::from(format!("bin/{stem}"))
    }
}

struct QualificationRoot {
    path: PathBuf,
    cleaned: bool,
}

impl QualificationRoot {
    fn prepare(path: &Path) -> Result<Self, String> {
        if path.exists()
            && fs::read_dir(path)
                .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
                .next()
                .is_some()
        {
            return Err("rolling qualification work root must be empty".to_string());
        }
        fs::create_dir_all(path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            cleaned: false,
        })
    }

    fn cleanup(&mut self) -> Result<(), String> {
        if self.path.exists() {
            fs::remove_dir_all(&self.path)
                .map_err(|error| format!("failed to remove {}: {error}", self.path.display()))?;
        }
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for QualificationRoot {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
