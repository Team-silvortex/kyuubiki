use kyuubiki_protocol::{
    AgentLifecycleDescriptor, RPC_VERSION, RpcMethod, RpcProtocolDescriptor, SOLVER_RPC_PROTOCOL,
    validate_agent_lifecycle_descriptor,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Admission {
    schema_version: String,
    configured: bool,
    max_active_executions: usize,
    active_execution_count: usize,
}

pub(super) fn policy() -> Value {
    json!({
        "schema_version":"kyuubiki.direct-mesh-selection/v1",
        "scope":"new_legacy_solver_dispatch_only",
        "required_metadata":["protocol", "lifecycle", "execution_admission", "runtime.health_score"],
        "missing_metadata":"ineligible_not_implicitly_ready",
        "eligibility":"supported_method_accepting_lifecycle_available_execution_capacity",
        "health_score":"ranking_after_eligibility_not_admission_or_recovery_proof",
        "active_count":"maximum_of_lifecycle_and_admission_snapshots",
        "fresh_discovery_per_dispatch":true,
        "slot_reserved_by_discovery":false,
        "automatic_retry_after_dispatch":false,
        "durable_recovery_reselection":false,
        "authentication":false
    })
}

pub(super) fn eligible_score(descriptor: &Value, method: &str) -> Result<u8, &'static str> {
    if descriptor["role"] != "solver_agent" {
        return Err("invalid_solver_role");
    }
    let protocol: RpcProtocolDescriptor = serde_json::from_value(descriptor["protocol"].clone())
        .map_err(|_| "invalid_solver_protocol")?;
    if protocol.name != SOLVER_RPC_PROTOCOL
        || protocol.rpc_version != RPC_VERSION
        || protocol.transport.kind != "tcp"
        || protocol.transport.encoding != "json"
        || protocol.transport.framing.as_deref() != Some("length_prefixed_u32")
    {
        return Err("invalid_solver_protocol");
    }
    let requested: RpcMethod =
        serde_json::from_value(json!(method)).map_err(|_| "unsupported_method")?;
    if !method.starts_with("solve_") || !protocol.methods.contains(&requested) {
        return Err("unsupported_method");
    }
    // Parse the explicit field: AgentDescriptor's compatibility default is not readiness proof.
    let lifecycle: AgentLifecycleDescriptor =
        serde_json::from_value(descriptor["lifecycle"].clone()).map_err(|_| "invalid_lifecycle")?;
    validate_agent_lifecycle_descriptor(&lifecycle).map_err(|_| "invalid_lifecycle")?;
    if lifecycle.state != "accepting" || !lifecycle.accepting_new_work {
        return Err("lifecycle_not_accepting");
    }
    let admission: Admission = serde_json::from_value(descriptor["execution_admission"].clone())
        .map_err(|_| "invalid_execution_admission")?;
    if admission.schema_version != "kyuubiki.agent-execution-admission/v1"
        || !admission.configured
        || !(1..=1024).contains(&admission.max_active_executions)
    {
        return Err("invalid_execution_admission");
    }
    // The Agent observes these counts under separate locks; do not mistake a race for a free slot.
    if admission
        .active_execution_count
        .max(lifecycle.active_execution_count)
        >= admission.max_active_executions
    {
        return Err("execution_at_capacity");
    }
    descriptor["runtime"]["health_score"]
        .as_u64()
        .filter(|score| *score <= 100)
        .map(|score| score as u8)
        .ok_or("invalid_health_score")
}

#[cfg(test)]
#[path = "direct_mesh_selection_tests.rs"]
mod tests;
