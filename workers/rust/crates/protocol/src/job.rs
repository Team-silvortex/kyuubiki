use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Preprocessing,
    Partitioning,
    Solving,
    Postprocessing,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preprocessing => "preprocessing",
            Self::Partitioning => "partitioning",
            Self::Solving => "solving",
            Self::Postprocessing => "postprocessing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub project_id: String,
    pub simulation_case_id: String,
    pub status: JobStatus,
    pub progress: f32,
    pub residual: Option<f64>,
    pub iteration: Option<u64>,
    pub worker_id: Option<String>,
}

impl Job {
    pub fn new(
        job_id: impl Into<String>,
        project_id: impl Into<String>,
        simulation_case_id: impl Into<String>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            project_id: project_id.into(),
            simulation_case_id: simulation_case_id.into(),
            status: JobStatus::Queued,
            progress: 0.0,
            residual: None,
            iteration: None,
            worker_id: None,
        }
    }

    pub fn apply_progress(&mut self, event: &ProgressEvent) {
        self.status = event.stage;
        self.progress = event.progress;
        self.residual = event.residual;
        self.iteration = event.iteration;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub job_id: String,
    pub stage: JobStatus,
    pub progress: f32,
    pub residual: Option<f64>,
    pub iteration: Option<u64>,
    pub peak_memory: Option<u64>,
    pub message: Option<String>,
}

impl ProgressEvent {
    pub fn new(job_id: impl Into<String>, stage: JobStatus, progress: f32) -> Self {
        Self {
            job_id: job_id.into(),
            stage,
            progress,
            residual: None,
            iteration: None,
            peak_memory: None,
            message: None,
        }
    }
}
