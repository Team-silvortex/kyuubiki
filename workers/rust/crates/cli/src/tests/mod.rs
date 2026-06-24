use std::collections::HashMap;

use super::{
    build_agent_descriptor, build_peer_descriptors, compute_cluster_health_score,
    filter_self_peers, handle_request_bytes, normalize_peer_addresses, parse_http_url,
};
use crate::config::{AgentConfig, Command, WorkerConfig};
use crate::transport::AgentReply;
use crate::worker::format_event;
use kyuubiki_protocol::{
    AgentDescriptor, Beam1dElementInput, Beam1dNodeInput, ClusterPeerDescriptor,
    ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput, ElectrostaticPlaneNodeInput,
    ElectrostaticPlaneQuadElementInput, ElectrostaticPlaneTriangleElementInput,
    HeatBar1dElementInput, HeatBar1dNodeInput, JobStatus, MagnetostaticBar1dElementInput,
    MagnetostaticBar1dNodeInput, PlaneNodeInput, PlaneQuadElementInput, PlaneTriangleElementInput,
    ProgressEvent, RPC_VERSION, RpcMethod, RpcRequest, SolveBarRequest, SolveBeam1dRequest,
    SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
    SolveHeatBar1dRequest, SolveMagnetostaticBar1dRequest, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest, SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest,
    SolveThermalBar1dRequest, SolveThermalBeam1dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, SolveTorsion1dRequest,
    SolveTruss3dRequest, Spring1dElementInput, Spring1dNodeInput, Spring2dElementInput,
    Spring2dNodeInput, Spring3dElementInput, Spring3dNodeInput, ThermalBar1dElementInput,
    ThermalBar1dNodeInput, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
    ThermalPlaneNodeInput, ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
    ThermalTruss2dElementInput, ThermalTruss2dNodeInput, ThermalTruss3dElementInput,
    ThermalTruss3dNodeInput, Torsion1dElementInput, Torsion1dNodeInput, Truss3dElementInput,
    Truss3dNodeInput,
};

mod config_and_transport;
mod core_field_rpc;
mod electrostatic_and_truss_rpc;
mod frame_and_mesh_rpc;
mod mechanics_rpc;
mod plane_rpc;
