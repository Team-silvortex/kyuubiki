use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::operator_sdk_workflow_extensions::operator_sdk_peak_summaries::{
    thermal_peak_summary, thermo_peak_summary,
};
use crate::workflow_diagnostics::{
    extract_thermal_result_diagnostics, extract_thermo_result_diagnostics,
};
use crate::workflow_guard_transforms::{benchmark_coupled_heat_pair, evaluate_thermal_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorRunContext, OperatorRunResult, SolveHeatPlaneQuad2dResult,
    SolveThermalPlaneQuad2dResult,
};
use serde_json::Value;

struct ThermalResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}

struct HeatPeakFluxOperator {
    descriptor: OperatorDescriptor,
}

struct ThermoResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}

struct ThermoPeakResponseOperator {
    descriptor: OperatorDescriptor,
}

struct EvaluateThermalGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkCoupledHeatPairOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for ThermalResultDiagnosticsOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            extract_thermal_result_diagnostics(input.payload, input.config),
        )
    }
}

impl JsonOperator for HeatPeakFluxOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let result: SolveHeatPlaneQuad2dResult =
            serde_json::from_value(input.payload).map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!("extract.heat_peak_flux expects a heat quad result: {error}"),
            })?;
        let peak_element = result
            .elements
            .iter()
            .max_by(|left, right| {
                left.heat_flux_magnitude
                    .partial_cmp(&right.heat_flux_magnitude)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "extract.heat_peak_flux expects at least one element".to_string(),
            })?;
        let diagnostics = extract_thermal_result_diagnostics(
            serde_json::to_value(&result).map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.heat_peak_flux could not serialize diagnostics payload: {error}"
                ),
            })?,
            Value::Null,
        )
        .map_err(|error| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message: format!("extract.heat_peak_flux diagnostics failed: {error}"),
        })?;

        run_summary_only(
            &self.descriptor.id,
            Ok(merge_summary_objects(
                diagnostics,
                thermal_peak_summary(&result, peak_element),
            )),
        )
    }
}

impl JsonOperator for ThermoResultDiagnosticsOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            extract_thermo_result_diagnostics(input.payload, input.config),
        )
    }
}

impl JsonOperator for ThermoPeakResponseOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let result: SolveThermalPlaneQuad2dResult =
            serde_json::from_value(input.payload).map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.thermo_peak_response expects a thermal quad result: {error}"
                ),
            })?;
        let peak_node = result
            .nodes
            .iter()
            .max_by(|left, right| {
                left.displacement_magnitude
                    .partial_cmp(&right.displacement_magnitude)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "extract.thermo_peak_response expects at least one node".to_string(),
            })?;
        let peak_element = result
            .elements
            .iter()
            .max_by(|left, right| {
                left.von_mises
                    .partial_cmp(&right.von_mises)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "extract.thermo_peak_response expects at least one element".to_string(),
            })?;
        let diagnostics = extract_thermo_result_diagnostics(
            serde_json::to_value(&result).map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.thermo_peak_response could not serialize diagnostics payload: {error}"
                ),
            })?,
            Value::Null,
        )
        .map_err(|error| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message: format!("extract.thermo_peak_response diagnostics failed: {error}"),
        })?;

        run_summary_only(
            &self.descriptor.id,
            Ok(merge_summary_objects(
                diagnostics,
                thermo_peak_summary(&result, peak_node, peak_element),
            )),
        )
    }
}

impl JsonOperator for EvaluateThermalGuardOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            evaluate_thermal_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkCoupledHeatPairOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            benchmark_coupled_heat_pair(input.payload, input.config),
        )
    }
}

pub(super) fn register_thermal_extract_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(ThermalResultDiagnosticsOperator {
            descriptor: descriptor("extract.thermal_result_diagnostics"),
        })
        .expect("extract.thermal_result_diagnostics should register");
    registry
        .register_json(HeatPeakFluxOperator {
            descriptor: descriptor("extract.heat_peak_flux"),
        })
        .expect("extract.heat_peak_flux should register");
    registry
        .register_json(ThermoResultDiagnosticsOperator {
            descriptor: descriptor("extract.thermo_result_diagnostics"),
        })
        .expect("extract.thermo_result_diagnostics should register");
    registry
        .register_json(ThermoPeakResponseOperator {
            descriptor: descriptor("extract.thermo_peak_response"),
        })
        .expect("extract.thermo_peak_response should register");
}

pub(super) fn register_thermal_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateThermalGuardOperator {
            descriptor: descriptor("transform.evaluate_thermal_guard"),
        })
        .expect("transform.evaluate_thermal_guard should register");
    registry
        .register_json(BenchmarkCoupledHeatPairOperator {
            descriptor: descriptor("transform.benchmark_coupled_heat_pair"),
        })
        .expect("transform.benchmark_coupled_heat_pair should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}

fn merge_summary_objects(base: Value, overlay: Value) -> Value {
    let mut merged = match base {
        Value::Object(object) => object,
        _ => serde_json::Map::new(),
    };
    if let Value::Object(overlay) = overlay {
        merged.extend(overlay);
    }
    Value::Object(merged)
}
