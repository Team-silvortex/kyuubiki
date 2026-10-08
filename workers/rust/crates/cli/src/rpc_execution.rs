use crate::agent_artifact::decode_solver_params;
use crate::agent_reply_writer::{self, SharedReplyWriter};
use crate::agent_state::{build_progress_frames, extract_job_id};
use crate::operator_task_runtime::{OperatorTaskRuntimeError, run_operator_task_ir};
use crate::transport::{AgentReply, HeartbeatHandle};
use crate::{agent_fault_injection, agent_lifecycle};
use kyuubiki_protocol::{RpcMethod, RpcRequest, RpcResponse};
use serde::{Serialize, de::DeserializeOwned};

pub(crate) fn handle_operator_task_ir(
    request: RpcRequest,
    writer: Option<SharedReplyWriter>,
) -> AgentReply {
    let request_id = request.id.clone();
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
    let retained = crate::agent_task_results::begin(&request, guard.generation());
    let reply = execute_operator_task_ir(request, writer, guard);
    let AgentReply::Stream(_, response) = &reply;
    crate::agent_task_results::finish(retained, response);
    reply
}

fn execute_operator_task_ir(
    request: RpcRequest,
    writer: Option<SharedReplyWriter>,
    guard: agent_lifecycle::ExecutionGuard,
) -> AgentReply {
    let request_id = request.id;
    let maybe_job_id = extract_job_id(&request.params);
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
        return operator_task_failure_reply(
            request_id,
            guard,
            heartbeat,
            OperatorTaskRuntimeError::with_task(
                "cancelled",
                "execution cancelled before computation",
                "before_execution",
                request.params.get("task_ir"),
            ),
            request.params.get("task_ir"),
        );
    }

    let result = match agent_fault_injection::with_solver_control(
        &guard,
        maybe_job_id.as_deref(),
        "run_operator_task_ir",
        || run_operator_task_ir(&request.params),
    ) {
        Ok(result) => result,
        Err(mut error) => {
            let control = guard.solver_control();
            if control.was_interrupted() {
                let stage = if control.last_checkpoint().is_some_and(|point| {
                    point.stage == kyuubiki_solver::solver_control::SolverStage::ResultDiagnostics
                }) {
                    "execute_diagnostics"
                } else {
                    "execute_solver"
                };
                error = OperatorTaskRuntimeError::with_task(
                    "cancelled",
                    error.message,
                    stage,
                    request.params.get("task_ir"),
                );
            }
            return operator_task_failure_reply(
                request_id,
                guard,
                heartbeat,
                error,
                request.params.get("task_ir"),
            );
        }
    };

    publish_operator_task_result(
        request_id,
        guard,
        heartbeat,
        writer.as_ref(),
        result,
        request.params.get("task_ir"),
    )
}

fn publish_operator_task_result(
    request_id: String,
    guard: agent_lifecycle::ExecutionGuard,
    heartbeat: Option<HeartbeatHandle>,
    writer: Option<&SharedReplyWriter>,
    result: serde_json::Value,
    task_ir: Option<&serde_json::Value>,
) -> AgentReply {
    // An in-flight heartbeat write may request cancellation while stop joins it.
    stop_heartbeat(heartbeat);
    if !guard.result_publication_allowed() {
        return operator_task_failure_reply(
            request_id,
            guard,
            None,
            OperatorTaskRuntimeError::with_task(
                "cancelled",
                "operator task was cancelled before result publication",
                "publish_result",
                task_ir,
            ),
            task_ir,
        );
    }

    agent_reply_writer::complete_execution(writer, guard);
    AgentReply::Stream(Vec::new(), RpcResponse::success(request_id, result))
}

fn operator_task_failure_reply(
    request_id: String,
    guard: agent_lifecycle::ExecutionGuard,
    heartbeat: Option<HeartbeatHandle>,
    error: OperatorTaskRuntimeError,
    task_ir: Option<&serde_json::Value>,
) -> AgentReply {
    stop_heartbeat(heartbeat);
    let control = guard.solver_control();
    let report = agent_lifecycle::fail_execution(guard, error.code, error.message);
    let receipt = bind_failure_report(&report, error.details, task_ir);
    let mut details = serde_json::to_value(&report).expect("failure report should serialize");
    details["solver_checkpoint"] = crate::agent_execution_control::checkpoint_json(&control);
    details["operator_task_failure_receipt"] = receipt;
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(request_id, report.reason_code, report.message, details),
    )
}

fn bind_failure_report(
    report: &crate::agent_watchdog::FailureReport,
    mut receipt: serde_json::Value,
    task_ir: Option<&serde_json::Value>,
) -> serde_json::Value {
    let stage = receipt["failure_stage"]
        .as_str()
        .unwrap_or("execute_operator_task");
    // A watchdog may have already recorded this generation's terminal reason.
    let bound = crate::operator_task_receipts::operator_task_failure_receipt(
        &report.reason_code,
        &report.message,
        stage,
        task_ir,
    );
    if let Some(fields) = receipt.as_object_mut() {
        fields.extend(bound.as_object().expect("failure receipt object").clone());
        receipt
    } else {
        bound
    }
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
    let request_id = request.id.clone();
    let method = rpc_method_name(&request.method);
    let maybe_job_id = extract_job_id(&request.params);
    let guard = match agent_reply_writer::begin_execution(
        writer.as_ref(),
        request_id.clone(),
        maybe_job_id,
        method,
    ) {
        Ok(guard) => guard,
        Err(error) => return execution_admission_error(request_id, error),
    };

    execute_solver(
        request,
        writer,
        guard,
        model_name,
        serialize_label,
        node_count,
        solver,
    )
}

fn execute_solver<Request, ResultValue, NodeCount, Solver>(
    request: RpcRequest,
    writer: Option<SharedReplyWriter>,
    guard: agent_lifecycle::ExecutionGuard,
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
        return cancelled_reply(
            request_id,
            guard,
            heartbeat,
            "execution cancelled before computation",
        );
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
                    report.reason_code.clone(),
                    report.message.clone(),
                    serde_json::to_value(report).expect("failure report should serialize"),
                ),
            );
        }
    };

    if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
        return cancelled_reply(
            request_id,
            guard,
            heartbeat,
            "execution cancelled before computation",
        );
    }
    match agent_fault_injection::with_solver_control(
        &guard,
        maybe_job_id.as_deref(),
        &method,
        || solver(&params),
    ) {
        Ok(result) => {
            if execution_cancelled(&guard, &request_id, maybe_job_id.as_deref()) {
                return cancelled_reply(request_id, guard, heartbeat, "job was cancelled");
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
                            report.reason_code.clone(),
                            report.message.clone(),
                            serde_json::to_value(report).expect("failure report should serialize"),
                        ),
                    );
                }
            };
            let progress_frames =
                build_progress_frames(model_name, &request_id, node_count(&params));
            stop_heartbeat(heartbeat);
            if !guard.result_publication_allowed() {
                return cancelled_reply(
                    request_id,
                    guard,
                    None,
                    "execution cancelled before result publication",
                );
            }
            agent_reply_writer::complete_execution(writer.as_ref(), guard);
            AgentReply::Stream(
                progress_frames,
                RpcResponse::success(request_id, encoded_result),
            )
        }
        Err(error) => {
            stop_heartbeat(heartbeat);
            let control = guard.solver_control();
            let code = if control.was_interrupted() {
                "cancelled"
            } else {
                "solve_failed"
            };
            let report = agent_lifecycle::fail_execution(guard, code, error);
            let reason_code = report.reason_code.clone();
            let message = report.message.clone();
            let mut details =
                serde_json::to_value(report).expect("failure report should serialize");
            details["solver_checkpoint"] =
                crate::agent_execution_control::checkpoint_json(&control);
            AgentReply::Stream(
                Vec::new(),
                RpcResponse::error_with_details(request_id, reason_code, message, details),
            )
        }
    }
}

fn execution_cancelled(
    guard: &agent_lifecycle::ExecutionGuard,
    _request_id: &str,
    _job_id: Option<&str>,
) -> bool {
    guard.cancellation_requested()
}

fn cancelled_reply(
    request_id: String,
    guard: agent_lifecycle::ExecutionGuard,
    heartbeat: Option<HeartbeatHandle>,
    message: &str,
) -> AgentReply {
    stop_heartbeat(heartbeat);
    let report = agent_lifecycle::fail_execution(guard, "cancelled", message);
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

#[cfg(test)]
#[path = "tests/operator_task_failure_reply.rs"]
mod tests;
