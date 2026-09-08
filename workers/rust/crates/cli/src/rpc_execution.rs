use crate::agent_artifact::decode_solver_params;
use crate::agent_reply_writer::{self, SharedReplyWriter};
use crate::agent_state::{
    build_progress_frames, extract_job_id, take_cancelled, take_execution_cancelled,
};
use crate::operator_task_runtime::run_operator_task_ir;
use crate::transport::{AgentReply, HeartbeatHandle};
use crate::{agent_fault_injection, agent_lifecycle};
use kyuubiki_protocol::{RpcMethod, RpcRequest, RpcResponse};
use serde::{Serialize, de::DeserializeOwned};

pub(crate) fn handle_operator_task_ir(
    request: RpcRequest,
    writer: Option<SharedReplyWriter>,
) -> AgentReply {
    let request_id = request.id;
    let maybe_job_id = extract_job_id(&request.params);
    let guard = match agent_reply_writer::begin_execution(
        writer.as_ref(),
        request_id.clone(),
        maybe_job_id.clone(),
        "run_operator_task_ir".to_string(),
    ) {
        Ok(guard) => guard,
        Err(error) => return execution_admission_error(request_id, error),
    };
    let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
        writer.clone().map(|shared_writer| {
            HeartbeatHandle::spawn(
                shared_writer,
                request_id.clone(),
                job_id.clone(),
                guard.clone(),
            )
        })
    });
    agent_fault_injection::wait_for_release(
        &guard,
        &request_id,
        maybe_job_id.as_deref(),
        "run_operator_task_ir",
    );
    if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
        return cancelled_reply(request_id, guard, heartbeat);
    }

    let result = match run_operator_task_ir(&request.params) {
        Ok(result) => result,
        Err(error) => {
            stop_heartbeat(heartbeat);
            let report = agent_lifecycle::fail_execution(guard, error.code, error.message);
            let mut details =
                serde_json::to_value(report).expect("failure report should serialize");
            details["operator_task_failure_receipt"] = error.details;
            let message = details["message"].as_str().unwrap_or_default().to_string();
            return AgentReply::Stream(
                Vec::new(),
                RpcResponse::error_with_details(request_id, error.code, message, details),
            );
        }
    };

    if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
        stop_heartbeat(heartbeat);
        let report =
            agent_lifecycle::fail_execution(guard, "cancelled", "operator task was cancelled");
        let reason_code = report.reason_code.clone();
        return AgentReply::Stream(
            Vec::new(),
            RpcResponse::error_with_details(
                request_id,
                reason_code,
                report.message.clone(),
                serde_json::to_value(report).expect("failure report should serialize"),
            ),
        );
    }

    stop_heartbeat(heartbeat);
    agent_reply_writer::complete_execution(writer.as_ref(), guard);
    AgentReply::Stream(Vec::new(), RpcResponse::success(request_id, result))
}

pub(crate) fn run_solver<Request, ResultValue, NodeCount, Solver>(
    request: RpcRequest,
    writer: Option<SharedReplyWriter>,
    model_name: &str,
    serialize_label: &str,
    node_count: NodeCount,
    solver: Solver,
) -> AgentReply
where
    Request: DeserializeOwned + 'static,
    ResultValue: Serialize,
    NodeCount: FnOnce(&Request) -> usize,
    Solver: FnOnce(&Request) -> Result<ResultValue, String>,
{
    let request_id = request.id;
    let method = rpc_method_name(&request.method);
    let maybe_job_id = extract_job_id(&request.params);
    let externalize_result = request.params.get("model_artifact_ref").is_some();
    let guard = match agent_reply_writer::begin_execution(
        writer.as_ref(),
        request_id.clone(),
        maybe_job_id.clone(),
        method.clone(),
    ) {
        Ok(guard) => guard,
        Err(error) => return execution_admission_error(request_id, error),
    };

    let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
        writer.clone().map(|shared_writer| {
            HeartbeatHandle::spawn(
                shared_writer,
                request_id.clone(),
                job_id.clone(),
                guard.clone(),
            )
        })
    });
    agent_fault_injection::wait_for_release(&guard, &request_id, maybe_job_id.as_deref(), &method);
    if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
        return cancelled_reply(request_id, guard, heartbeat);
    }

    let params = match decode_solver_params::<Request>(request.params) {
        Ok(params) => params,
        Err(error) => {
            stop_heartbeat(heartbeat);
            let report =
                agent_lifecycle::fail_execution(guard, "invalid_params", error.to_string());
            return AgentReply::Stream(
                Vec::new(),
                RpcResponse::error_with_details(
                    request_id,
                    "invalid_params",
                    report.message.clone(),
                    serde_json::to_value(report).expect("failure report should serialize"),
                ),
            );
        }
    };

    if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
        return cancelled_reply(request_id, guard, heartbeat);
    }
    match solver(&params) {
        Ok(result) => {
            if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
                stop_heartbeat(heartbeat);
                let report =
                    agent_lifecycle::fail_execution(guard, "cancelled", "job was cancelled");
                let reason_code = report.reason_code.clone();
                return AgentReply::Stream(
                    Vec::new(),
                    RpcResponse::error_with_details(
                        request_id,
                        reason_code,
                        report.message.clone(),
                        serde_json::to_value(report).expect("failure report should serialize"),
                    ),
                );
            }

            let encoded_result = if externalize_result {
                crate::agent_result_artifact::upload(&method, &result)
            } else {
                serde_json::to_value(result)
                    .map_err(|error| format!("failed to serialize {serialize_label}: {error}"))
            };
            let encoded_result = match encoded_result {
                Ok(result) => result,
                Err(error) => {
                    stop_heartbeat(heartbeat);
                    let report =
                        agent_lifecycle::fail_execution(guard, "result_transport_failed", error);
                    return AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error_with_details(
                            request_id,
                            "result_transport_failed",
                            report.message.clone(),
                            serde_json::to_value(report).expect("failure report should serialize"),
                        ),
                    );
                }
            };
            let progress_frames =
                build_progress_frames(model_name, &request_id, node_count(&params));
            stop_heartbeat(heartbeat);
            agent_reply_writer::complete_execution(writer.as_ref(), guard);
            AgentReply::Stream(
                progress_frames,
                RpcResponse::success(request_id, encoded_result),
            )
        }
        Err(error) => {
            stop_heartbeat(heartbeat);
            let report = agent_lifecycle::fail_execution(guard, "solve_failed", error);
            AgentReply::Stream(
                Vec::new(),
                RpcResponse::error_with_details(
                    request_id,
                    "solve_failed",
                    report.message.clone(),
                    serde_json::to_value(report).expect("failure report should serialize"),
                ),
            )
        }
    }
}

fn execution_cancelled(
    guard: &agent_lifecycle::ExecutionGuard,
    request_id: &str,
    job_id: Option<&str>,
) -> bool {
    let request_cancelled = take_execution_cancelled(request_id);
    let job_cancelled = job_id.is_some_and(take_cancelled);
    guard.cancellation_requested() || request_cancelled || job_cancelled
}

fn cancelled_reply(
    request_id: String,
    guard: agent_lifecycle::ExecutionGuard,
    heartbeat: Option<HeartbeatHandle>,
) -> AgentReply {
    stop_heartbeat(heartbeat);
    let report = agent_lifecycle::fail_execution(
        guard,
        "cancelled",
        "execution cancelled before computation",
    );
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(
            request_id,
            report.reason_code.clone(),
            report.message.clone(),
            serde_json::to_value(report).expect("failure report should serialize"),
        ),
    )
}

fn stop_heartbeat(heartbeat: Option<HeartbeatHandle>) {
    if let Some(heartbeat) = heartbeat {
        heartbeat.stop();
    }
}

fn execution_admission_error(
    request_id: String,
    error: agent_lifecycle::ExecutionAdmissionError,
) -> AgentReply {
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(
            request_id,
            error.reason_code.clone(),
            error.message.clone(),
            serde_json::to_value(error).expect("execution admission error should serialize"),
        ),
    )
}

fn rpc_method_name(method: &RpcMethod) -> String {
    serde_json::to_value(method)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| format!("{method:?}"))
}
