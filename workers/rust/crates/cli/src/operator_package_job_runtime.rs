use kyuubiki_protocol::{
    CancelJobRequest, ReleaseOperatorPackageJobRequest, RpcRequest, RpcResponse,
};
use serde_json::{Value, json};

use crate::agent_state::register_cancel;
use crate::operator_package_fetch_runtime::{
    release_orchestra_operator_job, validate_operator_package_job_id,
};
use crate::operator_package_runtime::ExternalOperatorTaskError;
use crate::transport::AgentReply;

pub(crate) fn handle_release_job(request: RpcRequest) -> AgentReply {
    let request_id = request.id;
    let params = match serde_json::from_value::<ReleaseOperatorPackageJobRequest>(request.params) {
        Ok(params) => params,
        Err(error) => return invalid_params(request_id, error),
    };
    match release_orchestra_operator_job(&params.job_id) {
        Ok(receipt) => success(request_id, receipt),
        Err(error) => release_error(request_id, &params.job_id, false, error),
    }
}

pub(crate) fn handle_cancel_job(request: RpcRequest) -> AgentReply {
    let request_id = request.id;
    let params = match serde_json::from_value::<CancelJobRequest>(request.params) {
        Ok(params) => params,
        Err(error) => return invalid_params(request_id, error),
    };
    if let Err(error) = validate_operator_package_job_id(&params.job_id) {
        return release_error(request_id, &params.job_id, false, error);
    }
    register_cancel(params.job_id.clone());
    match release_orchestra_operator_job(&params.job_id) {
        Ok(receipt) => success(
            request_id,
            json!({
                "cancelled": true,
                "operator_package_job_release": receipt
            }),
        ),
        Err(error) => release_error(request_id, &params.job_id, true, error),
    }
}

fn success(request_id: String, result: Value) -> AgentReply {
    AgentReply::Stream(Vec::new(), RpcResponse::success(request_id, result))
}

fn invalid_params(request_id: String, error: serde_json::Error) -> AgentReply {
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error(request_id, "invalid_params", error.to_string()),
    )
}

fn release_error(
    request_id: String,
    job_id: &str,
    cancel_registered: bool,
    error: ExternalOperatorTaskError,
) -> AgentReply {
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(
            request_id,
            error.code,
            error.message,
            json!({
                "schema_version": "kyuubiki.agent-operator-job-cache-release-failure/v1",
                "failure_stage": error.stage,
                "job_id": job_id,
                "cancel_registered": cancel_registered
            }),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_state::take_cancelled;
    use kyuubiki_protocol::RpcMethod;

    #[test]
    fn invalid_job_identity_does_not_register_cancellation() {
        let job_id = "invalid\noperator-job";
        let _ = take_cancelled(job_id);
        let reply = handle_cancel_job(RpcRequest {
            rpc_version: 1,
            id: "cancel-invalid-operator-job".to_string(),
            method: RpcMethod::CancelJob,
            params: json!({"job_id": job_id}),
        });
        let AgentReply::Stream(_, response) = reply;
        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error response").code,
            "operator_package_job_id_invalid"
        );
        assert!(!take_cancelled(job_id));
    }
}
