use serde_json::Value;

pub(super) fn supports(id: &str) -> bool {
    kind(id).is_some()
}

fn kind(id: &str) -> Option<&'static str> {
    match id {
        "extract.transport_result_diagnostics"
        | "extract.thermal_result_diagnostics"
        | "extract.electrostatic_result_diagnostics"
        | "extract.magnetostatic_result_diagnostics"
        | "extract.thermo_result_diagnostics"
        | "extract.stokes_flow_result_diagnostics" => Some("extract"),
        "transform.score_transport_quality"
        | "transform.score_thermal_quality"
        | "transform.score_electrostatic_quality"
        | "transform.score_magnetostatic_quality"
        | "transform.score_cfd_quality" => Some("transform"),
        _ => None,
    }
}

pub(super) fn run(id: &str, task: &Value) -> Result<Value, String> {
    let kind =
        kind(id).ok_or_else(|| format!("unsupported agent-native diagnostic operator: {id}"))?;
    if task.pointer("/operator/kind").and_then(Value::as_str) != Some(kind) {
        return Err(format!("{id} requires operator kind {kind}"));
    }
    let input = task
        .get("input_artifact")
        .ok_or("missing input_artifact")?
        .clone();
    let config = task.get("config").cloned().unwrap_or(Value::Null);
    if kind == "extract" {
        kyuubiki_engine::run_extract_operator(id, input, config)
    } else {
        kyuubiki_engine::run_transform_operator(id, input, config)
    }
}
