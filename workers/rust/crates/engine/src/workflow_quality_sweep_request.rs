use serde_json::{Map, Value};

pub fn build_quality_parameter_sweep_plan(payload: Value, config: Value) -> Result<Value, String> {
    let request = payload
        .get("request_payload")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if payload.get("action").and_then(Value::as_str) == Some("stop") {
        return Ok(serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": false,
            "sweep_action": "stop",
            "case_count_estimate": 0,
            "axes": [],
            "base": config.get("base").cloned().unwrap_or_else(|| serde_json::json!({})),
            "plan_summary": "Quality exploration stopped; no parameter sweep was planned.",
        }));
    }

    let search_space = request
        .get("search_space")
        .or_else(|| config.get("search_space"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.build_quality_parameter_sweep_plan requires search_space".to_string()
        })?;
    let base = config
        .get("base")
        .or_else(|| payload.get("base"))
        .or_else(|| request.get("base"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let samples = config_number(&config, "samples_per_axis", 3.0).round() as usize;
    let focus_field = request
        .get("optimization_hint")
        .or_else(|| payload.get("selected_iteration_hint"))
        .and_then(|hint| hint.get("focus_field"))
        .and_then(Value::as_str);
    let optimization_hint = request
        .get("optimization_hint")
        .or_else(|| payload.get("selected_iteration_hint"))
        .cloned()
        .unwrap_or(Value::Null);
    let focus_domain = optimization_hint
        .get("focus_domain")
        .and_then(Value::as_str);
    let coupled_readiness = request
        .get("coupled_readiness")
        .or_else(|| payload.get("selected_coupled_readiness"))
        .or_else(|| payload.get("coupled_readiness"))
        .cloned()
        .unwrap_or(Value::Null);
    let repair_strategy = repair_strategy_from_hint(&optimization_hint);
    let max_axes = config
        .get("max_axes")
        .and_then(Value::as_u64)
        .map(|value| value as usize);
    let max_cases = config_number(
        &config,
        "max_cases",
        request
            .get("max_candidates")
            .and_then(Value::as_f64)
            .unwrap_or(64.0),
    );
    let usable_axis_count = usable_search_space_axis_count(search_space, samples);
    let axes =
        prioritized_search_space_axes(search_space, samples, focus_field, focus_domain, max_axes);
    if axes.is_empty() {
        return Err(
            "transform.build_quality_parameter_sweep_plan requires usable search_space axes"
                .to_string(),
        );
    }
    let case_count_estimate = axes
        .iter()
        .map(|axis| {
            axis.get("values")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .product::<usize>();

    let budget_summary =
        sweep_budget_summary(usable_axis_count, &axes, case_count_estimate, max_cases);

    Ok(serde_json::json!({
        "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
        "sweep_enabled": true,
        "sweep_action": payload.get("action").and_then(Value::as_str).unwrap_or("continue"),
        "source_candidate_id": payload.get("selected_candidate_id").cloned().unwrap_or(Value::Null),
        "seed_metadata": request
            .get("seed_metadata")
            .or_else(|| payload.get("selected_candidate_metadata"))
            .cloned()
            .unwrap_or(Value::Null),
        "target_score": payload.get("target_score").cloned().unwrap_or(Value::Null),
        "optimization_hint": optimization_hint,
        "coupled_readiness": coupled_readiness.clone(),
        "repair_strategy": repair_strategy,
        "repair_focus": {
            "field": focus_field.map(Value::from).unwrap_or(Value::Null),
            "source": request
                .get("optimization_hint")
                .or_else(|| payload.get("selected_iteration_hint"))
                .and_then(|hint| hint.get("focus_source"))
                .cloned()
                .unwrap_or(Value::Null),
            "domain": request
                .get("optimization_hint")
                .or_else(|| payload.get("selected_iteration_hint"))
                .and_then(|hint| hint.get("focus_domain"))
                .cloned()
                .unwrap_or(Value::Null),
            "readiness_state": coupled_readiness
                .get("coupled_readiness_state")
                .cloned()
                .unwrap_or(Value::Null),
            "readiness_recommendation": coupled_readiness
                .get("coupled_readiness_recommendation")
                .cloned()
                .unwrap_or(Value::Null),
        },
        "focused_axis_path": axes.first()
            .and_then(|axis| axis.get("path"))
            .cloned()
            .unwrap_or(Value::Null),
        "id_prefix": config.get("id_prefix").and_then(Value::as_str).unwrap_or("quality_round"),
        "max_cases": max_cases,
        "case_count_estimate": case_count_estimate,
        "sweep_budget": budget_summary,
        "axes": axes,
        "base": base,
        "plan_summary": format!(
            "Quality parameter sweep planned with {} axis/axes and {case_count_estimate} estimated cases.",
            axes.len()
        ),
    }))
}

fn repair_strategy_from_hint(optimization_hint: &Value) -> &'static str {
    match optimization_hint.get("action").and_then(Value::as_str) {
        Some("fix_validation_failure") => "rerun_validation_focused_sweep",
        Some("fix_coupled_readiness") => "repair_coupled_readiness_sweep",
        Some("review_coupled_readiness") => "review_coupled_readiness_sweep",
        Some("fix_blocking_term") => "repair_blocking_term_sweep",
        Some("reduce_dominant_term") => "reduce_dominant_term_sweep",
        _ => "general_quality_sweep",
    }
}

fn usable_search_space_axis_count(search_space: &Map<String, Value>, samples: usize) -> usize {
    search_space
        .values()
        .filter(|spec| !axis_values(spec, samples).is_empty())
        .count()
}

fn prioritized_search_space_axes(
    search_space: &Map<String, Value>,
    samples: usize,
    focus_field: Option<&str>,
    focus_domain: Option<&str>,
    max_axes: Option<usize>,
) -> Vec<Value> {
    let mut axes = search_space
        .iter()
        .filter_map(|(path, spec)| {
            let values = axis_values(spec, samples);
            if values.is_empty() {
                None
            } else {
                Some(serde_json::json!({
                    "label": path,
                    "path": path,
                    "values": values,
                }))
            }
        })
        .collect::<Vec<_>>();
    axes.sort_by(|left, right| {
        let left_path = left.get("path").and_then(Value::as_str).unwrap_or("");
        let right_path = right.get("path").and_then(Value::as_str).unwrap_or("");
        focus_rank(left_path, focus_field, focus_domain)
            .cmp(&focus_rank(right_path, focus_field, focus_domain))
            .then_with(|| left_path.cmp(right_path))
    });
    if let Some(max_axes) = max_axes {
        axes.truncate(max_axes.max(1));
    }
    axes
}

fn sweep_budget_summary(
    usable_axis_count: usize,
    axes: &[Value],
    case_count_estimate: usize,
    max_cases: f64,
) -> Value {
    let planned_axis_count = axes.len();
    let case_budget_exceeded = max_cases.is_finite() && case_count_estimate as f64 > max_cases;
    let axis_budget_truncated = planned_axis_count < usable_axis_count;
    let planned_axis_paths = axes
        .iter()
        .filter_map(|axis| axis.get("path").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let recommended_axis_count = recommended_axis_count_for_budget(axes, max_cases);
    let status = if case_budget_exceeded {
        "case_budget_exceeded"
    } else if axis_budget_truncated {
        "axis_budget_truncated"
    } else {
        "ok"
    };
    let recommendation = if case_budget_exceeded && recommended_axis_count < planned_axis_count {
        "reduce_axis_count"
    } else if case_budget_exceeded {
        "reduce_samples_per_axis"
    } else if axis_budget_truncated {
        "schedule_followup_axis_batch"
    } else {
        "run_planned_sweep"
    };

    serde_json::json!({
        "status": status,
        "recommendation": recommendation,
        "usable_axis_count": usable_axis_count,
        "planned_axis_count": planned_axis_count,
        "planned_axis_paths": planned_axis_paths,
        "recommended_axis_count": recommended_axis_count,
        "axis_budget_truncated": axis_budget_truncated,
        "case_count_estimate": case_count_estimate,
        "max_cases": max_cases,
        "case_budget_exceeded": case_budget_exceeded,
    })
}

fn recommended_axis_count_for_budget(axes: &[Value], max_cases: f64) -> usize {
    if !max_cases.is_finite() {
        return axes.len();
    }
    let limit = max_cases.floor().max(1.0) as usize;
    let mut product = 1usize;
    let mut included = 0usize;
    for axis in axes {
        let value_count = axis
            .get("values")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if value_count == 0 {
            continue;
        }
        let Some(next_product) = product.checked_mul(value_count) else {
            break;
        };
        if next_product > limit {
            break;
        }
        product = next_product;
        included += 1;
    }
    included.max(1).min(axes.len())
}

fn focus_rank(path: &str, focus_field: Option<&str>, focus_domain: Option<&str>) -> u8 {
    if focus_field_matches(path, focus_field) {
        0
    } else if focus_domain_matches(path, focus_domain) {
        1
    } else {
        2
    }
}

fn focus_field_matches(path: &str, focus_field: Option<&str>) -> bool {
    let Some(focus_field) = focus_field.map(str::trim).filter(|field| !field.is_empty()) else {
        return false;
    };
    path == focus_field || path.ends_with(&format!(".{focus_field}"))
}

fn focus_domain_matches(path: &str, focus_domain: Option<&str>) -> bool {
    let Some(focus_domain) = focus_domain
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
    else {
        return false;
    };
    let path = path.to_ascii_lowercase();
    let focus_domain = focus_domain.to_ascii_lowercase();
    domain_aliases(&focus_domain)
        .iter()
        .any(|alias| path.contains(alias))
}

fn domain_aliases(domain: &str) -> &'static [&'static str] {
    match domain {
        "structural" => &[
            "structural",
            "stress",
            "strain",
            "stiffness",
            "displacement",
        ],
        "thermal" | "thermo" | "heat" => &["thermal", "thermo", "heat", "temperature"],
        "electrostatic" | "electric" => &[
            "electrostatic",
            "electric",
            "voltage",
            "permittivity",
            "charge",
        ],
        "magnetostatic" | "magnetic" => &["magnetostatic", "magnetic", "magnet", "permeability"],
        "cfd" | "fluid" => &["cfd", "fluid", "velocity", "pressure", "viscosity"],
        "transport" => &["transport", "diffusion", "concentration"],
        "acoustic" => &["acoustic", "sound"],
        "modal" => &["modal", "frequency", "mode"],
        "dynamic" => &["dynamic", "damping", "transient"],
        _ => &[],
    }
}

fn axis_values(spec: &Value, samples: usize) -> Vec<Value> {
    if let Some(values) = spec.as_array() {
        return values.clone();
    }
    if let Some(values) = spec.get("values").and_then(Value::as_array) {
        return values.clone();
    }
    let Some(min) = spec.get("min").and_then(Value::as_f64) else {
        return Vec::new();
    };
    let Some(max) = spec.get("max").and_then(Value::as_f64) else {
        return Vec::new();
    };
    if samples <= 1 {
        return Vec::new();
    }
    let step = (max - min) / (samples - 1) as f64;
    (0..samples)
        .map(|index| Value::from(min + step * index as f64))
        .collect()
}

fn config_number(config: &Value, field: &str, default_value: f64) -> f64 {
    config
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default_value)
}
