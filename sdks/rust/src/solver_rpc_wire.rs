//! Standalone implementation of the Agent v1 envelope, not a solver dependency.
use crate::{SdkError, SdkResult, solver_rpc_json::UniqueValue};
use serde::Deserialize;
use serde_json::{Value, value::RawValue};

#[derive(Deserialize)]
struct Frame<'a> {
    rpc_version: u8,
    id: String,
    ok: Option<bool>,
    event: Option<String>,
    #[serde(borrow)]
    result: Option<&'a RawValue>,
    error: Option<Failure>,
    #[serde(borrow)]
    progress: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Failure {
    code: String,
    message: String,
    details: Option<UniqueValue>,
}

#[derive(Deserialize)]
struct Progress {
    job_id: String,
    stage: String,
    progress: f32,
    residual: Option<f64>,
    iteration: Option<u64>,
    peak_memory: Option<u64>,
    message: Option<String>,
}

pub(crate) enum Reply {
    Progress(Value),
    Complete(Value),
}

pub(crate) fn decode(bytes: &[u8], request_id: &str, progress_budget: usize) -> SdkResult<Reply> {
    // Typed fields reject duplicate envelope keys rather than silently choosing one.
    let frame: Frame<'_> = serde_json::from_slice(bytes)?;
    if frame.rpc_version != 1 || frame.id != request_id {
        return Err(invalid("RPC response version or request identity mismatch"));
    }
    if let Some(event) = frame.event {
        if !matches!(event.as_str(), "progress" | "heartbeat")
            || frame.ok.is_some()
            || frame.result.is_some()
            || frame.error.is_some()
        {
            return Err(invalid("invalid or mixed RPC progress envelope"));
        }
        if bytes.len() > progress_budget {
            return Err(invalid("RPC progress byte budget exceeded"));
        }
        let progress = frame
            .progress
            .ok_or_else(|| invalid("missing RPC progress payload"))?;
        let progress: Progress = serde_json::from_str(progress.get())?;
        validate_progress(&progress)?;
        // Retain the original diagnostic document, including future optional fields.
        return Ok(Reply::Progress(
            serde_json::from_slice::<UniqueValue>(bytes)?.0,
        ));
    }
    if frame.progress.is_some() {
        return Err(invalid("terminal RPC response contains a progress payload"));
    }
    match (frame.ok, frame.result, frame.error) {
        (Some(true), Some(result), None) => Ok(Reply::Complete(
            serde_json::from_str::<UniqueValue>(result.get())?.0,
        )),
        (Some(false), None, Some(error))
            if !error.code.trim().is_empty() && !error.message.trim().is_empty() =>
        {
            Err(SdkError::Rpc {
                code: Some(error.code),
                message: error.message,
                request_id: Some(frame.id),
                details: error.details.map(|details| details.0),
            })
        }
        _ => Err(invalid(
            "RPC response must contain exactly one valid result or error state",
        )),
    }
}

fn validate_progress(event: &Progress) -> SdkResult<()> {
    if event.job_id.trim().is_empty()
        || event.job_id.len() > 128
        || event.job_id.chars().any(char::is_control)
        || !matches!(
            event.stage.as_str(),
            "queued"
                | "preprocessing"
                | "partitioning"
                | "solving"
                | "postprocessing"
                | "completed"
                | "failed"
                | "cancelled"
        )
        || !event.progress.is_finite()
        || !(0.0..=1.0).contains(&event.progress)
        || (event.stage == "completed" && event.progress != 1.0)
        || event.residual.is_some_and(|v| !v.is_finite() || v < 0.0)
        || event.message.as_ref().is_some_and(|v| v.len() > 4096)
    {
        return Err(invalid("invalid RPC progress payload"));
    }
    // Deserialize the optional counters as integers, even though they are diagnostic.
    let _ = (event.iteration, event.peak_memory);
    Ok(())
}

pub(crate) fn invalid(message: &str) -> SdkError {
    SdkError::Transport(message.into())
}
