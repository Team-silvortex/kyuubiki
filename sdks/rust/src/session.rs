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
    pub fn new(
        control_plane: Option<ControlPlaneClient>,
        solver_rpc: Option<SolverRpcClient>,
    ) -> Self {
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

    pub fn from_control_plane_with_auth(
        base_url: &str,
        auth: Option<KyuubikiAuth>,
    ) -> SdkResult<Self> {
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
        control_plane.submit_fem_job(solve_kind, payload)
    }

    pub fn submit_workflow_catalog_job(
        &self,
        workflow_id: &str,
        input_artifacts: &Value,
    ) -> SdkResult<Value> {
        let control_plane = self
            .control_plane
            .as_ref()
            .ok_or_else(|| SdkError::Transport("control plane client is not configured".into()))?;
        control_plane.submit_workflow_catalog_job(workflow_id, input_artifacts)
    }

    pub fn submit_workflow_graph_job(
        &self,
        graph: &Value,
        input_artifacts: &Value,
    ) -> SdkResult<Value> {
        let control_plane = self
            .control_plane
            .as_ref()
            .ok_or_else(|| SdkError::Transport("control plane client is not configured".into()))?;
        control_plane.submit_workflow_graph_job(graph, input_artifacts)
    }

    pub fn submit_jobs(&self, jobs: &[JobRequest]) -> SdkResult<Vec<Value>> {
        jobs.iter()
            .map(|job| self.submit_job(&job.solve_kind, &job.payload))
            .collect()
    }

    pub fn solve_direct(&self, solve_kind: &str, payload: Value) -> SdkResult<Value> {
        let solver_rpc = self.solver_rpc.as_ref().ok_or_else(|| SdkError::Rpc {
            message: "solver rpc client is not configured".into(),
            code: None,
        })?;
        let outcome = solver_rpc.solve_study(solve_kind, payload)?;
        Ok(outcome.result)
    }

    pub fn wait_for_job(
        &self,
        job_id: &str,
        poll_interval: Duration,
        timeout: Duration,
    ) -> SdkResult<JobWaitOutcome> {
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
                    });
                }
                _ => thread::sleep(poll_interval),
            }
        }

        Err(SdkError::Timeout(format!(
            "timed out waiting for job {job_id}"
        )))
    }

    pub fn submit_and_wait(
        &self,
        solve_kind: &str,
        payload: &Value,
        poll_interval: Duration,
        timeout: Duration,
    ) -> SdkResult<JobWaitOutcome> {
        let submitted = self.submit_job(solve_kind, payload)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?;

        self.wait_for_job(job_id, poll_interval, timeout)
    }

    pub fn submit_workflow_catalog_and_wait(
        &self,
        workflow_id: &str,
        input_artifacts: &Value,
        poll_interval: Duration,
        timeout: Duration,
    ) -> SdkResult<JobWaitOutcome> {
        let submitted = self.submit_workflow_catalog_job(workflow_id, input_artifacts)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?;
        self.wait_for_job(job_id, poll_interval, timeout)
    }

    pub fn submit_workflow_graph_and_wait(
        &self,
        graph: &Value,
        input_artifacts: &Value,
        poll_interval: Duration,
        timeout: Duration,
    ) -> SdkResult<JobWaitOutcome> {
        let submitted = self.submit_workflow_graph_job(graph, input_artifacts)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?;
        self.wait_for_job(job_id, poll_interval, timeout)
    }
}
