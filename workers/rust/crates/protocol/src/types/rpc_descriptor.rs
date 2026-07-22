use crate::{RPC_VERSION, SOLVER_RPC_PROTOCOL};

use super::rpc::{
    AgentClusterDescriptor, AgentDescriptor, CapabilityDescriptor, RpcMethod,
    RpcProtocolDescriptor, RuntimeAuthorityDescriptor, RuntimeEngineDescriptor,
    TransportDescriptor,
};

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
                RpcMethod::SolveTransientHeatBar1d,
                RpcMethod::SolveElectrostaticBar1d,
                RpcMethod::SolveMagnetostaticBar1d,
                RpcMethod::SolveAdvectionDiffusionBar1d,
                RpcMethod::SolveMagnetostaticPlaneTriangle2d,
                RpcMethod::SolveMagnetostaticPlaneQuad2d,
                RpcMethod::SolveElectrostaticPlaneTriangle2d,
                RpcMethod::SolveElectrostaticPlaneQuad2d,
                RpcMethod::SolveHeatPlaneTriangle2d,
                RpcMethod::SolveHeatPlaneQuad2d,
                RpcMethod::SolveStokesFlowPlaneTriangle2d,
                RpcMethod::SolveStokesFlowPlaneQuad2d,
                RpcMethod::SolveThermalTruss2d,
                RpcMethod::SolveThermalTruss3d,
                RpcMethod::SolveSpring1d,
                RpcMethod::SolveTransientSpring1d,
                RpcMethod::SolveHarmonicSpring1d,
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
                RpcMethod::SolveSolidTetra3d,
                RpcMethod::SolveModalFrame3d,
                RpcMethod::SolvePlaneTriangle2d,
                RpcMethod::SolveThermalPlaneTriangle2d,
                RpcMethod::SolvePlaneQuad2d,
                RpcMethod::SolveThermalPlaneQuad2d,
                RpcMethod::SolveFrame2d,
                RpcMethod::SolveModalFrame2d,
                RpcMethod::SolveBucklingBeam1d,
                RpcMethod::SolveBucklingFrame2d,
                RpcMethod::SolveFrame2dPDelta,
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
                capability("bar-1d", RpcMethod::SolveBar1d, &["bar", "cpu"]),
                capability(
                    "acoustic-bar-1d",
                    RpcMethod::SolveAcousticBar1d,
                    &["acoustic", "wave", "frequency", "duct", "cpu"],
                ),
                capability(
                    "thermal-bar-1d",
                    RpcMethod::SolveThermalBar1d,
                    &["bar", "thermal", "line", "cpu"],
                ),
                capability(
                    "heat-bar-1d",
                    RpcMethod::SolveHeatBar1d,
                    &["heat", "bar", "line", "cpu"],
                ),
                capability(
                    "transient-heat-bar-1d",
                    RpcMethod::SolveTransientHeatBar1d,
                    &["heat", "transient", "bar", "time", "cpu"],
                ),
                capability(
                    "electrostatic-bar-1d",
                    RpcMethod::SolveElectrostaticBar1d,
                    &["electromagnetic", "electrostatic", "bar", "line", "cpu"],
                ),
                capability(
                    "magnetostatic-bar-1d",
                    RpcMethod::SolveMagnetostaticBar1d,
                    &["electromagnetic", "magnetostatic", "bar", "line", "cpu"],
                ),
                capability(
                    "advection-diffusion-bar-1d",
                    RpcMethod::SolveAdvectionDiffusionBar1d,
                    &["transport", "advection", "diffusion", "bar", "cpu"],
                ),
                field_capability(
                    "magnetostatic-plane-triangle-2d",
                    RpcMethod::SolveMagnetostaticPlaneTriangle2d,
                    "magnetostatic",
                    "triangle",
                ),
                field_capability(
                    "magnetostatic-plane-quad-2d",
                    RpcMethod::SolveMagnetostaticPlaneQuad2d,
                    "magnetostatic",
                    "quad",
                ),
                field_capability(
                    "electrostatic-plane-triangle-2d",
                    RpcMethod::SolveElectrostaticPlaneTriangle2d,
                    "electrostatic",
                    "triangle",
                ),
                field_capability(
                    "electrostatic-plane-quad-2d",
                    RpcMethod::SolveElectrostaticPlaneQuad2d,
                    "electrostatic",
                    "quad",
                ),
                capability(
                    "heat-plane-triangle-2d",
                    RpcMethod::SolveHeatPlaneTriangle2d,
                    &["heat", "plane", "mesh", "cpu"],
                ),
                capability(
                    "heat-plane-quad-2d",
                    RpcMethod::SolveHeatPlaneQuad2d,
                    &["heat", "plane", "mesh", "quad", "cpu"],
                ),
                capability(
                    "thermal-truss-2d",
                    RpcMethod::SolveThermalTruss2d,
                    &["truss", "thermal", "plane", "cpu"],
                ),
                capability(
                    "thermal-truss-3d",
                    RpcMethod::SolveThermalTruss3d,
                    &["truss", "thermal", "space", "cpu"],
                ),
                capability(
                    "spring-1d",
                    RpcMethod::SolveSpring1d,
                    &["spring", "line", "support", "cpu"],
                ),
                capability(
                    "transient-spring-1d",
                    RpcMethod::SolveTransientSpring1d,
                    &["spring", "transient", "dynamics", "line", "cpu"],
                ),
                capability(
                    "harmonic-spring-1d",
                    RpcMethod::SolveHarmonicSpring1d,
                    &["spring", "harmonic", "frequency-response", "line", "cpu"],
                ),
                capability(
                    "spring-2d",
                    RpcMethod::SolveSpring2d,
                    &["spring", "plane", "support", "cpu"],
                ),
                capability(
                    "spring-3d",
                    RpcMethod::SolveSpring3d,
                    &["spring", "space", "support", "cpu"],
                ),
                capability(
                    "beam-1d",
                    RpcMethod::SolveBeam1d,
                    &["beam", "bending", "line", "cpu"],
                ),
                capability(
                    "thermal-beam-1d",
                    RpcMethod::SolveThermalBeam1d,
                    &["beam", "thermal", "bending", "line", "cpu"],
                ),
                capability(
                    "thermal-frame-2d",
                    RpcMethod::SolveThermalFrame2d,
                    &["frame", "thermal", "beam", "bending", "cpu"],
                ),
                capability(
                    "thermal-frame-3d",
                    RpcMethod::SolveThermalFrame3d,
                    &["frame", "space", "thermal", "beam", "bending", "cpu"],
                ),
                capability(
                    "torsion-1d",
                    RpcMethod::SolveTorsion1d,
                    &["torsion", "shaft", "line", "cpu"],
                ),
                capability("truss-2d", RpcMethod::SolveTruss2d, &["truss", "cpu"]),
                capability(
                    "truss-3d",
                    RpcMethod::SolveTruss3d,
                    &["truss", "space", "cpu"],
                ),
                capability(
                    "frame-3d",
                    RpcMethod::SolveFrame3d,
                    &["frame", "space", "beam", "bending", "cpu"],
                ),
                capability(
                    "solid-tetra-3d",
                    RpcMethod::SolveSolidTetra3d,
                    &["solid", "tetra", "space", "mesh", "cpu"],
                ),
                capability(
                    "modal-frame-3d",
                    RpcMethod::SolveModalFrame3d,
                    &["modal", "frame", "space", "vibration", "cpu"],
                ),
                capability(
                    "plane-triangle-2d",
                    RpcMethod::SolvePlaneTriangle2d,
                    &["plane", "mesh", "cpu"],
                ),
                capability(
                    "thermal-plane-triangle-2d",
                    RpcMethod::SolveThermalPlaneTriangle2d,
                    &["plane", "thermal", "mesh", "cpu"],
                ),
                capability(
                    "plane-quad-2d",
                    RpcMethod::SolvePlaneQuad2d,
                    &["plane", "mesh", "quad", "cpu"],
                ),
                capability(
                    "thermal-plane-quad-2d",
                    RpcMethod::SolveThermalPlaneQuad2d,
                    &["plane", "thermal", "mesh", "quad", "cpu"],
                ),
                capability(
                    "frame-2d",
                    RpcMethod::SolveFrame2d,
                    &["frame", "beam", "bending", "cpu"],
                ),
                capability(
                    "modal-frame-2d",
                    RpcMethod::SolveModalFrame2d,
                    &["modal", "frame", "vibration", "cpu"],
                ),
                capability(
                    "buckling-beam-1d",
                    RpcMethod::SolveBucklingBeam1d,
                    &["buckling", "stability", "beam", "eigenvalue", "cpu"],
                ),
                capability(
                    "buckling-frame-2d",
                    RpcMethod::SolveBucklingFrame2d,
                    &["buckling", "stability", "frame", "eigenvalue", "2d", "cpu"],
                ),
                capability(
                    "frame-2d-p-delta",
                    RpcMethod::SolveFrame2dPDelta,
                    &["stability", "frame", "p-delta", "imperfection", "2d", "cpu"],
                ),
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

fn capability(id: &str, method: RpcMethod, values: &[&str]) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: id.to_string(),
        role: "solver".to_string(),
        methods: vec![method],
        tags: tags(values),
    }
}

fn field_capability(id: &str, method: RpcMethod, kind: &str, shape: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: id.to_string(),
        role: "solver".to_string(),
        methods: vec![method],
        tags: tags(&["electromagnetic", kind, "plane", shape, "2d", "cpu"]),
    }
}

fn tags(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
