use crate::workflow_sweep_contract::{
    case_ids, finite, metric_field, number, object_or_null, optional_number, require_count,
    require_evaluated_row, require_evaluation_complete, text_option,
};
use serde_json::{Value, json};
use std::{cmp::Ordering, collections::HashSet};

pub(crate) const INFEASIBLE_PENALTY: f64 = 1.0e12;

pub(crate) struct ScoreIdentity {
    pub id: String,
    pub score: f64,
    pub feasible: bool,
}

impl ScoreIdentity {
    pub fn compare(&self, other: &Self) -> Ordering {
        other
            .feasible
            .cmp(&self.feasible)
            // Finite numeric comparison treats signed zeros as the same score.
            .then_with(|| {
                other
                    .score
                    .partial_cmp(&self.score)
                    .expect("validated finite scores")
            })
            .then_with(|| self.id.cmp(&other.id))
    }
}

pub(crate) struct Objective {
    pub field: String,
    pub goal: String,
    pub weight: f64,
    pub target: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

impl Objective {
    pub fn parse(raw: &Value, path: &str) -> Result<Self, String> {
        if !raw.is_object() {
            return Err(format!("{path} must be an object"));
        }
        let field = text_option(raw.get("field"), &format!("{path}.field"), "")?;
        metric_field(field)?;
        let goal = text_option(raw.get("goal"), &format!("{path}.goal"), "min")?;
        if !matches!(goal, "min" | "max" | "target") {
            return Err(format!("{path}: unsupported objective goal: {goal}"));
        }
        let weight = optional_number(raw.get("weight"), &format!("{path}.weight"))?.unwrap_or(1.0);
        if weight < 0.0 {
            return Err(format!("{path}.weight must be nonnegative"));
        }
        let target = optional_number(raw.get("target"), &format!("{path}.target"))?;
        let minimum = optional_number(raw.get("min_allowed"), &format!("{path}.min_allowed"))?;
        let maximum = optional_number(raw.get("max_allowed"), &format!("{path}.max_allowed"))?;
        if goal == "target" && target.is_none() {
            return Err(format!("{path}.target is required"));
        }
        if minimum.zip(maximum).is_some_and(|(min, max)| min > max) {
            return Err(format!("{path}.min_allowed exceeds max_allowed"));
        }
        Ok(Self {
            field: field.into(),
            goal: goal.into(),
            weight,
            target,
            minimum,
            maximum,
        })
    }

    pub fn allows(&self, value: f64) -> bool {
        self.minimum.is_none_or(|min| value >= min) && self.maximum.is_none_or(|max| value <= max)
    }

    pub fn score(&self, value: f64, path: &str) -> Result<f64, String> {
        let unweighted = match self.goal.as_str() {
            "min" => -value,
            "max" => value,
            "target" => -finite(value - self.target.expect("validated target"), path)?.abs(),
            _ => unreachable!("validated goal"),
        };
        finite(unweighted * self.weight, path)
    }

    fn term(&self, value: f64, path: &str) -> Result<Value, String> {
        let mut term = json!({"field":self.field, "goal":self.goal, "value":value,
            "weight":self.weight, "score":self.score(value,path)?, "feasible":self.allows(value)});
        for (key, value) in [
            ("target", self.target),
            ("min_allowed", self.minimum),
            ("max_allowed", self.maximum),
        ] {
            if let Some(value) = value {
                term[key] = json!(value);
            }
        }
        Ok(term)
    }
}

pub fn score_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "parameter sweep score config")?;
    require_evaluation_complete(&payload)?;
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or("parameter sweep score requires nonempty payload.rows")?;
    let ids = case_ids(rows)?;
    require_count(&payload, "row_count", rows.len())?;
    let objectives = config
        .get("objectives")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty())
        .ok_or("parameter sweep score requires nonempty config.objectives")?;
    let objectives = objectives
        .iter()
        .enumerate()
        .map(|(index, value)| Objective::parse(value, &format!("config.objectives[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = HashSet::new();
    for objective in &objectives {
        if !seen.insert(&objective.field) {
            return Err(format!("duplicate objective field {:?}", objective.field));
        }
    }

    let mut scored = rows
        .iter()
        .zip(ids)
        .map(|(row, id)| {
            require_evaluated_row(row, &format!("row {id}"))?;
            let mut total = 0.0;
            let mut feasible = true;
            let mut terms = Vec::with_capacity(objectives.len());
            for objective in &objectives {
                let path = format!("row {id} missing numeric field {}", objective.field);
                let value = number(row.get(&objective.field), &path)?;
                let term = objective.term(value, &format!("row {id}.{}", objective.field))?;
                total = finite(
                    total + term["score"].as_f64().expect("computed finite score"),
                    &format!("row {id} objective total"),
                )?;
                feasible &= objective.allows(value);
                terms.push(term);
            }
            if !feasible {
                total = finite(
                    total - INFEASIBLE_PENALTY,
                    &format!("row {id} objective penalty"),
                )?;
            }
            let mut result = row.clone();
            result["case_id"] = json!(id);
            result["objective_score"] = json!(total);
            result["objective_feasible"] = json!(feasible);
            result["objective_breakdown"] = json!(terms);
            Ok((
                ScoreIdentity {
                    id,
                    score: total,
                    feasible,
                },
                result,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    scored.sort_by(|left, right| left.0.compare(&right.0));
    let feasible = scored.iter().filter(|entry| entry.0.feasible).count();
    let best = scored
        .first()
        .filter(|entry| entry.0.feasible)
        .map(|entry| entry.1.clone());
    Ok(
        json!({"best":best, "scored_rows":scored.into_iter().map(|entry|entry.1).collect::<Vec<_>>(),
        "scored_count":rows.len(), "feasible_count":feasible, "selection_ready":feasible>0}),
    )
}
