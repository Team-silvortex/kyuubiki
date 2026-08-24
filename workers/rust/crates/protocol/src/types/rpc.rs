use crate::{ProgressEvent, RPC_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RPC_REQUEST_ID_MAX_BYTES: usize = 256;

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

pub const AGENT_CONTROL_LINK_SCHEMA: &str = "kyuubiki.agent-control-link/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentControlLinkDescriptor {
    pub schema_version: String,
    pub state: String,
    pub operation: String,
    pub orchestrator_bound: bool,
    pub attempt_count: u64,
    pub consecutive_failure_count: u32,
    pub successful_registration_count: u64,
    pub successful_heartbeat_count: u64,
    pub last_success_unix_ms: Option<u64>,
    pub last_failure_unix_ms: Option<u64>,
    pub last_failure_code: Option<String>,
    pub last_failure_message: Option<String>,
    pub next_retry_delay_ms: u64,
}

impl Default for AgentControlLinkDescriptor {
    fn default() -> Self {
        Self {
            schema_version: AGENT_CONTROL_LINK_SCHEMA.to_string(),
            state: "disabled".to_string(),
            operation: "none".to_string(),
            orchestrator_bound: false,
            attempt_count: 0,
            consecutive_failure_count: 0,
            successful_registration_count: 0,
            successful_heartbeat_count: 0,
            last_success_unix_ms: None,
            last_failure_unix_ms: None,
            last_failure_code: None,
            last_failure_message: None,
            next_retry_delay_ms: 0,
        }
    }
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
    #[serde(default)]
    pub control_plane_link: AgentControlLinkDescriptor,
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
    #[serde(rename = "solve_electric_conduction_plane_quad_2d")]
    SolveElectricConductionPlaneQuad2d,
    #[serde(rename = "solve_composite_thermo_electric_panel")]
    SolveCompositeThermoElectricPanel,
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
    #[serde(rename = "solve_cohesive_interface_1d")]
    SolveCohesiveInterface1d,
    #[serde(rename = "solve_cohesive_interface_2d")]
    SolveCohesiveInterface2d,
    #[serde(rename = "solve_cohesive_interface_mesh_2d")]
    SolveCohesiveInterfaceMesh2d,
    #[serde(rename = "solve_cohesive_interface_mesh_3d")]
    SolveCohesiveInterfaceMesh3d,
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
    #[serde(rename = "solve_frame_2d_material_p_delta")]
    SolveFrame2dMaterialPDelta,
    #[serde(rename = "solve_thermal_frame_2d")]
    SolveThermalFrame2d,
    #[serde(rename = "solve_thermal_frame_3d")]
    SolveThermalFrame3d,
    #[serde(rename = "release_operator_package_job")]
    ReleaseOperatorPackageJob,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseOperatorPackageJobRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcEnvelopeErrorCode {
    InvalidVersion,
    InvalidRequestId,
    InvalidResponseState,
    InvalidProgressEvent,
}

impl RpcEnvelopeErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidVersion => "invalid_version",
            Self::InvalidRequestId => "invalid_request_id",
            Self::InvalidResponseState => "invalid_response_state",
            Self::InvalidProgressEvent => "invalid_progress_event",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcEnvelopeValidationError {
    pub code: RpcEnvelopeErrorCode,
    pub message: String,
}

pub fn validate_rpc_request_envelope(
    request: &RpcRequest,
) -> Result<(), RpcEnvelopeValidationError> {
    validate_rpc_version(request.rpc_version)?;
    validate_rpc_id(&request.id)
}

pub fn validate_rpc_response_envelope(
    response: &RpcResponse,
) -> Result<(), RpcEnvelopeValidationError> {
    validate_rpc_version(response.rpc_version)?;
    validate_rpc_id(&response.id)?;
    let valid_state = if response.ok {
        response.result.is_some() && response.error.is_none()
    } else {
        response.result.is_none()
            && response.error.as_ref().is_some_and(|error| {
                !error.code.trim().is_empty() && !error.message.trim().is_empty()
            })
    };
    if !valid_state {
        return Err(envelope_error(
            RpcEnvelopeErrorCode::InvalidResponseState,
            "rpc response must contain exactly one valid result or error state",
        ));
    }
    Ok(())
}

pub fn validate_rpc_progress_envelope(
    progress: &RpcProgress,
) -> Result<(), RpcEnvelopeValidationError> {
    validate_rpc_version(progress.rpc_version)?;
    validate_rpc_id(&progress.id)?;
    if progress.event != "progress" && progress.event != "heartbeat" {
        return Err(envelope_error(
            RpcEnvelopeErrorCode::InvalidProgressEvent,
            "rpc progress event must be progress or heartbeat",
        ));
    }
    Ok(())
}

fn validate_rpc_version(rpc_version: u8) -> Result<(), RpcEnvelopeValidationError> {
    if rpc_version != RPC_VERSION {
        return Err(envelope_error(
            RpcEnvelopeErrorCode::InvalidVersion,
            format!("unsupported rpc version: {rpc_version}"),
        ));
    }
    Ok(())
}

fn validate_rpc_id(id: &str) -> Result<(), RpcEnvelopeValidationError> {
    if id.trim().is_empty()
        || id.len() > RPC_REQUEST_ID_MAX_BYTES
        || id.chars().any(char::is_control)
    {
        return Err(envelope_error(
            RpcEnvelopeErrorCode::InvalidRequestId,
            format!(
                "rpc request id must be non-empty, control-free, and at most {RPC_REQUEST_ID_MAX_BYTES} bytes"
            ),
        ));
    }
    Ok(())
}

fn envelope_error(
    code: RpcEnvelopeErrorCode,
    message: impl Into<String>,
) -> RpcEnvelopeValidationError {
    RpcEnvelopeValidationError {
        code,
        message: message.into(),
    }
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
