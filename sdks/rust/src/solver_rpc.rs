use crate::error::{SdkError, SdkResult};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const SOLVER_METHODS: &[(&str, &str)] = &[
    ("bar_1d", "solve_bar_1d"),
    ("thermal_bar_1d", "solve_thermal_bar_1d"),
    ("heat_bar_1d", "solve_heat_bar_1d"),
    ("transient_heat_bar_1d", "solve_transient_heat_bar_1d"),
    ("electrostatic_bar_1d", "solve_electrostatic_bar_1d"),
    ("magnetostatic_bar_1d", "solve_magnetostatic_bar_1d"),
    (
        "magnetostatic_plane_triangle_2d",
        "solve_magnetostatic_plane_triangle_2d",
    ),
    (
        "magnetostatic_plane_quad_2d",
        "solve_magnetostatic_plane_quad_2d",
    ),
    ("acoustic_bar_1d", "solve_acoustic_bar_1d"),
    ("beam_1d", "solve_beam_1d"),
    ("thermal_beam_1d", "solve_thermal_beam_1d"),
    ("torsion_1d", "solve_torsion_1d"),
    ("spring_1d", "solve_spring_1d"),
    ("transient_spring_1d", "solve_transient_spring_1d"),
    ("harmonic_spring_1d", "solve_harmonic_spring_1d"),
    ("nonlinear_spring_1d", "solve_nonlinear_spring_1d"),
    ("contact_gap_1d", "solve_contact_gap_1d"),
    ("spring_2d", "solve_spring_2d"),
    ("spring_3d", "solve_spring_3d"),
    ("truss_2d", "solve_truss_2d"),
    ("thermal_truss_2d", "solve_thermal_truss_2d"),
    ("frame_2d", "solve_frame_2d"),
    ("modal_frame_2d", "solve_modal_frame_2d"),
    ("buckling_beam_1d", "solve_buckling_beam_1d"),
    ("buckling_frame_2d", "solve_buckling_frame_2d"),
    ("thermal_frame_2d", "solve_thermal_frame_2d"),
    ("plane_triangle_2d", "solve_plane_triangle_2d"),
    ("heat_plane_triangle_2d", "solve_heat_plane_triangle_2d"),
    (
        "thermal_plane_triangle_2d",
        "solve_thermal_plane_triangle_2d",
    ),
    (
        "electrostatic_plane_triangle_2d",
        "solve_electrostatic_plane_triangle_2d",
    ),
    ("plane_quad_2d", "solve_plane_quad_2d"),
    ("heat_plane_quad_2d", "solve_heat_plane_quad_2d"),
    ("thermal_plane_quad_2d", "solve_thermal_plane_quad_2d"),
    (
        "electrostatic_plane_quad_2d",
        "solve_electrostatic_plane_quad_2d",
    ),
    (
        "stokes_flow_triangle_2d",
        "solve_stokes_flow_plane_triangle_2d",
    ),
    (
        "stokes_flow_plane_triangle_2d",
        "solve_stokes_flow_plane_triangle_2d",
    ),
    ("stokes_flow_quad_2d", "solve_stokes_flow_plane_quad_2d"),
    (
        "stokes_flow_plane_quad_2d",
        "solve_stokes_flow_plane_quad_2d",
    ),
    ("truss_3d", "solve_truss_3d"),
    ("thermal_truss_3d", "solve_thermal_truss_3d"),
    ("frame_3d", "solve_frame_3d"),
    ("solid_tetra_3d", "solve_solid_tetra_3d"),
    ("modal_frame_3d", "solve_modal_frame_3d"),
    ("thermal_frame_3d", "solve_thermal_frame_3d"),
];

pub struct SolverRpcClient {
    host: String,
    port: u16,
    timeout: Duration,
}

pub struct RpcCallOutcome {
    pub result: Value,
    pub progress_frames: Vec<Value>,
}

impl SolverRpcClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Duration::from_secs(15),
        }
    }

    pub fn ping(&self) -> SdkResult<RpcCallOutcome> {
        self.call("ping", json!({}))
    }

    pub fn describe_agent(&self) -> SdkResult<RpcCallOutcome> {
        self.call("describe_agent", json!({}))
    }

    pub fn solve_bar_1d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("bar_1d", payload)
    }

    pub fn solve_truss_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("truss_2d", payload)
    }

    pub fn solve_truss_3d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("truss_3d", payload)
    }

    pub fn solve_modal_frame_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("modal_frame_2d", payload)
    }

    pub fn solve_buckling_beam_1d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("buckling_beam_1d", payload)
    }

    pub fn solve_buckling_frame_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("buckling_frame_2d", payload)
    }

    pub fn solve_modal_frame_3d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("modal_frame_3d", payload)
    }

    pub fn solve_solid_tetra_3d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("solid_tetra_3d", payload)
    }

    pub fn solve_nonlinear_spring_1d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("nonlinear_spring_1d", payload)
    }

    pub fn solve_contact_gap_1d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("contact_gap_1d", payload)
    }

    pub fn solve_harmonic_spring_1d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("harmonic_spring_1d", payload)
    }

    pub fn solve_magnetostatic_plane_triangle_2d(
        &self,
        payload: Value,
    ) -> SdkResult<RpcCallOutcome> {
        self.solve_study("magnetostatic_plane_triangle_2d", payload)
    }

    pub fn solve_magnetostatic_plane_quad_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("magnetostatic_plane_quad_2d", payload)
    }

    pub fn solve_plane_triangle_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("plane_triangle_2d", payload)
    }

    pub fn solve_study(&self, solve_kind: &str, payload: Value) -> SdkResult<RpcCallOutcome> {
        let normalized = normalize_solve_kind(solve_kind);
        let method = SOLVER_METHODS
            .iter()
            .find_map(|(kind, method)| (*kind == normalized).then_some(*method))
            .ok_or_else(|| SdkError::Rpc {
                message: format!("unsupported solve kind: {solve_kind}"),
                code: None,
            })?;
        self.call(method, payload)
    }

    pub fn cancel_job(&self, job_id: &str) -> SdkResult<RpcCallOutcome> {
        self.call("cancel_job", json!({ "job_id": job_id }))
    }

    pub fn call(&self, method: &str, params: Value) -> SdkResult<RpcCallOutcome> {
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        let request_id = format!(
            "rust-sdk-{}",
            std::time::SystemTime::now()
                .elapsed()
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        );
        let payload = serde_json::to_vec(&json!({
            "rpc_version": 1,
            "id": request_id,
            "method": method,
            "params": params,
        }))?;
        let len = (payload.len() as u32).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(&payload)?;

        let mut progress_frames = Vec::new();
        loop {
            let mut header = [0_u8; 4];
            stream.read_exact(&mut header)?;
            let size = u32::from_be_bytes(header) as usize;
            let mut frame = vec![0_u8; size];
            stream.read_exact(&mut frame)?;
            let decoded: Value = serde_json::from_slice(&frame)?;
            if decoded.get("event").is_some() {
                progress_frames.push(decoded);
                continue;
            }
            if decoded.get("ok") == Some(&Value::Bool(true)) {
                return Ok(RpcCallOutcome {
                    result: decoded.get("result").cloned().unwrap_or(Value::Null),
                    progress_frames,
                });
            }
            let error = decoded.get("error");
            let message = error
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("rpc failed");
            let code = error
                .and_then(|error| error.get("code"))
                .and_then(Value::as_str)
                .map(str::to_string);
            return Err(SdkError::Rpc {
                message: message.to_string(),
                code,
            });
        }
    }
}

fn normalize_solve_kind(kind: &str) -> &str {
    match kind {
        "axial_bar_1d" => "bar_1d",
        "stokes_flow_plane_triangle_2d" => "stokes_flow_triangle_2d",
        "stokes_flow_plane_quad_2d" => "stokes_flow_quad_2d",
        other => other,
    }
}
