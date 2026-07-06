use serde_json::{Map, Value};

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

pub(crate) fn is_agent_native_builtin_operator(operator_id: &str) -> bool {
    operator_id == MATERIAL_THERMAL_SHOCK_OPERATOR
}

pub(crate) fn run_agent_native_builtin_task(
    operator_id: &str,
    task_ir: &Value,
) -> Result<Value, String> {
    match operator_id {
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

fn numeric_config(config: &Value, field: &str, default: f64) -> f64 {
    config.get(field).and_then(Value::as_f64).unwrap_or(default)
}

fn string_config(config: &Value, field: &str, default: &str) -> String {
    config
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn lookup_number(payload: &Value, field: &str) -> Option<f64> {
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

fn number_field(value: &Value, field: &str) -> Option<f64> {
    value.get(field).and_then(Value::as_f64)
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}
