use crate::{RemoteDeploymentPlan, default_remote_deployment_plan};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const REMOTE_DEPLOYMENT_JOURNAL_SCHEMA_VERSION: &str = "kyuubiki.remote-deployment-journal/v2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeploymentJournal {
    pub schema_version: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub target_ref: String,
    pub revision: u64,
    pub status: String,
    pub records: Vec<RemoteDeploymentJournalRecord>,
    pub journal_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeploymentJournalRecord {
    pub sequence: usize,
    pub step_id: String,
    pub phase: String,
    pub status: String,
    pub attempt: u32,
    pub idempotency_key: String,
    pub failure_class: String,
    pub failure_reason: Option<String>,
    pub local_record_path: String,
    pub remote_record_path: String,
}

impl RemoteDeploymentJournal {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote deployment journal".to_string(),
            format!("schema: {}", self.schema_version),
            format!("plan_id: {}", self.plan_id),
            format!("target_ref: {}", self.target_ref),
            format!("revision: {}", self.revision),
            format!("status: {}", self.status),
            format!("journal_digest: {}", self.journal_digest),
            "records:".to_string(),
        ];
        for record in &self.records {
            lines.push(format!(
                "  - {} [{}] attempt={}",
                record.step_id, record.status, record.attempt
            ));
            lines.push(format!("    phase: {}", record.phase));
            lines.push(format!("    idempotency_key: {}", record.idempotency_key));
            lines.push(format!("    failure_class: {}", record.failure_class));
            if let Some(reason) = &record.failure_reason {
                lines.push(format!("    failure_reason: {reason}"));
            }
            lines.push(format!(
                "    local_record_path: {}",
                record.local_record_path
            ));
            lines.push(format!(
                "    remote_record_path: {}",
                record.remote_record_path
            ));
        }
        lines.join("\n")
    }

    pub fn completed_step_ids(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|record| record.status == "completed")
            .map(|record| record.step_id.clone())
            .collect()
    }

    pub fn next_step_id(&self) -> Option<String> {
        self.records
            .iter()
            .find(|record| record.status != "completed")
            .map(|record| record.step_id.clone())
    }
}

pub fn default_remote_deployment_journal() -> RemoteDeploymentJournal {
    remote_deployment_journal_for_plan(&default_remote_deployment_plan(), "lab-remote-agent")
}

pub fn remote_deployment_journal_for_plan(
    plan: &RemoteDeploymentPlan,
    target_ref: &str,
) -> RemoteDeploymentJournal {
    let mut journal = RemoteDeploymentJournal {
        schema_version: REMOTE_DEPLOYMENT_JOURNAL_SCHEMA_VERSION.to_string(),
        plan_id: plan.plan_id.clone(),
        plan_digest: remote_deployment_plan_digest(plan),
        target_ref: target_ref.to_string(),
        revision: 0,
        status: "pending".to_string(),
        records: plan
            .steps
            .iter()
            .enumerate()
            .map(|(sequence, step)| RemoteDeploymentJournalRecord {
                sequence,
                step_id: step.id.clone(),
                phase: step.phase.clone(),
                status: "pending".to_string(),
                attempt: 0,
                idempotency_key: format!("{}:{}", target_ref, step.idempotency_key),
                failure_class: step.failure_class.clone(),
                failure_reason: None,
                local_record_path: format!(
                    ".kyuubiki/remote-journal/{}/{}.jsonl",
                    plan.plan_id, step.id
                ),
                remote_record_path: format!(
                    ".kyuubiki/remote-journal/{}/{}.jsonl",
                    target_ref, step.id
                ),
            })
            .collect(),
        journal_digest: String::new(),
    };
    refresh_journal_digest(&mut journal);
    journal
}

pub fn verify_remote_deployment_journal(
    plan: &RemoteDeploymentPlan,
    journal: &RemoteDeploymentJournal,
) -> Result<(), String> {
    if journal.schema_version != REMOTE_DEPLOYMENT_JOURNAL_SCHEMA_VERSION {
        return Err(format!(
            "remote deployment journal schema must be {REMOTE_DEPLOYMENT_JOURNAL_SCHEMA_VERSION}"
        ));
    }
    if journal.plan_id != plan.plan_id || journal.plan_digest != remote_deployment_plan_digest(plan)
    {
        return Err("remote deployment journal plan identity or digest mismatch".to_string());
    }
    if !valid_target_ref(&journal.target_ref) || journal.records.len() != plan.steps.len() {
        return Err("remote deployment journal target or record count is invalid".to_string());
    }
    if journal.journal_digest != remote_deployment_journal_digest(journal) {
        return Err("remote deployment journal digest mismatch".to_string());
    }
    verify_records(plan, journal)?;
    if journal.revision != expected_revision(&journal.records) {
        return Err("remote deployment journal revision does not match step attempts".to_string());
    }
    let expected_status = derived_status(&journal.records)?;
    if journal.status != expected_status {
        return Err(format!(
            "remote deployment journal status {} must be {expected_status}",
            journal.status
        ));
    }
    Ok(())
}

pub fn start_remote_deployment_step(
    plan: &RemoteDeploymentPlan,
    journal: &mut RemoteDeploymentJournal,
    step_id: &str,
) -> Result<(), String> {
    verify_remote_deployment_journal(plan, journal)?;
    let index = record_index(journal, step_id)?;
    if journal.records[..index]
        .iter()
        .any(|record| record.status != "completed")
    {
        return Err(format!("remote deployment step {step_id} is out of order"));
    }
    if journal.records[index + 1..]
        .iter()
        .any(|record| record.status != "pending")
    {
        return Err(format!(
            "remote deployment step {step_id} has an invalid later active step"
        ));
    }
    let record = &mut journal.records[index];
    if record.status != "pending" && record.status != "interrupted" {
        return Err(format!(
            "remote deployment step {step_id} cannot start from {}",
            record.status
        ));
    }
    record.status = "running".to_string();
    record.attempt += 1;
    record.failure_reason = None;
    advance_journal(journal)
}

pub fn complete_remote_deployment_step(
    plan: &RemoteDeploymentPlan,
    journal: &mut RemoteDeploymentJournal,
    step_id: &str,
) -> Result<(), String> {
    verify_remote_deployment_journal(plan, journal)?;
    let record = record_mut(journal, step_id)?;
    if record.status != "running" {
        return Err(format!(
            "remote deployment step {step_id} cannot complete from {}",
            record.status
        ));
    }
    record.status = "completed".to_string();
    record.failure_reason = None;
    advance_journal(journal)
}

pub fn interrupt_remote_deployment_step(
    plan: &RemoteDeploymentPlan,
    journal: &mut RemoteDeploymentJournal,
    step_id: &str,
    reason: &str,
) -> Result<(), String> {
    verify_remote_deployment_journal(plan, journal)?;
    if reason.trim().is_empty() {
        return Err("remote deployment interruption reason cannot be empty".to_string());
    }
    let record = record_mut(journal, step_id)?;
    if record.status != "running" {
        return Err(format!(
            "remote deployment step {step_id} cannot be interrupted from {}",
            record.status
        ));
    }
    record.status = "interrupted".to_string();
    record.failure_reason = Some(reason.to_string());
    advance_journal(journal)
}

pub fn remote_deployment_plan_digest(plan: &RemoteDeploymentPlan) -> String {
    digest(&serde_json::to_vec(plan).expect("remote deployment plan must serialize"))
}

pub fn remote_deployment_journal_digest(journal: &RemoteDeploymentJournal) -> String {
    let mut payload = journal.clone();
    payload.journal_digest.clear();
    digest(&serde_json::to_vec(&payload).expect("remote deployment journal must serialize"))
}

fn refresh_journal_digest(journal: &mut RemoteDeploymentJournal) {
    journal.journal_digest = remote_deployment_journal_digest(journal);
}

fn advance_journal(journal: &mut RemoteDeploymentJournal) -> Result<(), String> {
    journal.revision += 1;
    journal.status = derived_status(&journal.records)?;
    refresh_journal_digest(journal);
    Ok(())
}

fn verify_records(
    plan: &RemoteDeploymentPlan,
    journal: &RemoteDeploymentJournal,
) -> Result<(), String> {
    let mut seen_ids = BTreeSet::new();
    let mut incomplete_seen = false;
    let mut first_incomplete_index = None;
    for (index, (step, record)) in plan.steps.iter().zip(&journal.records).enumerate() {
        let expected_local_path = format!(
            ".kyuubiki/remote-journal/{}/{}.jsonl",
            plan.plan_id, step.id
        );
        let expected_remote_path = format!(
            ".kyuubiki/remote-journal/{}/{}.jsonl",
            journal.target_ref, step.id
        );
        if record.sequence != index
            || record.step_id != step.id
            || record.phase != step.phase
            || record.failure_class != step.failure_class
            || record.idempotency_key != format!("{}:{}", journal.target_ref, step.idempotency_key)
            || record.local_record_path != expected_local_path
            || record.remote_record_path != expected_remote_path
            || !seen_ids.insert(record.step_id.as_str())
        {
            return Err(format!(
                "remote deployment journal record {index} mismatches its plan"
            ));
        }
        verify_record_state(record)?;
        if record.status == "completed" {
            if incomplete_seen {
                return Err("remote deployment journal completed steps must form a prefix".into());
            }
        } else {
            incomplete_seen = true;
            first_incomplete_index.get_or_insert(index);
        }
    }
    if let Some(index) = first_incomplete_index {
        if journal.records[index + 1..]
            .iter()
            .any(|record| record.status != "pending")
        {
            return Err(
                "remote deployment journal only permits the first incomplete step to be active"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn verify_record_state(record: &RemoteDeploymentJournalRecord) -> Result<(), String> {
    match record.status.as_str() {
        "pending" if record.attempt == 0 && record.failure_reason.is_none() => Ok(()),
        "running" | "completed" if record.attempt > 0 && record.failure_reason.is_none() => Ok(()),
        "interrupted"
            if record.attempt > 0
                && record
                    .failure_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty()) =>
        {
            Ok(())
        }
        _ => Err(format!(
            "remote deployment journal step {} has invalid state",
            record.step_id
        )),
    }
}

fn derived_status(records: &[RemoteDeploymentJournalRecord]) -> Result<String, String> {
    if records.iter().all(|record| record.status == "completed") {
        return Ok("completed".to_string());
    }
    if records.iter().any(|record| record.status == "running") {
        return Ok("running".to_string());
    }
    if records.iter().any(|record| record.status == "interrupted") {
        return Ok("interrupted".to_string());
    }
    if records.iter().all(|record| record.status == "pending") {
        return Ok("pending".to_string());
    }
    if records.iter().any(|record| record.status == "pending") {
        return Ok("ready".to_string());
    }
    Err("remote deployment journal cannot derive status".to_string())
}

fn expected_revision(records: &[RemoteDeploymentJournalRecord]) -> u64 {
    records
        .iter()
        .map(|record| match record.status.as_str() {
            "pending" => 0,
            "running" => u64::from(record.attempt) * 2 - 1,
            "completed" | "interrupted" => u64::from(record.attempt) * 2,
            _ => 0,
        })
        .sum()
}

fn valid_target_ref(target_ref: &str) -> bool {
    !target_ref.is_empty()
        && target_ref.len() <= 128
        && target_ref.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        && target_ref != "."
        && target_ref != ".."
}

fn record_index(journal: &RemoteDeploymentJournal, step_id: &str) -> Result<usize, String> {
    journal
        .records
        .iter()
        .position(|record| record.step_id == step_id)
        .ok_or_else(|| format!("remote deployment journal has no step {step_id}"))
}

fn record_mut<'a>(
    journal: &'a mut RemoteDeploymentJournal,
    step_id: &str,
) -> Result<&'a mut RemoteDeploymentJournalRecord, String> {
    let index = record_index(journal, step_id)?;
    Ok(&mut journal.records[index])
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
