use kyuubiki_protocol::{
    BeginAgentDrainRequest, ResumeAgentAdmissionRequest, RpcMethod, RpcRequest, RpcResponse,
};

use crate::agent_lifecycle;
use crate::transport::AgentReply;

pub(crate) fn handle_begin_drain(request: RpcRequest) -> AgentReply {
    let request_id = request.id;
    let params = match serde_json::from_value::<BeginAgentDrainRequest>(request.params) {
        Ok(params) => params,
        Err(error) => return invalid_params(request_id, error),
    };
    lifecycle_result(
        request_id,
        agent_lifecycle::begin_drain(&params.controller_id, &params.reason),
    )
}

pub(crate) fn handle_describe(request: RpcRequest) -> AgentReply {
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::success(
            request.id,
            serde_json::to_value(agent_lifecycle::snapshot())
                .expect("lifecycle snapshot should serialize"),
        ),
    )
}

pub(crate) fn handle_resume(request: RpcRequest) -> AgentReply {
    let request_id = request.id;
    let params = match serde_json::from_value::<ResumeAgentAdmissionRequest>(request.params) {
        Ok(params) => params,
        Err(error) => return invalid_params(request_id, error),
    };
    lifecycle_result(
        request_id,
        agent_lifecycle::resume_admission(&params.controller_id, params.drain_generation),
    )
}

pub(crate) fn is_mutation(method: &RpcMethod) -> bool {
    matches!(
        method,
        RpcMethod::BeginAgentDrain | RpcMethod::ResumeAgentAdmission
    )
}

pub(crate) fn reject_non_loopback_mutation(request_id: String) -> AgentReply {
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(
            request_id,
            "agent_lifecycle_control_requires_loopback",
            "agent lifecycle mutations require a host-loopback control connection",
            serde_json::json!({
                "schema_version": "kyuubiki.agent-lifecycle-error/v1",
                "mutation_control_scope": "host_loopback",
                "lifecycle": agent_lifecycle::snapshot()
            }),
        ),
    )
}

fn lifecycle_result(
    request_id: String,
    result: Result<
        kyuubiki_protocol::AgentLifecycleDescriptor,
        agent_lifecycle::LifecycleControlError,
    >,
) -> AgentReply {
    match result {
        Ok(snapshot) => AgentReply::Stream(
            Vec::new(),
            RpcResponse::success(
                request_id,
                serde_json::to_value(snapshot).expect("lifecycle snapshot should serialize"),
            ),
        ),
        Err(error) => AgentReply::Stream(
            Vec::new(),
            RpcResponse::error_with_details(
                request_id,
                error.code.clone(),
                error.message.clone(),
                serde_json::to_value(error).expect("lifecycle error should serialize"),
            ),
        ),
    }
}

fn invalid_params(request_id: String, error: serde_json::Error) -> AgentReply {
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(
            request_id,
            "invalid_params",
            format!("invalid agent lifecycle parameters: {error}"),
            serde_json::json!({
                "schema_version": "kyuubiki.agent-lifecycle-error/v1",
                "lifecycle": agent_lifecycle::snapshot()
            }),
        ),
    )
}
