use crate::{
    DurableExecutionHandle, SdkResult,
    durable_execution::{hex, invalid},
    durable_execution_digest::request_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SCHEMA: &str = "kyuubiki.sdk-durable-execution-intent/v1";

/// Retain before submission together with the exact input and caller-owned endpoint.
/// This binds content and ledger identity; it is not authentication or retry authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableExecutionIntent {
    pub schema_version: String,
    pub store_id: String,
    pub execution_id: String,
    pub method: String,
    pub request_digest: String,
}

impl DurableExecutionIntent {
    pub fn new(
        store_id: &str,
        execution_id: &str,
        method: &str,
        params: &Value,
    ) -> SdkResult<Self> {
        hex(store_id, 32)?;
        hex(execution_id, 32)?;
        Ok(Self {
            schema_version: SCHEMA.into(),
            store_id: store_id.into(),
            execution_id: execution_id.into(),
            method: method.into(),
            request_digest: request_digest(method, params)?,
        })
    }

    pub(crate) fn validate(&self) -> SdkResult<()> {
        hex(&self.store_id, 32)?;
        hex(&self.execution_id, 32)?;
        hex(&self.request_digest, 64)?;
        if self.schema_version != SCHEMA
            || self.method.len() > 256
            || !(self.method.starts_with("solve_") || self.method == "run_operator_task_ir")
        {
            return Err(invalid("unsupported durable execution intent"));
        }
        Ok(())
    }

    pub fn validate_input(&self, params: &Value) -> SdkResult<()> {
        self.validate()?;
        if request_digest(&self.method, params)? != self.request_digest {
            return Err(invalid(
                "durable input changed since preparation; no dispatch authorized",
            ));
        }
        Ok(())
    }

    /// Reconcile an observed receipt even if the admission response was lost.
    /// Running/unknown/failed observations do not become completed results.
    pub fn handle_from_report(&self, report: &Value) -> SdkResult<DurableExecutionHandle> {
        self.validate()?;
        let handle = DurableExecutionHandle::from_report(report)?;
        if handle.store_id != self.store_id
            || handle.execution_id != self.execution_id
            || handle.method != self.method
            || handle.request_digest != self.request_digest
        {
            return Err(invalid(
                "durable receipt does not match the prepared input or identity; no replay authorized",
            ));
        }
        Ok(handle)
    }
}
