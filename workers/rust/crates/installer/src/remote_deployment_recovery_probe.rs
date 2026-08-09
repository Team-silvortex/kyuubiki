use crate::{
    complete_remote_deployment_step, default_remote_deployment_plan,
    prepare_remote_deployment_resume, read_remote_deployment_journal,
    remote_deployment_journal_for_plan, remote_deployment_journal_paths,
    start_remote_deployment_step, verify_remote_deployment_journal,
    write_remote_deployment_journal_atomic,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const INSTALLER_JOURNAL_REPLAY_FAULT_INJECTION_SCHEMA_VERSION: &str =
    "kyuubiki.installer-journal-replay-fault-injection/v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerJournalReplayFaultInjectionReport {
    pub schema_version: String,
    pub status: String,
    pub scenario_count: usize,
    pub process_loss_replay: InstallerJournalProcessLossScenario,
    pub digest_tamper_recovery: InstallerJournalDigestTamperScenario,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerJournalProcessLossScenario {
    pub status: String,
    pub interrupted_step_id: String,
    pub resume_step_id: String,
    pub completed_before_loss: Vec<String>,
    pub completed_step_replayed: bool,
    pub interrupted_attempt_before: u32,
    pub interrupted_attempt_after: u32,
    pub final_status: String,
    pub pending_count: usize,
    pub journal_digest_valid: bool,
    pub power_loss_sidecar_recovered: bool,
    pub partial_next_ignored: bool,
    pub probe_state_cleaned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerJournalDigestTamperScenario {
    pub status: String,
    pub digest_tamper_rejected: bool,
    pub error_class: String,
    pub valid_journal_preserved: bool,
    pub probe_state_cleaned: bool,
}

pub fn run_installer_journal_replay_fault_injection(
    temporary_root: &Path,
) -> Result<InstallerJournalReplayFaultInjectionReport, String> {
    let root = unique_probe_root(temporary_root)?;
    fs::create_dir_all(&root)
        .map_err(|error| format!("failed to create installer recovery probe root: {error}"))?;
    let result = run_probe_scenarios(&root);
    let cleanup_result = fs::remove_dir_all(&root)
        .map_err(|error| format!("failed to clean installer recovery probe root: {error}"));
    match (result, cleanup_result) {
        (Ok(mut report), Ok(())) => {
            report.process_loss_replay.probe_state_cleaned = !root.exists();
            report.digest_tamper_recovery.probe_state_cleaned = !root.exists();
            if !report.process_loss_replay.probe_state_cleaned
                || !report.digest_tamper_recovery.probe_state_cleaned
            {
                return Err("installer recovery probe state was not cleaned".to_string());
            }
            Ok(report)
        }
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(probe_error), Err(cleanup_error)) => Err(format!(
            "{probe_error}; additionally failed to clean probe state: {cleanup_error}"
        )),
    }
}

fn run_probe_scenarios(root: &Path) -> Result<InstallerJournalReplayFaultInjectionReport, String> {
    let process_loss_replay = run_process_loss_scenario(&root.join("process-loss"))?;
    let digest_tamper_recovery = run_digest_tamper_scenario(&root.join("digest-tamper"))?;
    Ok(InstallerJournalReplayFaultInjectionReport {
        schema_version: INSTALLER_JOURNAL_REPLAY_FAULT_INJECTION_SCHEMA_VERSION.to_string(),
        status: "passed".to_string(),
        scenario_count: 2,
        process_loss_replay,
        digest_tamper_recovery,
    })
}

fn run_process_loss_scenario(root: &Path) -> Result<InstallerJournalProcessLossScenario, String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("failed to create process-loss scenario root: {error}"))?;
    let plan = default_remote_deployment_plan();
    let journal_path = root.join("deployment-journal.json");
    let mut journal = remote_deployment_journal_for_plan(&plan, "probe-agent");
    write_remote_deployment_journal_atomic(&plan, &journal, &journal_path)?;

    for step_id in ["policy-check", "bootstrap-workspace"] {
        start_remote_deployment_step(&plan, &mut journal, step_id)?;
        write_remote_deployment_journal_atomic(&plan, &journal, &journal_path)?;
        complete_remote_deployment_step(&plan, &mut journal, step_id)?;
        write_remote_deployment_journal_atomic(&plan, &journal, &journal_path)?;
    }
    let completed_before_loss = journal.completed_step_ids();
    let interrupted_step_id = "sync-artifacts";
    start_remote_deployment_step(&plan, &mut journal, interrupted_step_id)?;
    write_remote_deployment_journal_atomic(&plan, &journal, &journal_path)?;

    let paths = remote_deployment_journal_paths(&journal_path);
    fs::rename(&paths.journal, &paths.previous)
        .map_err(|error| format!("failed to inject journal commit interruption: {error}"))?;
    fs::write(&paths.next, b"{\"partial\":")
        .map_err(|error| format!("failed to inject partial next journal: {error}"))?;

    let recovered = read_remote_deployment_journal(&plan, &journal_path)?;
    let power_loss_sidecar_recovered = paths.journal.exists() && !paths.previous.exists();
    let partial_next_ignored = !paths.next.exists();
    let interrupted_attempt_before = attempt_for(&recovered, interrupted_step_id)?;
    let (mut replayed, resume) = prepare_remote_deployment_resume(&plan, &recovered)?;
    let resume_step_id = resume
        .resume_step_id
        .clone()
        .ok_or_else(|| "process-loss replay unexpectedly has no resume step".to_string())?;
    write_remote_deployment_journal_atomic(&plan, &replayed, &journal_path)?;

    let pending_step_ids = resume.pending_step_ids.clone();
    for step_id in pending_step_ids {
        start_remote_deployment_step(&plan, &mut replayed, &step_id)?;
        write_remote_deployment_journal_atomic(&plan, &replayed, &journal_path)?;
        complete_remote_deployment_step(&plan, &mut replayed, &step_id)?;
        write_remote_deployment_journal_atomic(&plan, &replayed, &journal_path)?;
    }
    let final_journal = read_remote_deployment_journal(&plan, &journal_path)?;
    verify_remote_deployment_journal(&plan, &final_journal)?;
    let completed_step_replayed = final_journal.records[..completed_before_loss.len()]
        .iter()
        .any(|record| record.attempt != 1);
    let interrupted_attempt_after = attempt_for(&final_journal, interrupted_step_id)?;
    let pending_count = final_journal
        .records
        .iter()
        .filter(|record| record.status != "completed")
        .count();

    if resume_step_id != interrupted_step_id
        || completed_step_replayed
        || interrupted_attempt_before != 1
        || interrupted_attempt_after != 2
        || final_journal.status != "completed"
        || pending_count != 0
        || !power_loss_sidecar_recovered
        || !partial_next_ignored
    {
        return Err("installer process-loss replay invariants were not satisfied".to_string());
    }
    Ok(InstallerJournalProcessLossScenario {
        status: "passed".to_string(),
        interrupted_step_id: interrupted_step_id.to_string(),
        resume_step_id,
        completed_before_loss,
        completed_step_replayed,
        interrupted_attempt_before,
        interrupted_attempt_after,
        final_status: final_journal.status,
        pending_count,
        journal_digest_valid: true,
        power_loss_sidecar_recovered,
        partial_next_ignored,
        probe_state_cleaned: false,
    })
}

fn run_digest_tamper_scenario(root: &Path) -> Result<InstallerJournalDigestTamperScenario, String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("failed to create digest-tamper scenario root: {error}"))?;
    let plan = default_remote_deployment_plan();
    let journal_path = root.join("deployment-journal.json");
    let journal = remote_deployment_journal_for_plan(&plan, "probe-agent");
    write_remote_deployment_journal_atomic(&plan, &journal, &journal_path)?;

    let mut tampered = journal.clone();
    tampered.records[0].failure_class = "tampered".to_string();
    let error = verify_remote_deployment_journal(&plan, &tampered)
        .expect_err("tampered journal must fail digest verification");
    let digest_tamper_rejected = error.contains("digest mismatch");
    let preserved = read_remote_deployment_journal(&plan, &journal_path)?;
    let valid_journal_preserved = preserved == journal;
    if !digest_tamper_rejected || !valid_journal_preserved {
        return Err("installer digest-tamper recovery invariants were not satisfied".to_string());
    }
    Ok(InstallerJournalDigestTamperScenario {
        status: "passed".to_string(),
        digest_tamper_rejected,
        error_class: "journal_digest_mismatch".to_string(),
        valid_journal_preserved,
        probe_state_cleaned: false,
    })
}

fn attempt_for(journal: &crate::RemoteDeploymentJournal, step_id: &str) -> Result<u32, String> {
    journal
        .records
        .iter()
        .find(|record| record.step_id == step_id)
        .map(|record| record.attempt)
        .ok_or_else(|| format!("installer recovery probe has no step {step_id}"))
}

fn unique_probe_root(temporary_root: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))?
        .as_nanos();
    Ok(temporary_root.join(format!(
        "kyuubiki-installer-recovery-{}-{timestamp}",
        std::process::id()
    )))
}
