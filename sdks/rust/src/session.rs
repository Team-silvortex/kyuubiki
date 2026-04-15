use crate::{ControlPlaneClient, KyuubikiAuth, SdkError, SdkResult, SolverRpcClient};
use serde_json::Value;
use std::thread;
use std::time::{Duration, Instant};

pub struct KyuubikiSession {
    pub control_plane: Option<ControlPlaneClient>,
    pub solver_rpc: Option<SolverRpcClient>,
}

pub struct JobRequest {
    pub solve_kind: String,
    pub payload: Value,
}

pub struct JobWaitOutcome {
    pub terminal: Value,
    pub history: Vec<Value>,
}

impl KyuubikiSession {
    pub fn new(control_plane: Option<ControlPlaneClient>, solver_rpc: Option<SolverRpcClient>) -> Self {
        Self {
            control_plane,
            solver_rpc,
        }
    }

    pub fn from_control_plane(base_url: &str, token: Option<String>) -> SdkResult<Self> {
        Ok(Self {
            control_plane: Some(ControlPlaneClient::new_with_token(base_url, token)?),
            solver_rpc: None,
        })
    }

    pub fn from_control_plane_with_auth(base_url: &str, auth: Option<KyuubikiAuth>) -> SdkResult<Self> {
        Ok(Self {
            control_plane: Some(ControlPlaneClient::new_with_auth(base_url, auth)?),
            solver_rpc: None,
        })
    }

    pub fn with_solver_rpc(mut self, host: impl Into<String>, port: u16) -> Self {
        self.solver_rpc = Some(SolverRpcClient::new(host, port));
        self
    }

    pub fn submit_job(&self, solve_kind: &str, payload: &Value) -> SdkResult<Value> {
        let control_plane = self
            .control_plane
            .as_ref()
            .ok_or_else(|| SdkError::Transport("control plane client is not configured".into()))?;

        match normalize_kind(solve_kind) {
            "bar_1d" => control_plane.create_axial_bar_job(payload),
            "truss_2d" => control_plane.create_truss_2d_job(payload),
            "truss_3d" => control_plane.create_truss_3d_job(payload),
            "plane_triangle_2d" => control_plane.create_plane_triangle_2d_job(payload),
            _ => Err(SdkError::Rpc {
                message: format!("unsupported solve kind: {solve_kind}"),
                code: None,
            }),
        }
    }

    pub fn submit_jobs(&self, jobs: &[JobRequest]) -> SdkResult<Vec<Value>> {
        jobs.iter()
            .map(|job| self.submit_job(&job.solve_kind, &job.payload))
            .collect()
    }

    pub fn solve_direct(&self, solve_kind: &str, payload: Value) -> SdkResult<Value> {
        let solver_rpc = self
            .solver_rpc
            .as_ref()
            .ok_or_else(|| SdkError::Rpc {
                message: "solver rpc client is not configured".into(),
                code: None,
            })?;

        let outcome = match normalize_kind(solve_kind) {
            "bar_1d" => solver_rpc.solve_bar_1d(payload)?,
            "truss_2d" => solver_rpc.solve_truss_2d(payload)?,
            "truss_3d" => solver_rpc.solve_truss_3d(payload)?,
            "plane_triangle_2d" => solver_rpc.solve_plane_triangle_2d(payload)?,
            _ => {
                return Err(SdkError::Rpc {
                    message: format!("unsupported solve kind: {solve_kind}"),
                    code: None,
                })
            }
        };

        Ok(outcome.result)
    }

    pub fn wait_for_job(&self, job_id: &str, poll_interval: Duration, timeout: Duration) -> SdkResult<JobWaitOutcome> {
        let control_plane = self
            .control_plane
            .as_ref()
            .ok_or_else(|| SdkError::Transport("control plane client is not configured".into()))?;

        let deadline = Instant::now() + timeout;
        let mut history = Vec::new();
        let mut last_status: Option<String> = None;
        let mut last_progress: Option<Value> = None;

        while Instant::now() <= deadline {
            let payload = control_plane.fetch_job(job_id)?;
            let job = payload.get("job").and_then(Value::as_object);
            let status = job
                .and_then(|job| job.get("status"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let progress = job.and_then(|job| job.get("progress")).cloned();

            if status != last_status || progress != last_progress {
                history.push(payload.clone());
                last_status = status.clone();
                last_progress = progress;
            }

            match status.as_deref() {
                Some("completed" | "failed" | "cancelled") => {
                    return Ok(JobWaitOutcome {
                        terminal: payload,
                        history,
                    })
                }
                _ => thread::sleep(poll_interval),
            }
        }

        Err(SdkError::Timeout(format!("timed out waiting for job {job_id}")))
    }

    pub fn submit_and_wait(&self, solve_kind: &str, payload: &Value, poll_interval: Duration, timeout: Duration) -> SdkResult<JobWaitOutcome> {
        let submitted = self.submit_job(solve_kind, payload)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?;

        self.wait_for_job(job_id, poll_interval, timeout)
    }
}

fn normalize_kind(kind: &str) -> &str {
    match kind {
        "bar_1d" => "bar_1d",
        "truss_2d" => "truss_2d",
        "truss_3d" => "truss_3d",
        "plane_triangle_2d" => "plane_triangle_2d",
        other => other,
    }
}
