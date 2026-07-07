use serde_json::{Value, json};

pub fn build_composite_materialized_candidate_report(
    result_payloads: &[Value],
) -> Result<Value, String> {
    if result_payloads.is_empty() {
        return Err("materialized composite report requires result payloads".to_string());
    }
    let mut rows = result_payloads
        .iter()
        .map(materialized_candidate_row)
        .collect::<Result<Vec<_>, _>>()?;
    apply_materialized_scores(&mut rows);
    rows.sort_by(|left, right| {
        score(right)
            .partial_cmp(&score(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row["rank"] = json!(index + 1);
    }
    let warnings = materialized_report_warnings(&rows);
    Ok(json!({
        "schema_version": "kyuubiki.composite-materialized-candidate-report/v1",
        "study": "material.composite_thermo_electric_panel.v1",
        "objective": "rank materialized mixed-material panel reruns",
        "coupling": "sequential_electrostatic_to_heat_to_thermal_stress",
        "candidate_count": rows.len(),
        "winner_candidate_id": rows
            .first()
            .and_then(|row| row.get("candidate_id"))
            .cloned()
            .unwrap_or(Value::Null),
        "candidates": rows,
        "warnings": warnings,
        "reliability": {
            "schema_version": "kyuubiki.material-reliability-envelope/v1",
            "posture": "prototype_materialized_rerun_only",
            "quality_gates": materialized_quality_gates(result_payloads)
        }
    }))
}

fn materialized_candidate_row(payload: &Value) -> Result<Value, String> {
    let result = descend_result_payload(payload);
    let research = result
        .get("research")
        .ok_or_else(|| "materialized result is missing research metadata".to_string())?;
    let candidate_id = required_str(research, "candidate_id")?;
    let parameters = research
        .get("screening_parameters")
        .ok_or_else(|| format!("{candidate_id} is missing screening_parameters"))?;
    let max_electric_field_v_m = read_path_f64(result, &["electrostatic", "max_electric_field"]);
    let max_temperature_c = read_path_f64(result, &["heat", "max_temperature"]);
    let max_thermal_stress_pa = read_path_f64(result, &["thermal", "max_stress"]);
    let breakdown_safety_factor = max_electric_field_v_m
        .filter(|field| *field > 0.0)
        .and_then(|field| {
            read_path_f64(parameters, &["dielectric_breakdown_field_v_m"]).map(|v| v / field)
        });
    Ok(json!({
        "candidate_id": candidate_id,
        "candidate_label": research
            .get("candidate_label")
            .and_then(Value::as_str)
            .unwrap_or(candidate_id),
        "source_candidate_id": research.get("source_candidate_id").cloned().unwrap_or(Value::Null),
        "source_draft_id": research.get("source_draft_id").cloned().unwrap_or(Value::Null),
        "strategy": research.get("strategy").cloned().unwrap_or(Value::Null),
        "rank": 0,
        "score": 0.0,
        "max_electric_field_v_m": max_electric_field_v_m,
        "max_temperature_c": max_temperature_c,
        "max_thermal_stress_pa": max_thermal_stress_pa,
        "breakdown_safety_factor": breakdown_safety_factor,
        "interface_risk_score": read_path_f64(parameters, &["interface_risk_score"]),
        "areal_mass_kg_m2": read_path_f64(parameters, &["areal_mass_kg_m2"]),
        "materials": research.get("materials").cloned().unwrap_or(Value::Null),
        "missing_metrics": missing_metrics(&[
            ("max_electric_field_v_m", max_electric_field_v_m),
            ("max_temperature_c", max_temperature_c),
            ("max_thermal_stress_pa", max_thermal_stress_pa),
            ("breakdown_safety_factor", breakdown_safety_factor),
            ("interface_risk_score", read_path_f64(parameters, &["interface_risk_score"])),
            ("areal_mass_kg_m2", read_path_f64(parameters, &["areal_mass_kg_m2"])),
        ])
    }))
}

fn apply_materialized_scores(rows: &mut [Value]) {
    let fields = values(rows, "max_electric_field_v_m");
    let temps = values(rows, "max_temperature_c");
    let stresses = values(rows, "max_thermal_stress_pa");
    let margins = values(rows, "breakdown_safety_factor");
    let risks = values(rows, "interface_risk_score");
    let masses = values(rows, "areal_mass_kg_m2");
    for row in rows {
        let score = 0.23 * normalize_min(row["max_electric_field_v_m"].as_f64(), &fields)
            + 0.23 * normalize_min(row["max_temperature_c"].as_f64(), &temps)
            + 0.22 * normalize_min(row["max_thermal_stress_pa"].as_f64(), &stresses)
            + 0.15 * normalize_max(row["breakdown_safety_factor"].as_f64(), &margins)
            + 0.12 * normalize_min(row["interface_risk_score"].as_f64(), &risks)
            + 0.05 * normalize_min(row["areal_mass_kg_m2"].as_f64(), &masses);
        row["score"] = json!(score);
    }
}

fn score(row: &Value) -> f64 {
    row["score"].as_f64().unwrap_or(0.0)
}

fn materialized_report_warnings(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .flat_map(|row| {
            row["missing_metrics"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|metric| format!("{} is missing {metric}", required_candidate_id(row)))
        })
        .chain(rows.iter().filter_map(|row| {
            let risk = row["interface_risk_score"].as_f64()?;
            (risk > 0.70).then(|| {
                format!(
                    "{} exceeds prototype interface risk threshold: {:.3}",
                    required_candidate_id(row),
                    risk
                )
            })
        }))
        .collect()
}

fn materialized_quality_gates(result_payloads: &[Value]) -> Vec<Value> {
    let rows = result_payloads
        .iter()
        .filter_map(|payload| materialized_candidate_row(payload).ok())
        .collect::<Vec<_>>();
    vec![
        gate(
            "gate.breakdown_margin.prototype",
            "breakdown_safety_factor",
            ">=",
            1.5,
            min_value(&rows, "breakdown_safety_factor"),
        ),
        gate(
            "gate.max_temperature.prototype",
            "max_temperature_c",
            "<=",
            140.0,
            max_value(&rows, "max_temperature_c"),
        ),
        gate(
            "gate.interface_risk.prototype",
            "interface_risk_score",
            "<=",
            0.70,
            max_value(&rows, "interface_risk_score"),
        ),
        gate(
            "gate.result_completeness",
            "complete_candidate_count",
            ">=",
            rows.len() as f64,
            Some(
                rows.iter()
                    .filter(|row| row["missing_metrics"].as_array().is_some_and(Vec::is_empty))
                    .count() as f64,
            ),
        ),
    ]
}

fn gate(id: &str, metric: &str, op: &str, threshold: f64, observed: Option<f64>) -> Value {
    let pass = observed.is_some_and(|value| {
        if op == ">=" {
            value >= threshold
        } else {
            value <= threshold
        }
    });
    json!({ "id": id, "metric": metric, "operator": op, "threshold": threshold, "observed": observed, "status": if pass { "pass" } else { "violate" } })
}

fn values(rows: &[Value], key: &str) -> Vec<f64> {
    rows.iter().filter_map(|row| row[key].as_f64()).collect()
}

fn min_value(rows: &[Value], key: &str) -> Option<f64> {
    values(rows, key).into_iter().reduce(f64::min)
}

fn max_value(rows: &[Value], key: &str) -> Option<f64> {
    values(rows, key).into_iter().reduce(f64::max)
}

fn normalize_min(value: Option<f64>, values: &[f64]) -> f64 {
    value.map(|v| normalize(v, values, true)).unwrap_or(0.0)
}

fn normalize_max(value: Option<f64>, values: &[f64]) -> f64 {
    value.map(|v| normalize(v, values, false)).unwrap_or(0.0)
}

fn normalize(value: f64, values: &[f64], minimize: bool) -> f64 {
    let (min, max) = values.iter().fold((value, value), |(min, max), next| {
        (min.min(*next), max.max(*next))
    });
    if (max - min).abs() < f64::EPSILON {
        1.0
    } else if minimize {
        (max - value) / (max - min)
    } else {
        (value - min) / (max - min)
    }
}

fn missing_metrics(metrics: &[(&str, Option<f64>)]) -> Vec<String> {
    metrics
        .iter()
        .filter(|(_, value)| value.is_none())
        .map(|(metric, _)| (*metric).to_string())
        .collect()
}

fn required_candidate_id(row: &Value) -> &str {
    row["candidate_id"].as_str().unwrap_or("unknown")
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("materialized result research is missing {key}"))
}

fn descend_result_payload(payload: &Value) -> &Value {
    let mut current = payload;
    for _ in 0..4 {
        let Some(next) = current.get("result") else {
            break;
        };
        current = next;
    }
    current
}

fn read_path_f64(payload: &Value, path: &[&str]) -> Option<f64> {
    path.iter()
        .try_fold(payload, |current, key| current.get(*key))?
        .as_f64()
}
