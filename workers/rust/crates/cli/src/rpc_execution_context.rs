use crate::agent_lifecycle::{ExecutionAdmissionError, ExecutionGuard};
use crate::agent_reply_writer::{self, SharedReplyWriter};

pub(crate) enum ExecutionContext {
    Transport(Option<SharedReplyWriter>),
    Durable(ExecutionGuard),
}

impl ExecutionContext {
    pub(crate) fn writer(&self) -> Option<SharedReplyWriter> {
        match self {
            Self::Transport(writer) => writer.clone(),
            Self::Durable(_) => None,
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
            Self::Durable(guard) => Ok(guard.clone()),
        }
    }

    pub(crate) fn complete(&self, guard: ExecutionGuard) {
        match self {
            Self::Transport(writer) => {
                agent_reply_writer::complete_execution(writer.as_ref(), guard)
            }
            // The durable owner retains capacity through receipt commit, not transport delivery.
            Self::Durable(_) => drop(guard),
        }
    }
}
