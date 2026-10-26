use crate::error::{SdkError, SdkResult};
use crate::{SolverRpcLimits, solver_rpc_transport};
use serde_json::{Value, json};

const SOLVER_METHODS: &[(&str, &str)] = &[
    ("bar_1d", "solve_bar_1d"),
    ("thermal_bar_1d", "solve_thermal_bar_1d"),
    (
        "advection_diffusion_bar_1d",
        "solve_advection_diffusion_bar_1d",
    ),
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
    ("frame_2d_p_delta", "solve_frame_2d_p_delta"),
    (
        "frame_2d_material_p_delta",
        "solve_frame_2d_material_p_delta",
    ),
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
        "electric_conduction_plane_quad_2d",
        "solve_electric_conduction_plane_quad_2d",
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
    limits: SolverRpcLimits,
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
            limits: SolverRpcLimits::default(),
        }
    }

    /// Caller-owned resource policy; errors never imply task cancellation or retry.
    pub fn with_limits(mut self, limits: SolverRpcLimits) -> SdkResult<Self> {
        limits.validate()?;
        self.limits = limits;
        Ok(self)
    }

    pub fn limits(&self) -> &SolverRpcLimits {
        &self.limits
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

    pub fn solve_frame_2d_p_delta(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("frame_2d_p_delta", payload)
    }

    pub fn solve_frame_2d_material_p_delta(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.solve_study("frame_2d_material_p_delta", payload)
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
                request_id: None,
                details: None,
            })?;
        self.call(method, payload)
    }

    pub fn cancel_job(&self, job_id: &str) -> SdkResult<RpcCallOutcome> {
        self.call("cancel_job", json!({ "job_id": job_id }))
    }

    pub fn release_operator_package_job(&self, job_id: &str) -> SdkResult<RpcCallOutcome> {
        self.call("release_operator_package_job", json!({ "job_id": job_id }))
    }

    pub fn call(&self, method: &str, params: Value) -> SdkResult<RpcCallOutcome> {
        self.call_identified(method, params)
            .map(|(_, outcome)| outcome)
    }

    pub(crate) fn call_identified(
        &self,
        method: &str,
        params: Value,
    ) -> SdkResult<(String, RpcCallOutcome)> {
        solver_rpc_transport::call_identified(&self.host, self.port, &self.limits, method, params)
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
