use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use kyuubiki_protocol::{DurableAgentExecutionQuery, DurableAgentTask, RpcRequest, RpcResponse};
use serde_json::{Value, json};

use crate::agent_execution_store::{self, Admission, Store};
use crate::agent_reply_writer::SharedReplyWriter;
use crate::transport::AgentReply;

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static OWNERS: OnceLock<Mutex<HashMap<String, crate::agent_lifecycle::ExecutionGuard>>> =
    OnceLock::new();
static RECEIPT_HOLD: OnceLock<PathBuf> = OnceLock::new();
const HOLD_ENV: &str = "KYUUBIKI_AGENT_FAULT_INJECTION_RECEIPT_HOLD_FILE";

pub(crate) fn configure() -> Result<(), String> {
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
    policy["async_submission"] = json!({"method":"submit_durable_agent_task",
        "admission_boundary":"execution_slot_reserved_and_admission_synced",
        "release_boundary":"receipt_commit_finished",
        "disconnect_cancels_execution":false, "queued_execution":false});
    policy["recovery_controls"] = json!({"list_method":"list_durable_agent_executions",
        "cancel_method":"cancel_durable_agent_execution", "max_page_entries":64,
        "index_includes_results":false, "cancellation_scope":"exact_async_owner",
        "cancellation_is_terminal_confirmation":false});
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
    let (admission, lease) = with_store(|store| {
        store.admit_with(&task, &process, &request.id, || {
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
        })
    })?;
    let record = match admission {
        Admission::Existing(report) => return Ok(report),
        Admission::New(record) => record,
    };
    let active = ActiveExecution(record.execution_id.clone());
    let report = with_store(|store| store.lookup(&record.execution_id))?;
    let mut lease = lease.ok_or("missing durable execution reservation")?;
    owners()
        .lock()
        .map_err(|_| "durable owner registry unavailable")?
        .insert(
            record.execution_id.clone(),
            lease
                .guard
                .as_ref()
                .ok_or("missing execution guard")?
                .clone(),
        );
    lease.id = Some(record.execution_id.clone());
    std::thread::Builder::new()
        .name("agent-durable-execution".into())
        .spawn(move || {
            let _active = active;
            let mut lease = lease;
            let inner = RpcRequest {
                rpc_version: kyuubiki_protocol::RPC_VERSION,
                id: record.request_id.clone(),
                method: task.method,
                params: task.params,
            };
            // No socket is retained: a lost admission response cannot cancel this computation.
            let AgentReply::Stream(_, response) = crate::rpc::handle_reserved_request(
                inner,
                lease.guard.as_ref().expect("reserved execution").clone(),
            );
            match with_store(|store| store.finish(&record, response)) {
                Ok(_) => {
                    wait_after_receipt(&record.execution_id);
                    crate::agent_lifecycle::complete_execution(
                        lease.guard.take().expect("reserved execution"),
                    );
                }
                Err(error) => eprintln!("durable receipt commit failed: {error}"),
            }
        })
        .map_err(|e| format!("durable execution worker unavailable: {e}"))?;
    Ok(report)
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
    let AgentReply::Stream(_, response) = crate::rpc::handle_request(inner, writer);
    let report = with_store(|store| store.finish(&record, response))?;
    wait_after_receipt(&record.execution_id);
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
        .and_then(|query| with_store(|store| store.lookup(&query.execution_id)));
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
