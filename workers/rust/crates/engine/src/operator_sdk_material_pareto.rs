use crate::operator_sdk_runtime::{run_summary_only, WorkflowOperatorEnvelope};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};
use serde_json::{Map, Value};

struct ExtractMaterialParetoFrontierOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for ExtractMaterialParetoFrontierOperator {
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
            extract_material_pareto_frontier(input.payload, input.config),
        )
    }
}

pub(crate) fn register_material_pareto_operator(
    registry: &mut OperatorRegistry,
    descriptor: fn(&str) -> OperatorDescriptor,
) {
    registry
        .register_json(ExtractMaterialParetoFrontierOperator {
            descriptor: descriptor("transform.extract_material_pareto_frontier"),
        })
        .expect("transform.extract_material_pareto_frontier should register");
}

pub fn extract_material_pareto_frontier(payload: Value, config: Value) -> Result<Value, String> {
    let objectives = parse_pareto_objectives(&config)?;
    let candidates = pareto_candidate_entries(&payload)?;
    if candidates.is_empty() {
        return Err(
            "transform.extract_material_pareto_frontier requires candidate rows".to_string(),
        );
    }

    let include_infeasible = config
        .get("include_infeasible")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut evaluated = candidates
        .into_iter()
        .map(|candidate| evaluate_pareto_candidate(candidate, &objectives, &config))
        .collect::<Result<Vec<_>, String>>()?;
    evaluated.sort_by(|left, right| {
        right
            .feasible
            .cmp(&left.feasible)
            .then_with(|| right.objective_score.total_cmp(&left.objective_score))
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let mut frontier = Vec::new();
    let mut dominated = Vec::new();
    for (index, candidate) in evaluated.iter().enumerate() {
        if !include_infeasible && !candidate.feasible {
            dominated.push(candidate.dominated_summary("infeasible"));
            continue;
        }
        let dominator = evaluated
            .iter()
            .enumerate()
            .find(|(other_index, other)| {
                *other_index != index
                    && (include_infeasible || other.feasible)
                    && pareto_dominates(other, candidate, &objectives)
            })
            .map(|(_, other)| other.candidate_id.as_str());
        if let Some(dominator_id) = dominator {
            dominated.push(candidate.dominated_summary(dominator_id));
        } else {
            frontier.push(candidate.frontier_summary());
        }
    }
    frontier.sort_by(|left, right| compare_frontier_entries(left, right));
    let best_id = frontier
        .first()
        .and_then(|entry| entry.get("candidate_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let feasible_count = evaluated.iter().filter(|entry| entry.feasible).count();

    Ok(serde_json::json!({
        "material_pareto_candidate_count": evaluated.len(),
        "material_pareto_feasible_count": feasible_count,
        "material_pareto_frontier_count": frontier.len(),
        "material_pareto_best_candidate_id": best_id,
        "material_pareto_objectives": objectives.iter().map(ParetoObjective::summary).collect::<Vec<_>>(),
        "material_pareto_frontier": frontier,
        "material_pareto_dominated": dominated,
    }))
}

struct ParetoObjective {
    field: String,
    goal: String,
    weight: f64,
}

struct ParetoCandidate {
    candidate_id: String,
    summary: Map<String, Value>,
}

struct EvaluatedParetoCandidate {
    candidate_id: String,
    summary: Map<String, Value>,
    metrics: Map<String, Value>,
    feasible: bool,
    objective_score: f64,
}

impl ParetoObjective {
    fn summary(&self) -> Value {
        serde_json::json!({
            "field": self.field,
            "goal": self.goal,
            "weight": self.weight,
        })
    }

    fn no_worse(&self, left: f64, right: f64) -> bool {
        match self.goal.as_str() {
            "max" => left >= right,
            _ => left <= right,
        }
    }

    fn strictly_better(&self, left: f64, right: f64) -> bool {
        match self.goal.as_str() {
            "max" => left > right,
            _ => left < right,
        }
    }

    fn weighted_score(&self, value: f64) -> f64 {
        match self.goal.as_str() {
            "max" => value * self.weight,
            _ => -value * self.weight,
        }
    }
}

impl EvaluatedParetoCandidate {
    fn frontier_summary(&self) -> Value {
        serde_json::json!({
            "candidate_id": self.candidate_id,
            "feasible": self.feasible,
            "objective_score": self.objective_score,
            "metrics": self.metrics,
            "summary": self.summary,
        })
    }

    fn dominated_summary(&self, dominated_by: &str) -> Value {
        serde_json::json!({
            "candidate_id": self.candidate_id,
            "feasible": self.feasible,
            "objective_score": self.objective_score,
            "dominated_by": dominated_by,
            "metrics": self.metrics,
        })
    }
}

fn parse_pareto_objectives(config: &Value) -> Result<Vec<ParetoObjective>, String> {
    let objectives = config
        .get("objectives")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "transform.extract_material_pareto_frontier requires config.objectives".to_string()
        })?;
    if objectives.is_empty() {
        return Err(
            "transform.extract_material_pareto_frontier objectives must not be empty".to_string(),
        );
    }
    objectives
        .iter()
        .enumerate()
        .map(|(index, objective)| {
            let field = objective
                .get("field")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    format!(
                        "transform.extract_material_pareto_frontier objective {index} requires field"
                    )
                })?;
            let goal = objective
                .get("goal")
                .and_then(Value::as_str)
                .unwrap_or("min");
            if goal != "min" && goal != "max" {
                return Err(format!(
                    "transform.extract_material_pareto_frontier objective {index} has unsupported goal {goal}"
                ));
            }
            Ok(ParetoObjective {
                field: field.to_string(),
                goal: goal.to_string(),
                weight: objective
                    .get("weight")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
            })
        })
        .collect()
}

fn pareto_candidate_entries(payload: &Value) -> Result<Vec<ParetoCandidate>, String> {
    if let Some(rows) = payload.get("rows").and_then(Value::as_array) {
        return rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let summary = row.as_object().ok_or_else(|| {
                    format!(
                        "transform.extract_material_pareto_frontier row {index} must be an object"
                    )
                })?;
                Ok(ParetoCandidate {
                    candidate_id: candidate_id_from_summary(summary, index),
                    summary: summary.clone(),
                })
            })
            .collect();
    }
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_object)
        .or_else(|| payload.as_object())
        .ok_or_else(|| {
            "transform.extract_material_pareto_frontier expects rows or candidates".to_string()
        })?;
    Ok(candidates
        .iter()
        .filter_map(|(candidate_id, summary)| {
            summary.as_object().map(|entry| ParetoCandidate {
                candidate_id: candidate_id.clone(),
                summary: entry.clone(),
            })
        })
        .collect())
}

fn candidate_id_from_summary(summary: &Map<String, Value>, index: usize) -> String {
    summary
        .get("candidate_id")
        .or_else(|| summary.get("case_id"))
        .or_else(|| summary.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("candidate_{index}"))
}

fn evaluate_pareto_candidate(
    candidate: ParetoCandidate,
    objectives: &[ParetoObjective],
    config: &Value,
) -> Result<EvaluatedParetoCandidate, String> {
    let mut metrics = Map::new();
    let mut objective_score = 0.0;
    for objective in objectives {
        let value =
            lookup_summary_numeric(&candidate.summary, &objective.field).ok_or_else(|| {
                format!(
                "transform.extract_material_pareto_frontier candidate {} missing numeric field {}",
                candidate.candidate_id, objective.field
            )
            })?;
        metrics.insert(objective.field.clone(), Value::from(value));
        objective_score += objective.weighted_score(value);
    }
    Ok(EvaluatedParetoCandidate {
        candidate_id: candidate.candidate_id,
        feasible: pareto_candidate_is_feasible(&candidate.summary, config),
        summary: candidate.summary,
        metrics,
        objective_score,
    })
}

fn pareto_candidate_is_feasible(summary: &Map<String, Value>, config: &Value) -> bool {
    let field = config
        .get("feasible_field")
        .and_then(Value::as_str)
        .unwrap_or("material_status");
    match summary.get(field) {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value == "pass" || value == "feasible",
        Some(Value::Number(value)) => value.as_f64().is_some_and(|entry| entry != 0.0),
        _ => summary
            .get("objective_feasible")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    }
}

fn pareto_dominates(
    left: &EvaluatedParetoCandidate,
    right: &EvaluatedParetoCandidate,
    objectives: &[ParetoObjective],
) -> bool {
    if left.feasible && !right.feasible {
        return true;
    }
    if !left.feasible && right.feasible {
        return false;
    }
    let mut any_strictly_better = false;
    for objective in objectives {
        let Some(left_value) = left.metrics.get(&objective.field).and_then(Value::as_f64) else {
            return false;
        };
        let Some(right_value) = right.metrics.get(&objective.field).and_then(Value::as_f64) else {
            return false;
        };
        if !objective.no_worse(left_value, right_value) {
            return false;
        }
        any_strictly_better |= objective.strictly_better(left_value, right_value);
    }
    any_strictly_better
}

fn compare_frontier_entries(left: &Value, right: &Value) -> std::cmp::Ordering {
    right
        .get("objective_score")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NEG_INFINITY)
        .total_cmp(
            &left
                .get("objective_score")
                .and_then(Value::as_f64)
                .unwrap_or(f64::NEG_INFINITY),
        )
        .then_with(|| {
            left.get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .cmp(
                    right
                        .get("candidate_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
        })
}

fn lookup_summary_numeric(summary: &Map<String, Value>, field: &str) -> Option<f64> {
    summary
        .get(field)
        .and_then(Value::as_f64)
        .or_else(|| lookup_path(&Value::Object(summary.clone()), field))
}

fn lookup_path(payload: &Value, field: &str) -> Option<f64> {
    let mut current = payload;
    for part in field.split('.') {
        current = current.get(part)?;
    }
    current.as_f64()
}
