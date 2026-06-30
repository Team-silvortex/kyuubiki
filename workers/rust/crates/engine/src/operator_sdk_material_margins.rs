use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};
use serde_json::{Map, Value};

struct MaterialMarginOperator {
    descriptor: OperatorDescriptor,
}

struct RankMaterialCandidatesOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for MaterialMarginOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            evaluate_material_margins(input.payload, input.config),
        )
    }
}

impl JsonOperator for RankMaterialCandidatesOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            rank_material_candidates(input.payload, input.config),
        )
    }
}

pub(crate) fn register_material_margin_operator(
    registry: &mut OperatorRegistry,
    descriptor: fn(&str) -> OperatorDescriptor,
) {
    registry
        .register_json(MaterialMarginOperator {
            descriptor: descriptor("transform.evaluate_material_margins"),
        })
        .expect("transform.evaluate_material_margins should register");
    registry
        .register_json(RankMaterialCandidatesOperator {
            descriptor: descriptor("transform.rank_material_candidates"),
        })
        .expect("transform.rank_material_candidates should register");
}

pub fn evaluate_material_margins(payload: Value, config: Value) -> Result<Value, String> {
    let limits = config
        .get("limits")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.evaluate_material_margins expects config.limits object".to_string()
        })?;
    if limits.is_empty() {
        return Err("transform.evaluate_material_margins requires at least one limit".to_string());
    }

    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .unwrap_or("material");
    let mut output = Map::new();
    let mut constraint_count = 0u64;
    let mut violation_count = 0u64;
    let mut critical: Option<MarginEvaluation> = None;

    for (field, spec) in limits {
        let Some(limit) = limit_value(spec) else {
            continue;
        };
        if !(limit.is_finite() && limit > 0.0) {
            continue;
        }
        let Some(actual) = lookup_numeric(&payload, field) else {
            continue;
        };
        let direction = limit_direction(spec);
        let failure_index = match direction {
            "min" => limit / actual,
            "abs" => actual.abs() / limit,
            _ => actual / limit,
        };
        if !failure_index.is_finite() {
            continue;
        }

        constraint_count += 1;
        if failure_index > 1.0 {
            violation_count += 1;
        }
        output.insert(format!("{prefix}_{field}_actual"), Value::from(actual));
        output.insert(format!("{prefix}_{field}_limit"), Value::from(limit));
        output.insert(
            format!("{prefix}_{field}_failure_index"),
            Value::from(failure_index),
        );
        output.insert(
            format!("{prefix}_{field}_safety_factor"),
            Value::from(1.0 / failure_index),
        );

        let evaluation = MarginEvaluation {
            field: field.to_string(),
            actual,
            limit,
            failure_index,
        };
        if critical
            .as_ref()
            .is_none_or(|current| evaluation.failure_index > current.failure_index)
        {
            critical = Some(evaluation);
        }
    }

    let Some(critical) = critical else {
        return Err(
            "transform.evaluate_material_margins found no matching numeric limits".to_string(),
        );
    };

    output.insert(
        format!("{prefix}_constraint_count"),
        Value::from(constraint_count),
    );
    output.insert(
        format!("{prefix}_violation_count"),
        Value::from(violation_count),
    );
    output.insert(
        format!("{prefix}_failure_index"),
        Value::from(critical.failure_index),
    );
    output.insert(
        format!("{prefix}_safety_factor"),
        Value::from(1.0 / critical.failure_index),
    );
    output.insert(
        format!("{prefix}_critical_metric"),
        Value::from(critical.field),
    );
    output.insert(
        format!("{prefix}_critical_actual"),
        Value::from(critical.actual),
    );
    output.insert(
        format!("{prefix}_critical_limit"),
        Value::from(critical.limit),
    );
    output.insert(
        format!("{prefix}_status"),
        Value::from(if violation_count == 0 { "pass" } else { "fail" }),
    );
    Ok(Value::Object(output))
}

pub fn rank_material_candidates(payload: Value, config: Value) -> Result<Value, String> {
    let prefix = config
        .get("margin_prefix")
        .and_then(Value::as_str)
        .unwrap_or("material");
    let entries = material_candidate_entries(&payload)?;
    if entries.is_empty() {
        return Err("transform.rank_material_candidates requires candidate summaries".to_string());
    }

    let mut ranked = Vec::new();
    let mut failure_reasons = Map::new();
    for (candidate_id, summary) in entries {
        let failure_index = lookup_prefixed(summary, prefix, "failure_index").unwrap_or(f64::MAX);
        let safety_factor = lookup_prefixed(summary, prefix, "safety_factor").unwrap_or(0.0);
        let violation_count = lookup_prefixed(summary, prefix, "violation_count").unwrap_or(1.0);
        let status = lookup_prefixed_text(summary, prefix, "status").unwrap_or("unknown");
        let critical_metric =
            lookup_prefixed_text(summary, prefix, "critical_metric").unwrap_or("unknown");
        let feasible = status == "pass" && violation_count == 0.0 && failure_index <= 1.0;
        if !feasible {
            let count = failure_reasons
                .get(critical_metric)
                .and_then(Value::as_u64)
                .unwrap_or(0)
                + 1;
            failure_reasons.insert(critical_metric.to_string(), Value::from(count));
        }
        ranked.push(CandidateRank {
            candidate_id,
            feasible,
            safety_factor,
            failure_index,
            critical_metric: critical_metric.to_string(),
            summary: summary.clone(),
        });
    }

    ranked.sort_by(|left, right| {
        right
            .feasible
            .cmp(&left.feasible)
            .then_with(|| right.safety_factor.total_cmp(&left.safety_factor))
            .then_with(|| left.failure_index.total_cmp(&right.failure_index))
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    let best = ranked.first().ok_or_else(|| {
        "transform.rank_material_candidates could not rank candidates".to_string()
    })?;
    let feasible_count = ranked.iter().filter(|entry| entry.feasible).count();

    let mut output = Map::new();
    output.insert(
        "material_candidate_count".to_string(),
        Value::from(ranked.len()),
    );
    output.insert(
        "material_feasible_count".to_string(),
        Value::from(feasible_count),
    );
    output.insert(
        "material_best_candidate_id".to_string(),
        Value::from(best.candidate_id.clone()),
    );
    output.insert(
        "material_best_candidate_feasible".to_string(),
        Value::from(best.feasible),
    );
    output.insert(
        "material_best_safety_factor".to_string(),
        Value::from(best.safety_factor),
    );
    output.insert(
        "material_best_failure_index".to_string(),
        Value::from(best.failure_index),
    );
    output.insert(
        "material_failure_reasons".to_string(),
        Value::Object(failure_reasons),
    );
    output.insert(
        "material_rankings".to_string(),
        Value::Array(
            ranked
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "candidate_id": entry.candidate_id,
                        "feasible": entry.feasible,
                        "safety_factor": entry.safety_factor,
                        "failure_index": entry.failure_index,
                        "critical_metric": entry.critical_metric
                    })
                })
                .collect(),
        ),
    );
    if config
        .get("include_best_summary")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        output.insert(
            "material_best_summary".to_string(),
            Value::Object(best.summary.clone()),
        );
    }
    Ok(Value::Object(output))
}

struct MarginEvaluation {
    field: String,
    actual: f64,
    limit: f64,
    failure_index: f64,
}

struct CandidateRank {
    candidate_id: String,
    feasible: bool,
    safety_factor: f64,
    failure_index: f64,
    critical_metric: String,
    summary: Map<String, Value>,
}

fn material_candidate_entries(
    payload: &Value,
) -> Result<Vec<(String, &Map<String, Value>)>, String> {
    if let Some(candidates) = payload.get("candidates").and_then(Value::as_object) {
        return Ok(candidates
            .iter()
            .filter_map(|(candidate_id, summary)| {
                summary
                    .as_object()
                    .map(|entry| (candidate_id.clone(), entry))
            })
            .collect());
    }
    payload
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter_map(|(candidate_id, summary)| {
                    summary
                        .as_object()
                        .map(|entry| (candidate_id.clone(), entry))
                })
                .collect()
        })
        .ok_or_else(|| "transform.rank_material_candidates expects an object payload".to_string())
}

fn lookup_prefixed(summary: &Map<String, Value>, prefix: &str, field: &str) -> Option<f64> {
    summary
        .get(&format!("{prefix}_{field}"))
        .and_then(Value::as_f64)
}

fn lookup_prefixed_text<'a>(
    summary: &'a Map<String, Value>,
    prefix: &str,
    field: &str,
) -> Option<&'a str> {
    summary
        .get(&format!("{prefix}_{field}"))
        .and_then(Value::as_str)
}

fn limit_value(spec: &Value) -> Option<f64> {
    spec.as_f64()
        .or_else(|| spec.get("limit").and_then(Value::as_f64))
        .or_else(|| spec.get("max").and_then(Value::as_f64))
        .or_else(|| spec.get("min").and_then(Value::as_f64))
}

fn limit_direction(spec: &Value) -> &str {
    spec.get("direction")
        .and_then(Value::as_str)
        .or_else(|| spec.get("kind").and_then(Value::as_str))
        .unwrap_or_else(|| {
            if spec.get("min").is_some() {
                "min"
            } else {
                "max"
            }
        })
}

fn lookup_numeric(payload: &Value, field: &str) -> Option<f64> {
    payload
        .get(field)
        .and_then(Value::as_f64)
        .or_else(|| lookup_path(payload, field))
        .or_else(|| {
            payload
                .get("summary")
                .and_then(|summary| lookup_numeric(summary, field))
        })
}

fn lookup_path(payload: &Value, field: &str) -> Option<f64> {
    let mut current = payload;
    for part in field.split('.') {
        current = current.get(part)?;
    }
    current.as_f64()
}
