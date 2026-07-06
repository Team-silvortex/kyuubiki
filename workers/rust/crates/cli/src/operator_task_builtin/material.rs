mod scoring;

use serde_json::{Map, Value};

const MATERIAL_MARGINS_OPERATOR: &str = "transform.evaluate_material_margins";
const MATERIAL_RANK_OPERATOR: &str = "transform.rank_material_candidates";
const MATERIAL_SCORE_OPERATOR: &str = "transform.score_material_candidates";
const MATERIAL_THERMAL_SHOCK_OPERATOR: &str = "transform.evaluate_material_thermal_shock";

#[derive(Debug, Clone, PartialEq)]
struct ThermalShockConfig {
    constraint_factor: f64,
    safety_factor_target: f64,
    temperature_delta_field: String,
    thermal_expansion_field: String,
    youngs_modulus_field: String,
    poisson_ratio_field: String,
    strength_field: String,
    fallback_strength_field: String,
    fracture_toughness_field: String,
    flaw_size_field: String,
}

pub(crate) fn is_material_builtin_operator(operator_id: &str) -> bool {
    matches!(
        operator_id,
        MATERIAL_MARGINS_OPERATOR
            | MATERIAL_RANK_OPERATOR
            | MATERIAL_SCORE_OPERATOR
            | MATERIAL_THERMAL_SHOCK_OPERATOR
    )
}

pub(crate) fn run_material_builtin_task(
    operator_id: &str,
    task_ir: &Value,
) -> Result<Value, String> {
    match operator_id {
        MATERIAL_MARGINS_OPERATOR => evaluate_material_margins(
            task_ir
                .get("input_artifact")
                .ok_or_else(|| "missing input_artifact".to_string())?,
            task_ir.get("config").unwrap_or(&Value::Object(Map::new())),
        ),
        MATERIAL_RANK_OPERATOR => rank_material_candidates(
            task_ir
                .get("input_artifact")
                .ok_or_else(|| "missing input_artifact".to_string())?,
            task_ir.get("config").unwrap_or(&Value::Object(Map::new())),
        ),
        MATERIAL_SCORE_OPERATOR => scoring::score_material_candidates(
            task_ir
                .get("input_artifact")
                .ok_or_else(|| "missing input_artifact".to_string())?,
            task_ir.get("config").unwrap_or(&Value::Object(Map::new())),
        ),
        MATERIAL_THERMAL_SHOCK_OPERATOR => evaluate_material_thermal_shock(
            task_ir
                .get("input_artifact")
                .ok_or_else(|| "missing input_artifact".to_string())?,
            task_ir.get("config").unwrap_or(&Value::Object(Map::new())),
        ),
        _ => Err(format!(
            "unsupported agent-native builtin operator: {operator_id}"
        )),
    }
}

fn evaluate_material_margins(payload: &Value, config: &Value) -> Result<Value, String> {
    let limits = config
        .get("limits")
        .and_then(Value::as_object)
        .filter(|limits| !limits.is_empty())
        .ok_or_else(|| "missing_material_limits".to_string())?;

    let mut evaluations = Vec::new();
    for (field, spec) in limits {
        let Some((limit, direction)) = parse_limit(spec) else {
            continue;
        };
        let Some(actual) = lookup_number(payload, field) else {
            continue;
        };
        if limit <= 0.0 {
            continue;
        }
        let failure_index = match direction.as_str() {
            "min" => limit / actual,
            "abs" => actual.abs() / limit,
            _ => actual / limit,
        };
        evaluations.push((field.clone(), actual, limit, failure_index));
    }

    if evaluations.is_empty() {
        return Err("missing_matching_material_limits".to_string());
    }

    let prefix = string_config(config, "output_prefix", "material");
    let critical = evaluations
        .iter()
        .max_by(|left, right| {
            left.3
                .partial_cmp(&right.3)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("evaluations should not be empty");
    let violation_count = evaluations
        .iter()
        .filter(|(_, _, _, failure_index)| *failure_index > 1.0)
        .count();

    let mut output = Map::new();
    for (field, actual, limit, failure_index) in &evaluations {
        output.insert(format!("{prefix}_{field}_actual"), Value::from(*actual));
        output.insert(format!("{prefix}_{field}_limit"), Value::from(*limit));
        output.insert(
            format!("{prefix}_{field}_failure_index"),
            Value::from(*failure_index),
        );
        output.insert(
            format!("{prefix}_{field}_safety_factor"),
            Value::from(1.0 / *failure_index),
        );
    }
    output.insert(
        format!("{prefix}_constraint_count"),
        Value::from(evaluations.len()),
    );
    output.insert(
        format!("{prefix}_violation_count"),
        Value::from(violation_count),
    );
    output.insert(format!("{prefix}_failure_index"), Value::from(critical.3));
    output.insert(
        format!("{prefix}_safety_factor"),
        Value::from(1.0 / critical.3),
    );
    output.insert(
        format!("{prefix}_critical_metric"),
        Value::from(critical.0.clone()),
    );
    output.insert(format!("{prefix}_critical_actual"), Value::from(critical.1));
    output.insert(format!("{prefix}_critical_limit"), Value::from(critical.2));
    output.insert(
        format!("{prefix}_status"),
        Value::from(if violation_count == 0 { "pass" } else { "fail" }),
    );

    Ok(Value::Object(output))
}

fn rank_material_candidates(payload: &Value, config: &Value) -> Result<Value, String> {
    let prefix = string_config(config, "margin_prefix", "material");
    let candidates = material_candidate_entries(payload);
    if candidates.is_empty() {
        return Err("missing_material_candidates".to_string());
    }

    let mut rankings = candidates
        .iter()
        .map(|(candidate_id, summary)| rank_candidate(candidate_id, summary, &prefix))
        .collect::<Vec<_>>();

    rankings.sort_by(|left, right| {
        left.feasible
            .cmp(&right.feasible)
            .reverse()
            .then_with(|| {
                right
                    .safety_factor
                    .partial_cmp(&left.safety_factor)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                left.failure_index
                    .partial_cmp(&right.failure_index)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let best = rankings.first().expect("rankings should not be empty");
    let feasible_count = rankings.iter().filter(|ranking| ranking.feasible).count();

    let mut output = serde_json::json!({
        "material_candidate_count": rankings.len(),
        "material_feasible_count": feasible_count,
        "material_best_candidate_id": best.candidate_id,
        "material_best_candidate_feasible": best.feasible,
        "material_best_safety_factor": best.safety_factor,
        "material_best_failure_index": best.failure_index,
        "material_failure_reasons": failure_reasons(&rankings),
        "material_rankings": rankings.iter().map(ranking_summary).collect::<Vec<_>>()
    });

    if config.get("include_best_summary").and_then(Value::as_bool) != Some(false) {
        output["material_best_summary"] = best.summary.clone();
    }

    Ok(output)
}

fn evaluate_material_thermal_shock(payload: &Value, config: &Value) -> Result<Value, String> {
    let config = parse_config(config)?;
    let candidates = candidate_entries(payload);
    if candidates.is_empty() {
        return Err("missing_material_thermal_shock_candidates".to_string());
    }

    let mut assessments = candidates
        .iter()
        .map(|(candidate_id, summary)| assess_candidate(candidate_id, summary, &config))
        .collect::<Result<Vec<_>, _>>()?;

    assessments.sort_by(|left, right| {
        let right_safety = number_field(right, "thermal_shock_safety_factor").unwrap_or(0.0);
        let left_safety = number_field(left, "thermal_shock_safety_factor").unwrap_or(0.0);
        right_safety
            .partial_cmp(&left_safety)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                string_field(left, "candidate_id")
                    .unwrap_or_default()
                    .cmp(&string_field(right, "candidate_id").unwrap_or_default())
            })
    });

    let best = assessments.first();
    let pass_count = assessments
        .iter()
        .filter(|assessment| string_field(assessment, "thermal_shock_status") == Some("pass"))
        .count();

    let mut output = serde_json::json!({
        "material_thermal_shock_candidate_count": assessments.len(),
        "material_thermal_shock_pass_count": pass_count,
        "material_thermal_shock_best_candidate_id": best
            .and_then(|assessment| string_field(assessment, "candidate_id"))
            .unwrap_or_default(),
        "material_thermal_shock_best_safety_factor": best
            .and_then(|assessment| number_field(assessment, "thermal_shock_safety_factor"))
            .unwrap_or(0.0),
        "material_thermal_shock_assessments": assessments
    });

    if output["material_thermal_shock_assessments"]
        .as_array()
        .map(|items| items.len() == 1)
        .unwrap_or(false)
    {
        let single = output["material_thermal_shock_assessments"][0].clone();
        output["material_thermal_shock_status"] = single["thermal_shock_status"].clone();
        output["material_thermal_shock_risk_index"] = single["thermal_shock_risk_index"].clone();
        output["material_thermal_shock_safety_factor"] =
            single["thermal_shock_safety_factor"].clone();
        output["material_thermal_shock_estimated_stress"] =
            single["thermal_shock_estimated_stress"].clone();
    }

    Ok(output)
}

#[derive(Debug, Clone, PartialEq)]
struct MaterialRanking {
    candidate_id: String,
    feasible: bool,
    safety_factor: f64,
    failure_index: f64,
    critical_metric: String,
    summary: Value,
}

fn rank_candidate(candidate_id: &str, summary: &Value, prefix: &str) -> MaterialRanking {
    let failure_index =
        lookup_number(summary, &format!("{prefix}_failure_index")).unwrap_or(f64::INFINITY);
    let safety_factor = lookup_number(summary, &format!("{prefix}_safety_factor")).unwrap_or(0.0);
    let violation_count =
        lookup_number(summary, &format!("{prefix}_violation_count")).unwrap_or(1.0);
    let status = summary
        .get(format!("{prefix}_status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let critical_metric = summary
        .get(format!("{prefix}_critical_metric"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    MaterialRanking {
        candidate_id: candidate_id.to_string(),
        feasible: status == "pass" && violation_count == 0.0 && failure_index <= 1.0,
        safety_factor,
        failure_index,
        critical_metric,
        summary: summary.clone(),
    }
}

fn ranking_summary(ranking: &MaterialRanking) -> Value {
    serde_json::json!({
        "candidate_id": ranking.candidate_id,
        "feasible": ranking.feasible,
        "safety_factor": ranking.safety_factor,
        "failure_index": ranking.failure_index,
        "critical_metric": ranking.critical_metric
    })
}

fn failure_reasons(rankings: &[MaterialRanking]) -> Value {
    let mut reasons = Map::new();
    for ranking in rankings.iter().filter(|ranking| !ranking.feasible) {
        let count = reasons
            .get(&ranking.critical_metric)
            .and_then(Value::as_i64)
            .unwrap_or(0)
            + 1;
        reasons.insert(ranking.critical_metric.clone(), Value::from(count));
    }
    Value::Object(reasons)
}

fn parse_config(config: &Value) -> Result<ThermalShockConfig, String> {
    let constraint_factor = numeric_config(config, "constraint_factor", 1.0);
    let safety_factor_target = numeric_config(config, "safety_factor_target", 1.0);

    if constraint_factor <= 0.0 || safety_factor_target <= 0.0 {
        return Err("invalid_material_thermal_shock_config".to_string());
    }

    Ok(ThermalShockConfig {
        constraint_factor,
        safety_factor_target,
        temperature_delta_field: string_config(
            config,
            "temperature_delta_field",
            "temperature_delta",
        ),
        thermal_expansion_field: string_config(
            config,
            "thermal_expansion_field",
            "thermal_expansion",
        ),
        youngs_modulus_field: string_config(config, "youngs_modulus_field", "youngs_modulus"),
        poisson_ratio_field: string_config(config, "poisson_ratio_field", "poisson_ratio"),
        strength_field: string_config(config, "strength_field", "yield_strength"),
        fallback_strength_field: string_config(
            config,
            "fallback_strength_field",
            "tensile_strength",
        ),
        fracture_toughness_field: string_config(
            config,
            "fracture_toughness_field",
            "fracture_toughness",
        ),
        flaw_size_field: string_config(config, "flaw_size_field", "flaw_size"),
    })
}

fn assess_candidate(
    candidate_id: &str,
    summary: &Value,
    config: &ThermalShockConfig,
) -> Result<Value, String> {
    let delta_t = temperature_delta(summary, config)
        .filter(|value| *value > 0.0)
        .ok_or_else(|| "missing_material_thermal_shock_property".to_string())?;
    let alpha = lookup_number(summary, &config.thermal_expansion_field)
        .filter(|value| *value > 0.0)
        .ok_or_else(|| "missing_material_thermal_shock_property".to_string())?;
    let youngs_modulus = lookup_number(summary, &config.youngs_modulus_field)
        .filter(|value| *value > 0.0)
        .ok_or_else(|| "missing_material_thermal_shock_property".to_string())?;
    let strength = strength(summary, config)
        .filter(|value| *value > 0.0)
        .ok_or_else(|| "missing_material_thermal_shock_property".to_string())?;

    let poisson_ratio = lookup_number(summary, &config.poisson_ratio_field).unwrap_or(0.0);
    let thermal_strain = alpha * delta_t;
    let thermal_stress = thermal_stress(youngs_modulus, thermal_strain, poisson_ratio, config);
    let stress_index = thermal_stress / strength;
    let fracture_index = fracture_index(summary, thermal_stress, config);
    let risk_index = stress_index.max(fracture_index);
    let safety_factor = config.safety_factor_target / risk_index;

    Ok(serde_json::json!({
        "candidate_id": candidate_id,
        "thermal_shock_status": if safety_factor >= 1.0 { "pass" } else { "fail" },
        "thermal_shock_risk_index": risk_index,
        "thermal_shock_safety_factor": safety_factor,
        "thermal_shock_temperature_delta": delta_t,
        "thermal_shock_thermal_strain": thermal_strain,
        "thermal_shock_estimated_stress": thermal_stress,
        "thermal_shock_strength_limit": strength,
        "thermal_shock_stress_index": stress_index,
        "thermal_shock_fracture_index": fracture_index,
        "summary": summary
    }))
}

fn candidate_entries(payload: &Value) -> Vec<(String, Value)> {
    payload
        .get("candidates")
        .and_then(Value::as_object)
        .map(|candidates| {
            candidates
                .iter()
                .filter(|(_, value)| value.is_object())
                .map(|(id, value)| (id.clone(), value.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            if payload.is_object() {
                vec![("summary".to_string(), payload.clone())]
            } else {
                Vec::new()
            }
        })
}

pub(super) fn material_candidate_entries(payload: &Value) -> Vec<(String, Value)> {
    let Some(entries) = payload
        .get("candidates")
        .and_then(Value::as_object)
        .or_else(|| payload.as_object())
    else {
        return Vec::new();
    };

    entries
        .iter()
        .filter(|(_, value)| value.is_object())
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect()
}

fn parse_limit(spec: &Value) -> Option<(f64, String)> {
    if let Some(limit) = spec.as_f64() {
        return Some((limit, "max".to_string()));
    }
    let object = spec.as_object()?;
    let limit = object
        .get("limit")
        .or_else(|| object.get("max"))
        .or_else(|| object.get("min"))
        .and_then(Value::as_f64)?;
    let direction = object
        .get("direction")
        .or_else(|| object.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or(if object.contains_key("min") {
            "min"
        } else {
            "max"
        });
    Some((limit, direction.to_string()))
}

fn temperature_delta(summary: &Value, config: &ThermalShockConfig) -> Option<f64> {
    lookup_number(summary, &config.temperature_delta_field).or_else(|| {
        let max_temp = lookup_number(summary, "max_temperature")?;
        let min_temp = lookup_number(summary, "min_temperature")?;
        Some((max_temp - min_temp).abs())
    })
}

fn thermal_stress(
    youngs_modulus: f64,
    thermal_strain: f64,
    poisson_ratio: f64,
    config: &ThermalShockConfig,
) -> f64 {
    let denominator = if poisson_ratio > -1.0 && poisson_ratio < 0.49 {
        1.0 - poisson_ratio
    } else {
        1.0
    };
    youngs_modulus * thermal_strain * config.constraint_factor / denominator
}

fn strength(summary: &Value, config: &ThermalShockConfig) -> Option<f64> {
    lookup_number(summary, &config.strength_field)
        .or_else(|| lookup_number(summary, &config.fallback_strength_field))
}

fn fracture_index(summary: &Value, thermal_stress: f64, config: &ThermalShockConfig) -> f64 {
    let Some(toughness) = lookup_number(summary, &config.fracture_toughness_field) else {
        return 0.0;
    };
    let Some(flaw_size) = lookup_number(summary, &config.flaw_size_field) else {
        return 0.0;
    };
    if toughness > 0.0 && flaw_size > 0.0 {
        thermal_stress * (std::f64::consts::PI * flaw_size).sqrt() / toughness
    } else {
        0.0
    }
}

pub(super) fn numeric_config(config: &Value, field: &str, default: f64) -> f64 {
    config.get(field).and_then(Value::as_f64).unwrap_or(default)
}

fn string_config(config: &Value, field: &str, default: &str) -> String {
    config
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

pub(super) fn lookup_number(payload: &Value, field: &str) -> Option<f64> {
    lookup_path(payload, field)
        .or_else(|| {
            payload
                .get("summary")
                .and_then(|summary| lookup_path(summary, field))
        })
        .and_then(Value::as_f64)
}

fn lookup_path<'a>(payload: &'a Value, field: &str) -> Option<&'a Value> {
    let mut current = payload;
    for segment in field.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

pub(super) fn number_field(value: &Value, field: &str) -> Option<f64> {
    value.get(field).and_then(Value::as_f64)
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}
