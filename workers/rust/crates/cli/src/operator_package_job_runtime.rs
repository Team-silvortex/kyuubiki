use kyuubiki_protocol::{
    CancelJobRequest, ReleaseOperatorPackageJobRequest, RpcRequest, RpcResponse,
};
use serde_json::{Value, json};

use crate::agent_execution_control::CancelRegistrationError;
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
    registered_cancellation_reply(
        request_id,
        &params.job_id,
        register_cancel(params.job_id.clone()),
        || release_orchestra_operator_job(&params.job_id),
    )
}

fn registered_cancellation_reply(
    request_id: String,
    job_id: &str,
    registration: Result<(), CancelRegistrationError>,
    cleanup: impl FnOnce() -> Result<Value, ExternalOperatorTaskError>,
) -> AgentReply {
    if let Err(error) = registration {
        return AgentReply::Stream(
            Vec::new(),
            RpcResponse::error_with_details(
                request_id,
                error.code(),
                error.message(),
                json!({
                    "failure_stage":"register_cancellation", "job_id":job_id,
                    "cancel_registered":false, "pending_cancellation_created":false,
                    "execution_terminal_confirmed":false, "operator_package_cleanup_performed":false,
                    "automatic_replay_authorized":false
                }),
            ),
        );
    }
    cancellation_receipt(request_id, job_id, cleanup())
}

fn cancellation_receipt(
    request_id: String,
    job_id: &str,
    cleanup: Result<Value, ExternalOperatorTaskError>,
) -> AgentReply {
    let release = cleanup.unwrap_or_else(|error| release_failure_details(job_id, true, &error));
    success(
        request_id,
        json!({
            "schema_version": "kyuubiki.agent-job-cancellation/v1",
            "job_id": job_id, "cancel_registered": true,
            "cancelled": true, "execution_terminal_confirmed": false,
            "operator_package_job_release": release,
        }),
    )
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
    let details = release_failure_details(job_id, cancel_registered, &error);
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::error_with_details(request_id, error.code, error.message, details),
    )
}

fn release_failure_details(
    job_id: &str,
    cancel_registered: bool,
    error: &ExternalOperatorTaskError,
) -> Value {
    let mut details = json!({
        "schema_version": "kyuubiki.agent-operator-job-cache-release-failure/v1",
        "status": "failed", "failure_stage": error.stage,
        "job_id": job_id, "error_code": error.code, "message": error.message,
        "cancel_registered": cancel_registered,
        "execution_terminal_confirmed": false, "automatic_replay_authorized": false
    });
    // Only admission rejection proves that cleanup never started.
    if matches!(
        error.code,
        "operator_package_job_release_busy" | "operator_package_job_release_unavailable"
    ) {
        details["operator_package_cleanup_performed"] = json!(false);
        if error.code == "operator_package_job_release_busy" {
            details["status"] = json!("deferred");
            details["retry_method"] = json!("release_operator_package_job");
            details["required_action"] = json!("retry_cleanup_after_package_lifecycle_idle");
        } else {
            details["required_action"] = json!("explicit_stopped_inspection_and_restart");
        }
    }
    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_state::take_cancelled;
    use kyuubiki_protocol::RpcMethod;

    #[test]
    fn rejected_cancellation_never_claims_success_or_releases_job_packages() {
        for error in [
            CancelRegistrationError::PendingCapacity,
            CancelRegistrationError::Unavailable,
            CancelRegistrationError::InvalidJobId,
        ] {
            let AgentReply::Stream(frames, response) = registered_cancellation_reply(
                "rejected-cancel".into(),
                "preserved-job",
                Err(error),
                || panic!("rejected cancellation must not run cleanup"),
            );
            assert!(frames.is_empty());
            assert!(!response.ok);
            assert!(response.result.is_none());
            let rejection = response.error.unwrap();
            assert_eq!(rejection.code, error.code());
            let details = rejection.details.unwrap();
            for field in [
                "cancel_registered",
                "pending_cancellation_created",
                "execution_terminal_confirmed",
                "operator_package_cleanup_performed",
                "automatic_replay_authorized",
            ] {
                assert_eq!(details[field], false, "{field}");
            }
        }
    }

    #[test]
    fn accepted_cancellation_preserves_a_separate_cache_cleanup_failure() {
        let job_id = "cancel-cache-cleanup-fault";
        let control = crate::agent_execution_control::begin(
            "cancel-cache-cleanup-running".into(),
            1,
            Some(job_id.into()),
        )
        .unwrap();
        let AgentReply::Stream(_, response) = registered_cancellation_reply(
            "cancel-cache-cleanup".into(),
            job_id,
            register_cancel(job_id.into()),
            || {
                Err(ExternalOperatorTaskError {
                    code: "operator_package_activation_failed",
                    stage: "activate_operator_registry",
                    message: "injected cleanup failure".into(),
                })
            },
        );
        assert!(response.ok);
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["job_id"], job_id);
        assert_eq!(result["cancel_registered"], true);
        assert_eq!(result["execution_terminal_confirmed"], false);
        assert_eq!(result["operator_package_job_release"]["status"], "failed");
        assert_eq!(
            result["operator_package_job_release"]["error_code"],
            "operator_package_activation_failed"
        );
        assert!(control.solver.cancellation_requested());
        drop(control);
        let next = crate::agent_execution_control::begin(
            "cancel-cache-cleanup-rerun".into(),
            2,
            Some(job_id.into()),
        )
        .unwrap();
        assert!(!next.solver.cancellation_requested());
    }

    #[test]
    fn accepted_cancellation_does_not_claim_execution_is_terminal() {
        let cleanup = json!({"disposition":"already_released", "job_id":"cancel-ack"});
        let AgentReply::Stream(_, response) = cancellation_receipt(
            "cancel-ack-request".into(),
            "cancel-ack",
            Ok(cleanup.clone()),
        );
        let result = response.result.unwrap();
        assert!(response.ok);
        assert_eq!(
            result["schema_version"],
            "kyuubiki.agent-job-cancellation/v1"
        );
        assert_eq!(result["cancel_registered"], true);
        assert_eq!(result["execution_terminal_confirmed"], false);
        assert_eq!(result["operator_package_job_release"], cleanup);
    }

    #[test]
    fn busy_cleanup_receipt_authorizes_only_explicit_cleanup_retry() {
        for (code, status, retry) in [
            ("operator_package_job_release_busy", "deferred", true),
            ("operator_package_job_release_unavailable", "failed", false),
            ("operator_package_activation_failed", "failed", false),
        ] {
            let error = ExternalOperatorTaskError {
                code,
                stage: "injected",
                message: "injected release failure".into(),
            };
            let AgentReply::Stream(_, reply) =
                cancellation_receipt("cancel-busy".into(), "job", Err(error));
            assert!(reply.ok);
            let result = reply.result.unwrap();
            assert_eq!(result["cancel_registered"], true);
            let failure = &result["operator_package_job_release"];
            assert_eq!(failure["status"], status);
            assert_eq!(failure["cancel_registered"], true);
            assert_eq!(failure["execution_terminal_confirmed"], false);
            assert_eq!(failure["automatic_replay_authorized"], false);
            assert_eq!(failure["retry_method"].is_string(), retry);
            if code == "operator_package_activation_failed" {
                assert!(failure.get("operator_package_cleanup_performed").is_none());
            } else {
                assert_eq!(failure["operator_package_cleanup_performed"], false);
            }
        }
    }

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
