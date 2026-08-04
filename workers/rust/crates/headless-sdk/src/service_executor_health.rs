use serde_json::Value;

pub(crate) fn with_discovered_solver_endpoints(mut health: Value) -> Value {
    let solver_endpoints = health
        .get("solver_agents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|agent| {
            let host = agent.get("host")?.as_str()?.trim();
            let port = agent.get("port")?.as_u64()?;
            let host = if host.contains(':') && !host.starts_with('[') {
                format!("[{host}]")
            } else {
                host.to_string()
            };
            Some(Value::String(format!("{host}:{port}")))
        })
        .collect();
    if let Some(object) = health.as_object_mut() {
        object.insert(
            "solver_endpoints".to_string(),
            Value::Array(solver_endpoints),
        );
    }
    health
}
