use kyuubiki_protocol::{
    AGENT_EXECUTION_CANCELLATION_SCHEMA, AgentLifecycleDescriptor, CancelExecutionRequest,
    RpcRequest, RpcResponse,
};
use serde_json::json;

use crate::{agent_execution_control, agent_lifecycle, transport::AgentReply};

pub(crate) fn handle_cancel(request: RpcRequest) -> AgentReply {
    handle_cancel_in(request, agent_lifecycle::snapshot())
}

fn handle_cancel_in(request: RpcRequest, lifecycle: AgentLifecycleDescriptor) -> AgentReply {
    let params = serde_json::from_value::<CancelExecutionRequest>(request.params)
        .map_err(|error| error.to_string())
        .and_then(|target| {
            target.validate()?;
            Ok(target)
        });
    let target = match params {
        Ok(target) => target,
        Err(message) => return error(request.id, "invalid_params", message),
    };
    if lifecycle.state == "unavailable" {
        return error(
            request.id,
            "agent_lifecycle_unavailable",
            "agent process identity is unavailable".into(),
        );
    }
    let matched = if target.process_instance_id == lifecycle.process_instance_id {
        match agent_execution_control::register_execution_cancel(&target) {
            Ok(matched) => matched,
            Err(message) => return error(request.id, "execution_control_unavailable", message),
        }
    } else {
        false
    };
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::success(
            request.id,
            json!({
                "schema_version":AGENT_EXECUTION_CANCELLATION_SCHEMA,
                "execution_target":target,
                "status":if matched {"requested"} else {"target_not_observed"},
                "cancel_registered":matched, "execution_terminal_confirmed":false,
                "pending_cancellation_created":false, "operator_package_cleanup_performed":false,
                "automatic_replay_authorized":false
            }),
        ),
    )
}

fn error(request_id: String, code: &str, message: String) -> AgentReply {
    AgentReply::Stream(Vec::new(), RpcResponse::error(request_id, code, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_protocol::RpcMethod;

    #[test]
    fn unavailable_process_identity_rejects_cancel_without_marking_current_or_future_execution() {
        let job = "scoped-cancel-unavailable-job";
        let id = "scoped-cancel-unavailable-request";
        let active = agent_execution_control::begin(id.into(), 40, Some(job.into())).unwrap();
        let request = RpcRequest {
            rpc_version: 1,
            id: "scoped-cancel-unavailable-control".into(),
            method: RpcMethod::CancelExecution,
            params: json!({"process_instance_id":"unavailable", "request_id":id,
                "generation":40, "job_id":job}),
        };
        let lifecycle = AgentLifecycleDescriptor {
            process_instance_id: "unavailable".into(),
            state: "unavailable".into(),
            accepting_new_work: false,
            ..AgentLifecycleDescriptor::default()
        };
        let AgentReply::Stream(frames, response) = handle_cancel_in(request, lifecycle);
        assert!(frames.is_empty());
        assert!(!response.ok, "unavailable identity accepted: {response:?}");
        assert_eq!(response.error.unwrap().code, "agent_lifecycle_unavailable");
        assert!(!active.solver.cancellation_requested());
        assert!(!agent_execution_control::take_cancelled(job));
        drop(active);
        let next = agent_execution_control::begin(id.into(), 41, Some(job.into())).unwrap();
        assert!(!next.solver.cancellation_requested());
    }
}
