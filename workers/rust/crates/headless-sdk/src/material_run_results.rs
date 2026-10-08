use super::{find_material_study, preflight, typed_run_failure_context, value_run_failure_context};
use crate::service_executor_job_receipt::{validate_job_receipt, validate_result_envelope};
use crate::{HEADLESS_EXECUTION_RUN_SCHEMA_VERSION, HeadlessRunReport};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn invalid(detail: impl std::fmt::Display) -> String {
    format!("material-report run contract mismatch: {detail}")
}

pub(super) fn is_run_input(value: &Value) -> bool {
    value.get("steps").is_some()
        || value
            .get("schema_version")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema.starts_with("kyuubiki.headless-execution-run/"))
}

struct Step<'a> {
    index: usize,
    action: &'a str,
    status: &'a str,
    payload: &'a Value,
    preview: &'a Value,
}

pub(super) struct RetainedRun<'a> {
    mode: &'a str,
    steps: Vec<Step<'a>>,
}

impl<'a> RetainedRun<'a> {
    pub(super) fn typed(report: &'a HeadlessRunReport) -> Result<Self, String> {
        RunHeader {
            schema: Some(&report.schema_version),
            mode: Some(&report.mode),
            status: Some(&report.status),
            valid: report.validation.ok
                && report.validation.issue_count == 0
                && report.validation.issues.is_empty(),
            halted: report.blocked_by_confirmation.is_some()
                || report.execution_summary.failure.is_some(),
            count: Some(report.executed_step_count as u64),
            step_count: report.steps.len(),
            failure_context: typed_run_failure_context(report),
        }
        .validate()?;
        let run = Self {
            mode: &report.mode,
            steps: report
                .steps
                .iter()
                .map(|step| Step {
                    index: step.index,
                    action: &step.action,
                    status: &step.status,
                    payload: &step.payload,
                    preview: &step.result_preview,
                })
                .collect(),
        };
        run.validate()?;
        Ok(run)
    }

    pub(super) fn json(report: &'a Value) -> Result<Self, String> {
        let steps = report
            .get("steps")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("steps must be an explicit array"))?;
        RunHeader {
            schema: report.get("schema_version").and_then(Value::as_str),
            mode: report.get("mode").and_then(Value::as_str),
            status: report.get("status").and_then(Value::as_str),
            valid: report.pointer("/validation/ok").and_then(Value::as_bool) == Some(true)
                && report
                    .pointer("/validation/issue_count")
                    .and_then(Value::as_u64)
                    == Some(0)
                && report
                    .pointer("/validation/issues")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty),
            halted: report
                .get("blocked_by_confirmation")
                .is_some_and(|value| !value.is_null())
                || report
                    .pointer("/execution_summary/failure")
                    .is_some_and(|value| !value.is_null()),
            count: report.get("executed_step_count").and_then(Value::as_u64),
            step_count: steps.len(),
            failure_context: value_run_failure_context(report),
        }
        .validate()?;
        let run = Self {
            mode: report["mode"].as_str().expect("validated execution mode"),
            steps: steps
                .iter()
                .map(|step| {
                    Ok(Step {
                        index: step
                            .get("index")
                            .and_then(Value::as_u64)
                            .and_then(|index| usize::try_from(index).ok())
                            .ok_or_else(|| {
                                invalid("step index must be a represented unsigned integer")
                            })?,
                        action: step
                            .get("action")
                            .and_then(Value::as_str)
                            .ok_or_else(|| invalid("step action is missing"))?,
                        status: step
                            .get("status")
                            .and_then(Value::as_str)
                            .ok_or_else(|| invalid("step status is missing"))?,
                        payload: step
                            .get("payload")
                            .ok_or_else(|| invalid("step payload is missing"))?,
                        preview: step
                            .get("result_preview")
                            .ok_or_else(|| invalid("step result_preview is missing"))?,
                    })
                })
                .collect::<Result<_, String>>()?,
        };
        run.validate()?;
        Ok(run)
    }

    fn validate(&self) -> Result<(), String> {
        let mut fetches = 0;
        for (position, step) in self.steps.iter().enumerate() {
            if step.index != position + 1 || step.status != "executed" {
                return Err(invalid(format!(
                    "step {} must have its ordered index and status=executed, received {}",
                    step.index, step.status
                )));
            }
            if self.mode != "execute:mock"
                && (step.preview.get("preview_only") == Some(&Value::Bool(true))
                    || step.preview.pointer("/result/kind").and_then(Value::as_str)
                        == Some("simulated_result"))
            {
                return Err(invalid(format!(
                    "step {} retains a preview rather than a computation receipt",
                    step.index
                )));
            }
            if step.action == "result_fetch" {
                let job_id = requested_job(step)?;
                validate_result_envelope(job_id, step.preview)
                    .map_err(|error| invalid(format!("step {}: {}", step.index, error.message)))?;
                if step.preview["result"]
                    .get("$kyuubiki_report_summary")
                    .is_some()
                {
                    return Err(invalid(format!(
                        "step {} retains a compacted summary, not an object result",
                        step.index
                    )));
                }
                fetches += 1;
            }
        }
        if fetches == 0 {
            return Err(invalid(
                "headless execution run report does not contain successful result_fetch payloads",
            ));
        }
        Ok(())
    }

    pub(super) fn validate_study(&self, study: &str) -> Result<(), String> {
        let study = find_material_study(study)
            .ok_or_else(|| format!("unsupported material report study: {study}"))?;
        let (action, candidates) = preflight::expected_candidates(study.id)?;
        let mut roots = BTreeMap::new();
        let mut identities = BTreeSet::new();
        let mut waited = BTreeSet::new();
        let mut position = 0;
        for step in &self.steps {
            if step.action == action {
                let candidate = step
                    .payload
                    .pointer("/research/candidate_id")
                    .and_then(Value::as_str)
                    .filter(|id| candidates.contains(id))
                    .ok_or_else(|| {
                        invalid(format!("step {} has an unknown candidate", step.index))
                    })?;
                if step
                    .payload
                    .pointer("/research/study")
                    .and_then(Value::as_str)
                    .and_then(find_material_study)
                    .is_none_or(|declared| declared.id != study.id)
                {
                    return Err(invalid(format!(
                        "step {} candidate {candidate} has a mismatched study",
                        step.index
                    )));
                }
                let job_id = receipt_job(step)?;
                if !identities.insert(candidate) || roots.insert(job_id, candidate).is_some() {
                    return Err(invalid("candidate or submitted job identity is repeated"));
                }
            } else if step.action == "job_wait" {
                let requested = requested_job(step)?;
                if roots.contains_key(requested) {
                    if receipt_job(step)? != requested
                        || step.preview.get("status").and_then(Value::as_str) != Some("completed")
                    {
                        return Err(invalid(format!(
                            "step {} must confirm its candidate job completed",
                            step.index
                        )));
                    }
                    waited.insert(requested);
                }
            } else if step.action == "result_fetch" {
                let job_id = requested_job(step)?;
                let candidate = roots.get(job_id).ok_or_else(|| {
                    invalid(format!(
                        "step {} reads an unowned candidate job",
                        step.index
                    ))
                })?;
                if !waited.contains(job_id) || candidates.get(position) != Some(candidate) {
                    return Err(invalid(format!(
                        "step {} lacks an owned completed wait or violates candidate result order",
                        step.index
                    )));
                }
                if let Some(research) = step.preview["result"].get("research") {
                    if !research.is_object()
                        || research
                            .get("candidate_id")
                            .is_some_and(|id| id.as_str() != Some(*candidate))
                        || research.get("study").is_some_and(|id| {
                            id.as_str()
                                .and_then(find_material_study)
                                .is_none_or(|declared| declared.id != study.id)
                        })
                    {
                        return Err(invalid(format!(
                            "step {} returned research metadata contradicts its candidate job",
                            step.index
                        )));
                    }
                }
                position += 1;
            }
        }
        if identities.len() != candidates.len() || position != candidates.len() {
            return Err(invalid(
                "fixed study requires every unique candidate solve, completed wait and ordered result",
            ));
        }
        Ok(())
    }

    pub(super) fn payloads(&self) -> Vec<Value> {
        self.steps
            .iter()
            .filter(|step| step.action == "result_fetch")
            .map(|step| step.preview["result"].clone())
            .collect()
    }
}

struct RunHeader<'a> {
    schema: Option<&'a str>,
    mode: Option<&'a str>,
    status: Option<&'a str>,
    valid: bool,
    halted: bool,
    count: Option<u64>,
    step_count: usize,
    failure_context: Option<String>,
}

impl RunHeader<'_> {
    fn validate(self) -> Result<(), String> {
        if self.schema != Some(HEADLESS_EXECUTION_RUN_SCHEMA_VERSION) {
            return Err(invalid("unsupported or missing execution run schema"));
        }
        if self.status != Some("ok") {
            return Err(invalid(format!(
                "run status must be ok, received {}; {}",
                self.status.unwrap_or("missing"),
                self.failure_context
                    .as_deref()
                    .unwrap_or("unfinished runs cannot supply fixed material results")
            )));
        }
        if !self.mode.is_some_and(|mode| {
            mode.strip_prefix("execute:")
                .is_some_and(|name| !name.trim().is_empty())
        }) {
            return Err(invalid("run mode must declare an executor, not dry_run"));
        }
        if !self.valid || self.halted || self.count != Some(self.step_count as u64) {
            return Err(invalid(
                "run validation, confirmation/failure state or executed step count is contradictory",
            ));
        }
        Ok(())
    }
}

fn requested_job<'a>(step: &Step<'a>) -> Result<&'a str, String> {
    let primary = step.payload.get("job_id");
    let alias = step.payload.get("jobId");
    if primary.is_some() && alias.is_some() && primary != alias {
        return Err(invalid(format!(
            "step {} has contradictory job aliases",
            step.index
        )));
    }
    primary
        .or(alias)
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| invalid(format!("step {} lacks a resolved job identity", step.index)))
}

fn receipt_job<'a>(step: &Step<'a>) -> Result<&'a str, String> {
    let id = step
        .preview
        .get("job_id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            invalid(format!(
                "step {} lacks a submitted job identity",
                step.index
            ))
        })?;
    if step.preview.get("job").is_some() {
        validate_job_receipt(id, step.preview)
            .map_err(|error| invalid(format!("step {}: {}", step.index, error.message)))?;
    }
    Ok(id)
}
