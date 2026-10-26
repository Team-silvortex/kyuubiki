use crate::{
    SdkResult, SolverRpcClient,
    durable_execution::{hex, invalid},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Admission identity only, not a terminal receipt, input document or result.
/// Inspect the original execution and bind it to retained input before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableExecutionIndexEntry {
    pub schema_version: String,
    pub execution_id: String,
    pub request_digest: String,
    pub request_id: String,
    pub process_instance_id: String,
    pub method: String,
}

/// One bounded, read-only index observation. Pages are not a transactionally
/// consistent snapshot: concurrent admissions may require a later explicit scan.
/// Absence never authorizes replay; store identity is not authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableExecutionIndexPage {
    pub store_id: String,
    pub executions: Vec<DurableExecutionIndexEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema_version: String,
    store_id: String,
    automatic_replay_authorized: bool,
    executions: Vec<DurableExecutionIndexEntry>,
    next_cursor: Option<String>,
}

impl DurableExecutionIndexPage {
    /// Validate a v1 index against the exact caller-owned store, cursor and page
    /// bound. This checks consistency, not authenticity or execution completion.
    pub fn from_report(
        report: &Value,
        store_id: &str,
        after: Option<&str>,
        limit: usize,
    ) -> SdkResult<Self> {
        validate_query(store_id, after, limit)?;
        let entries = report["executions"]
            .as_array()
            .ok_or_else(|| invalid("durable index requires an executions array"))?;
        if entries.len() > limit || report.get("next_cursor").is_none() {
            return Err(invalid(
                "durable index exceeds page bound or omits its cursor state",
            ));
        }
        let index = Index::deserialize(report)?;
        if index.schema_version != "kyuubiki.agent-durable-execution-index/v1"
            || index.store_id != store_id
            || index.automatic_replay_authorized
        {
            return Err(invalid("durable index identity or replay policy mismatch"));
        }
        let mut previous = after;
        for entry in &index.executions {
            if entry.schema_version != "kyuubiki.agent-durable-execution/v1" {
                return Err(invalid("unsupported durable index admission schema"));
            }
            hex(&entry.execution_id, 32)?;
            hex(&entry.request_digest, 64)?;
            for value in [&entry.request_id, &entry.process_instance_id, &entry.method] {
                if value.trim().is_empty()
                    || value.len() > 256
                    || value.chars().any(char::is_control)
                {
                    return Err(invalid("invalid durable index admission identity"));
                }
            }
            if previous.is_some_and(|id| entry.execution_id.as_str() <= id) {
                return Err(invalid(
                    "durable index entries must strictly advance past the cursor",
                ));
            }
            previous = Some(&entry.execution_id);
        }
        if let Some(next) = &index.next_cursor {
            hex(next, 32)?;
            if index.executions.len() != limit || Some(next.as_str()) != previous {
                return Err(invalid(
                    "durable index continuation must name the last full-page entry",
                ));
            }
        }
        Ok(Self {
            store_id: index.store_id,
            executions: index.executions,
            next_cursor: index.next_cursor,
        })
    }
}

impl SolverRpcClient {
    /// Read exactly one page from the pinned Agent store, with a limit of 1..=64.
    /// Does not fetch receipts, follow cursors, submit, cancel or retry tasks.
    /// Retain the original store ID across pages; do not replace it from discovery.
    pub fn list_durable_agent_executions(
        &self,
        store_id: &str,
        after: Option<&str>,
        limit: usize,
    ) -> SdkResult<DurableExecutionIndexPage> {
        validate_query(store_id, after, limit)?;
        let report = self
            .call(
                "list_durable_agent_executions",
                json!({"expected_store_id":store_id, "after":after, "limit":limit}),
            )?
            .result;
        DurableExecutionIndexPage::from_report(&report, store_id, after, limit)
    }
}

fn validate_query(store_id: &str, after: Option<&str>, limit: usize) -> SdkResult<()> {
    hex(store_id, 32)?;
    if let Some(id) = after {
        hex(id, 32)?;
    }
    if !(1..=64).contains(&limit) {
        return Err(invalid("durable index page limit must be between 1 and 64"));
    }
    Ok(())
}
