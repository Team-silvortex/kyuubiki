mod operator_sdk_focus_chain_operators;
mod operator_sdk_peak_summaries;

use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::operator_sdk_workflow_extensions::operator_sdk_focus_chain_operators::register_focus_chain_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_peak_summaries::{
    electrostatic_peak_summary, thermal_peak_summary, thermo_peak_summary,
};
use crate::workflow_bundle_exports::export_diagnostics_bundle_markdown;
use crate::workflow_bundle_transforms::{
    compose_diagnostics_bundle, compose_diagnostics_report_payload,
    evaluate_diagnostics_bundle_guard,
};
use crate::workflow_diagnostics::{
    extract_electrostatic_result_diagnostics, extract_thermal_result_diagnostics,
    extract_thermo_result_diagnostics,
};
use crate::workflow_guard_transforms::{benchmark_coupled_heat_pair, evaluate_thermal_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorRunContext, OperatorRunResult, SolveElectrostaticPlaneQuad2dResult,
    SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dResult,
};
use serde_json::{Map, Value};

struct ElectrostaticResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}
struct ElectrostaticPeakFieldOperator {
    descriptor: OperatorDescriptor,
}
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
struct ComposeDiagnosticsBundleOperator {
    descriptor: OperatorDescriptor,
}
struct EvaluateDiagnosticsBundleGuardOperator {
    descriptor: OperatorDescriptor,
}
struct ComposeDiagnosticsReportPayloadOperator {
    descriptor: OperatorDescriptor,
}
struct DiagnosticsBundleMarkdownOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for ElectrostaticResultDiagnosticsOperator {
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
            extract_electrostatic_result_diagnostics(input.payload, input.config),
        )
    }
}

impl JsonOperator for ElectrostaticPeakFieldOperator {
    type Input = WorkflowOperatorEnvelope;
    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }
    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let result: SolveElectrostaticPlaneQuad2dResult = serde_json::from_value(input.payload)
            .map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.electrostatic_peak_field expects an electrostatic quad result: {error}"
                ),
            })?;
        let peak_element = result
            .elements
            .iter()
            .max_by(|left, right| {
                left.electric_field_magnitude
                    .partial_cmp(&right.electric_field_magnitude)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "extract.electrostatic_peak_field expects at least one element"
                    .to_string(),
            })?;
        let diagnostics = extract_electrostatic_result_diagnostics(
            serde_json::to_value(&result).map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.electrostatic_peak_field could not serialize diagnostics payload: {error}"
                ),
            })?,
            serde_json::Value::Null,
        )
        .map_err(|error| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message: format!("extract.electrostatic_peak_field diagnostics failed: {error}"),
        })?;

        run_summary_only(
            &self.descriptor.id,
            Ok(merge_summary_objects(
                diagnostics,
                electrostatic_peak_summary(&result, peak_element),
            )),
        )
    }
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
            serde_json::Value::Null,
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
            serde_json::Value::Null,
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

fn merge_summary_objects(base: Value, overlay: Value) -> Value {
    let mut merged = match base {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    if let Value::Object(overlay) = overlay {
        merged.extend(overlay);
    }
    Value::Object(merged)
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

impl JsonOperator for ComposeDiagnosticsBundleOperator {
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
            compose_diagnostics_bundle(input.payload, input.config),
        )
    }
}

impl JsonOperator for EvaluateDiagnosticsBundleGuardOperator {
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
            evaluate_diagnostics_bundle_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for ComposeDiagnosticsReportPayloadOperator {
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
            compose_diagnostics_report_payload(input.payload, input.config),
        )
    }
}

impl JsonOperator for DiagnosticsBundleMarkdownOperator {
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
            export_diagnostics_bundle_markdown(input.payload, input.config),
        )
    }
}

pub fn register_workflow_extract_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(ElectrostaticResultDiagnosticsOperator {
            descriptor: descriptor("extract.electrostatic_result_diagnostics"),
        })
        .expect("extract.electrostatic_result_diagnostics should register");
    registry
        .register_json(ElectrostaticPeakFieldOperator {
            descriptor: descriptor("extract.electrostatic_peak_field"),
        })
        .expect("extract.electrostatic_peak_field should register");
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

pub fn register_workflow_transform_extensions(registry: &mut OperatorRegistry) {
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
    registry
        .register_json(ComposeDiagnosticsBundleOperator {
            descriptor: descriptor("transform.compose_diagnostics_bundle"),
        })
        .expect("transform.compose_diagnostics_bundle should register");
    registry
        .register_json(EvaluateDiagnosticsBundleGuardOperator {
            descriptor: descriptor("transform.evaluate_diagnostics_bundle_guard"),
        })
        .expect("transform.evaluate_diagnostics_bundle_guard should register");
    registry
        .register_json(ComposeDiagnosticsReportPayloadOperator {
            descriptor: descriptor("transform.compose_diagnostics_report_payload"),
        })
        .expect("transform.compose_diagnostics_report_payload should register");
    register_focus_chain_transform_extensions(registry);
}

pub fn register_workflow_export_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(DiagnosticsBundleMarkdownOperator {
            descriptor: descriptor("export.diagnostics_bundle_markdown"),
        })
        .expect("export.diagnostics_bundle_markdown should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id)
        .unwrap_or_else(|| panic!("missing built-in descriptor for {operator_id}"))
}
