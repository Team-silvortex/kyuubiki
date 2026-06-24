use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde::de::DeserializeOwned;

use kyuubiki_protocol::{
    CancelJobRequest, RPC_VERSION, RpcMethod, RpcRequest, RpcResponse, SolveBarRequest,
    SolveBeam1dRequest, SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
    SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
    SolveMagnetostaticBar1dRequest, SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest,
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBar1dRequest,
    SolveThermalBeam1dRequest, SolveThermalFrame2dRequest, SolveThermalFrame3dRequest,
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, SolveThermalTruss2dRequest,
    SolveThermalTruss3dRequest, SolveTorsion1dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
};
use kyuubiki_solver::{
    solve_bar_1d, solve_beam_1d, solve_electrostatic_bar_1d, solve_electrostatic_plane_quad_2d,
    solve_electrostatic_plane_triangle_2d, solve_frame_2d, solve_frame_3d, solve_heat_bar_1d,
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d, solve_magnetostatic_bar_1d,
    solve_plane_quad_2d, solve_plane_triangle_2d, solve_spring_1d, solve_spring_2d,
    solve_spring_3d, solve_thermal_bar_1d, solve_thermal_beam_1d, solve_thermal_frame_2d,
    solve_thermal_frame_3d, solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d,
    solve_thermal_truss_2d, solve_thermal_truss_3d, solve_torsion_1d, solve_truss_2d,
    solve_truss_3d,
};

use crate::agent_state::{
    agent_descriptor, build_progress_frames, extract_job_id, register_cancel, take_cancelled,
};
use crate::transport::{AgentReply, HeartbeatHandle};

pub(crate) fn handle_request(
    request: RpcRequest,
    writer: Option<Arc<Mutex<TcpStream>>>,
) -> AgentReply {
    if request.rpc_version != RPC_VERSION {
        return AgentReply::Stream(
            Vec::new(),
            RpcResponse::error(
                request.id,
                "invalid_version",
                format!("unsupported rpc version: {}", request.rpc_version),
            ),
        );
    }

    match request.method {
        RpcMethod::Ping => AgentReply::Stream(
            Vec::new(),
            RpcResponse::success(request.id, serde_json::json!({ "pong": true })),
        ),
        RpcMethod::DescribeAgent => AgentReply::Stream(
            Vec::new(),
            RpcResponse::success(
                request.id,
                serde_json::to_value(agent_descriptor())
                    .expect("agent descriptor should serialize"),
            ),
        ),
        RpcMethod::CancelJob => handle_cancel_job(request),
        RpcMethod::SolveBar1d => run_solver::<SolveBarRequest, _, _, _>(
            request,
            writer,
            "axial bar",
            "bar result",
            |params| params.elements + 1,
            solve_bar_1d,
        ),
        RpcMethod::SolveThermalBar1d => run_solver::<SolveThermalBar1dRequest, _, _, _>(
            request,
            writer,
            "1d thermal bar",
            "thermal bar result",
            |params| params.nodes.len(),
            solve_thermal_bar_1d,
        ),
        RpcMethod::SolveHeatBar1d => run_solver::<SolveHeatBar1dRequest, _, _, _>(
            request,
            writer,
            "1d heat bar",
            "heat bar result",
            |params| params.nodes.len(),
            solve_heat_bar_1d,
        ),
        RpcMethod::SolveElectrostaticBar1d => {
            run_solver::<SolveElectrostaticBar1dRequest, _, _, _>(
                request,
                writer,
                "1d electrostatic bar",
                "electrostatic bar result",
                |params| params.nodes.len(),
                solve_electrostatic_bar_1d,
            )
        }
        RpcMethod::SolveMagnetostaticBar1d => {
            run_solver::<SolveMagnetostaticBar1dRequest, _, _, _>(
                request,
                writer,
                "1d magnetostatic bar",
                "magnetostatic bar result",
                |params| params.nodes.len(),
                solve_magnetostatic_bar_1d,
            )
        }
        RpcMethod::SolveThermalTruss2d => run_solver::<SolveThermalTruss2dRequest, _, _, _>(
            request,
            writer,
            "2d thermal truss",
            "thermal truss 2d result",
            |params| params.nodes.len(),
            solve_thermal_truss_2d,
        ),
        RpcMethod::SolveThermalTruss3d => run_solver::<SolveThermalTruss3dRequest, _, _, _>(
            request,
            writer,
            "3d thermal truss",
            "thermal truss 3d result",
            |params| params.nodes.len(),
            solve_thermal_truss_3d,
        ),
        RpcMethod::SolveSpring1d => run_solver::<SolveSpring1dRequest, _, _, _>(
            request,
            writer,
            "1d spring",
            "spring result",
            |params| params.nodes.len(),
            solve_spring_1d,
        ),
        RpcMethod::SolveSpring2d => run_solver::<SolveSpring2dRequest, _, _, _>(
            request,
            writer,
            "2d spring",
            "spring 2d result",
            |params| params.nodes.len(),
            solve_spring_2d,
        ),
        RpcMethod::SolveSpring3d => run_solver::<SolveSpring3dRequest, _, _, _>(
            request,
            writer,
            "3d spring",
            "spring 3d result",
            |params| params.nodes.len(),
            solve_spring_3d,
        ),
        RpcMethod::SolveBeam1d => run_solver::<SolveBeam1dRequest, _, _, _>(
            request,
            writer,
            "1d beam",
            "beam result",
            |params| params.nodes.len(),
            solve_beam_1d,
        ),
        RpcMethod::SolveThermalBeam1d => run_solver::<SolveThermalBeam1dRequest, _, _, _>(
            request,
            writer,
            "1d thermal beam",
            "thermal beam result",
            |params| params.nodes.len(),
            solve_thermal_beam_1d,
        ),
        RpcMethod::SolveTorsion1d => run_solver::<SolveTorsion1dRequest, _, _, _>(
            request,
            writer,
            "1d torsion shaft",
            "torsion result",
            |params| params.nodes.len(),
            solve_torsion_1d,
        ),
        RpcMethod::SolveTruss2d => run_solver::<SolveTruss2dRequest, _, _, _>(
            request,
            writer,
            "2d truss",
            "truss result",
            |params| params.nodes.len(),
            solve_truss_2d,
        ),
        RpcMethod::SolveTruss3d => run_solver::<SolveTruss3dRequest, _, _, _>(
            request,
            writer,
            "3d truss",
            "3d truss result",
            |params| params.nodes.len(),
            solve_truss_3d,
        ),
        RpcMethod::SolvePlaneTriangle2d => run_solver::<SolvePlaneTriangle2dRequest, _, _, _>(
            request,
            writer,
            "2d plane triangle",
            "plane result",
            |params| params.nodes.len(),
            solve_plane_triangle_2d,
        ),
        RpcMethod::SolveHeatPlaneTriangle2d => {
            run_solver::<SolveHeatPlaneTriangle2dRequest, _, _, _>(
                request,
                writer,
                "2d heat plane triangle",
                "heat plane triangle result",
                |params| params.nodes.len(),
                solve_heat_plane_triangle_2d,
            )
        }
        RpcMethod::SolveThermalPlaneTriangle2d => {
            run_solver::<SolveThermalPlaneTriangle2dRequest, _, _, _>(
                request,
                writer,
                "2d thermal plane triangle",
                "thermal plane result",
                |params| params.nodes.len(),
                solve_thermal_plane_triangle_2d,
            )
        }
        RpcMethod::SolveElectrostaticPlaneTriangle2d => {
            run_solver::<SolveElectrostaticPlaneTriangle2dRequest, _, _, _>(
                request,
                writer,
                "2d electrostatic plane triangle",
                "electrostatic plane triangle result",
                |params| params.nodes.len(),
                solve_electrostatic_plane_triangle_2d,
            )
        }
        RpcMethod::SolveElectrostaticPlaneQuad2d => {
            run_solver::<SolveElectrostaticPlaneQuad2dRequest, _, _, _>(
                request,
                writer,
                "2d electrostatic plane quad",
                "electrostatic plane quad result",
                |params| params.nodes.len(),
                solve_electrostatic_plane_quad_2d,
            )
        }
        RpcMethod::SolveHeatPlaneQuad2d => run_solver::<SolveHeatPlaneQuad2dRequest, _, _, _>(
            request,
            writer,
            "2d heat plane quad",
            "heat plane quad result",
            |params| params.nodes.len(),
            solve_heat_plane_quad_2d,
        ),
        RpcMethod::SolvePlaneQuad2d => run_solver::<SolvePlaneQuad2dRequest, _, _, _>(
            request,
            writer,
            "2d plane quad",
            "plane quad result",
            |params| params.nodes.len(),
            solve_plane_quad_2d,
        ),
        RpcMethod::SolveThermalPlaneQuad2d => {
            run_solver::<SolveThermalPlaneQuad2dRequest, _, _, _>(
                request,
                writer,
                "2d thermal plane quad",
                "thermal plane quad result",
                |params| params.nodes.len(),
                solve_thermal_plane_quad_2d,
            )
        }
        RpcMethod::SolveFrame2d => run_solver::<SolveFrame2dRequest, _, _, _>(
            request,
            writer,
            "2d frame",
            "frame result",
            |params| params.nodes.len(),
            solve_frame_2d,
        ),
        RpcMethod::SolveThermalFrame2d => run_solver::<SolveThermalFrame2dRequest, _, _, _>(
            request,
            writer,
            "2d thermal frame",
            "thermal frame result",
            |params| params.nodes.len(),
            solve_thermal_frame_2d,
        ),
        RpcMethod::SolveFrame3d => run_solver::<SolveFrame3dRequest, _, _, _>(
            request,
            writer,
            "3d frame",
            "frame 3d result",
            |params| params.nodes.len(),
            solve_frame_3d,
        ),
        RpcMethod::SolveThermalFrame3d => run_solver::<SolveThermalFrame3dRequest, _, _, _>(
            request,
            writer,
            "3d thermal frame",
            "thermal frame 3d result",
            |params| params.nodes.len(),
            solve_thermal_frame_3d,
        ),
    }
}

fn handle_cancel_job(request: RpcRequest) -> AgentReply {
    let params = match serde_json::from_value::<CancelJobRequest>(request.params.clone()) {
        Ok(params) => params,
        Err(error) => {
            return AgentReply::Stream(
                Vec::new(),
                RpcResponse::error(request.id, "invalid_params", error.to_string()),
            );
        }
    };

    register_cancel(params.job_id);
    AgentReply::Stream(
        Vec::new(),
        RpcResponse::success(request.id, serde_json::json!({ "cancelled": true })),
    )
}

fn run_solver<Request, ResultValue, NodeCount, Solver>(
    request: RpcRequest,
    writer: Option<Arc<Mutex<TcpStream>>>,
    model_name: &str,
    serialize_label: &str,
    node_count: NodeCount,
    solver: Solver,
) -> AgentReply
where
    Request: DeserializeOwned,
    ResultValue: Serialize,
    NodeCount: FnOnce(&Request) -> usize,
    Solver: FnOnce(&Request) -> Result<ResultValue, String>,
{
    let request_id = request.id;
    let maybe_job_id = extract_job_id(&request.params);
    let params = match serde_json::from_value::<Request>(request.params) {
        Ok(params) => params,
        Err(error) => {
            return AgentReply::Stream(
                Vec::new(),
                RpcResponse::error(request_id, "invalid_params", error.to_string()),
            );
        }
    };

    let heartbeat = maybe_job_id.as_ref().and_then(|job_id| {
        writer.clone().map(|shared_writer| {
            HeartbeatHandle::spawn(shared_writer, request_id.clone(), job_id.clone())
        })
    });

    match solver(&params) {
        Ok(result) => {
            if let Some(job_id) = maybe_job_id.as_deref() {
                if take_cancelled(job_id) {
                    stop_heartbeat(heartbeat);
                    return AgentReply::Stream(
                        Vec::new(),
                        RpcResponse::error(request_id, "cancelled", "job was cancelled"),
                    );
                }
            }

            let progress_frames =
                build_progress_frames(model_name, &request_id, node_count(&params));
            stop_heartbeat(heartbeat);
            AgentReply::Stream(
                progress_frames,
                RpcResponse::success(
                    request_id,
                    serde_json::to_value(result)
                        .unwrap_or_else(|_| panic!("{serialize_label} should serialize")),
                ),
            )
        }
        Err(error) => {
            stop_heartbeat(heartbeat);
            AgentReply::Stream(
                Vec::new(),
                RpcResponse::error(request_id, "solve_failed", error),
            )
        }
    }
}

fn stop_heartbeat(heartbeat: Option<HeartbeatHandle>) {
    if let Some(heartbeat) = heartbeat {
        heartbeat.stop();
    }
}
