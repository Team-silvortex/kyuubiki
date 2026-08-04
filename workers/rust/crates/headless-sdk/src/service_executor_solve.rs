use crate::service_executor::{
    execute_direct_fem_submit, execute_job_wait, execute_result_fetch,
    normalize_job_submission_result, request_json, required_path_segment,
};
use crate::{HeadlessExecutorError, HeadlessExecutorOutcome, direct_fem_submit_route};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub(crate) fn execute_direct_mesh_solve(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let resolved_payload = resolve_direct_mesh_source(base_url, api_token, payload)?;
    if !has_explicit_solver_endpoints(&resolved_payload)
        && let Some(action) =
            find_study_kind(&resolved_payload).and_then(direct_fem_action_for_study_kind)
    {
        let model = resolved_payload
            .get("input")
            .or_else(|| resolved_payload.get("model_payload"))
            .cloned()
            .ok_or_else(|| error("direct_mesh_solve requires input or model_payload"))?;
        return execute_direct_fem_submit(base_url, api_token, &action, &json!({ "model": model }));
    }
    let body = direct_mesh_request(&resolved_payload)?;
    let result = request_json(
        base_url,
        api_token,
        "POST",
        "/api/direct-mesh/solve",
        Some(body),
    )?;
    let mut normalized = normalize_job_submission_result(result);
    if let Some(endpoint) = normalized
        .get("raw")
        .and_then(|raw| raw.get("direct_mesh"))
        .and_then(|mesh| mesh.get("endpoint"))
        .cloned()
    {
        normalized
            .as_object_mut()
            .expect("normalized job result is an object")
            .insert("endpoint".to_string(), endpoint);
    }
    Ok(outcome(normalized))
}

fn has_explicit_solver_endpoints(payload: &Value) -> bool {
    payload
        .get("endpoints")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty())
}

fn resolve_direct_mesh_source(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<Value, HeadlessExecutorError> {
    if payload.get("input").is_some_and(Value::is_object)
        || payload.get("model_payload").is_some_and(Value::is_object)
    {
        return Ok(payload.clone());
    }
    if pick_string(payload, &["model_version_id", "modelVersionId"]).is_some() {
        return load_model_reference(
            base_url,
            api_token,
            payload,
            &["model_version_id", "modelVersionId"],
            "model_version_id",
            "model-versions",
            "version",
        );
    }
    if pick_string(payload, &["model_id", "modelId"]).is_some() {
        return load_model_reference(
            base_url,
            api_token,
            payload,
            &["model_id", "modelId"],
            "model_id",
            "models",
            "model",
        );
    }
    Ok(payload.clone())
}

fn load_model_reference(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
    id_keys: &[&str],
    canonical_id_key: &str,
    route: &str,
    envelope_key: &str,
) -> Result<Value, HeadlessExecutorError> {
    let id = required_path_segment(payload, id_keys)?;
    let envelope = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/{route}/{id}"),
        None,
    )?;
    let model = envelope
        .get(envelope_key)
        .and_then(Value::as_object)
        .ok_or_else(|| error(format!("could not load {envelope_key} {id}")))?;
    let mut resolved = payload.as_object().cloned().unwrap_or_default();
    resolved.insert(canonical_id_key.to_string(), Value::String(id.to_string()));
    resolved.insert(
        "model_payload".to_string(),
        model.get("payload").cloned().unwrap_or(Value::Null),
    );
    copy_if_missing(&mut resolved, "study_kind", model.get("kind"));
    copy_if_missing(&mut resolved, "project_id", model.get("project_id"));
    Ok(Value::Object(resolved))
}

pub(crate) fn execute_solve_from_model_version(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let model_version_id = required_path_segment(payload, &["model_version_id", "modelVersionId"])?;
    let envelope = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/model-versions/{model_version_id}"),
        None,
    )?;
    let version = envelope
        .get("version")
        .and_then(Value::as_object)
        .ok_or_else(|| error(format!("could not load model version {model_version_id}")))?;
    if let Some(action) = version
        .get("kind")
        .and_then(Value::as_str)
        .and_then(direct_fem_action_for_study_kind)
    {
        let model = version.get("payload").cloned().unwrap_or(Value::Null);
        let mut outcome =
            execute_direct_fem_submit(base_url, api_token, &action, &json!({ "model": model }))?;
        outcome
            .result
            .as_object_mut()
            .ok_or_else(|| error("model-version solve returned a non-object result"))?
            .insert(
                "model_version_id".to_string(),
                Value::String(model_version_id.to_string()),
            );
        return Ok(outcome);
    }
    let mut resolved = payload.as_object().cloned().unwrap_or_default();
    resolved.insert(
        "model_version_id".to_string(),
        Value::String(model_version_id.to_string()),
    );
    resolved.insert(
        "model_payload".to_string(),
        version.get("payload").cloned().unwrap_or(Value::Null),
    );
    copy_if_missing(&mut resolved, "study_kind", version.get("kind"));
    copy_if_missing(&mut resolved, "project_id", version.get("project_id"));
    let mut outcome = execute_direct_mesh_solve(base_url, api_token, &Value::Object(resolved))?;
    outcome
        .result
        .as_object_mut()
        .ok_or_else(|| error("model-version solve returned a non-object result"))?
        .insert(
            "model_version_id".to_string(),
            Value::String(model_version_id.to_string()),
        );
    Ok(outcome)
}

fn direct_fem_action_for_study_kind(study_kind: &str) -> Option<String> {
    let action = match study_kind {
        "axial_bar_1d" => "solve_bar_1d".to_string(),
        other => format!("solve_{other}"),
    };
    direct_fem_submit_route(&action).is_some().then_some(action)
}

pub(crate) fn execute_solve_and_wait_from_model_version(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let solved = execute_solve_from_model_version(base_url, api_token, payload)?;
    let job_id = solved
        .result
        .get("job_id")
        .or_else(|| solved.result.get("job").and_then(|job| job.get("job_id")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| error("solve response did not contain a job_id"))?;
    let mut wait_payload =
        Map::from_iter([("job_id".to_string(), Value::String(job_id.to_string()))]);
    copy_selected(payload, &mut wait_payload, &["interval_ms", "timeout_ms"]);
    let waited = execute_job_wait(base_url, api_token, &Value::Object(wait_payload))?;
    let mut result_payload =
        Map::from_iter([("job_id".to_string(), Value::String(job_id.to_string()))]);
    copy_selected(
        payload,
        &mut result_payload,
        &["prefer_job_result", "direct_mesh"],
    );
    let fetched = execute_result_fetch(base_url, api_token, &Value::Object(result_payload))?;
    let status = waited
        .result
        .get("status")
        .cloned()
        .unwrap_or_else(|| Value::String("completed".to_string()));
    let model_version_id = solved
        .result
        .get("model_version_id")
        .cloned()
        .unwrap_or(Value::Null);
    let endpoint = solved
        .result
        .get("endpoint")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(outcome(json!({
        "job_id": job_id,
        "status": status,
        "model_version_id": model_version_id,
        "endpoint": endpoint,
        "solve": solved.result,
        "wait": waited.result,
        "result": fetched.result,
    })))
}

fn direct_mesh_request(payload: &Value) -> Result<Value, HeadlessExecutorError> {
    let study_kind = find_study_kind(payload)
        .ok_or_else(|| error("direct_mesh_solve requires study_kind or model payload metadata"))?;
    let raw_input = payload
        .get("input")
        .or_else(|| payload.get("model_payload"))
        .and_then(Value::as_object)
        .ok_or_else(|| error("direct_mesh_solve requires input or model_payload"))?;
    let input = normalize_study_input(study_kind, raw_input, payload)?;
    let endpoints = payload
        .get("endpoints")
        .cloned()
        .ok_or_else(|| error("direct_mesh_solve requires endpoints"))?;
    Ok(json!({
        "study_kind": study_kind,
        "input": input,
        "endpoints": endpoints,
        "selection_mode": pick_string(payload, &["selection_mode", "selectionMode"])
            .unwrap_or("first_reachable"),
    }))
}

fn normalize_study_input(
    study_kind: &str,
    input: &Map<String, Value>,
    payload: &Value,
) -> Result<Value, HeadlessExecutorError> {
    let mut normalized = match study_kind {
        "axial_bar_1d" => normalize_axial_bar(input)?,
        "beam_1d" | "thermal_beam_1d" | "thermal_truss_2d" | "truss_2d" | "frame_2d"
        | "thermal_frame_2d" | "truss_3d" | "thermal_truss_3d" => {
            normalize_mesh(input, payload, ElementMode::Youngs, NodeMode::Plain)?
        }
        "plane_triangle_2d" | "plane_quad_2d" => {
            normalize_mesh(input, payload, ElementMode::Plane, NodeMode::Plain)?
        }
        "thermal_plane_triangle_2d" | "thermal_plane_quad_2d" => {
            normalize_mesh(input, payload, ElementMode::Plane, NodeMode::ThermalPlane)?
        }
        "heat_plane_triangle_2d" | "heat_plane_quad_2d" => {
            normalize_mesh(input, payload, ElementMode::StripMaterial, NodeMode::Heat)?
        }
        "thermal_bar_1d" | "heat_bar_1d" | "torsion_1d" | "spring_1d" | "spring_2d"
        | "spring_3d" => normalize_mesh(input, payload, ElementMode::Plain, NodeMode::Plain)?,
        other => return Err(error(format!("unsupported direct mesh study kind {other}"))),
    };
    let object = normalized
        .as_object_mut()
        .expect("normalized study input is an object");
    copy_selected(payload, object, &["project_id", "model_version_id"]);
    Ok(normalized)
}

fn normalize_axial_bar(input: &Map<String, Value>) -> Result<Value, HeadlessExecutorError> {
    let youngs_gpa = input
        .get("youngs_modulus_gpa")
        .and_then(Value::as_f64)
        .ok_or_else(|| error("axial_bar_1d requires youngs_modulus_gpa"))?;
    Ok(json!({
        "length": required_value(input, "length")?,
        "area": required_value(input, "area")?,
        "elements": required_value(input, "elements")?,
        "tip_force": required_value(input, "tip_force")?,
        "youngs_modulus": youngs_gpa * 1.0e9,
    }))
}

#[derive(Clone, Copy)]
enum ElementMode {
    Plain,
    StripMaterial,
    Youngs,
    Plane,
}

#[derive(Clone, Copy)]
enum NodeMode {
    Plain,
    Heat,
    ThermalPlane,
}

fn normalize_mesh(
    input: &Map<String, Value>,
    payload: &Value,
    element_mode: ElementMode,
    node_mode: NodeMode,
) -> Result<Value, HeadlessExecutorError> {
    let nodes = required_array(input, "nodes")?
        .iter()
        .map(|node| normalize_node(node, node_mode))
        .collect::<Result<Vec<_>, _>>()?;
    let materials = material_lookup(input, payload);
    let elements = required_array(input, "elements")?
        .iter()
        .map(|element| normalize_element(element, element_mode, &materials))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "nodes": nodes, "elements": elements }))
}

fn normalize_node(value: &Value, mode: NodeMode) -> Result<Value, HeadlessExecutorError> {
    let mut node = value
        .as_object()
        .cloned()
        .ok_or_else(|| error("study node must be an object"))?;
    match mode {
        NodeMode::Plain => {}
        NodeMode::Heat => {
            node.entry("temperature".to_string()).or_insert(json!(0));
            node.entry("heat_load".to_string()).or_insert(json!(0));
        }
        NodeMode::ThermalPlane => {
            node.entry("temperature_delta".to_string())
                .or_insert(json!(0));
        }
    }
    Ok(Value::Object(node))
}

fn normalize_element(
    value: &Value,
    mode: ElementMode,
    materials: &HashMap<String, Map<String, Value>>,
) -> Result<Value, HeadlessExecutorError> {
    let mut element = value
        .as_object()
        .cloned()
        .ok_or_else(|| error("study element must be an object"))?;
    if matches!(mode, ElementMode::Plain) {
        return Ok(Value::Object(element));
    }
    let material = element
        .remove("material_id")
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|id| materials.get(&id));
    if matches!(mode, ElementMode::Youngs | ElementMode::Plane) {
        copy_material_field(&mut element, material, "youngs_modulus");
    }
    if matches!(mode, ElementMode::Plane) {
        copy_material_field(&mut element, material, "poisson_ratio");
    }
    Ok(Value::Object(element))
}

fn material_lookup(
    input: &Map<String, Value>,
    payload: &Value,
) -> HashMap<String, Map<String, Value>> {
    payload
        .get("materials")
        .or_else(|| input.get("materials"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|material| {
            let object = material.as_object()?;
            let id = object.get("id")?.as_str()?.to_string();
            Some((id, object.clone()))
        })
        .collect()
}

fn copy_material_field(
    element: &mut Map<String, Value>,
    material: Option<&Map<String, Value>>,
    key: &str,
) {
    if let Some(value) = material.and_then(|material| material.get(key)).cloned() {
        element.insert(key.to_string(), value);
    }
}

fn find_study_kind(payload: &Value) -> Option<&str> {
    pick_string(payload, &["study_kind", "studyKind"])
        .or_else(|| infer_study_kind(payload.get("model_payload")?))
}

fn infer_study_kind(value: &Value) -> Option<&str> {
    pick_string(value, &["study_kind", "kind"]).or_else(|| {
        value
            .get("analysis_metadata")
            .and_then(|metadata| pick_string(metadata, &["study_kind"]))
    })
}

fn pick_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn required_value(input: &Map<String, Value>, key: &str) -> Result<Value, HeadlessExecutorError> {
    input
        .get(key)
        .cloned()
        .ok_or_else(|| error(format!("study input requires {key}")))
}

fn required_array<'a>(
    input: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a [Value], HeadlessExecutorError> {
    input
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| error(format!("study input requires {key} array")))
}

fn copy_if_missing(target: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if !target.contains_key(key) {
        if let Some(value) = value.cloned() {
            target.insert(key.to_string(), value);
        }
    }
}

fn copy_selected(source: &Value, target: &mut Map<String, Value>, keys: &[&str]) {
    for key in keys {
        if let Some(value) = source.get(*key).cloned() {
            target.insert((*key).to_string(), value);
        }
    }
}

fn outcome(result: Value) -> HeadlessExecutorOutcome {
    HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result,
    }
}

fn error(message: impl Into<String>) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: message.into(),
    }
}
