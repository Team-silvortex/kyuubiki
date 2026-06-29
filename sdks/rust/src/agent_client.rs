use crate::{
    KyuubikiSession, SdkError, SdkResult, WorkflowOutputManifest, WorkflowProgression,
    WorkflowRuntimeSnapshot, WorkflowValidatedArtifacts, build_workflow_output_manifest,
    normalize_workflow_progression, normalize_workflow_runtime,
    validate_workflow_result_against_graph,
};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailureClass {
    Timeout,
    Transport,
    Auth,
    NotFound,
    Server,
    Http,
    Rpc,
    Failed,
    Cancelled,
    Pending,
    Completed,
    Unknown,
}

pub struct RetryPolicy {
    pub max_attempts: usize,
    pub retry_on: Vec<FailureClass>,
    pub backoff: Duration,
    pub backoff_multiplier: f32,
}

pub struct StudyRunOutcome {
    pub submitted: Value,
    pub terminal: Value,
    pub history: Vec<Value>,
    pub result: Option<Value>,
    pub workflow_runtime: Option<WorkflowRuntimeSnapshot>,
    pub workflow_progression: Option<WorkflowProgression>,
    pub output_manifest: Option<WorkflowOutputManifest>,
    pub validated_outputs: Option<WorkflowValidatedArtifacts>,
}

pub struct RetriedStudyRunOutcome {
    pub outcome: StudyRunOutcome,
    pub attempt_count: usize,
    pub attempts: Vec<(usize, FailureClass, String)>,
}

pub struct KyuubikiAgentClient {
    pub session: KyuubikiSession,
}

impl KyuubikiAgentClient {
    pub fn new(session: KyuubikiSession) -> Self {
        Self { session }
    }

    pub fn run_study(
        &self,
        solve_kind: &str,
        payload: &Value,
        poll_interval: Duration,
        timeout: Duration,
        include_result: bool,
    ) -> SdkResult<StudyRunOutcome> {
        let submitted = self.session.submit_job(solve_kind, payload)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?
            .to_string();
        let waited = self.session.wait_for_job(&job_id, poll_interval, timeout)?;
        self.build_run_outcome(submitted, &job_id, waited, include_result, None)
    }

    pub fn run_workflow_catalog(
        &self,
        workflow_id: &str,
        input_artifacts: &Value,
        workflow_graph: Option<&Value>,
        poll_interval: Duration,
        timeout: Duration,
        include_result: bool,
    ) -> SdkResult<StudyRunOutcome> {
        let fetched_graph = if workflow_graph.is_none() {
            self.fetch_workflow_catalog_graph(workflow_id)?
        } else {
            None
        };
        let resolved_graph = workflow_graph.or(fetched_graph.as_ref());
        let submitted = self
            .session
            .submit_workflow_catalog_job(workflow_id, input_artifacts)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?
            .to_string();
        let waited = self.session.wait_for_job(&job_id, poll_interval, timeout)?;
        self.build_run_outcome(submitted, &job_id, waited, include_result, resolved_graph)
    }

    pub fn run_workflow_graph(
        &self,
        graph: &Value,
        input_artifacts: &Value,
        poll_interval: Duration,
        timeout: Duration,
        include_result: bool,
    ) -> SdkResult<StudyRunOutcome> {
        let submitted = self
            .session
            .submit_workflow_graph_job(graph, input_artifacts)?;
        let job_id = submitted
            .get("job")
            .and_then(|job| job.get("job_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| SdkError::Transport("submit response did not include job_id".into()))?
            .to_string();
        let waited = self.session.wait_for_job(&job_id, poll_interval, timeout)?;
        self.build_run_outcome(submitted, &job_id, waited, include_result, Some(graph))
    }

    fn build_run_outcome(
        &self,
        submitted: Value,
        job_id: &str,
        waited: crate::session::JobWaitOutcome,
        include_result: bool,
        workflow_graph: Option<&Value>,
    ) -> SdkResult<StudyRunOutcome> {
        let result = if include_result {
            match waited
                .terminal
                .get("job")
                .and_then(|job| job.get("status"))
                .and_then(Value::as_str)
            {
                Some("completed") => self.fetch_result_optional(job_id)?,
                _ => None,
            }
        } else {
            None
        };
        let workflow_runtime = match result.as_ref() {
            Some(result) => Some(normalize_workflow_runtime(result)?),
            None => None,
        };
        let workflow_progression = Some(normalize_workflow_progression(
            &waited.history,
            result.as_ref(),
        )?);
        let (output_manifest, validated_outputs) = match (workflow_graph, result.as_ref()) {
            (Some(graph), Some(result)) => {
                let graph = serde_json::from_value(graph.clone())?;
                let manifest = build_workflow_output_manifest(&graph)?;
                let validated = validate_workflow_result_against_graph(&graph, result)?;
                (Some(manifest), Some(validated))
            }
            _ => (None, None),
        };

        Ok(StudyRunOutcome {
            submitted,
            terminal: waited.terminal,
            history: waited.history,
            result,
            workflow_runtime,
            workflow_progression,
            output_manifest,
            validated_outputs,
        })
    }

    pub fn fetch_job_bundle(
        &self,
        job_id: &str,
        include_result: bool,
    ) -> SdkResult<(Value, Option<Value>)> {
        let control_plane =
            self.session.control_plane.as_ref().ok_or_else(|| {
                SdkError::Transport("control plane client is not configured".into())
            })?;
        let job = control_plane.fetch_job(job_id)?;
        let result = if include_result {
            self.fetch_result_optional(job_id)?
        } else {
            None
        };
        Ok((job, result))
    }

    pub fn browse_result_chunks(
        &self,
        job_id: &str,
        kind: &str,
        offset: usize,
        limit: usize,
    ) -> SdkResult<Value> {
        let control_plane =
            self.session.control_plane.as_ref().ok_or_else(|| {
                SdkError::Transport("control plane client is not configured".into())
            })?;
        control_plane.fetch_result_chunk(job_id, kind, Some(offset), Some(limit))
    }

    pub fn iter_result_chunks<'a>(
        &'a self,
        job_id: impl Into<String>,
        kind: impl Into<String>,
        page_size: usize,
        start_offset: usize,
        max_pages: Option<usize>,
    ) -> ResultChunkIter<'a> {
        ResultChunkIter {
            client: self,
            job_id: job_id.into(),
            kind: kind.into(),
            offset: start_offset,
            limit: page_size,
            max_pages,
            pages: 0,
            done: false,
        }
    }

    pub fn run_study_with_retry(
        &self,
        solve_kind: &str,
        payload: &Value,
        poll_interval: Duration,
        timeout: Duration,
        include_result: bool,
        policy: &RetryPolicy,
    ) -> SdkResult<RetriedStudyRunOutcome> {
        let mut attempts = Vec::new();
        let mut backoff = policy.backoff;

        for attempt in 1..=policy.max_attempts {
            match self.run_study(solve_kind, payload, poll_interval, timeout, include_result) {
                Ok(outcome) => {
                    return Ok(RetriedStudyRunOutcome {
                        outcome,
                        attempt_count: attempt,
                        attempts,
                    });
                }
                Err(error) => {
                    let class = Self::classify_error(&error);
                    let message = error.to_string();
                    let retryable = policy.retry_on.contains(&class);
                    attempts.push((attempt, class.clone(), message));
                    if attempt >= policy.max_attempts || !retryable {
                        return Err(error);
                    }
                    std::thread::sleep(backoff);
                    backoff =
                        Duration::from_secs_f32(backoff.as_secs_f32() * policy.backoff_multiplier);
                }
            }
        }

        Err(SdkError::Transport("retry loop exited unexpectedly".into()))
    }

    pub fn classify_error(error: &SdkError) -> FailureClass {
        match error {
            SdkError::Timeout(_) => FailureClass::Timeout,
            SdkError::Transport(_) | SdkError::Io(_) => FailureClass::Transport,
            SdkError::Rpc { .. } => FailureClass::Rpc,
            SdkError::HttpStatus {
                status_code: 401 | 403,
                ..
            } => FailureClass::Auth,
            SdkError::HttpStatus {
                status_code: 404, ..
            } => FailureClass::NotFound,
            SdkError::HttpStatus { status_code, .. } if *status_code >= 500 => FailureClass::Server,
            SdkError::HttpStatus { .. } => FailureClass::Http,
            _ => FailureClass::Unknown,
        }
    }

    pub fn classify_terminal(terminal: &Value) -> FailureClass {
        match terminal
            .get("job")
            .and_then(|job| job.get("status"))
            .and_then(Value::as_str)
        {
            Some("completed") => FailureClass::Completed,
            Some("failed") => FailureClass::Failed,
            Some("cancelled") => FailureClass::Cancelled,
            Some(_) => FailureClass::Pending,
            None => FailureClass::Unknown,
        }
    }

    fn fetch_result_optional(&self, job_id: &str) -> SdkResult<Option<Value>> {
        let control_plane = match self.session.control_plane.as_ref() {
            Some(client) => client,
            None => return Ok(None),
        };

        match control_plane.fetch_result(job_id) {
            Ok(value) => Ok(Some(value)),
            Err(SdkError::HttpStatus {
                status_code: 404, ..
            }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn fetch_workflow_catalog_graph(&self, workflow_id: &str) -> SdkResult<Option<Value>> {
        let control_plane = match self.session.control_plane.as_ref() {
            Some(client) => client,
            None => return Ok(None),
        };
        let descriptor = control_plane.fetch_workflow_catalog_workflow(workflow_id)?;
        Ok(descriptor
            .get("workflow")
            .and_then(|workflow| workflow.get("graph"))
            .cloned())
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            retry_on: vec![FailureClass::Timeout, FailureClass::Transport],
            backoff: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        }
    }
}

pub struct ResultChunkIter<'a> {
    client: &'a KyuubikiAgentClient,
    job_id: String,
    kind: String,
    offset: usize,
    limit: usize,
    max_pages: Option<usize>,
    pages: usize,
    done: bool,
}

impl<'a> Iterator for ResultChunkIter<'a> {
    type Item = SdkResult<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if let Some(max_pages) = self.max_pages {
            if self.pages >= max_pages {
                self.done = true;
                return None;
            }
        }

        let page =
            self.client
                .browse_result_chunks(&self.job_id, &self.kind, self.offset, self.limit);

        match page {
            Ok(payload) => {
                let returned =
                    payload.get("returned").and_then(Value::as_u64).unwrap_or(0) as usize;
                let total = payload.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
                self.pages += 1;
                if returned == 0 || self.offset + returned >= total {
                    self.done = true;
                } else {
                    self.offset += returned;
                }
                Some(Ok(payload))
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}
