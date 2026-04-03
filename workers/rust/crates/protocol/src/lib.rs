use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBarRequest {
    pub length: f64,
    pub area: f64,
    pub youngs_modulus: f64,
    pub elements: usize,
    pub tip_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeResult {
    pub index: usize,
    pub x: f64,
    pub displacement: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementResult {
    pub index: usize,
    pub x1: f64,
    pub x2: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBarResult {
    pub input: SolveBarRequest,
    pub nodes: Vec<NodeResult>,
    pub elements: Vec<ElementResult>,
    pub tip_displacement: f64,
    pub reaction_force: f64,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcMethod {
    #[serde(rename = "solve_bar_1d")]
    SolveBar1d,
    #[serde(rename = "solve_truss_2d")]
    SolveTruss2d,
    #[serde(rename = "solve_truss_3d")]
    SolveTruss3d,
    #[serde(rename = "solve_plane_triangle_2d")]
    SolvePlaneTriangle2d,
    #[serde(rename = "cancel_job")]
    CancelJob,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcRequest {
    pub rpc_version: u8,
    pub id: String,
    pub method: RpcMethod,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcProgress {
    pub rpc_version: u8,
    pub id: String,
    pub event: String,
    pub progress: ProgressEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse {
    pub rpc_version: u8,
    pub id: String,
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelJobRequest {
    pub job_id: String,
}

impl RpcProgress {
    pub fn new(id: impl Into<String>, progress: ProgressEvent) -> Self {
        Self {
            rpc_version: 1,
            id: id.into(),
            event: "progress".to_string(),
            progress,
        }
    }

    pub fn heartbeat(id: impl Into<String>, progress: ProgressEvent) -> Self {
        Self {
            rpc_version: 1,
            id: id.into(),
            event: "heartbeat".to_string(),
            progress,
        }
    }
}

impl RpcResponse {
    pub fn success(id: impl Into<String>, result: Value) -> Self {
        Self {
            rpc_version: 1,
            id: id.into(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(
        id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rpc_version: 1,
            id: id.into(),
            ok: false,
            result: None,
            error: Some(RpcError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss2dRequest {
    pub nodes: Vec<TrussNodeInput>,
    pub elements: Vec<TrussElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss2dResult {
    pub input: SolveTruss2dRequest,
    pub nodes: Vec<TrussNodeResult>,
    pub elements: Vec<TrussElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_z: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub load_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss3dRequest {
    pub nodes: Vec<Truss3dNodeInput>,
    pub elements: Vec<Truss3dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss3dResult {
    pub input: SolveTruss3dRequest,
    pub nodes: Vec<Truss3dNodeResult>,
    pub elements: Vec<Truss3dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneTriangle2dRequest {
    pub nodes: Vec<PlaneNodeInput>,
    pub elements: Vec<PlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub strain_x: f64,
    pub strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub von_mises: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneTriangle2dResult {
    pub input: SolvePlaneTriangle2dRequest,
    pub nodes: Vec<PlaneNodeResult>,
    pub elements: Vec<PlaneTriangleElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnalysisResult {
    Bar1d(SolveBarResult),
    Truss2d(SolveTruss2dResult),
    Truss3d(SolveTruss3dResult),
    PlaneTriangle2d(SolvePlaneTriangle2dResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultChunkKind {
    Nodes,
    Elements,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultChunkRequest {
    pub kind: ResultChunkKind,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultChunkResponse {
    pub kind: ResultChunkKind,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub total: usize,
    pub items: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::{
        Job, JobStatus, ProgressEvent, RpcMethod, RpcProgress, RpcRequest, RpcResponse,
        SolveBarRequest, SolvePlaneTriangle2dRequest, SolveTruss3dRequest,
    };

    #[test]
    fn applies_progress_to_job() {
        let mut job = Job::new("job-1", "project-1", "case-1");
        let mut event = ProgressEvent::new("job-1", JobStatus::Solving, 0.5);
        event.iteration = Some(12);
        event.residual = Some(1.0e-4);

        job.apply_progress(&event);

        assert_eq!(job.status, JobStatus::Solving);
        assert_eq!(job.progress, 0.5);
        assert_eq!(job.iteration, Some(12));
        assert_eq!(job.residual, Some(1.0e-4));
    }

    #[test]
    fn exposes_lowercase_status_names() {
        assert_eq!(JobStatus::Solving.as_str(), "solving");
        assert_eq!(JobStatus::Completed.as_str(), "completed");
    }

    #[test]
    fn serializes_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: 1,
            id: "rpc-1".to_string(),
            method: RpcMethod::SolveBar1d,
            params: serde_json::to_value(SolveBarRequest {
                length: 1.0,
                area: 0.01,
                youngs_modulus: 210.0e9,
                elements: 3,
                tip_force: 1000.0,
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveBar1d);
        assert_eq!(decoded.rpc_version, 1);
        assert_eq!(decoded.id, "rpc-1");
        let params: SolveBarRequest = serde_json::from_value(decoded.params).expect("params");
        assert_eq!(params.elements, 3);
    }

    #[test]
    fn serializes_plane_triangle_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: 1,
            id: "rpc-plane".to_string(),
            method: RpcMethod::SolvePlaneTriangle2d,
            params: serde_json::to_value(SolvePlaneTriangle2dRequest {
                nodes: vec![],
                elements: vec![],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolvePlaneTriangle2d);
        assert_eq!(decoded.id, "rpc-plane");
    }

    #[test]
    fn serializes_truss_3d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: 1,
            id: "rpc-truss-3d".to_string(),
            method: RpcMethod::SolveTruss3d,
            params: serde_json::to_value(SolveTruss3dRequest {
                nodes: vec![],
                elements: vec![],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveTruss3d);
        assert_eq!(decoded.id, "rpc-truss-3d");
    }

    #[test]
    fn builds_error_responses() {
        let response = RpcResponse::error("rpc-1", "invalid_request", "unsupported method");

        assert!(!response.ok);
        assert!(response.result.is_none());
        assert_eq!(response.rpc_version, 1);
        assert_eq!(response.id, "rpc-1");
        assert_eq!(
            response.error.expect("error payload").code,
            "invalid_request"
        );
    }

    #[test]
    fn serializes_progress_frames() {
        let progress = RpcProgress::new(
            "rpc-1",
            ProgressEvent::new("job-1", JobStatus::Solving, 0.5),
        );

        let json = serde_json::to_string(&progress).expect("progress should serialize");
        let decoded: RpcProgress = serde_json::from_str(&json).expect("progress should decode");

        assert_eq!(decoded.id, "rpc-1");
        assert_eq!(decoded.event, "progress");
        assert_eq!(decoded.progress.job_id, "job-1");
    }
}
