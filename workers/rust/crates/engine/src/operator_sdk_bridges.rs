use crate::bridge::{
    attach_bridge_diagnostics, bridge_electrostatic_result_to_heat_plane_quad_model,
    bridge_electrostatic_result_to_heat_plane_triangle_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use crate::heat_bridge::{
    bridge_heat_result_to_thermal_plane_quad_model_with_contract,
    bridge_heat_result_to_thermal_plane_triangle_model_with_contract,
    resolve_heat_to_thermo_bridge_contract,
};
use crate::magnetostatic_bridge::{
    bridge_magnetostatic_result_to_heat_plane_quad_model,
    resolve_magnetostatic_to_heat_bridge_contract,
};
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorRunContext, OperatorRunResult, SolveElectrostaticPlaneQuad2dResult,
    SolveElectrostaticPlaneTriangle2dResult, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest, SolveHeatPlaneTriangle2dResult,
    SolveMagnetostaticPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest,
};

struct HeatToThermoQuadBridgeOperator {
    descriptor: OperatorDescriptor,
}

struct HeatToThermoTriangleBridgeOperator {
    descriptor: OperatorDescriptor,
}

struct ElectrostaticToHeatQuadBridgeOperator {
    descriptor: OperatorDescriptor,
}

struct ElectrostaticToHeatTriangleBridgeOperator {
    descriptor: OperatorDescriptor,
}

struct MagnetostaticToHeatQuadBridgeOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for HeatToThermoQuadBridgeOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let heat_result: SolveHeatPlaneQuad2dResult = serde_json::from_value(input.payload)
            .map_err(|error| OperatorSdkError::DecodeInput {
                operator_id: self.descriptor.id.clone(),
                message: error.to_string(),
            })?;
        let contract =
            resolve_heat_to_thermo_bridge_contract(&input.config).map_err(|message| {
                OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message,
                }
            })?;
        let seed_model_value = input
            .config
            .get("seed_model")
            .cloned()
            .unwrap_or(input.config);
        let thermo_seed_model: SolveThermalPlaneQuad2dRequest =
            serde_json::from_value(seed_model_value).map_err(|error| {
                OperatorSdkError::DecodeInput {
                    operator_id: self.descriptor.id.clone(),
                    message: error.to_string(),
                }
            })?;
        let (bridged, diagnostics) = bridge_heat_result_to_thermal_plane_quad_model_with_contract(
            &heat_result,
            &thermo_seed_model,
            &contract,
        )
        .map_err(|message| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message,
        })?;
        run_summary_only(
            &self.descriptor.id,
            attach_bridge_diagnostics(&bridged, &diagnostics),
        )
    }
}

impl JsonOperator for HeatToThermoTriangleBridgeOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let heat_result: SolveHeatPlaneTriangle2dResult = serde_json::from_value(input.payload)
            .map_err(|error| OperatorSdkError::DecodeInput {
                operator_id: self.descriptor.id.clone(),
                message: error.to_string(),
            })?;
        let contract =
            resolve_heat_to_thermo_bridge_contract(&input.config).map_err(|message| {
                OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message,
                }
            })?;
        let seed_model_value = input
            .config
            .get("seed_model")
            .cloned()
            .unwrap_or(input.config);
        let thermo_seed_model: SolveThermalPlaneTriangle2dRequest =
            serde_json::from_value(seed_model_value).map_err(|error| {
                OperatorSdkError::DecodeInput {
                    operator_id: self.descriptor.id.clone(),
                    message: error.to_string(),
                }
            })?;
        let (bridged, diagnostics) =
            bridge_heat_result_to_thermal_plane_triangle_model_with_contract(
                &heat_result,
                &thermo_seed_model,
                &contract,
            )
            .map_err(|message| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message,
            })?;
        run_summary_only(
            &self.descriptor.id,
            attach_bridge_diagnostics(&bridged, &diagnostics),
        )
    }
}

impl JsonOperator for ElectrostaticToHeatQuadBridgeOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let electrostatic_result: SolveElectrostaticPlaneQuad2dResult =
            serde_json::from_value(input.payload).map_err(|error| {
                OperatorSdkError::DecodeInput {
                    operator_id: self.descriptor.id.clone(),
                    message: error.to_string(),
                }
            })?;
        let seed_model_value =
            input
                .config
                .get("seed_model")
                .cloned()
                .ok_or_else(|| OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message:
                        "bridge.electrostatic_field_to_heat_quad_2d requires config.seed_model"
                            .to_string(),
                })?;
        let heat_seed_model: SolveHeatPlaneQuad2dRequest = serde_json::from_value(seed_model_value)
            .map_err(|error| OperatorSdkError::DecodeInput {
                operator_id: self.descriptor.id.clone(),
                message: error.to_string(),
            })?;
        let contract =
            resolve_electrostatic_to_heat_bridge_contract(&input.config).map_err(|message| {
                OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message,
                }
            })?;
        let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_quad_model(
            &electrostatic_result,
            &heat_seed_model,
            &contract,
        )
        .map_err(|message| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message,
        })?;
        run_summary_only(
            &self.descriptor.id,
            attach_bridge_diagnostics(&bridged, &diagnostics),
        )
    }
}

impl JsonOperator for ElectrostaticToHeatTriangleBridgeOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let electrostatic_result: SolveElectrostaticPlaneTriangle2dResult =
            serde_json::from_value(input.payload).map_err(|error| {
                OperatorSdkError::DecodeInput {
                    operator_id: self.descriptor.id.clone(),
                    message: error.to_string(),
                }
            })?;
        let seed_model_value =
            input
                .config
                .get("seed_model")
                .cloned()
                .ok_or_else(|| OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message:
                        "bridge.electrostatic_field_to_heat_triangle_2d requires config.seed_model"
                            .to_string(),
                })?;
        let heat_seed_model: SolveHeatPlaneTriangle2dRequest =
            serde_json::from_value(seed_model_value).map_err(|error| {
                OperatorSdkError::DecodeInput {
                    operator_id: self.descriptor.id.clone(),
                    message: error.to_string(),
                }
            })?;
        let contract =
            resolve_electrostatic_to_heat_bridge_contract(&input.config).map_err(|message| {
                OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message,
                }
            })?;
        let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_triangle_model(
            &electrostatic_result,
            &heat_seed_model,
            &contract,
        )
        .map_err(|message| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message,
        })?;
        run_summary_only(
            &self.descriptor.id,
            attach_bridge_diagnostics(&bridged, &diagnostics),
        )
    }
}

impl JsonOperator for MagnetostaticToHeatQuadBridgeOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let magnetostatic_result: SolveMagnetostaticPlaneQuad2dResult =
            serde_json::from_value(input.payload).map_err(|error| {
                OperatorSdkError::DecodeInput {
                    operator_id: self.descriptor.id.clone(),
                    message: error.to_string(),
                }
            })?;
        let seed_model_value =
            input
                .config
                .get("seed_model")
                .cloned()
                .ok_or_else(|| OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message:
                        "bridge.magnetostatic_field_to_heat_quad_2d requires config.seed_model"
                            .to_string(),
                })?;
        let heat_seed_model: SolveHeatPlaneQuad2dRequest = serde_json::from_value(seed_model_value)
            .map_err(|error| OperatorSdkError::DecodeInput {
                operator_id: self.descriptor.id.clone(),
                message: error.to_string(),
            })?;
        let contract =
            resolve_magnetostatic_to_heat_bridge_contract(&input.config).map_err(|message| {
                OperatorSdkError::Handler {
                    operator_id: self.descriptor.id.clone(),
                    message,
                }
            })?;
        let (bridged, diagnostics) = bridge_magnetostatic_result_to_heat_plane_quad_model(
            &magnetostatic_result,
            &heat_seed_model,
            &contract,
        )
        .map_err(|message| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message,
        })?;
        run_summary_only(
            &self.descriptor.id,
            attach_bridge_diagnostics(&bridged, &diagnostics),
        )
    }
}

pub(crate) fn register_bridge_transform_operators(
    registry: &mut OperatorRegistry,
    descriptor: fn(&str) -> OperatorDescriptor,
) {
    registry
        .register_json(HeatToThermoQuadBridgeOperator {
            descriptor: descriptor("bridge.temperature_field_to_thermo_quad_2d"),
        })
        .expect("bridge.temperature_field_to_thermo_quad_2d should register");
    registry
        .register_json(HeatToThermoTriangleBridgeOperator {
            descriptor: descriptor("bridge.temperature_field_to_thermo_triangle_2d"),
        })
        .expect("bridge.temperature_field_to_thermo_triangle_2d should register");
    registry
        .register_json(ElectrostaticToHeatQuadBridgeOperator {
            descriptor: descriptor("bridge.electrostatic_field_to_heat_quad_2d"),
        })
        .expect("bridge.electrostatic_field_to_heat_quad_2d should register");
    registry
        .register_json(ElectrostaticToHeatTriangleBridgeOperator {
            descriptor: descriptor("bridge.electrostatic_field_to_heat_triangle_2d"),
        })
        .expect("bridge.electrostatic_field_to_heat_triangle_2d should register");
    registry
        .register_json(MagnetostaticToHeatQuadBridgeOperator {
            descriptor: descriptor("bridge.magnetostatic_field_to_heat_quad_2d"),
        })
        .expect("bridge.magnetostatic_field_to_heat_quad_2d should register");
}
