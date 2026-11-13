//! Optional solver package: the host transports JSON and never owns Q9 element math.
mod bridge;
mod bridge_validation;
pub use bridge::{BRIDGE_ID, BRIDGE_SCHEMA, HeatToThermalQ9Operator};

use kyuubiki_operator_sdk::{
    JsonOperator, OperatorDescriptorBuilder, OperatorJsonAbiBuffer, OperatorRegistry,
    OperatorSdkError, execute_operator_json_abi, free_operator_json_abi_buffer,
    operator_port_with_dataset, partial_validation,
};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorRunContext, OperatorRunRequest, OperatorRunResult,
    SolveThermalQ9RectRequest,
};
use serde::Deserialize;
use std::sync::OnceLock;

pub const OPERATOR_ID: &str = "solve.thermal_plane_q9_rect_2d";
pub const PACKAGE_VERSION: &str = "0.1.0";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    payload: SolveThermalQ9RectRequest,
    #[serde(default)]
    config: EmptyConfig,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyConfig {}

pub struct ThermalQ9Operator {
    descriptor: OperatorDescriptor,
}

impl Default for ThermalQ9Operator {
    fn default() -> Self {
        let descriptor = OperatorDescriptorBuilder::new(
            OPERATOR_ID,
            OperatorKind::Solver,
            "mechanical",
            "thermal_q9_rect",
        )
            .version(PACKAGE_VERSION)
            .summary("Full-integration rectangular Q9 plane stress with prescribed nodal temperature deltas.")
            .capability_tags(["thermal","plane_stress","q9","rectangular_only","bounded"])
            .input_schema("kyuubiki.thermal-plane-q9-rect.input","1")
            .output_schema("kyuubiki.thermal-plane-q9-rect.output","1")
            .input_port(operator_port_with_dataset("input","study_model/thermal_q9_rect_2d","Q9 mesh and nodal temperature deltas","thermal_q9_rect_input"))
            .output_port(operator_port_with_dataset("result","result/thermal_q9_rect_2d","Q9 displacements, mean stresses and integrated energies","thermal_q9_rect_result"))
            .validation(partial_validation("q9_rect_affine_bilayer_and_invalid_input"))
            .build();
        Self { descriptor }
    }
}

impl JsonOperator for ThermalQ9Operator {
    type Input = Input;
    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }
    fn run_typed(
        &self,
        input: Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let _ = input.config;
        let result = kyuubiki_solver::solve_thermal_q9_rect(&input.payload).map_err(|message| {
            OperatorSdkError::Handler {
                operator_id: OPERATOR_ID.into(),
                message,
            }
        })?;
        let summary = serde_json::to_value(result).map_err(|error| OperatorSdkError::Handler {
            operator_id: OPERATOR_ID.into(),
            message: error.to_string(),
        })?;
        Ok(OperatorRunResult {
            operator_id: OPERATOR_ID.into(),
            summary,
            artifacts: vec![],
        })
    }
}

pub fn run(request: OperatorRunRequest) -> Result<OperatorRunResult, OperatorSdkError> {
    static REGISTRY: OnceLock<OperatorRegistry> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let mut r = OperatorRegistry::new();
            r.register_json(ThermalQ9Operator::default())
                .expect("single Q9 registration");
            r.register_json(HeatToThermalQ9Operator::default())
                .expect("single Q9 bridge registration");
            r
        })
        .run(request)
}

/// # Safety
/// Pointers and lengths must satisfy the operator SDK's stable JSON C ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_thermal_q9_operator_json(
    ptr: *const u8,
    len: usize,
    out: *mut OperatorJsonAbiBuffer,
) -> i32 {
    unsafe { execute_operator_json_abi(ptr, len, out, run) }
}

/// # Safety
/// Return a buffer allocated by this package exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kyuubiki_operator_json_free(buffer: OperatorJsonAbiBuffer) {
    unsafe { free_operator_json_abi_buffer(buffer) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_operator_sdk::{decode_operator_json_abi_response, operator_descriptor_readiness};
    use serde_json::json;

    #[test]
    fn json_abi_executes_q9_and_returns_owned_json() {
        let nodes: Vec<_> = (0..9)
            .map(|i| {
                json!({"id":format!("n{i}"),"x":(i%3) as f64/2.0,"y":(i/3) as f64/2.0,
            "load_x":0.0,"load_y":0.0,"fix_x":i==0,"fix_y":i==0||i==2,"temperature_delta":10.0})
            })
            .collect();
        let request = OperatorRunRequest {
            operator_id: OPERATOR_ID.into(),
            context: OperatorRunContext::default(),
            input: json!({"payload":{"schema_version":kyuubiki_protocol::THERMAL_Q9_RECT_SCHEMA,"nodes":nodes,
            "elements":[{"id":"e0","nodes":[0,1,2,3,4,5,6,7,8],"thickness":0.01,"youngs_modulus":70e9,"poisson_ratio":0.3,"thermal_expansion":12e-6}]},"config":{}}),
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let mut out = OperatorJsonAbiBuffer::empty();
        let status = unsafe { run_thermal_q9_operator_json(bytes.as_ptr(), bytes.len(), &mut out) };
        let result = unsafe {
            decode_operator_json_abi_response(status, std::slice::from_raw_parts(out.ptr, out.len))
        };
        unsafe { kyuubiki_operator_json_free(out) };
        let r = result.unwrap();
        assert_eq!(r.operator_id, OPERATOR_ID);
        assert!((r.summary["nodes"][8]["ux"].as_f64().unwrap() - 120e-6).abs() < 1e-12);
        let mut bad = request;
        bad.input["config"]["reduced_integration"] = json!(true);
        assert!(run(bad).is_err());
        assert!(operator_descriptor_readiness(ThermalQ9Operator::default().descriptor()).ok);
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../kyuubiki-operator.json")).unwrap();
        assert_eq!(manifest["package_version"], PACKAGE_VERSION);
        assert_eq!(manifest["operators"][0]["operator_id"], OPERATOR_ID);
        assert_eq!(manifest["operators"][0]["kind"], "solver");
        assert_eq!(manifest["operators"][1]["operator_id"], BRIDGE_ID);
        assert_eq!(manifest["operators"][1]["kind"], "workflow_bridge");
        assert!(operator_descriptor_readiness(HeatToThermalQ9Operator::default().descriptor()).ok);
    }
}
