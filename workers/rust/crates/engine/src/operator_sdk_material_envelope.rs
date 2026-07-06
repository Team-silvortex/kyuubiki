use crate::operator_sdk_runtime::{run_summary_only, WorkflowOperatorEnvelope};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

struct MaterialStudyEnvelopeOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for MaterialStudyEnvelopeOperator {
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
            compose_material_study_envelope(input.payload, input.config),
        )
    }
}

pub(crate) fn register_material_envelope_operator(
    registry: &mut OperatorRegistry,
    descriptor: fn(&str) -> OperatorDescriptor,
) {
    registry
        .register_json(MaterialStudyEnvelopeOperator {
            descriptor: descriptor("transform.compose_material_study_envelope"),
        })
        .expect("transform.compose_material_study_envelope should register");
}

pub fn compose_material_study_envelope(payload: Value, config: Value) -> Result<Value, String> {
    if let Some(entries) = material_candidate_entries(&payload)? {
        return compose_material_study_envelope_batch(entries, config);
    }
    compose_single_material_study_envelope(payload, config)
}

fn compose_material_study_envelope_batch(
    entries: Vec<(String, Value)>,
    config: Value,
) -> Result<Value, String> {
    if entries.is_empty() {
        return Err(
            "transform.compose_material_study_envelope requires at least one candidate".to_string(),
        );
    }

    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .unwrap_or("material_envelope")
        .to_string();
    let mut candidates = Map::new();
    let mut envelopes = Vec::with_capacity(entries.len());
    for (candidate_id, mut row) in entries {
        if let Value::Object(object) = &mut row {
            object
                .entry("candidate_id")
                .or_insert_with(|| Value::String(candidate_id.clone()));
        }
        let envelope = compose_single_material_study_envelope(row, config.clone())?;
        candidates.insert(candidate_id.clone(), envelope.clone());
        envelopes.push(envelope);
    }

    let best = envelopes
        .iter()
        .filter(|entry| {
            entry
                .get(format!("{prefix}_status"))
                .and_then(Value::as_str)
                == Some("pass")
        })
        .min_by(|left, right| {
            let left_score = left
                .get(format!("{prefix}_score"))
                .and_then(Value::as_f64)
                .unwrap_or(f64::MAX);
            let right_score = right
                .get(format!("{prefix}_score"))
                .and_then(Value::as_f64)
                .unwrap_or(f64::MAX);
            left_score.total_cmp(&right_score)
        })
        .or_else(|| {
            envelopes.iter().min_by(|left, right| {
                let left_index = left
                    .get(format!("{prefix}_failure_index"))
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::MAX);
                let right_index = right
                    .get(format!("{prefix}_failure_index"))
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::MAX);
                left_index.total_cmp(&right_index)
            })
        });
    let best_id = best
        .and_then(|entry| entry.get(format!("{prefix}_candidate_id")))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Ok(serde_json::json!({
        format!("{prefix}_batch_contract"): "kyuubiki.material_study_envelope_batch/v1",
        format!("{prefix}_candidate_count"): envelopes.len(),
        format!("{prefix}_best_candidate_id"): best_id,
        "candidates": candidates,
        "envelopes": envelopes,
    }))
}

fn compose_single_material_study_envelope(payload: Value, config: Value) -> Result<Value, String> {
    let specs = material_metric_specs(&config)?;
    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .unwrap_or("material_envelope");
    let mut metrics = Vec::new();
    let mut domains = BTreeSet::new();

    for spec in specs {
        let Some(actual) = lookup_metric_value(&payload, &spec) else {
            continue;
        };
        let Some(limit) = spec.limit else {
            continue;
        };
        if !(actual.is_finite() && limit.is_finite() && limit > 0.0) {
            continue;
        }
        let failure_index = failure_index(actual, limit, &spec.direction);
        if !failure_index.is_finite() {
            continue;
        }
        domains.insert(spec.source.clone());
        metrics.push(EvaluatedMetric {
            source: spec.source,
            field: spec.field,
            alias: spec.alias,
            actual,
            limit,
            direction: spec.direction,
            weight: spec.weight,
            failure_index,
        });
    }

    if metrics.is_empty() {
        return Err(
            "transform.compose_material_study_envelope found no matching numeric metrics"
                .to_string(),
        );
    }

    let critical = metrics
        .iter()
        .max_by(|left, right| left.failure_index.total_cmp(&right.failure_index))
        .expect("metrics should not be empty");
    let score = metrics
        .iter()
        .map(|metric| metric.failure_index * metric.weight)
        .sum::<f64>();
    let violation_count = metrics
        .iter()
        .filter(|metric| metric.failure_index > 1.0)
        .count();
    let status = if violation_count == 0 { "pass" } else { "fail" };
    let candidate_id = candidate_id(&payload, &config).unwrap_or_else(|| "candidate".to_string());

    Ok(serde_json::json!({
        format!("{prefix}_contract"): "kyuubiki.material_study_envelope/v1",
        format!("{prefix}_candidate_id"): candidate_id,
        format!("{prefix}_domain_count"): domains.len(),
        format!("{prefix}_domains"): domains.into_iter().collect::<Vec<_>>(),
        format!("{prefix}_metric_count"): metrics.len(),
        format!("{prefix}_violation_count"): violation_count,
        format!("{prefix}_status"): status,
        format!("{prefix}_score"): score,
        format!("{prefix}_failure_index"): critical.failure_index,
        format!("{prefix}_safety_factor"): 1.0 / critical.failure_index,
        format!("{prefix}_critical_metric"): critical.metric_id(),
        format!("{prefix}_critical_actual"): critical.actual,
        format!("{prefix}_critical_limit"): critical.limit,
        format!("{prefix}_metrics"): metrics.iter().map(EvaluatedMetric::summary).collect::<Vec<_>>(),
    }))
}

#[derive(Clone)]
struct MetricSpec {
    source: String,
    field: String,
    alias: String,
    limit: Option<f64>,
    direction: String,
    weight: f64,
}

struct EvaluatedMetric {
    source: String,
    field: String,
    alias: String,
    actual: f64,
    limit: f64,
    direction: String,
    weight: f64,
    failure_index: f64,
}

impl EvaluatedMetric {
    fn metric_id(&self) -> String {
        format!("{}.{}", self.source, self.alias)
    }

    fn summary(&self) -> Value {
        serde_json::json!({
            "source": self.source,
            "field": self.field,
            "alias": self.alias,
            "actual": self.actual,
            "limit": self.limit,
            "direction": self.direction,
            "weight": self.weight,
            "failure_index": self.failure_index,
            "safety_factor": 1.0 / self.failure_index,
            "weighted_score": self.failure_index * self.weight,
            "status": if self.failure_index <= 1.0 { "pass" } else { "fail" },
        })
    }
}

fn material_metric_specs(config: &Value) -> Result<Vec<MetricSpec>, String> {
    if let Some(metrics) = config.get("metrics").and_then(Value::as_array) {
        if metrics.is_empty() {
            return Err(
                "transform.compose_material_study_envelope metrics must not be empty".into(),
            );
        }
        return metrics
            .iter()
            .enumerate()
            .map(|(index, metric)| metric_spec_from_value(metric, index))
            .collect();
    }
    Ok(default_metric_specs())
}

fn metric_spec_from_value(metric: &Value, index: usize) -> Result<MetricSpec, String> {
    let source = metric
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("summary");
    let field = metric
        .get("field")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("material envelope metric {index} requires field"))?;
    Ok(MetricSpec {
        source: source.to_string(),
        field: field.to_string(),
        alias: metric
            .get("alias")
            .and_then(Value::as_str)
            .unwrap_or(field)
            .to_string(),
        limit: metric_limit(metric),
        direction: metric
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("max")
            .to_string(),
        weight: metric
            .get("weight")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(1.0),
    })
}

fn default_metric_specs() -> Vec<MetricSpec> {
    [
        ("thermal", "max_temperature", "temperature", 120.0, 2.0),
        ("thermal", "max_heat_flux", "heat_flux", 30.0, 1.5),
        ("structural", "max_stress", "stress", 250.0, 2.0),
        ("structural", "max_displacement", "displacement", 0.01, 1.0),
        (
            "electrostatic",
            "max_electric_field",
            "electric_field",
            5.0,
            1.5,
        ),
        (
            "magnetostatic",
            "max_flux_density",
            "flux_density",
            2.0,
            1.5,
        ),
        ("cfd", "fluid_reynolds_peak", "reynolds", 2000.0, 1.0),
    ]
    .into_iter()
    .map(|(source, field, alias, limit, weight)| MetricSpec {
        source: source.to_string(),
        field: field.to_string(),
        alias: alias.to_string(),
        limit: Some(limit),
        direction: "max".to_string(),
        weight,
    })
    .collect()
}

fn lookup_metric_value(payload: &Value, spec: &MetricSpec) -> Option<f64> {
    source_payload(payload, &spec.source)
        .and_then(|source| lookup_path(source, &spec.field))
        .or_else(|| lookup_path(payload, &format!("{}.{}", spec.source, spec.field)))
        .or_else(|| lookup_path(payload, &spec.field))
}

fn source_payload<'a>(payload: &'a Value, source: &str) -> Option<&'a Value> {
    payload
        .get("summaries")
        .and_then(|summaries| summaries.get(source))
        .or_else(|| payload.get(source))
        .or_else(|| (source == "summary").then_some(payload))
}

fn lookup_path(payload: &Value, path: &str) -> Option<f64> {
    let mut current = payload;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    current.as_f64()
}

fn metric_limit(metric: &Value) -> Option<f64> {
    metric
        .get("limit")
        .and_then(Value::as_f64)
        .or_else(|| metric.get("max").and_then(Value::as_f64))
        .or_else(|| metric.get("min").and_then(Value::as_f64))
}

fn failure_index(actual: f64, limit: f64, direction: &str) -> f64 {
    match direction {
        "min" => limit / actual.max(1.0e-12),
        "abs" => actual.abs() / limit,
        _ => actual / limit,
    }
}

fn candidate_id(payload: &Value, config: &Value) -> Option<String> {
    config
        .get("candidate_id")
        .and_then(Value::as_str)
        .or_else(|| payload.get("candidate_id").and_then(Value::as_str))
        .or_else(|| payload.get("id").and_then(Value::as_str))
        .map(str::to_string)
}

fn material_candidate_entries(payload: &Value) -> Result<Option<Vec<(String, Value)>>, String> {
    if let Some(rows) = payload.get("rows").and_then(Value::as_array) {
        return rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let object = row.as_object().ok_or_else(|| {
                    format!("transform.compose_material_study_envelope row {index} must be object")
                })?;
                Ok((entry_candidate_id(object, index), row.clone()))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Some);
    }
    if let Some(candidates) = payload.get("candidates").and_then(Value::as_object) {
        return candidates
            .iter()
            .map(|(candidate_id, candidate)| {
                if !candidate.is_object() {
                    return Err(format!(
                        "transform.compose_material_study_envelope candidate {candidate_id} must be object"
                    ));
                }
                Ok((candidate_id.clone(), candidate.clone()))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Some);
    }
    Ok(None)
}

fn entry_candidate_id(object: &Map<String, Value>, index: usize) -> String {
    object
        .get("candidate_id")
        .or_else(|| object.get("case_id"))
        .or_else(|| object.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("candidate_{}", index + 1))
}
