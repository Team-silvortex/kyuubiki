//! Durable Agent recovery is a read of the original execution, never a retry policy.
use crate::durable_execution_digest::{result_digest, terminal_digest};
use crate::{DurableExecutionIntent, SdkError, SdkResult, SolverRpcClient};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const SCHEMA: &str = "kyuubiki.agent-durable-execution/v1";

/// Persist this handle with the submitted input. Store identity is not authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableExecutionHandle {
    pub store_id: String,
    pub execution_id: String,
    pub request_digest: String,
    pub process_instance_id: String,
    pub request_id: String,
    pub method: String,
}

impl DurableExecutionHandle {
    /// Bind to an observed admission/terminal receipt, not an unverified input document.
    pub fn from_report(report: &Value) -> SdkResult<Self> {
        validate_report(report)?;
        let handle = Self {
            store_id: text(report, "store_id")?.into(),
            execution_id: text(report, "execution_id")?.into(),
            request_digest: text(report, "request_digest")?.into(),
            process_instance_id: text(report, "process_instance_id")?.into(),
            request_id: text(report, "request_id")?.into(),
            method: text(report, "method")?.into(),
        };
        handle.validate()?;
        Ok(handle)
    }

    fn validate(&self) -> SdkResult<()> {
        hex(&self.store_id, 32)?;
        hex(&self.execution_id, 32)?;
        hex(&self.request_digest, 64)?;
        for value in [&self.process_instance_id, &self.request_id, &self.method] {
            if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err(invalid("missing or oversized execution identity"));
            }
        }
        Ok(())
    }

    pub fn validate_report(&self, report: &Value) -> SdkResult<()> {
        self.validate()?;
        if Self::from_report(report)? != *self {
            return Err(invalid(
                "durable execution ownership changed; no replay authorized",
            ));
        }
        Ok(())
    }

    /// Reconcile a caller-retained observation with a later one of this execution.
    /// A retained completed/failed outcome cannot change or lose its payload.
    /// Nonterminal observations make no terminal commitment; this does not retry,
    /// authenticate records or persist history for the caller.
    pub fn validate_followup(&self, retained: &Value, observed: &Value) -> SdkResult<()> {
        self.validate_report(retained)?;
        let commitment = terminal_commitment(retained)?;
        self.validate_report(observed)?;
        validate_terminal_commitment(commitment, observed)
    }

    /// Bind a separately retained/decoded result to this execution's completed
    /// receipt. Preserves numeric types, binary64 bits and signed zero while
    /// ignoring object-key order. This is consistency, not physical validation,
    /// authentication or verification of the original file bytes.
    pub fn validate_completed_result(&self, report: &Value, result: &Value) -> SdkResult<()> {
        let original = self.completed_result(report)?;
        if result_digest(original)? != result_digest(result)? {
            return Err(invalid(
                "result does not match the original completed receipt",
            ));
        }
        Ok(())
    }

    /// An outer successful RPC only confirms observation; it is not solver success.
    pub fn completed_result<'a>(&self, report: &'a Value) -> SdkResult<&'a Value> {
        self.validate_report(report)?;
        if report["status"] == "failed" {
            let failure = &report["response"]["error"];
            return Err(SdkError::Rpc {
                code: Some(text(failure, "code")?.into()),
                message: text(failure, "message")?.into(),
                request_id: Some(self.request_id.clone()),
                details: failure
                    .get("details")
                    .filter(|value| !value.is_null())
                    .cloned(),
            });
        }
        if report["status"] != "completed" {
            return Err(invalid(
                "execution is not completed; no result or replay authorized",
            ));
        }
        Ok(&report["response"]["result"])
    }
}

impl SolverRpcClient {
    /// Submit once using a caller-retained ID and a previously observed store identity.
    /// On transport loss inspect that ID; do not create a new ID automatically.
    pub fn submit_durable_agent_task(
        &self,
        store_id: &str,
        execution_id: &str,
        method: &str,
        params: Value,
    ) -> SdkResult<Value> {
        let intent = DurableExecutionIntent::new(store_id, execution_id, method, &params)?;
        self.dispatch_durable_task(&intent, params)
    }

    /// Validate unchanged input against a caller-persisted intent before sending once.
    pub fn submit_prepared_durable_agent_task(
        &self,
        intent: &DurableExecutionIntent,
        params: Value,
    ) -> SdkResult<Value> {
        intent.validate_input(&params)?;
        self.dispatch_durable_task(intent, params)
    }

    fn dispatch_durable_task(
        &self,
        intent: &DurableExecutionIntent,
        params: Value,
    ) -> SdkResult<Value> {
        let report = self
            .call(
                "submit_durable_agent_task",
                json!({
                    "expected_store_id":intent.store_id, "execution_id":intent.execution_id,
                    "method":intent.method, "params":params
                }),
            )?
            .result;
        intent.handle_from_report(&report)?;
        Ok(report)
    }

    /// Read only the original execution and verify the prepared content digest.
    /// `not_found` remains observable but never grants reexecution or result access.
    pub fn inspect_durable_agent_task(&self, intent: &DurableExecutionIntent) -> SdkResult<Value> {
        intent.validate()?;
        let report =
            self.inspect_durable_agent_execution(&intent.store_id, &intent.execution_id)?;
        if report["status"] != "not_found" {
            intent.handle_from_report(&report)?;
        }
        Ok(report)
    }

    /// Read-only lookup also works after a lost admission response. `not_found` is not
    /// permission to submit again; retained-ledger loss requires operator inspection.
    pub fn inspect_durable_agent_execution(
        &self,
        store_id: &str,
        execution_id: &str,
    ) -> SdkResult<Value> {
        hex(store_id, 32)?;
        hex(execution_id, 32)?;
        let report = self
            .call(
                "fetch_durable_agent_execution",
                json!({
                    "expected_store_id":store_id, "execution_id":execution_id
                }),
            )?
            .result;
        if report["schema_version"] != SCHEMA
            || report["store_id"] != store_id
            || report["execution_id"] != execution_id
            || report["automatic_replay_authorized"] != false
        {
            return Err(invalid(
                "durable inspection identity or replay policy mismatch",
            ));
        }
        if report["status"] == "not_found" {
            if !report["response"].is_null() {
                return Err(invalid("missing execution cannot publish a result"));
            }
        } else {
            DurableExecutionHandle::from_report(&report)?;
        }
        Ok(report)
    }

    pub fn fetch_durable_agent_execution(
        &self,
        handle: &DurableExecutionHandle,
    ) -> SdkResult<Value> {
        handle.validate()?;
        let report =
            self.inspect_durable_agent_execution(&handle.store_id, &handle.execution_id)?;
        handle.validate_report(&report)?;
        Ok(report)
    }

    /// Fetch once and reconcile against the caller's retained receipt, not only
    /// its identity handle. A missing/conflicting observation never causes replay.
    pub fn fetch_durable_agent_execution_after(&self, retained: &Value) -> SdkResult<Value> {
        let handle = DurableExecutionHandle::from_report(retained)?;
        // Reject unusable retained content before I/O; keep only its small digest
        // for comparison so large terminal payloads are not hashed twice.
        let commitment = terminal_commitment(retained)?;
        let observed = self.fetch_durable_agent_execution(&handle)?;
        validate_terminal_commitment(commitment, &observed)?;
        Ok(observed)
    }
}

// Callers have already validated receipt shape and execution identity.
fn terminal_commitment(report: &Value) -> SdkResult<Option<[u8; 32]>> {
    if matches!(report["status"].as_str(), Some("completed" | "failed")) {
        terminal_digest(report).map(Some)
    } else {
        Ok(None)
    }
}

fn validate_terminal_commitment(commitment: Option<[u8; 32]>, observed: &Value) -> SdkResult<()> {
    if commitment.is_some() && commitment != terminal_commitment(observed)? {
        return Err(invalid(
            "durable terminal observation changed; preserve original evidence; no replay authorized",
        ));
    }
    Ok(())
}

fn validate_report(report: &Value) -> SdkResult<()> {
    if report["schema_version"] != SCHEMA
        || report["automatic_replay_authorized"] != false
        || report["resumable"] != false
    {
        return Err(invalid("unsupported durable receipt or replay policy"));
    }
    let response = &report["response"];
    if report.get("response").is_none() {
        return Err(invalid(
            "durable receipt is missing its explicit response state",
        ));
    }
    match report["status"].as_str() {
        Some("running" | "outcome_unknown") if response.is_null() => Ok(()),
        Some("completed" | "failed") => {
            let success = report["status"] == "completed";
            if response["rpc_version"] != 1
                || response["id"] != report["request_id"]
                || response["ok"] != success
                || !response["event"].is_null()
                || !response["progress"].is_null()
                || (success && (response["result"].is_null() || !response["error"].is_null()))
                || (!success && (!response["result"].is_null() || !response["error"].is_object()))
            {
                return Err(invalid(
                    "durable terminal response does not match its receipt",
                ));
            }
            if !success
                && ["code", "message"].iter().any(|key| {
                    response["error"][*key]
                        .as_str()
                        .is_none_or(|value| value.trim().is_empty())
                })
            {
                return Err(invalid(
                    "durable failure requires a non-empty code and message",
                ));
            }
            Ok(())
        }
        _ => Err(invalid("unsupported durable status or premature result")),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> SdkResult<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| invalid("missing durable identity field"))
}

pub(crate) fn hex(value: &str, length: usize) -> SdkResult<()> {
    if value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(invalid(
            "durable identity must be fixed-length lowercase hex",
        ))
    }
}

pub(crate) fn invalid(message: &str) -> SdkError {
    SdkError::Transport(message.into())
}
