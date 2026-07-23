use crate::{ProgressEvent, RPC_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportDescriptor {
    pub kind: String,
    pub framing: Option<String>,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub role: String,
    pub methods: Vec<RpcMethod>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterPeerDescriptor {
    pub address: String,
    pub status: String,
    pub failure_count: u32,
    pub last_seen_unix_s: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentClusterDescriptor {
    pub cluster_id: Option<String>,
    pub runtime_mode: String,
    pub headless: bool,
    pub cluster_size: usize,
    pub health_score: u8,
    pub peers: Vec<ClusterPeerDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAuthorityDescriptor {
    pub control_mode: String,
    pub authority_mode: String,
    pub orchestrator_id: Option<String>,
    pub orchestrator_session_id: Option<String>,
    pub accepts_multi_orchestrator_binding: bool,
    pub agent_library_replication: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEngineDescriptor {
    pub engine_id: String,
    pub engine_name: String,
    pub lifecycle: String,
    pub task_source: String,
    pub operator_source: String,
    pub operator_cache_policy: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcProtocolDescriptor {
    pub name: String,
    pub rpc_version: u8,
    pub transport: TransportDescriptor,
    pub methods: Vec<RpcMethod>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDescriptor {
    pub program: String,
    pub role: String,
    pub protocol: RpcProtocolDescriptor,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub deployment_modes: Vec<String>,
    pub runtime: AgentClusterDescriptor,
    pub authority: RuntimeAuthorityDescriptor,
    pub engine: RuntimeEngineDescriptor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcMethod {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "describe_agent")]
    DescribeAgent,
    #[serde(rename = "run_operator_task_ir")]
    RunOperatorTaskIr,
    #[serde(rename = "solve_bar_1d")]
    SolveBar1d,
    #[serde(rename = "solve_acoustic_bar_1d")]
    SolveAcousticBar1d,
    #[serde(rename = "solve_thermal_bar_1d")]
    SolveThermalBar1d,
    #[serde(rename = "solve_heat_bar_1d")]
    SolveHeatBar1d,
    #[serde(rename = "solve_transient_heat_bar_1d")]
    SolveTransientHeatBar1d,
    #[serde(rename = "solve_electrostatic_bar_1d")]
    SolveElectrostaticBar1d,
    #[serde(rename = "solve_magnetostatic_bar_1d")]
    SolveMagnetostaticBar1d,
    #[serde(rename = "solve_advection_diffusion_bar_1d")]
    SolveAdvectionDiffusionBar1d,
    #[serde(rename = "solve_magnetostatic_plane_triangle_2d")]
    SolveMagnetostaticPlaneTriangle2d,
    #[serde(rename = "solve_magnetostatic_plane_quad_2d")]
    SolveMagnetostaticPlaneQuad2d,
    #[serde(rename = "solve_electrostatic_plane_triangle_2d")]
    SolveElectrostaticPlaneTriangle2d,
    #[serde(rename = "solve_electrostatic_plane_quad_2d")]
    SolveElectrostaticPlaneQuad2d,
    #[serde(rename = "solve_heat_plane_triangle_2d")]
    SolveHeatPlaneTriangle2d,
    #[serde(rename = "solve_heat_plane_quad_2d")]
    SolveHeatPlaneQuad2d,
    #[serde(rename = "solve_stokes_flow_plane_triangle_2d")]
    SolveStokesFlowPlaneTriangle2d,
    #[serde(rename = "solve_stokes_flow_plane_quad_2d")]
    SolveStokesFlowPlaneQuad2d,
    #[serde(rename = "solve_thermal_truss_2d")]
    SolveThermalTruss2d,
    #[serde(rename = "solve_thermal_truss_3d")]
    SolveThermalTruss3d,
    #[serde(rename = "solve_spring_1d")]
    SolveSpring1d,
    #[serde(rename = "solve_transient_spring_1d")]
    SolveTransientSpring1d,
    #[serde(rename = "solve_harmonic_spring_1d")]
    SolveHarmonicSpring1d,
    #[serde(rename = "solve_nonlinear_spring_1d")]
    SolveNonlinearSpring1d,
    #[serde(rename = "solve_contact_gap_1d")]
    SolveContactGap1d,
    #[serde(rename = "solve_spring_2d")]
    SolveSpring2d,
    #[serde(rename = "solve_spring_3d")]
    SolveSpring3d,
    #[serde(rename = "solve_beam_1d")]
    SolveBeam1d,
    #[serde(rename = "solve_thermal_beam_1d")]
    SolveThermalBeam1d,
    #[serde(rename = "solve_torsion_1d")]
    SolveTorsion1d,
    #[serde(rename = "solve_truss_2d")]
    SolveTruss2d,
    #[serde(rename = "solve_truss_3d")]
    SolveTruss3d,
    #[serde(rename = "solve_frame_3d")]
    SolveFrame3d,
    #[serde(rename = "solve_solid_tetra_3d")]
    SolveSolidTetra3d,
    #[serde(rename = "solve_modal_frame_3d")]
    SolveModalFrame3d,
    #[serde(rename = "solve_plane_triangle_2d")]
    SolvePlaneTriangle2d,
    #[serde(rename = "solve_thermal_plane_triangle_2d")]
    SolveThermalPlaneTriangle2d,
    #[serde(rename = "solve_plane_quad_2d")]
    SolvePlaneQuad2d,
    #[serde(rename = "solve_thermal_plane_quad_2d")]
    SolveThermalPlaneQuad2d,
    #[serde(rename = "solve_frame_2d")]
    SolveFrame2d,
    #[serde(rename = "solve_modal_frame_2d")]
    SolveModalFrame2d,
    #[serde(rename = "solve_buckling_beam_1d")]
    SolveBucklingBeam1d,
    #[serde(rename = "solve_buckling_frame_2d")]
    SolveBucklingFrame2d,
    #[serde(rename = "solve_frame_2d_p_delta")]
    SolveFrame2dPDelta,
    #[serde(rename = "solve_frame_2d_p_delta_path")]
    SolveFrame2dPDeltaPath,
    #[serde(rename = "solve_thermal_frame_2d")]
    SolveThermalFrame2d,
    #[serde(rename = "solve_thermal_frame_3d")]
    SolveThermalFrame3d,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
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
            rpc_version: RPC_VERSION,
            id: id.into(),
            event: "progress".to_string(),
            progress,
        }
    }

    pub fn heartbeat(id: impl Into<String>, progress: ProgressEvent) -> Self {
        Self {
            rpc_version: RPC_VERSION,
            id: id.into(),
            event: "heartbeat".to_string(),
            progress,
        }
    }
}

impl RpcResponse {
    pub fn success(id: impl Into<String>, result: Value) -> Self {
        Self {
            rpc_version: RPC_VERSION,
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
            rpc_version: RPC_VERSION,
            id: id.into(),
            ok: false,
            result: None,
            error: Some(RpcError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
        }
    }

    pub fn error_with_details(
        id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            rpc_version: RPC_VERSION,
            id: id.into(),
            ok: false,
            result: None,
            error: Some(RpcError {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            }),
        }
    }
}
