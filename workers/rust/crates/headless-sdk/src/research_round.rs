use crate::{
    HEADLESS_EXECUTION_RUN_SCHEMA_VERSION, HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION,
    HeadlessExecutionBatch, HeadlessParameterPatchReceipt, HeadlessRisk, HeadlessRunReport,
    headless_batch_content_sha256, validate_batch,
};
use kyuubiki_protocol::canonical_json_sha256;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION: &str =
    "kyuubiki.headless-research-round-spec/v1";
pub const HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION: &str =
    "kyuubiki.headless-research-round-evidence/v1";

const MAX_RESEARCH_METRICS: usize = 128;
const MAX_METRIC_POINTER_BYTES: usize = 1_024;
const MAX_RESEARCH_ITERATION: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessResearchMetricObjective {
    Minimize,
    Maximize,
    Observe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessResearchMetricSpec {
    pub metric_id: String,
    pub pointer: String,
    pub unit: String,
    pub objective: HeadlessResearchMetricObjective,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessResearchRoundSpec {
    pub schema_version: String,
    pub round_id: String,
    pub workflow_id: String,
    pub iteration: u64,
    pub primary_metric_ids: Vec<String>,
    pub metrics: Vec<HeadlessResearchMetricSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessResearchMetricObservation {
    pub metric_id: String,
    pub pointer: String,
    pub unit: String,
    pub objective: HeadlessResearchMetricObjective,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessResearchRoundLink {
    pub round_id: String,
    pub iteration: u64,
    pub evidence_sha256: String,
    pub batch_content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessResearchRoundEvidence {
    pub schema_version: String,
    pub round_id: String,
    pub workflow_id: String,
    pub iteration: u64,
    pub qualified: bool,
    pub batch_content_sha256: String,
    pub run_report_sha256: String,
    pub run_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch_receipt: Option<HeadlessParameterPatchReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_round: Option<HeadlessResearchRoundLink>,
    pub primary_metric_ids: Vec<String>,
    pub metrics: Vec<HeadlessResearchMetricObservation>,
}

pub fn validate_headless_research_round_spec(
    spec: &HeadlessResearchRoundSpec,
) -> Result<(), String> {
    if spec.schema_version != HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION {
        return Err(format!(
            "unsupported headless research round spec schema: {}",
            spec.schema_version
        ));
    }
    validate_identifier("round_id", &spec.round_id)?;
    if spec.workflow_id.trim().is_empty()
        || spec.workflow_id.len() > 256
        || spec
            .workflow_id
            .chars()
            .any(|character| character.is_control())
    {
        return Err(
            "headless research round workflow_id must be a visible 1..=256 byte value".to_string(),
        );
    }
    if spec.iteration == 0 || spec.iteration > MAX_RESEARCH_ITERATION {
        return Err(format!(
            "headless research round iteration must be within 1..={MAX_RESEARCH_ITERATION}"
        ));
    }
    if spec.metrics.is_empty() || spec.metrics.len() > MAX_RESEARCH_METRICS {
        return Err(format!(
            "headless research round must define between 1 and {MAX_RESEARCH_METRICS} metrics"
        ));
    }

    let mut metric_ids = BTreeSet::new();
    let mut pointers = BTreeSet::new();
    for metric in &spec.metrics {
        validate_identifier("metric_id", &metric.metric_id)?;
        validate_metric_pointer(&metric.pointer)?;
        validate_unit(&metric.unit)?;
        if !metric_ids.insert(metric.metric_id.as_str()) {
            return Err(format!(
                "headless research round contains duplicate metric_id {}",
                metric.metric_id
            ));
        }
        if !pointers.insert(metric.pointer.as_str()) {
            return Err(format!(
                "headless research round contains duplicate metric pointer {}",
                metric.pointer
            ));
        }
    }
    if spec.primary_metric_ids.is_empty() {
        return Err("headless research round requires at least one primary metric".to_string());
    }
    let mut primary_ids = BTreeSet::new();
    for metric_id in &spec.primary_metric_ids {
        if !primary_ids.insert(metric_id.as_str()) {
            return Err(format!(
                "headless research round contains duplicate primary metric {metric_id}"
            ));
        }
        if !metric_ids.contains(metric_id.as_str()) {
            return Err(format!(
                "headless research round primary metric {metric_id} is not defined"
            ));
        }
    }
    Ok(())
}

pub fn build_headless_research_round_evidence(
    batch: &HeadlessExecutionBatch,
    report: &HeadlessRunReport,
    spec: &HeadlessResearchRoundSpec,
    patch_receipt: Option<&HeadlessParameterPatchReceipt>,
    previous: Option<&HeadlessResearchRoundEvidence>,
) -> Result<HeadlessResearchRoundEvidence, String> {
    validate_headless_research_round_spec(spec)?;
    if spec.workflow_id != batch.workflow_id {
        return Err(format!(
            "headless research round workflow mismatch: spec={}, batch={}",
            spec.workflow_id, batch.workflow_id
        ));
    }
    validate_execution(batch, report)?;
    let batch_content_sha256 = headless_batch_content_sha256(batch)?;
    let previous_round =
        validate_lineage(batch, spec, &batch_content_sha256, patch_receipt, previous)?;
    let report_value = serde_json::to_value(report)
        .map_err(|error| format!("failed to encode headless run report: {error}"))?;
    let metrics = collect_metrics(report, &report_value, &spec.metrics)?;

    Ok(HeadlessResearchRoundEvidence {
        schema_version: HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION.to_string(),
        round_id: spec.round_id.clone(),
        workflow_id: batch.workflow_id.clone(),
        iteration: spec.iteration,
        qualified: true,
        batch_content_sha256,
        run_report_sha256: canonical_json_sha256(&report_value),
        run_mode: report.mode.clone(),
        patch_receipt: patch_receipt.cloned(),
        previous_round,
        primary_metric_ids: spec.primary_metric_ids.clone(),
        metrics,
    })
}

pub fn validate_headless_research_round_evidence(
    evidence: &HeadlessResearchRoundEvidence,
) -> Result<(), String> {
    if evidence.schema_version != HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION
        || !evidence.qualified
        || evidence.run_mode != "execute:service"
        || !is_sha256(&evidence.batch_content_sha256)
        || !is_sha256(&evidence.run_report_sha256)
    {
        return Err("headless research round evidence is not qualified".to_string());
    }
    let spec = evidence_spec(evidence);
    validate_headless_research_round_spec(&spec)?;
    if evidence
        .metrics
        .iter()
        .any(|metric| !metric.value.is_finite())
    {
        return Err("headless research round evidence has a non-finite metric".to_string());
    }
    match evidence.iteration {
        1 if evidence.previous_round.is_some() || evidence.patch_receipt.is_some() => {
            return Err("headless research round evidence has invalid lineage".to_string());
        }
        2.. if evidence.previous_round.is_none() || evidence.patch_receipt.is_none() => {
            return Err("headless research round evidence has incomplete lineage".to_string());
        }
        _ => {}
    }
    if let Some(link) = &evidence.previous_round {
        validate_identifier("previous round_id", &link.round_id)?;
        if link.round_id == evidence.round_id
            || link.iteration.checked_add(1) != Some(evidence.iteration)
            || !is_sha256(&link.evidence_sha256)
            || !is_sha256(&link.batch_content_sha256)
        {
            return Err("headless research round evidence has invalid lineage".to_string());
        }
    }
    if let Some(receipt) = &evidence.patch_receipt {
        validate_identifier("patch_id", &receipt.patch_id)?;
        if receipt.schema_version != HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION
            || receipt.change_count == 0
            || receipt.change_count > 256
            || receipt.workflow_id != evidence.workflow_id
            || receipt.after_sha256 != evidence.batch_content_sha256
            || receipt.before_sha256 == receipt.after_sha256
            || !is_sha256(&receipt.before_sha256)
        {
            return Err("headless research round evidence has invalid patch lineage".to_string());
        }
        if evidence
            .previous_round
            .as_ref()
            .is_some_and(|link| receipt.before_sha256 != link.batch_content_sha256)
        {
            return Err(
                "headless research round evidence patch does not continue its lineage".to_string(),
            );
        }
    }
    Ok(())
}

pub fn verify_headless_research_round_evidence(
    batch: &HeadlessExecutionBatch,
    report: &HeadlessRunReport,
    evidence: &HeadlessResearchRoundEvidence,
    previous: Option<&HeadlessResearchRoundEvidence>,
) -> Result<(), String> {
    validate_headless_research_round_evidence(evidence)?;
    let rebuilt = build_headless_research_round_evidence(
        batch,
        report,
        &evidence_spec(evidence),
        evidence.patch_receipt.as_ref(),
        previous,
    )?;
    if rebuilt != *evidence {
        return Err(
            "headless research round evidence does not match its batch, report, or lineage"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_execution(
    batch: &HeadlessExecutionBatch,
    report: &HeadlessRunReport,
) -> Result<(), String> {
    if report.schema_version != HEADLESS_EXECUTION_RUN_SCHEMA_VERSION {
        return Err(format!(
            "headless research round requires {HEADLESS_EXECUTION_RUN_SCHEMA_VERSION}, got {}",
            report.schema_version
        ));
    }
    if report.workflow_id != batch.workflow_id {
        return Err(format!(
            "headless research round workflow mismatch: batch={}, report={}",
            batch.workflow_id, report.workflow_id
        ));
    }
    if report.mode != "execute:service" {
        return Err(format!(
            "headless research round requires execute:service evidence, got {}",
            report.mode
        ));
    }
    if report.status != "ok" || !report.validation.ok {
        return Err(format!(
            "headless research round requires a successful validated run, got status {}",
            report.status
        ));
    }
    let expected_validation = validate_batch(batch);
    if !expected_validation.ok {
        return Err(format!(
            "headless research round batch is invalid: {}",
            expected_validation.issues.join("; ")
        ));
    }
    if report.validation != expected_validation || report.warning_count != batch.warnings.len() {
        return Err(
            "headless research round run report validation does not match the effective batch"
                .to_string(),
        );
    }
    if report.blocked_by_confirmation.is_some()
        || report.executed_step_count != batch.steps.len()
        || report.steps.len() != batch.steps.len()
    {
        return Err(
            "headless research round requires every batch step to complete without blocking"
                .to_string(),
        );
    }
    for (batch_step, report_step) in batch.steps.iter().zip(&report.steps) {
        let requires_confirmation = matches!(
            batch_step.risk,
            HeadlessRisk::Sensitive | HeadlessRisk::Destructive
        );
        if report_step.index != batch_step.index
            || report_step.action != batch_step.action
            || report_step.risk != batch_step.risk
            || report_step.status != "executed"
            || report_step.requires_confirmation != requires_confirmation
        {
            return Err(
                "headless research round run report steps do not match the effective batch"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn validate_lineage(
    batch: &HeadlessExecutionBatch,
    spec: &HeadlessResearchRoundSpec,
    batch_content_sha256: &str,
    patch_receipt: Option<&HeadlessParameterPatchReceipt>,
    previous: Option<&HeadlessResearchRoundEvidence>,
) -> Result<Option<HeadlessResearchRoundLink>, String> {
    if spec.iteration == 1 {
        if previous.is_some() {
            return Err("headless research round 1 cannot declare a previous round".to_string());
        }
        if patch_receipt.is_some() {
            return Err(
                "headless research round 1 must start from an effective baseline without a parameter patch"
                    .to_string(),
            );
        }
        return Ok(None);
    }

    let previous = previous.ok_or_else(|| {
        "headless research round iteration 2 or later requires previous-round evidence".to_string()
    })?;
    let receipt = patch_receipt.ok_or_else(|| {
        "headless research round iteration 2 or later requires a parameter patch receipt"
            .to_string()
    })?;
    validate_previous_evidence(previous)?;
    if previous.workflow_id != batch.workflow_id {
        return Err("headless research round cannot cross workflow boundaries".to_string());
    }
    if previous.iteration.checked_add(1) != Some(spec.iteration) {
        return Err(format!(
            "headless research round iteration is not contiguous: previous={}, current={}",
            previous.iteration, spec.iteration
        ));
    }
    if previous.round_id == spec.round_id {
        return Err("headless research round_id must change between iterations".to_string());
    }
    validate_receipt_target(batch, batch_content_sha256, Some(receipt))?;
    if receipt.before_sha256 != previous.batch_content_sha256 {
        return Err(
            "headless research round parameter patch does not start from the previous batch"
                .to_string(),
        );
    }
    let previous_value = serde_json::to_value(previous)
        .map_err(|error| format!("failed to encode previous research evidence: {error}"))?;
    Ok(Some(HeadlessResearchRoundLink {
        round_id: previous.round_id.clone(),
        iteration: previous.iteration,
        evidence_sha256: canonical_json_sha256(&previous_value),
        batch_content_sha256: previous.batch_content_sha256.clone(),
    }))
}

fn validate_receipt_target(
    batch: &HeadlessExecutionBatch,
    batch_content_sha256: &str,
    receipt: Option<&HeadlessParameterPatchReceipt>,
) -> Result<(), String> {
    let Some(receipt) = receipt else {
        return Ok(());
    };
    if receipt.schema_version != HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION
        || receipt.change_count == 0
        || receipt.change_count > 256
        || !is_sha256(&receipt.before_sha256)
        || !is_sha256(&receipt.after_sha256)
        || receipt.workflow_id != batch.workflow_id
        || receipt.after_sha256 != batch_content_sha256
    {
        return Err(
            "headless research round parameter patch receipt does not match the current batch"
                .to_string(),
        );
    }
    if receipt.before_sha256 == receipt.after_sha256 {
        return Err("headless research round parameter patch did not change input".to_string());
    }
    Ok(())
}

fn validate_previous_evidence(evidence: &HeadlessResearchRoundEvidence) -> Result<(), String> {
    validate_headless_research_round_evidence(evidence).map_err(|error| {
        error.replacen(
            "headless research round evidence",
            "headless research round previous evidence",
            1,
        )
    })
}

fn evidence_spec(evidence: &HeadlessResearchRoundEvidence) -> HeadlessResearchRoundSpec {
    HeadlessResearchRoundSpec {
        schema_version: HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION.to_string(),
        round_id: evidence.round_id.clone(),
        workflow_id: evidence.workflow_id.clone(),
        iteration: evidence.iteration,
        primary_metric_ids: evidence.primary_metric_ids.clone(),
        metrics: evidence
            .metrics
            .iter()
            .map(|metric| HeadlessResearchMetricSpec {
                metric_id: metric.metric_id.clone(),
                pointer: metric.pointer.clone(),
                unit: metric.unit.clone(),
                objective: metric.objective,
            })
            .collect(),
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn collect_metrics(
    report: &HeadlessRunReport,
    report_value: &Value,
    specs: &[HeadlessResearchMetricSpec],
) -> Result<Vec<HeadlessResearchMetricObservation>, String> {
    specs
        .iter()
        .map(|spec| {
            let step_index = metric_step_index(&spec.pointer)?;
            if step_index >= report.steps.len() {
                return Err(format!(
                    "headless research metric {} references missing report step {}",
                    spec.metric_id, step_index
                ));
            }
            let value = report_value
                .pointer(&spec.pointer)
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    format!(
                        "headless research metric {} is missing or non-numeric at {}",
                        spec.metric_id, spec.pointer
                    )
                })?;
            Ok(HeadlessResearchMetricObservation {
                metric_id: spec.metric_id.clone(),
                pointer: spec.pointer.clone(),
                unit: spec.unit.clone(),
                objective: spec.objective,
                value,
            })
        })
        .collect()
}

fn validate_identifier(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "headless research round {label} must be a safe non-empty identifier"
        ));
    }
    Ok(())
}

fn validate_unit(unit: &str) -> Result<(), String> {
    if unit.trim().is_empty()
        || unit.len() > 64
        || unit.chars().any(|character| character.is_control())
    {
        return Err(
            "headless research metric unit must be a visible 1..=64 byte value".to_string(),
        );
    }
    Ok(())
}

fn validate_metric_pointer(pointer: &str) -> Result<(), String> {
    if pointer.len() > MAX_METRIC_POINTER_BYTES
        || pointer.chars().any(|character| character.is_control())
    {
        return Err("headless research metric pointer is invalid or too long".to_string());
    }
    metric_step_index(pointer).map(|_| ())
}

fn metric_step_index(pointer: &str) -> Result<usize, String> {
    let segments = pointer.split('/').collect::<Vec<_>>();
    if segments.len() < 6
        || segments[0] != ""
        || segments[1] != "steps"
        || segments[3] != "result_preview"
        || !matches!(segments[4], "result" | "metrics")
        || segments[5].is_empty()
        || segments[2..]
            .iter()
            .any(|segment| !is_canonical_pointer_segment(segment))
    {
        return Err(format!(
            "headless research metric may only read /steps/<zero-based-index>/result_preview/result/... or /metrics/... paths, got {pointer}"
        ));
    }
    segments[2].parse::<usize>().map_err(|_| {
        format!(
            "headless research metric may only read /steps/<zero-based-index>/result_preview/result/... or /metrics/... paths, got {pointer}"
        )
    })
}

fn is_canonical_pointer_segment(segment: &str) -> bool {
    if segment.is_empty() || segment.starts_with('+') {
        return false;
    }
    if segment.bytes().all(|byte| byte.is_ascii_digit())
        && segment.len() > 1
        && segment.starts_with('0')
    {
        return false;
    }
    let bytes = segment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~' {
            index += 1;
            if index >= bytes.len() || !matches!(bytes[index], b'0' | b'1') {
                return false;
            }
        }
        index += 1;
    }
    true
}

#[cfg(test)]
#[path = "research_round_tests.rs"]
mod tests;
