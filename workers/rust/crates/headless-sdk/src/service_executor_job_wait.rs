use crate::service_executor::{
    normalize_job_state_result, request_json_with_deadline, required_path_segment,
};
use crate::{HeadlessExecutorError, HeadlessExecutorOutcome};
use serde_json::{Value, json};
use std::thread;
use std::time::{Duration, Instant};

const TERMINAL_JOB_STATUSES: &[&str] = &["completed", "failed", "cancelled"];
const DEFAULT_INTERVAL_MS: u64 = 1_000;
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_TOTAL_TIMEOUT_MS: u64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResumePolicy {
    Fixed,
    ServerDeadline,
}

impl ResumePolicy {
    fn parse(payload: &Value) -> Result<Self, HeadlessExecutorError> {
        let mut selected = None;
        for key in ["resume_policy", "resumePolicy"] {
            let Some(value) = payload.get(key) else {
                continue;
            };
            let policy = match value
                .as_str()
                .ok_or_else(|| validation_error("resume_policy must be a string".into()))?
            {
                "fixed" | "none" => Self::Fixed,
                "server_deadline" => Self::ServerDeadline,
                other => {
                    return Err(validation_error(format!(
                        "unsupported resume_policy {other}; expected fixed or server_deadline"
                    )));
                }
            };
            if selected.is_some_and(|previous| previous != policy) {
                return Err(validation_error(
                    "conflicting resume_policy|resumePolicy aliases".into(),
                ));
            }
            selected = Some(policy);
        }
        Ok(selected.unwrap_or(Self::Fixed))
    }

    fn label(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::ServerDeadline => "server_deadline",
        }
    }
}

#[derive(Debug, Default)]
struct ServerWindow {
    phase: Option<String>,
    remaining_ms: Option<u64>,
    observed_at: Option<Instant>,
}

impl ServerWindow {
    fn deadline(&self) -> Option<Instant> {
        self.observed_at?
            .checked_add(Duration::from_millis(self.remaining_ms?))
    }

    fn remaining_at(&self, now: Instant) -> Self {
        Self {
            phase: self.phase.clone(),
            remaining_ms: self
                .deadline()
                .map(|deadline| duration_ms(deadline.saturating_duration_since(now)))
                .or(self.remaining_ms),
            observed_at: Some(now),
        }
    }
}

struct WaitOptions {
    interval_ms: u64,
    timeout_ms: u64,
    max_total_timeout_ms: u64,
    resume_policy: ResumePolicy,
}

impl WaitOptions {
    fn parse(payload: &Value) -> Result<Self, HeadlessExecutorError> {
        let interval_ms =
            wait_integer(payload, &["interval_ms", "intervalMs"], DEFAULT_INTERVAL_MS)?;
        let timeout_ms = wait_integer(payload, &["timeout_ms", "timeoutMs"], DEFAULT_TIMEOUT_MS)?;
        let max_total_timeout_ms = wait_integer(
            payload,
            &["max_total_timeout_ms", "maxTotalTimeoutMs"],
            timeout_ms,
        )?;
        validate_wait_budget(interval_ms, timeout_ms, max_total_timeout_ms)?;
        Ok(Self {
            interval_ms,
            timeout_ms,
            max_total_timeout_ms,
            resume_policy: ResumePolicy::parse(payload)?,
        })
    }
}

pub(crate) fn validate_job_wait_options(payload: &Value) -> Result<(), HeadlessExecutorError> {
    WaitOptions::parse(payload).map(|_| ())
}

fn wait_integer(
    payload: &Value,
    keys: &[&str],
    default: u64,
) -> Result<u64, HeadlessExecutorError> {
    let mut selected = None;
    for key in keys {
        let Some(value) = payload.get(*key) else {
            continue;
        };
        let parsed = value
            .as_u64()
            .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
            .ok_or_else(|| validation_error(format!("{key} must be a positive integer")))?;
        if selected.is_some_and(|previous| previous != parsed) {
            return Err(validation_error(format!(
                "conflicting wait timing aliases: {}",
                keys.join("|")
            )));
        }
        selected = Some(parsed);
    }
    Ok(selected.unwrap_or(default))
}

struct WaitProgress {
    started_at: Instant,
    hard_deadline: Instant,
    window_deadline: Instant,
    poll_attempts: u64,
    resume_count: u64,
    status: String,
    server_window: ServerWindow,
}

impl WaitProgress {
    fn timeout_error(&self, job_id: &str, policy: ResumePolicy) -> HeadlessExecutorError {
        let now = Instant::now();
        wait_timeout_error(
            job_id,
            &self.status,
            policy,
            self.poll_attempts,
            self.resume_count,
            now.duration_since(self.started_at),
            now >= self.hard_deadline,
            &self.server_window.remaining_at(now),
        )
    }

    fn advance_window(
        &mut self,
        job_id: &str,
        options: &WaitOptions,
    ) -> Result<(), HeadlessExecutorError> {
        let now = Instant::now();
        if now < self.window_deadline && now < self.hard_deadline {
            return Ok(());
        }
        let server_deadline = self
            .server_window
            .deadline()
            .filter(|deadline| *deadline > now);
        if options.resume_policy != ResumePolicy::ServerDeadline || now >= self.hard_deadline {
            return Err(self.timeout_error(job_id, options.resume_policy));
        }
        let Some(server_deadline) = server_deadline else {
            return Err(self.timeout_error(job_id, options.resume_policy));
        };
        self.resume_count += 1;
        self.window_deadline = (now + Duration::from_millis(options.timeout_ms))
            .min(self.hard_deadline)
            .min(server_deadline);
        Ok(())
    }

    fn request_deadline(&self, policy: ResumePolicy) -> Instant {
        // A previously observed server deadline may authorize an in-flight read
        // across a soft window, but can never extend the caller's total budget.
        let allowed = if policy == ResumePolicy::ServerDeadline {
            self.server_window
                .deadline()
                .unwrap_or(self.window_deadline)
                .max(self.window_deadline)
        } else {
            self.window_deadline
        };
        allowed.min(self.hard_deadline)
    }
}

pub(crate) fn execute_job_wait(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_path_segment(payload, &["job_id", "jobId"])?;
    let options = WaitOptions::parse(payload)?;
    let started_at = Instant::now();
    let mut progress = WaitProgress {
        started_at,
        hard_deadline: started_at + Duration::from_millis(options.max_total_timeout_ms),
        window_deadline: started_at + Duration::from_millis(options.timeout_ms),
        poll_attempts: 0,
        resume_count: 0,
        status: "unknown".into(),
        server_window: ServerWindow::default(),
    };

    loop {
        progress.advance_window(job_id, &options)?;
        let request_deadline = progress.request_deadline(options.resume_policy);
        let request_started_at = Instant::now();
        progress.poll_attempts += 1;
        let response = request_json_with_deadline(
            base_url,
            api_token,
            "GET",
            &format!("/api/v1/jobs/{job_id}/status"),
            None,
            Some(request_deadline),
        );
        if Instant::now() >= request_deadline {
            return Err(progress.timeout_error(job_id, options.resume_policy));
        }
        let result = response.map_err(|error| HeadlessExecutorError {
            message: format!("failed while waiting for job {job_id}: {}", error.message),
        })?;
        let mut normalized = normalize_job_state_result(result);
        if Instant::now() >= request_deadline {
            return Err(progress.timeout_error(job_id, options.resume_policy));
        }
        let status = normalized
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if TERMINAL_JOB_STATUSES.contains(&status) {
            progress.advance_window(job_id, &options)?;
            reject_unsuccessful_terminal_job(job_id, &normalized)?;
            attach_wait_metadata(
                &mut normalized,
                options.resume_policy,
                progress.poll_attempts,
                progress.resume_count,
                started_at.elapsed(),
            );
            return Ok(HeadlessExecutorOutcome {
                status: "executed".to_string(),
                result: normalized,
            });
        }

        progress.status = status.to_string();
        progress.server_window = server_window(&normalized);
        progress.server_window.observed_at = Some(request_started_at);
        progress.advance_window(job_id, &options)?;
        let sleep = Duration::from_millis(options.interval_ms).min(
            progress
                .window_deadline
                .min(progress.hard_deadline)
                .saturating_duration_since(Instant::now()),
        );
        if !sleep.is_zero() {
            thread::sleep(sleep);
        }
    }
}

pub(crate) fn reject_unsuccessful_terminal_job(
    job_id: &str,
    job: &Value,
) -> Result<(), HeadlessExecutorError> {
    let status = job
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if status == "completed" {
        return Ok(());
    }
    let detail = job
        .get("job")
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("service job did not complete successfully");
    Err(HeadlessExecutorError {
        message: format!("service job {job_id} reached terminal status {status}: {detail}"),
    })
}

fn validate_wait_budget(
    interval_ms: u64,
    timeout_ms: u64,
    max_total_timeout_ms: u64,
) -> Result<(), HeadlessExecutorError> {
    if interval_ms == 0 || timeout_ms == 0 || max_total_timeout_ms == 0 {
        return Err(validation_error(
            "wait timing values must be positive integers".to_string(),
        ));
    }
    if timeout_ms > max_total_timeout_ms {
        return Err(validation_error(
            "timeout_ms must not exceed max_total_timeout_ms".to_string(),
        ));
    }
    if max_total_timeout_ms > MAX_TOTAL_TIMEOUT_MS {
        return Err(validation_error(format!(
            "max_total_timeout_ms must not exceed {MAX_TOTAL_TIMEOUT_MS}"
        )));
    }
    Ok(())
}

fn server_window(result: &Value) -> ServerWindow {
    let Some(timing) = result.pointer("/job/status_detail/timing") else {
        return ServerWindow::default();
    };
    let phase = timing
        .get("phase")
        .and_then(Value::as_str)
        .map(str::to_string);
    let elapsed_ms = match phase.as_deref() {
        Some("queue") => timing.get("queue_wait_ms").and_then(Value::as_u64),
        Some("execution") => timing.get("execution_elapsed_ms").and_then(Value::as_u64),
        _ => None,
    };
    let remaining_ms = timing
        .get("effective_timeout_ms")
        .and_then(Value::as_u64)
        .zip(elapsed_ms)
        .map(|(limit, elapsed)| limit.saturating_sub(elapsed));
    ServerWindow {
        phase,
        remaining_ms,
        observed_at: None,
    }
}

fn attach_wait_metadata(
    result: &mut Value,
    policy: ResumePolicy,
    poll_attempts: u64,
    resume_count: u64,
    elapsed: Duration,
) {
    let Some(object) = result.as_object_mut() else {
        return;
    };
    object.insert(
        "wait".to_string(),
        json!({
            "policy": policy.label(),
            "poll_attempts": poll_attempts,
            "resume_count": resume_count,
            "elapsed_ms": duration_ms(elapsed),
        }),
    );
}

#[allow(clippy::too_many_arguments)]
fn wait_timeout_error(
    job_id: &str,
    status: &str,
    policy: ResumePolicy,
    poll_attempts: u64,
    resume_count: u64,
    elapsed: Duration,
    total_budget_exhausted: bool,
    server_window: &ServerWindow,
) -> HeadlessExecutorError {
    let reason = if total_budget_exhausted {
        "client_total_budget_exhausted"
    } else if policy == ResumePolicy::ServerDeadline && server_window.remaining_ms == Some(0) {
        "server_deadline_exhausted"
    } else if policy == ResumePolicy::ServerDeadline {
        "server_timing_unavailable"
    } else {
        "client_window_exhausted"
    };
    HeadlessExecutorError {
        message: format!(
            "timed out waiting for job {job_id}; timeout_reason={reason}; last_status={status}; wait_policy={}; poll_attempts={poll_attempts}; resume_count={resume_count}; elapsed_ms={}; server_phase={}; server_remaining_ms={}",
            policy.label(),
            duration_ms(elapsed),
            server_window.phase.as_deref().unwrap_or("unknown"),
            server_window
                .remaining_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ),
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

fn validation_error(message: String) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("job_wait validation failed: {message}"),
    }
}

#[cfg(test)]
#[path = "service_executor_job_wait_tests.rs"]
mod deadline_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn server_deadline_policy_resumes_same_job_until_completion() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let port = listener.local_addr().expect("test address").port();
        let handle = std::thread::spawn(move || {
            for (index, status) in ["running", "running", "completed"].into_iter().enumerate() {
                let (mut stream, _) = listener.accept().expect("accept status request");
                let mut request = [0_u8; 4096];
                let bytes = stream.read(&mut request).expect("read status request");
                assert!(
                    String::from_utf8_lossy(&request[..bytes])
                        .starts_with("GET /api/v1/jobs/job-long/status HTTP/1.1")
                );
                if index == 1 {
                    std::thread::sleep(Duration::from_millis(150));
                }
                let body = json!({
                    "job": {
                        "job_id": "job-long",
                        "status": status,
                        "progress": index as f64 / 2.0,
                        "status_detail": {"timing": {
                            "phase": "execution",
                            "effective_timeout_ms": 10_000,
                            "execution_elapsed_ms": index * 150
                        }}
                    }
                })
                .to_string();
                write_response(&mut stream, &body);
            }
        });

        let outcome = execute_job_wait(
            &format!("http://127.0.0.1:{port}"),
            None,
            &json!({
                "job_id": "job-long",
                "interval_ms": 150,
                "timeout_ms": 100,
                "resume_policy": "server_deadline",
                "max_total_timeout_ms": 1_200
            }),
        )
        .expect("server deadline policy should resume polling");

        handle.join().expect("test server should finish");
        assert_eq!(outcome.result["status"], "completed");
        assert_eq!(outcome.result["wait"]["policy"], "server_deadline");
        assert_eq!(outcome.result["wait"]["poll_attempts"], 3);
        assert!(outcome.result["wait"]["resume_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn server_deadline_policy_fails_closed_without_server_timing() {
        let window = server_window(&json!({"job": {"status": "running"}}));
        let error = wait_timeout_error(
            "job-unknown",
            "running",
            ResumePolicy::ServerDeadline,
            2,
            0,
            Duration::from_millis(10),
            false,
            &window,
        );
        assert!(
            error
                .message
                .contains("timeout_reason=server_timing_unavailable")
        );
    }

    #[test]
    fn rejects_unbounded_or_inverted_wait_budgets() {
        assert!(validate_wait_budget(0, 10, 10).is_err());
        assert!(validate_wait_budget(1, 20, 10).is_err());
        assert!(validate_wait_budget(1, 10, MAX_TOTAL_TIMEOUT_MS + 1).is_err());
    }

    fn write_response(stream: &mut impl Write, body: &str) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write status response");
    }
}
