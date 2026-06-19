use serde_json::Value;

pub fn select_focus_payload(payload: Value, config: Value) -> Result<Value, String> {
    resolve_focus_payload(&payload, &config, "transform.select_focus_payload")
}

pub fn compose_focus_chain_input(payload: Value, config: Value) -> Result<Value, String> {
    let focus_payload =
        resolve_focus_payload(&payload, &config, "transform.compose_focus_chain_input")?;
    let focus_object = focus_payload.as_object().ok_or_else(|| {
        "transform.compose_focus_chain_input expects a focus payload object".to_string()
    })?;
    let metric_id = focus_object
        .get("metric_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "transform.compose_focus_chain_input focus payload is missing metric_id".to_string()
        })?;
    let source = focus_object.get("source").cloned().unwrap_or(Value::Null);
    let value = focus_object.get("value").cloned().unwrap_or(Value::Null);
    let value_field = focus_object
        .get("value_field")
        .cloned()
        .unwrap_or(Value::Null);
    let context = focus_object
        .get("context")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let bindings = config
        .get("bindings")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let annotations = config
        .get("annotations")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let mut chain = serde_json::Map::from_iter([
        (
            "chain_contract".to_string(),
            Value::from("kyuubiki.workflow_focus_chain_input/v1"),
        ),
        ("chain_kind".to_string(), Value::from("focus_chain_input")),
        ("metric_id".to_string(), Value::from(metric_id)),
        ("source".to_string(), source),
        ("value".to_string(), value),
        ("value_field".to_string(), value_field),
        ("focus_payload".to_string(), focus_payload),
        ("context".to_string(), context),
        ("bindings".to_string(), bindings),
        ("annotations".to_string(), annotations),
    ]);
    if let Some(target_operator) = config.get("target_operator").cloned() {
        chain.insert("target_operator".to_string(), target_operator);
    }
    if let Some(stage) = config.get("stage").cloned() {
        chain.insert("stage".to_string(), stage);
    }
    Ok(Value::Object(chain))
}

pub fn compose_focus_bridge_request(payload: Value, config: Value) -> Result<Value, String> {
    let chain_input = resolve_focus_chain_input(&payload, &config)?;
    let chain_object = chain_input.as_object().ok_or_else(|| {
        "transform.compose_focus_bridge_request expects a chain input object".to_string()
    })?;
    let bridge_operator = config
        .get("bridge_operator")
        .and_then(Value::as_str)
        .or_else(|| chain_object.get("target_operator").and_then(Value::as_str))
        .ok_or_else(|| {
            "transform.compose_focus_bridge_request requires config.bridge_operator or chain target_operator"
                .to_string()
        })?;
    if !bridge_operator.starts_with("bridge.") {
        return Err(
            "transform.compose_focus_bridge_request bridge_operator must start with bridge."
                .to_string(),
        );
    }
    let metric_id = chain_object
        .get("metric_id")
        .cloned()
        .unwrap_or(Value::Null);
    let source = chain_object.get("source").cloned().unwrap_or(Value::Null);
    let focus_value = chain_object.get("value").cloned().unwrap_or(Value::Null);
    let focus_payload = chain_object
        .get("focus_payload")
        .cloned()
        .unwrap_or(Value::Null);
    let bindings = chain_object
        .get("bindings")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let annotations = chain_object
        .get("annotations")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let stage = chain_object.get("stage").cloned();

    let bridge_config = serde_json::json!({
        "seed_model": config.get("seed_model").cloned().unwrap_or(Value::Null),
        "contract": config.get("contract").cloned().unwrap_or(Value::Null),
    });
    let mut request = serde_json::Map::from_iter([
        (
            "request_contract".to_string(),
            Value::from("kyuubiki.workflow_focus_bridge_request/v1"),
        ),
        (
            "request_kind".to_string(),
            Value::from("focus_bridge_request"),
        ),
        ("bridge_operator".to_string(), Value::from(bridge_operator)),
        ("metric_id".to_string(), metric_id),
        ("source".to_string(), source),
        ("focus_value".to_string(), focus_value),
        ("focus_chain_input".to_string(), chain_input),
        ("focus_payload".to_string(), focus_payload),
        ("bridge_config".to_string(), bridge_config),
        ("bindings".to_string(), bindings),
        ("annotations".to_string(), annotations),
    ]);
    if let Some(stage) = stage {
        request.insert("stage".to_string(), stage);
    }
    if let Some(bridge_payload_source) = config.get("bridge_payload_source").cloned() {
        request.insert("bridge_payload_source".to_string(), bridge_payload_source);
    }
    Ok(Value::Object(request))
}

pub fn resolve_focus_bridge_execution(payload: Value, config: Value) -> Result<Value, String> {
    if payload
        .as_object()
        .and_then(|object| object.get("execution_contract"))
        .and_then(Value::as_str)
        == Some("kyuubiki.workflow_focus_bridge_execution/v1")
    {
        return Ok(payload);
    }
    let bridge_request = resolve_named_focus_bridge_request(&payload, &config)?;
    let request_object = bridge_request.as_object().ok_or_else(|| {
        "transform.resolve_focus_bridge_execution expects a bridge request object".to_string()
    })?;
    let bridge_operator = request_object
        .get("bridge_operator")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "transform.resolve_focus_bridge_execution bridge request is missing bridge_operator"
                .to_string()
        })?;
    let bridge_payload = resolve_named_bridge_payload(&payload, &config).ok_or_else(|| {
        "transform.resolve_focus_bridge_execution requires config.bridge_payload".to_string()
    })?;
    let bridge_config = request_object
        .get("bridge_config")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    Ok(serde_json::json!({
        "execution_contract": "kyuubiki.workflow_focus_bridge_execution/v1",
        "execution_kind": "focus_bridge_execution",
        "operator_id": bridge_operator,
        "bridge_payload": bridge_payload,
        "bridge_config": bridge_config,
        "bridge_request": bridge_request,
        "metric_id": request_object.get("metric_id").cloned().unwrap_or(Value::Null),
        "focus_value": request_object.get("focus_value").cloned().unwrap_or(Value::Null),
        "bindings": request_object.get("bindings").cloned().unwrap_or_else(|| Value::Object(serde_json::Map::new())),
        "annotations": request_object.get("annotations").cloned().unwrap_or_else(|| Value::Object(serde_json::Map::new())),
        "bridge_payload_source": config
            .get("bridge_payload_source")
            .cloned()
            .or_else(|| request_object.get("bridge_payload_source").cloned())
            .unwrap_or(Value::Null),
    }))
}

pub fn execute_focus_bridge_execution(payload: Value, config: Value) -> Result<Value, String> {
    let execution = resolve_focus_bridge_execution(payload, config)?;
    let execution_object = execution.as_object().ok_or_else(|| {
        "transform.execute_focus_bridge_execution expects an execution payload object".to_string()
    })?;
    let operator_id = execution_object
        .get("operator_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "transform.execute_focus_bridge_execution execution payload is missing operator_id"
                .to_string()
        })?;
    if !operator_id.starts_with("bridge.") {
        return Err(
            "transform.execute_focus_bridge_execution operator_id must start with bridge."
                .to_string(),
        );
    }
    let bridge_payload = execution_object
        .get("bridge_payload")
        .cloned()
        .ok_or_else(|| {
            "transform.execute_focus_bridge_execution execution payload is missing bridge_payload"
                .to_string()
        })?;
    let bridge_config = execution_object
        .get("bridge_config")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let bridge_result = crate::workflow_executor::run_transform_operator(
        operator_id,
        bridge_payload,
        bridge_config,
    )?;
    Ok(serde_json::json!({
        "result_contract": "kyuubiki.workflow_focus_bridge_result/v1",
        "result_kind": "focus_bridge_result",
        "operator_id": operator_id,
        "bridge_result": bridge_result,
        "execution_payload": execution,
        "metric_id": execution_object.get("metric_id").cloned().unwrap_or(Value::Null),
        "focus_value": execution_object.get("focus_value").cloned().unwrap_or(Value::Null),
        "bridge_payload_source": execution_object
            .get("bridge_payload_source")
            .cloned()
            .unwrap_or(Value::Null),
    }))
}

fn resolve_focus_chain_input(payload: &Value, config: &Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.compose_focus_bridge_request expects an object payload".to_string()
    })?;
    if object.get("chain_contract").and_then(Value::as_str)
        == Some("kyuubiki.workflow_focus_chain_input/v1")
    {
        return Ok(payload.clone());
    }
    compose_focus_chain_input(payload.clone(), config.clone())
}

fn resolve_focus_bridge_request(payload: &Value, config: &Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.resolve_focus_bridge_execution expects an object payload".to_string()
    })?;
    if object.get("request_contract").and_then(Value::as_str)
        == Some("kyuubiki.workflow_focus_bridge_request/v1")
    {
        return Ok(payload.clone());
    }
    compose_focus_bridge_request(payload.clone(), config.clone())
}

fn resolve_named_focus_bridge_request(payload: &Value, config: &Value) -> Result<Value, String> {
    if let Some(request) = payload.as_object().and_then(|object| {
        object
            .get("request")
            .or_else(|| object.get("bridge_request"))
    }) {
        return resolve_focus_bridge_request(request, &Value::Null);
    }
    resolve_focus_bridge_request(payload, config)
}

fn resolve_named_bridge_payload(payload: &Value, config: &Value) -> Option<Value> {
    payload
        .as_object()
        .and_then(|object| object.get("bridge_payload"))
        .cloned()
        .or_else(|| config.get("bridge_payload").cloned())
}

fn resolve_focus_payload(
    payload: &Value,
    config: &Value,
    operator_id: &str,
) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| format!("{operator_id} expects an object payload"))?;
    if object.get("focus_contract").and_then(Value::as_str)
        == Some("kyuubiki.workflow_focus_payload/v1")
    {
        return Ok(payload.clone());
    }
    let metric_id = config
        .get("metric_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "{operator_id} requires config.metric_id when payload is not already a focus payload"
            )
        })?;
    let focus_payloads = object
        .get("report_focus_payloads")
        .and_then(Value::as_object)
        .or_else(|| Some(object))
        .ok_or_else(|| {
            format!("{operator_id} expects report_focus_payloads or a focus payload map")
        })?;
    focus_payloads
        .get(metric_id)
        .cloned()
        .ok_or_else(|| format!("{operator_id} could not find metric_id {metric_id}"))
}
