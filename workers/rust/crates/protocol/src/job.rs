use serde::{Deserialize, Serialize};

pub const JOB_ID_MAX_BYTES: usize = 128;
pub const PROGRESS_MESSAGE_MAX_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressValidationErrorCode {
    InvalidJobId,
    JobIdMismatch,
    InvalidProgress,
    InvalidResidual,
    InvalidMessage,
    ProgressRegression,
    StageRegression,
    TerminalJobMutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressValidationError {
    pub code: ProgressValidationErrorCode,
    pub message: String,
}

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

    fn active_rank(self) -> Option<u8> {
        match self {
            Self::Queued => Some(0),
            Self::Preprocessing => Some(1),
            Self::Partitioning => Some(2),
            Self::Solving => Some(3),
            Self::Postprocessing => Some(4),
            Self::Completed | Self::Failed | Self::Cancelled => None,
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

    pub fn apply_progress(&mut self, event: &ProgressEvent) -> Result<(), ProgressValidationError> {
        validate_progress_event(event)?;
        if event.job_id != self.job_id {
            return Err(progress_error(
                ProgressValidationErrorCode::JobIdMismatch,
                format!(
                    "progress event job {} does not match target job {}",
                    event.job_id, self.job_id
                ),
            ));
        }
        if matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        ) {
            return Err(progress_error(
                ProgressValidationErrorCode::TerminalJobMutation,
                format!("cannot apply progress to terminal job {}", self.job_id),
            ));
        }
        if event.progress < self.progress {
            return Err(progress_error(
                ProgressValidationErrorCode::ProgressRegression,
                format!(
                    "progress cannot move backwards from {} to {}",
                    self.progress, event.progress
                ),
            ));
        }
        if event
            .stage
            .active_rank()
            .zip(self.status.active_rank())
            .is_some_and(|(next_rank, current_rank)| next_rank < current_rank)
        {
            return Err(progress_error(
                ProgressValidationErrorCode::StageRegression,
                format!(
                    "job stage cannot move backwards from {} to {}",
                    self.status.as_str(),
                    event.stage.as_str()
                ),
            ));
        }

        self.status = event.stage;
        self.progress = event.progress;
        self.residual = event.residual;
        self.iteration = event.iteration;
        Ok(())
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

pub fn validate_progress_event(event: &ProgressEvent) -> Result<(), ProgressValidationError> {
    if event.job_id.trim().is_empty()
        || event.job_id.len() > JOB_ID_MAX_BYTES
        || event.job_id.chars().any(char::is_control)
    {
        return Err(progress_error(
            ProgressValidationErrorCode::InvalidJobId,
            format!(
                "progress job id must be non-empty, control-free, and at most {JOB_ID_MAX_BYTES} bytes"
            ),
        ));
    }
    if !event.progress.is_finite() || !(0.0..=1.0).contains(&event.progress) {
        return Err(progress_error(
            ProgressValidationErrorCode::InvalidProgress,
            "progress must be finite and between 0 and 1",
        ));
    }
    if event.stage == JobStatus::Completed && event.progress != 1.0 {
        return Err(progress_error(
            ProgressValidationErrorCode::InvalidProgress,
            "completed progress events must report progress 1",
        ));
    }
    if event
        .residual
        .is_some_and(|residual| !residual.is_finite() || residual < 0.0)
    {
        return Err(progress_error(
            ProgressValidationErrorCode::InvalidResidual,
            "progress residual must be finite and non-negative",
        ));
    }
    if event
        .message
        .as_ref()
        .is_some_and(|message| message.len() > PROGRESS_MESSAGE_MAX_BYTES)
    {
        return Err(progress_error(
            ProgressValidationErrorCode::InvalidMessage,
            format!("progress message exceeds {PROGRESS_MESSAGE_MAX_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn progress_error(
    code: ProgressValidationErrorCode,
    message: impl Into<String>,
) -> ProgressValidationError {
    ProgressValidationError {
        code,
        message: message.into(),
    }
}
