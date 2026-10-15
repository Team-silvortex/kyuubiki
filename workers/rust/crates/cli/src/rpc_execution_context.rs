use std::cell::RefCell;

use kyuubiki_protocol::RpcRequest;
use serde_json::Value;

use crate::agent_lifecycle::{ExecutionAdmissionError, ExecutionGuard};
use crate::agent_reply_writer::{self, SharedReplyWriter};
use crate::agent_task_results::{self, Reservation};

pub(crate) type DeferredTaskReceipt = RefCell<Option<Reservation>>;

pub(crate) fn observe_committed_response(guard: &ExecutionGuard, response: &Value) {
    if response["ok"] == false {
        crate::agent_lifecycle::fail_execution(
            guard.clone(),
            response["error"]["code"]
                .as_str()
                .unwrap_or("durable_execution_failed"),
            response["error"]["message"]
                .as_str()
                .unwrap_or("durable execution failed"),
        );
    }
}

pub(crate) struct DurableTransportOwner {
    writer: Option<SharedReplyWriter>,
    guard: RefCell<Option<ExecutionGuard>>,
}

impl DurableTransportOwner {
    pub(crate) fn new(writer: Option<SharedReplyWriter>) -> Self {
        Self {
            writer,
            guard: RefCell::new(None),
        }
    }

    fn begin(
        &self,
        request_id: String,
        job_id: Option<String>,
        method: String,
    ) -> Result<ExecutionGuard, ExecutionAdmissionError> {
        let guard =
            agent_reply_writer::begin_execution(self.writer.as_ref(), request_id, job_id, method)?;
        self.guard.replace(Some(guard.clone()));
        Ok(guard)
    }

    pub(crate) fn complete(mut self) {
        if let Some(guard) = self.guard.get_mut().take() {
            // Network ownership still extends through final response delivery.
            agent_reply_writer::complete_execution(self.writer.as_ref(), guard);
        }
    }

    pub(crate) fn observe_receipt(&self, response: &Value) {
        if let Some(guard) = self.guard.borrow().as_ref() {
            observe_committed_response(guard, response);
        }
    }

    pub(crate) fn fail_commit(mut self) {
        self.fail(
            "durable_receipt_commit_failed",
            "durable receipt commit failed; inspect the original execution before any new attempt",
        );
    }

    fn fail(&mut self, code: &str, message: &str) {
        if let Some(guard) = self.guard.get_mut().take() {
            crate::agent_lifecycle::fail_execution(guard, code, message);
        }
    }
}

impl Drop for DurableTransportOwner {
    fn drop(&mut self) {
        self.fail(
            "durable_execution_aborted",
            "synchronous durable execution owner ended before receipt commit completed",
        );
    }
}

pub(crate) enum ExecutionContext<'a> {
    Transport(Option<SharedReplyWriter>),
    Durable(ExecutionGuard, &'a DeferredTaskReceipt),
    DurableTransport(&'a DurableTransportOwner, &'a DeferredTaskReceipt),
}

impl ExecutionContext<'_> {
    pub(crate) fn writer(&self) -> Option<SharedReplyWriter> {
        match self {
            Self::Transport(writer) => writer.clone(),
            Self::DurableTransport(owner, _) => owner.writer.clone(),
            Self::Durable(_, _) => None,
        }
    }

    pub(crate) fn begin_task_retention(
        &self,
        request: &RpcRequest,
        generation: u64,
    ) -> Option<Reservation> {
        let reservation = agent_task_results::begin(request, generation);
        match self {
            Self::Transport(_) => reservation,
            Self::Durable(_, deferred) | Self::DurableTransport(_, deferred) => {
                // Only the durable owner may publish, after its receipt is committed.
                deferred.replace(reservation);
                None
            }
        }
    }

    pub(crate) fn begin(
        &self,
        request_id: String,
        job_id: Option<String>,
        method: String,
    ) -> Result<ExecutionGuard, ExecutionAdmissionError> {
        match self {
            Self::Transport(writer) => {
                agent_reply_writer::begin_execution(writer.as_ref(), request_id, job_id, method)
            }
            Self::DurableTransport(owner, _) => owner.begin(request_id, job_id, method),
            Self::Durable(guard, _) => Ok(guard.clone()),
        }
    }

    pub(crate) fn complete(&self, guard: ExecutionGuard) {
        match self {
            Self::Transport(writer) => {
                agent_reply_writer::complete_execution(writer.as_ref(), guard)
            }
            // Durable owners decide completion only after receipt commit.
            Self::Durable(_, _) | Self::DurableTransport(_, _) => drop(guard),
        }
    }
}

#[cfg(test)]
#[path = "tests/rpc_execution_context.rs"]
mod tests;
