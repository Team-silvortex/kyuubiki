use serde_json::{Map, Value};

use crate::workflow_sweep_contract::{checked_case_count, count_option, object_or_null};

pub fn build_quality_parameter_sweep_plan(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "quality sweep config")?;
    let request = payload.get("request_payload").unwrap_or(&Value::Null);
    object_or_null(request, "quality sweep request_payload")?;

    let optimization_hint = request
        .get("optimization_hint")
        .or_else(|| payload.get("selected_iteration_hint"))
        .cloned()
        .unwrap_or(Value::Null);
    if payload
        .get("source_ranking_complete")
        .and_then(Value::as_bool)
        == Some(false)
        || optimization_hint.get("action").and_then(Value::as_str)
            == Some("repair_rejected_candidates")
    {
        return Ok(serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": false,
            "sweep_action": "replan",
            "sweep_blocking_reason": "candidate_evaluation_incomplete",
            "source_rejected_candidates": payload.get("source_rejected_candidates").cloned().unwrap_or_else(|| serde_json::json!([])),
            "optimization_hint": optimization_hint,
            "case_count_estimate": 0,
            "axes": [],
            "base": config.get("base").cloned().unwrap_or_else(|| serde_json::json!({})),
            "plan_summary": "Repair incomplete candidate evaluations before planning another quality sweep.",
        }));
    }

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
    let samples = count_option(config.get("samples_per_axis"), "samples_per_axis", 3, 2)?;
    let focus_field = request
        .get("optimization_hint")
        .or_else(|| payload.get("selected_iteration_hint"))
        .and_then(|hint| hint.get("focus_field"))
        .and_then(Value::as_str);
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
    let max_axes = count_option(config.get("max_axes"), "max_axes", search_space.len(), 1)?;
    let max_cases = count_option(
        config
            .get("max_cases")
            .or_else(|| request.get("max_candidates")),
        "max_cases",
        64,
        0,
    )?;
    let mut planned_axes = search_space_axes(search_space, samples)?;
    if planned_axes.is_empty() {
        return Err(
            "transform.build_quality_parameter_sweep_plan requires usable search_space axes"
                .to_string(),
        );
    }
    let usable_axis_count = planned_axes.len();
    planned_axes.sort_by(|left, right| {
        focus_rank(left.path, focus_field, focus_domain)
            .cmp(&focus_rank(right.path, focus_field, focus_domain))
            .then_with(|| left.path.cmp(right.path))
    });
    planned_axes.truncate(max_axes);
    let case_count_estimate = checked_case_count(planned_axes.iter().map(PlannedAxis::len))?;
    // A blocked plan is diagnostic-only: do not allocate samples just to reject them.
    let axes = planned_axes
        .iter()
        .map(|axis| axis.materialize(case_count_estimate > max_cases))
        .collect::<Result<Vec<_>, _>>()?;

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

fn search_space_axes(
    search_space: &Map<String, Value>,
    samples: usize,
) -> Result<Vec<PlannedAxis<'_>>, String> {
    search_space
        .iter()
        .map(|(path, spec)| {
            if path.trim().is_empty() {
                return Err("search_space axis path must not be empty".to_string());
            }
            let values = if spec.is_array() || spec.get("values").is_some() {
                let values = spec
                    .as_array()
                    .or_else(|| spec.get("values")?.as_array())
                    .filter(|values| !values.is_empty())
                    .ok_or_else(|| format!("search_space axis {path} requires nonempty values"))?;
                PlannedValues::Explicit(values)
            } else {
                let endpoint = |field| {
                    spec.get(field)
                        .and_then(Value::as_f64)
                        .filter(|number| number.is_finite())
                        .ok_or_else(|| format!("search_space axis {path} requires finite {field}"))
                };
                PlannedValues::Range {
                    min: endpoint("min")?,
                    max: endpoint("max")?,
                    samples,
                }
            };
            Ok(PlannedAxis { path, values })
        })
        .collect()
}

fn sweep_budget_summary(
    usable_axis_count: usize,
    axes: &[Value],
    case_count_estimate: usize,
    max_cases: usize,
) -> Value {
    let planned_axis_count = axes.len();
    let case_budget_exceeded = case_count_estimate > max_cases;
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
    let recommendation = if max_cases == 0 {
        "increase_case_budget"
    } else if case_budget_exceeded
        && recommended_axis_count > 0
        && recommended_axis_count < planned_axis_count
    {
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

fn recommended_axis_count_for_budget(axes: &[Value], max_cases: usize) -> usize {
    let mut product = 1usize;
    let mut included = 0usize;
    for axis in axes {
        let value_count = axis.get("value_count").and_then(Value::as_u64).unwrap_or(0) as usize;
        if value_count == 0 {
            continue;
        }
        let Some(next_product) = product.checked_mul(value_count) else {
            break;
        };
        if next_product > max_cases {
            break;
        }
        product = next_product;
        included += 1;
    }
    included
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

struct PlannedAxis<'a> {
    path: &'a str,
    values: PlannedValues<'a>,
}

enum PlannedValues<'a> {
    Explicit(&'a Vec<Value>),
    Range { min: f64, max: f64, samples: usize },
}

impl PlannedAxis<'_> {
    fn len(&self) -> usize {
        match &self.values {
            PlannedValues::Explicit(values) => values.len(),
            PlannedValues::Range { samples, .. } => *samples,
        }
    }

    fn materialize(&self, deferred: bool) -> Result<Value, String> {
        let mut axis = serde_json::json!({
            "label": self.path, "path": self.path, "value_count": self.len(),
        });
        if deferred {
            axis["values_deferred"] = Value::Bool(true);
        } else {
            axis["values"] = Value::Array(match &self.values {
                PlannedValues::Explicit(values) => (*values).clone(),
                PlannedValues::Range { min, max, samples } => {
                    let mut values = Vec::new();
                    values.try_reserve_exact(*samples).map_err(|error| {
                        format!("search_space axis {} allocation failed: {error}", self.path)
                    })?;
                    for index in 0..*samples {
                        let t = index as f64 / (samples - 1) as f64;
                        let value = if index == 0 || min == max {
                            *min
                        } else if index == samples - 1 {
                            *max
                        } else if min.is_sign_negative() == max.is_sign_negative() {
                            min + (max - min) * t
                        } else {
                            min * (1.0 - t) + max * t
                        };
                        if !value.is_finite() {
                            return Err(format!(
                                "search_space axis {} sample is not finite",
                                self.path
                            ));
                        }
                        values.push(Value::from(value));
                    }
                    values
                }
            });
        }
        Ok(axis)
    }
}
