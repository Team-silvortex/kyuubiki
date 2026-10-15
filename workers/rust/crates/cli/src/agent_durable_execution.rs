use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use kyuubiki_protocol::{DurableAgentExecutionQuery, DurableAgentTask, RpcRequest, RpcResponse};
use serde_json::{Value, json};

use crate::agent_execution_store::{self, Admission, AdmissionFailure, Record, Store};
use crate::agent_reply_writer::SharedReplyWriter;
use crate::rpc_execution_context::{
    DeferredTaskReceipt, DurableTransportOwner, observe_committed_response,
};
use crate::transport::AgentReply;

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static OWNERS: OnceLock<Mutex<HashMap<String, crate::agent_lifecycle::ExecutionGuard>>> =
    OnceLock::new();
static RECEIPT_HOLD: OnceLock<PathBuf> = OnceLock::new();
const HOLD_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_RECEIPT_HOLD_FILE";
static START_FAILURE: OnceLock<String> = OnceLock::new();
const START_FAILURE_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_START_FAILURE_EXECUTION_ID";
static WORKER_PANIC: OnceLock<String> = OnceLock::new();
const PANIC_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_DURABLE_PANIC_EXECUTION_ID";
const PANIC_CODE: &str = "durable_execution_panicked";
const PANIC_MESSAGE: &str =
    "durable worker panicked; result discarded; inspect diagnostics before a new attempt";

pub(crate) fn configure() -> Result<(), String> {
    if let Some(id) = std::env::var_os(PANIC_ENV) {
        let id = id
            .into_string()
            .map_err(|_| "invalid worker panic identity")?;
        kyuubiki_protocol::validate_durable_execution_id(&id)?;
        WORKER_PANIC
            .set(id)
            .map_err(|_| "worker panic already configured")?;
    }
    if let Some(id) = std::env::var_os(START_FAILURE_ENV) {
        let id = id
            .into_string()
            .map_err(|_| "invalid start failure identity")?;
        kyuubiki_protocol::validate_durable_execution_id(&id)?;
        START_FAILURE
            .set(id)
            .map_err(|_| "start failure already configured")?;
    }
    if let Some(path) = std::env::var_os(HOLD_ENV) {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(format!("{HOLD_ENV} must be an absolute path"));
        }
        RECEIPT_HOLD
            .set(path)
            .map_err(|_| "receipt hold already configured")?;
    }
    if let Some(path) = std::env::var_os(agent_execution_store::STORE_ENV) {
        let store = Store::open(&PathBuf::from(path))?;
        STORE
            .set(Mutex::new(store))
            .map_err(|_| "execution store already configured")?;
    }
    Ok(())
}

pub(crate) fn policy() -> Value {
    let mut policy = match STORE.get() {
        Some(store) => match store.lock() {
            Ok(store) => store.policy(),
            Err(_) => {
                json!({"enabled":true, "available":false, "automatic_replay_authorized":false})
            }
        },
        None => agent_execution_store::policy(false),
    };
    policy["receipt_fault_injection"] = json!({"enabled":RECEIPT_HOLD.get().is_some(),
        "configuration_env":HOLD_ENV, "phase":"after_receipt_sync_before_transport",
        "scope":"exact_execution_id", "max_hold_seconds":120});
    policy["start_fault_injection"] = json!({"enabled":START_FAILURE.get().is_some(),
        "configuration_env":START_FAILURE_ENV, "phase":"before_worker_spawn",
        "scope":"exact_execution_id"});
    policy["panic_fault_injection"] = json!({"enabled":WORKER_PANIC.get().is_some(),
        "configuration_env":PANIC_ENV, "phase":"after_dispatch_before_receipt",
        "scope":"exact_execution_id"});
    policy["async_submission"] = json!({"method":"submit_durable_agent_task",
        "admission_boundary":"execution_slot_reserved_and_admission_synced",
        "release_boundary":"receipt_commit_finished",
        "rejection_schema":kyuubiki_protocol::DURABLE_AGENT_REJECTION_SCHEMA,
        "reservation_rejection":"synced_failed_receipt_without_engine_dispatch",
        "reservation_rejection_code":"durable_admission_rejected",
        "worker_start_failure":"synced_failed_receipt_before_engine_dispatch",
        "worker_start_failure_code":"durable_execution_start_failed",
        "worker_panic":"synced_failed_receipt_after_rust_unwind",
        "worker_panic_code":PANIC_CODE, "panic_rollback_guaranteed":false,
        "capacity_rejection":"transient_decision_without_eviction",
        "disconnect_cancels_execution":false, "queued_execution":false});
    policy["recovery_controls"] = json!({"list_method":"list_durable_agent_executions",
        "cancel_method":"cancel_durable_agent_execution", "max_page_entries":64,
        "index_includes_results":false, "cancellation_scope":"exact_async_owner",
        "cancellation_is_terminal_confirmation":false});
    policy["task_result_cache"] = json!({"publication_boundary":"durable_receipt_commit",
        "response_source":"committed_receipt", "commit_failure":"result_not_retained",
        "survives_agent_restart":false, "duplicate_rebuilds_cache":false});
    policy["synchronous_execution"] = json!({"method":"run_durable_agent_task",
        "completion_boundary":"receipt_commit_then_transport_delivery",
        "commit_failure_code":"durable_receipt_commit_failed",
        "unwind_diagnostic_code":"durable_execution_aborted",
        "unwind_commits_failure_receipt":false});
    policy
}

fn with_store<T>(f: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
    let store = STORE
        .get()
        .ok_or("durable execution requires KYUUBIKI_AGENT_EXECUTION_STORE; no memory fallback")?;
    let mut store = store.lock().map_err(|_| "execution store is unavailable")?;
    f(&mut store)
}

pub(crate) fn handle_run(request: RpcRequest, writer: Option<SharedReplyWriter>) -> AgentReply {
    let result = run(request.params, &request.id, writer);
    reply(request.id, result)
}

pub(crate) fn handle_submit(request: RpcRequest) -> AgentReply {
    reply(request.id.clone(), submit(request))
}

fn submit(request: RpcRequest) -> Result<Value, String> {
    let task: DurableAgentTask =
        serde_json::from_value(request.params).map_err(|e| e.to_string())?;
    task.validate()?;
    let process = crate::agent_lifecycle::snapshot().process_instance_id;
    let decision = with_store(|store| {
        Ok(store.admit_with(&task, &process, &request.id, || {
            let method = serde_json::to_value(&task.method).map_err(|e| e.to_string())?;
            crate::agent_lifecycle::begin_execution(
                request.id.clone(),
                crate::agent_state::extract_job_id(&task.params),
                method.as_str().ok_or("invalid method")?.into(),
            )
            .map(|guard| DurableLease {
                guard: Some(guard),
                id: None,
            })
            .map_err(|e| format!("{}: {}", e.reason_code, e.message))
        }))
    })?;
    let (admission, lease) = match decision {
        Ok(admitted) => admitted,
        Err(AdmissionFailure::Rejected(report)) => return Ok(report),
        Err(AdmissionFailure::Uncertain(error)) => return Err(error),
    };
    let record = match admission {
        Admission::Existing(report) => {
            if matches!(
                report["response"]["error"]["code"].as_str(),
                Some("durable_admission_rejected" | "durable_execution_start_failed")
            ) {
                wait_after_receipt(&task.execution_id);
            }
            return Ok(report);
        }
        Admission::New(record) => record,
    };
    let active = ActiveExecution(record.execution_id.clone());
    PendingExecution {
        record,
        task,
        active,
        lease: lease.ok_or("missing durable execution reservation")?,
    }
    .start()
}

struct PendingExecution {
    record: Record,
    task: DurableAgentTask,
    active: ActiveExecution,
    lease: DurableLease,
}

impl PendingExecution {
    fn prepare(&mut self) -> Result<Value, String> {
        let report = with_store(|store| store.lookup(&self.record.execution_id))?;
        let guard = self
            .lease
            .guard
            .as_ref()
            .ok_or("missing execution guard")?
            .clone();
        owners()
            .lock()
            .map_err(|_| "durable owner registry unavailable")?
            .insert(self.record.execution_id.clone(), guard);
        self.lease.id = Some(self.record.execution_id.clone());
        Ok(report)
    }

    fn start(mut self) -> Result<Value, String> {
        let report = match self.prepare() {
            Ok(report) => report,
            Err(error) => return self.fail_before_start(error),
        };
        // Keep the admission and slot in the caller until spawn succeeds. A failed spawn
        // drops its closure; capturing the owner there would abandon it before receipt sync.
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Self>(1);
        let spawned = if START_FAILURE.get() == Some(&self.record.execution_id) {
            Err(std::io::Error::other(
                "injected durable worker start failure",
            ))
        } else {
            std::thread::Builder::new()
                .name("agent-durable-execution".into())
                .spawn(move || {
                    if let Ok(work) = receiver.recv() {
                        work.execute();
                    }
                })
        };
        if let Err(error) = spawned {
            return self
                .fail_before_start(format!("durable execution worker unavailable: {error}"));
        }
        if let Err(error) = sender.send(self) {
            // Send failure returns ownership and proves that the worker did not receive it.
            return error
                .0
                .fail_before_start("durable worker ended before ownership transfer".into());
        }
        Ok(report)
    }

    fn fail_before_start(mut self, message: String) -> Result<Value, String> {
        let response = RpcResponse::error_with_details(
            self.record.request_id.clone(),
            "durable_execution_start_failed",
            message.clone(),
            json!({"phase":"before_engine_dispatch", "execution_started":false,
                "automatic_replay_authorized":false}),
        );
        let report = with_store(|store| store.finish(&self.record, response))?;
        wait_after_receipt(&self.record.execution_id);
        if let Some(guard) = self.lease.guard.take() {
            crate::agent_lifecycle::fail_execution(
                guard,
                "durable_execution_start_failed",
                message,
            );
        }
        Ok(report)
    }

    fn execute(self) {
        let Self {
            record,
            task,
            active: _active,
            mut lease,
        } = self;
        let inner = RpcRequest {
            rpc_version: kyuubiki_protocol::RPC_VERSION,
            id: record.request_id.clone(),
            method: task.method,
            params: task.params,
        };
        // No socket is retained: a lost admission response cannot cancel this computation.
        let guard = lease.guard.as_ref().expect("reserved execution").clone();
        let retained = DeferredTaskReceipt::default();
        let (response, panicked) = catch_worker_response(&record.request_id, || {
            let reply = crate::rpc::handle_reserved_request(inner, guard.clone(), &retained);
            if WORKER_PANIC.get() == Some(&record.execution_id) {
                panic!("injected durable worker panic after dispatch");
            }
            reply
        });
        if panicked {
            // The retained lease still owns capacity while the failure receipt is committed.
            crate::agent_lifecycle::fail_execution(guard.clone(), PANIC_CODE, PANIC_MESSAGE);
        }
        drop(guard);
        match with_store(|store| store.finish(&record, response)) {
            Ok(report) => {
                observe_committed_response(
                    lease.guard.as_ref().expect("reserved execution"),
                    &report["response"],
                );
                crate::agent_task_results::finish(retained.take(), &report["response"]);
                wait_after_receipt(&record.execution_id);
                crate::agent_lifecycle::complete_execution(
                    lease.guard.take().expect("reserved execution"),
                );
            }
            Err(error) => eprintln!("durable receipt commit failed: {error}"),
        }
    }
}

fn catch_worker_response(
    request_id: &str,
    dispatch: impl FnOnce() -> AgentReply,
) -> (RpcResponse, bool) {
    // Only request execution is caught: ledger locks/commits stay outside this boundary.
    // Discard request-local state on unwind; never bypass a poisoned shared lock or retry.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(dispatch)) {
        Ok(AgentReply::Stream(_, response)) => (response, false),
        Err(_) => (
            RpcResponse::error_with_details(
                request_id,
                PANIC_CODE,
                PANIC_MESSAGE,
                json!({"phase":"async_worker_unwind", "execution_may_have_started":true,
                "partial_result_discarded":true, "external_effects_rolled_back":false,
                "automatic_replay_authorized":false}),
            ),
            true,
        ),
    }
}

fn owners() -> &'static Mutex<HashMap<String, crate::agent_lifecycle::ExecutionGuard>> {
    OWNERS.get_or_init(|| Mutex::new(HashMap::new()))
}

struct DurableLease {
    guard: Option<crate::agent_lifecycle::ExecutionGuard>,
    id: Option<String>,
}

impl Drop for DurableLease {
    fn drop(&mut self) {
        if let Some(id) = &self.id {
            if let Ok(mut owners) = owners().lock() {
                owners.remove(id);
            }
        }
        if let Some(guard) = self.guard.take() {
            crate::agent_lifecycle::fail_execution(
                guard,
                "durable_execution_aborted",
                "durable execution owner ended before receipt commit completed",
            );
        }
    }
}

fn run(
    params: Value,
    request_id: &str,
    writer: Option<SharedReplyWriter>,
) -> Result<Value, String> {
    let task: DurableAgentTask = serde_json::from_value(params).map_err(|e| e.to_string())?;
    task.validate()?;
    let process = crate::agent_lifecycle::snapshot().process_instance_id;
    let record = match with_store(|store| store.admit(&task, &process, request_id))? {
        Admission::Existing(report) => return Ok(report),
        Admission::New(record) => record,
    };
    let _active = ActiveExecution(record.execution_id.clone());
    let inner = RpcRequest {
        rpc_version: kyuubiki_protocol::RPC_VERSION,
        id: record.request_id.clone(),
        method: task.method,
        params: task.params,
    };
    // The existing engine path keeps watchdog, cancellation and operator-package isolation.
    // Its final response is not delivered until this wrapper durably commits the receipt.
    let owner = DurableTransportOwner::new(writer);
    let retained = DeferredTaskReceipt::default();
    let AgentReply::Stream(_, response) =
        crate::rpc::handle_durable_request(inner, &owner, &retained);
    let report = match with_store(|store| store.finish(&record, response)) {
        Ok(report) => report,
        Err(error) => {
            owner.fail_commit();
            return Err(error);
        }
    };
    owner.observe_receipt(&report["response"]);
    crate::agent_task_results::finish(retained.take(), &report["response"]);
    wait_after_receipt(&record.execution_id);
    owner.complete();
    Ok(report)
}

fn wait_after_receipt(execution_id: &str) {
    let Some(path) = RECEIPT_HOLD.get() else {
        return;
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while std::time::Instant::now() < deadline
        && std::fs::read_to_string(path)
            .ok()
            .is_some_and(|value| value.trim() == execution_id)
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

pub(crate) fn handle_fetch(request: RpcRequest) -> AgentReply {
    let result = serde_json::from_value::<DurableAgentExecutionQuery>(request.params)
        .map_err(|e| e.to_string())
        .and_then(|query| {
            with_store(|store| {
                store.check_owner(query.expected_store_id.as_deref())?;
                store.lookup(&query.execution_id)
            })
        });
    reply(request.id, result)
}

pub(crate) fn handle_list(request: RpcRequest) -> AgentReply {
    let result =
        serde_json::from_value::<kyuubiki_protocol::DurableAgentExecutionList>(request.params)
            .map_err(|e| e.to_string())
            .and_then(|query| with_store(|store| store.list(&query)));
    reply(request.id, result)
}

pub(crate) fn handle_cancel(request: RpcRequest) -> AgentReply {
    let result = serde_json::from_value::<kyuubiki_protocol::DurableAgentExecutionCancel>(request.params)
        .map_err(|e| e.to_string()).and_then(|query| {
            query.validate()?;
            // Hold the receipt boundary while selecting the original live owner. Never resolve
            // a reused transport/job ID and never create a pending cancellation for future work.
            with_store(|store| {
                store.check_owner(query.expected_store_id.as_deref())?;
                let report = store.lookup(&query.execution_id)?;
                if report["process_instance_id"] != query.process_instance_id
                    || report["request_digest"] != query.request_digest {
                    return Err("durable cancellation identity mismatch; inspect the original execution".into());
                }
                let status = if report["status"] != "running" {
                    "not_running"
                } else {
                    let owners = owners().lock().map_err(|_| "durable owner registry unavailable")?;
                    if let Some(guard) = owners.get(&query.execution_id) {
                        guard.request_cancellation();
                        "requested"
                    } else { "owner_not_observed" }
                };
                Ok(json!({"schema_version":kyuubiki_protocol::DURABLE_AGENT_CANCELLATION_SCHEMA,
                    "store_id":report["store_id"],
                    "execution_id":query.execution_id, "process_instance_id":query.process_instance_id,
                    "request_digest":query.request_digest, "status":status,
                    "cancel_registered":status == "requested", "execution_terminal_confirmed":false,
                    "automatic_replay_authorized":false}))
            })
        });
    reply(request.id, result)
}

fn reply(id: String, result: Result<Value, String>) -> AgentReply {
    let response = match result {
        Ok(report) => RpcResponse::success(id, report),
        Err(message) => RpcResponse::error_with_details(
            id,
            "durable_execution_unavailable",
            message,
            json!({"automatic_replay_authorized":false, "required_action":"inspect_execution_by_original_id"}),
        ),
    };
    AgentReply::Stream(Vec::new(), response)
}

struct ActiveExecution(String);
impl Drop for ActiveExecution {
    fn drop(&mut self) {
        let _ = with_store(|store| {
            store.abandon(&self.0);
            Ok(())
        });
    }
}

#[cfg(test)]
mod panic_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn worker_unwind_discards_local_state_and_never_exposes_panic_payload() {
        struct LocalState<'a>(&'a Cell<bool>);
        impl Drop for LocalState<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let dropped = Cell::new(false);
        let (response, panicked) = catch_worker_response("original", || {
            let _local = LocalState(&dropped);
            panic!("private input or credential must not enter the receipt");
        });
        assert!(panicked && dropped.get());
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["id"], "original");
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], PANIC_CODE);
        assert_eq!(
            value["error"]["details"]["execution_may_have_started"],
            true
        );
        assert_eq!(
            value["error"]["details"]["automatic_replay_authorized"],
            false
        );
        assert!(value["error"]["details"].get("execution_started").is_none());
        assert!(!value.to_string().contains("credential"));
    }

    #[test]
    fn worker_boundary_preserves_both_success_and_ordinary_failure() {
        for original in [
            RpcResponse::success("original", json!({"value":2})),
            RpcResponse::error("original", "invalid_params", "invalid input"),
        ] {
            let expected = serde_json::to_value(&original).unwrap();
            let (actual, panicked) =
                catch_worker_response("original", || AgentReply::Stream(Vec::new(), original));
            assert!(!panicked);
            assert_eq!(serde_json::to_value(actual).unwrap(), expected);
        }
    }
}
