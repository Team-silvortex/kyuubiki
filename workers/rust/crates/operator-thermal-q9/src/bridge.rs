use kyuubiki_operator_sdk::{
    JsonOperator, OperatorDescriptorBuilder, OperatorSdkError, operator_port_with_dataset,
    partial_validation,
};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorKind, OperatorRunContext, OperatorRunResult,
    SolveHeatPlaneQuad2dResult, SolveThermalQ9RectRequest,
};
use serde::Deserialize;

pub const BRIDGE_ID: &str = "workflow_bridge.heat_to_thermal_q9_rect_2d";
pub const BRIDGE_SCHEMA: &str = "kyuubiki.heat-to-thermal-q9-nodal/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    payload: Payload,
    #[serde(default)]
    config: super::EmptyConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    schema_version: String,
    temperature_unit: String,
    reference_temperature: f64,
    heat_result: SolveHeatPlaneQuad2dResult,
    seed_model: SolveThermalQ9RectRequest,
}

pub struct HeatToThermalQ9Operator {
    descriptor: OperatorDescriptor,
}

impl Default for HeatToThermalQ9Operator {
    fn default() -> Self {
        Self {
            descriptor: OperatorDescriptorBuilder::new(
                BRIDGE_ID,
                OperatorKind::WorkflowBridge,
                "thermal_mechanical",
                "thermal_q9_rect",
            )
            .version(super::PACKAGE_VERSION)
            .summary("Strict identity-matched Kelvin transfer from four Q4 heat cells per Q9 cell.")
            .capability_tags(["heat", "q9", "matching_nodes", "kelvin", "bounded"])
            .input_schema("kyuubiki.heat-to-thermal-q9-nodal.input", "1")
            .output_schema("kyuubiki.thermal-plane-q9-rect.input", "1")
            .input_port(operator_port_with_dataset(
                "input",
                "result/heat_plane_quad_2d",
                "Heat result, reference temperature and Q9 seed model",
                "heat_q9_transfer_input",
            ))
            .output_port(operator_port_with_dataset(
                "model",
                "study_model/thermal_q9_rect_2d",
                "Q9 model with validated nodal temperature deltas",
                "thermal_q9_rect_input",
            ))
            .validation(partial_validation(
                "matching_q4_heat_q9_bilayer_and_invalid_input",
            ))
            .build(),
        }
    }
}

impl JsonOperator for HeatToThermalQ9Operator {
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
        let payload = input.payload;
        let failed = |message: String| OperatorSdkError::Handler {
            operator_id: BRIDGE_ID.into(),
            message,
        };
        if payload.schema_version != BRIDGE_SCHEMA || payload.temperature_unit != "kelvin" {
            return Err(failed(
                "Q9 bridge requires its versioned schema and explicit kelvin temperatures".into(),
            ));
        }
        if !payload.reference_temperature.is_finite() || payload.reference_temperature < 0.0 {
            return Err(failed(
                "Q9 bridge reference temperature must be finite and nonnegative kelvin".into(),
            ));
        }
        let mut model = payload.seed_model;
        kyuubiki_solver::validate_thermal_q9_rect_model(&model).map_err(failed)?;
        let temperatures =
            super::bridge_validation::validate(&payload.heat_result, &model).map_err(failed)?;
        for (node, temperature) in model.nodes.iter_mut().zip(temperatures) {
            node.temperature_delta = temperature - payload.reference_temperature;
            if !node.temperature_delta.is_finite() {
                return Err(failed(
                    "Q9 bridge temperature delta is not representable".into(),
                ));
            }
        }
        let summary = serde_json::to_value(model).map_err(|e| failed(e.to_string()))?;
        Ok(OperatorRunResult {
            operator_id: BRIDGE_ID.into(),
            summary,
            artifacts: vec![],
        })
    }
}
