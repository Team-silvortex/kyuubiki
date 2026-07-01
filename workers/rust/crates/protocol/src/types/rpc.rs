use crate::{ProgressEvent, RPC_VERSION, SOLVER_RPC_PROTOCOL};
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
    #[serde(rename = "solve_electrostatic_bar_1d")]
    SolveElectrostaticBar1d,
    #[serde(rename = "solve_magnetostatic_bar_1d")]
    SolveMagnetostaticBar1d,
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
    #[serde(rename = "solve_stokes_flow_plane_quad_2d")]
    SolveStokesFlowPlaneQuad2d,
    #[serde(rename = "solve_thermal_truss_2d")]
    SolveThermalTruss2d,
    #[serde(rename = "solve_thermal_truss_3d")]
    SolveThermalTruss3d,
    #[serde(rename = "solve_spring_1d")]
    SolveSpring1d,
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

impl RpcProtocolDescriptor {
    pub fn solver_agent_default() -> Self {
        Self {
            name: SOLVER_RPC_PROTOCOL.to_string(),
            rpc_version: RPC_VERSION,
            transport: TransportDescriptor {
                kind: "tcp".to_string(),
                framing: Some("length_prefixed_u32".to_string()),
                encoding: "json".to_string(),
            },
            methods: vec![
                RpcMethod::Ping,
                RpcMethod::DescribeAgent,
                RpcMethod::RunOperatorTaskIr,
                RpcMethod::SolveBar1d,
                RpcMethod::SolveAcousticBar1d,
                RpcMethod::SolveThermalBar1d,
                RpcMethod::SolveHeatBar1d,
                RpcMethod::SolveElectrostaticBar1d,
                RpcMethod::SolveMagnetostaticBar1d,
                RpcMethod::SolveMagnetostaticPlaneTriangle2d,
                RpcMethod::SolveMagnetostaticPlaneQuad2d,
                RpcMethod::SolveElectrostaticPlaneTriangle2d,
                RpcMethod::SolveElectrostaticPlaneQuad2d,
                RpcMethod::SolveHeatPlaneTriangle2d,
                RpcMethod::SolveHeatPlaneQuad2d,
                RpcMethod::SolveStokesFlowPlaneQuad2d,
                RpcMethod::SolveThermalTruss2d,
                RpcMethod::SolveThermalTruss3d,
                RpcMethod::SolveSpring1d,
                RpcMethod::SolveNonlinearSpring1d,
                RpcMethod::SolveContactGap1d,
                RpcMethod::SolveSpring2d,
                RpcMethod::SolveSpring3d,
                RpcMethod::SolveBeam1d,
                RpcMethod::SolveThermalBeam1d,
                RpcMethod::SolveTorsion1d,
                RpcMethod::SolveTruss2d,
                RpcMethod::SolveTruss3d,
                RpcMethod::SolveFrame3d,
                RpcMethod::SolveModalFrame3d,
                RpcMethod::SolvePlaneTriangle2d,
                RpcMethod::SolveThermalPlaneTriangle2d,
                RpcMethod::SolvePlaneQuad2d,
                RpcMethod::SolveThermalPlaneQuad2d,
                RpcMethod::SolveFrame2d,
                RpcMethod::SolveModalFrame2d,
                RpcMethod::SolveThermalFrame2d,
                RpcMethod::SolveThermalFrame3d,
                RpcMethod::CancelJob,
            ],
        }
    }
}

impl AgentDescriptor {
    pub fn solver_agent_default() -> Self {
        Self {
            program: "kyuubiki-rust-agent".to_string(),
            role: "solver_agent".to_string(),
            protocol: RpcProtocolDescriptor::solver_agent_default(),
            capabilities: vec![
                CapabilityDescriptor {
                    id: "bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveBar1d],
                    tags: vec!["bar".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "acoustic-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveAcousticBar1d],
                    tags: tags(&["acoustic", "wave", "frequency", "duct", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalBar1d],
                    tags: tags(&["bar", "thermal", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "heat-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveHeatBar1d],
                    tags: tags(&["heat", "bar", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "electrostatic-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveElectrostaticBar1d],
                    tags: tags(&["electromagnetic", "electrostatic", "bar", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "magnetostatic-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveMagnetostaticBar1d],
                    tags: tags(&["electromagnetic", "magnetostatic", "bar", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "magnetostatic-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveMagnetostaticPlaneTriangle2d],
                    tags: field_plane_tags("magnetostatic", "triangle"),
                },
                CapabilityDescriptor {
                    id: "magnetostatic-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveMagnetostaticPlaneQuad2d],
                    tags: field_plane_tags("magnetostatic", "quad"),
                },
                CapabilityDescriptor {
                    id: "electrostatic-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveElectrostaticPlaneTriangle2d],
                    tags: field_plane_tags("electrostatic", "triangle"),
                },
                CapabilityDescriptor {
                    id: "electrostatic-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveElectrostaticPlaneQuad2d],
                    tags: field_plane_tags("electrostatic", "quad"),
                },
                CapabilityDescriptor {
                    id: "heat-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveHeatPlaneTriangle2d],
                    tags: tags(&["heat", "plane", "mesh", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "heat-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveHeatPlaneQuad2d],
                    tags: tags(&["heat", "plane", "mesh", "quad", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-truss-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalTruss2d],
                    tags: tags(&["truss", "thermal", "plane", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-truss-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalTruss3d],
                    tags: tags(&["truss", "thermal", "space", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "spring-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveSpring1d],
                    tags: tags(&["spring", "line", "support", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "spring-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveSpring2d],
                    tags: tags(&["spring", "plane", "support", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "spring-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveSpring3d],
                    tags: tags(&["spring", "space", "support", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "beam-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveBeam1d],
                    tags: tags(&["beam", "bending", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-beam-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalBeam1d],
                    tags: tags(&["beam", "thermal", "bending", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-frame-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalFrame2d],
                    tags: tags(&["frame", "thermal", "beam", "bending", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-frame-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalFrame3d],
                    tags: tags(&["frame", "space", "thermal", "beam", "bending", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "torsion-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveTorsion1d],
                    tags: tags(&["torsion", "shaft", "line", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "truss-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveTruss2d],
                    tags: vec!["truss".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "truss-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveTruss3d],
                    tags: vec!["truss".to_string(), "space".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "frame-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveFrame3d],
                    tags: tags(&["frame", "space", "beam", "bending", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "modal-frame-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveModalFrame3d],
                    tags: tags(&["modal", "frame", "space", "vibration", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolvePlaneTriangle2d],
                    tags: vec!["plane".to_string(), "mesh".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "thermal-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalPlaneTriangle2d],
                    tags: tags(&["plane", "thermal", "mesh", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolvePlaneQuad2d],
                    tags: tags(&["plane", "mesh", "quad", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "thermal-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalPlaneQuad2d],
                    tags: tags(&["plane", "thermal", "mesh", "quad", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "frame-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveFrame2d],
                    tags: tags(&["frame", "beam", "bending", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "modal-frame-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveModalFrame2d],
                    tags: tags(&["modal", "frame", "vibration", "cpu"]),
                },
                CapabilityDescriptor {
                    id: "control".to_string(),
                    role: "runtime".to_string(),
                    methods: vec![
                        RpcMethod::Ping,
                        RpcMethod::DescribeAgent,
                        RpcMethod::RunOperatorTaskIr,
                        RpcMethod::CancelJob,
                    ],
                    tags: vec!["control".to_string(), "general".to_string()],
                },
            ],
            deployment_modes: vec![
                "local".to_string(),
                "cloud".to_string(),
                "distributed".to_string(),
            ],
            runtime: AgentClusterDescriptor {
                cluster_id: None,
                runtime_mode: "standalone".to_string(),
                headless: true,
                cluster_size: 1,
                health_score: 100,
                peers: vec![],
            },
            authority: RuntimeAuthorityDescriptor {
                control_mode: "standalone".to_string(),
                authority_mode: "self_directed".to_string(),
                orchestrator_id: None,
                orchestrator_session_id: None,
                accepts_multi_orchestrator_binding: false,
                agent_library_replication: "central_fetch".to_string(),
            },
            engine: RuntimeEngineDescriptor {
                engine_id: "kyuubiki-engine/local".to_string(),
                engine_name: "kyuubiki-rust-engine".to_string(),
                lifecycle: "agent_embedded".to_string(),
                task_source: "manual_or_sdk".to_string(),
                operator_source: "bound_orchestra_fetch".to_string(),
                operator_cache_policy: "temporary_execution_cache".to_string(),
            },
        }
    }
}

fn tags(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn field_plane_tags(kind: &str, shape: &str) -> Vec<String> {
    tags(&["electromagnetic", kind, "plane", shape, "2d", "cpu"])
}
