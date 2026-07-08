use serde_json::{Value, json};

pub(crate) fn next_round_optimization_objectives(
    decision: &str,
    report: &Value,
    focus_candidate_ids: &[String],
    violated_gate_ids: &[String],
) -> Value {
    let metric_objectives = report
        .get("metric_specs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|metric| metric.get("objective").and_then(Value::as_str) != Some("observe"))
        .filter_map(metric_objective)
        .collect::<Vec<_>>();
    let mode = match decision {
        "repair_or_rerun" => "data_repair",
        "mitigate_design_risk" => "risk_constrained_search",
        _ => "winner_neighborhood_expansion",
    };
    let guidance = optimization_guidance(decision, violated_gate_ids);
    json!({
        "schema_version": "kyuubiki.material-next-round-optimization-objectives/v1",
        "mode": mode,
        "decision": decision,
        "winner_candidate_id": report.get("winner_candidate_id").and_then(Value::as_str),
        "focus_candidate_ids": focus_candidate_ids,
        "violated_quality_gate_ids": violated_gate_ids,
        "metric_objectives": metric_objectives,
        "primary_metric_ids": primary_metric_ids(report),
        "guidance": guidance,
    })
}

fn metric_objective(metric: &Value) -> Option<Value> {
    Some(json!({
        "metric_id": metric.get("id")?.as_str()?,
        "label": metric.get("label").and_then(Value::as_str).unwrap_or("--"),
        "objective": metric.get("objective").and_then(Value::as_str).unwrap_or("observe"),
        "weight": metric.get("weight").and_then(Value::as_f64).unwrap_or(0.0),
        "unit": metric.get("unit").and_then(Value::as_str).unwrap_or(""),
    }))
}

fn primary_metric_ids(report: &Value) -> Vec<String> {
    report
        .get("metric_specs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|metric| metric.get("objective").and_then(Value::as_str) != Some("observe"))
        .filter(|metric| {
            metric
                .get("weight")
                .and_then(Value::as_f64)
                .is_none_or(|weight| weight > 0.0)
        })
        .filter_map(|metric| metric.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

fn optimization_guidance(decision: &str, violated_gate_ids: &[String]) -> String {
    if decision == "repair_or_rerun" {
        return "repair missing metrics before changing candidate geometry or material parameters"
            .to_string();
    }
    if decision == "mitigate_design_risk" {
        return format!(
            "generate conservative neighbors that preserve score while reducing quality-gate risk: {}",
            violated_gate_ids.join(", ")
        );
    }
    "expand around the incumbent winner while preserving all passed quality gates".to_string()
}
