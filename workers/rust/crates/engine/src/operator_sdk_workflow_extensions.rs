mod operator_sdk_acoustic_transforms;
mod operator_sdk_cfd_transforms;
mod operator_sdk_dynamic_transforms;
mod operator_sdk_electrostatic_transforms;
mod operator_sdk_focus_chain_operators;
mod operator_sdk_magnetostatic_transforms;
mod operator_sdk_modal_transforms;
mod operator_sdk_peak_summaries;
mod operator_sdk_structural_transforms;
mod operator_sdk_thermal_transforms;
mod operator_sdk_transport_transforms;

use crate::catalog::describe_built_in_operator;
use crate::electrostatic_diagnostics::extract_electrostatic_result_diagnostics;
use crate::magnetostatic_diagnostics::extract_magnetostatic_result_diagnostics;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::operator_sdk_workflow_extensions::operator_sdk_acoustic_transforms::register_acoustic_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_cfd_transforms::{
    register_cfd_extract_extensions, register_cfd_transform_extensions,
};
use crate::operator_sdk_workflow_extensions::operator_sdk_dynamic_transforms::register_dynamic_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_electrostatic_transforms::register_electrostatic_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_focus_chain_operators::register_focus_chain_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_magnetostatic_transforms::register_magnetostatic_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_modal_transforms::register_modal_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_peak_summaries::{
    electrostatic_peak_summary, magnetostatic_peak_summary,
};
use crate::operator_sdk_workflow_extensions::operator_sdk_structural_transforms::register_structural_transform_extensions;
use crate::operator_sdk_workflow_extensions::operator_sdk_thermal_transforms::{
    register_thermal_extract_extensions, register_thermal_transform_extensions,
};
use crate::operator_sdk_workflow_extensions::operator_sdk_transport_transforms::{
    register_transport_extract_extensions, register_transport_transform_extensions,
};
use crate::workflow_bundle_exports::export_diagnostics_bundle_markdown;
use crate::workflow_bundle_transforms::{
    compose_diagnostics_bundle, compose_diagnostics_report_payload,
    evaluate_diagnostics_bundle_guard,
};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{
    OperatorDescriptor, OperatorRunContext, OperatorRunResult, SolveElectrostaticPlaneQuad2dResult,
    SolveMagnetostaticPlaneQuad2dResult,
};
use serde_json::{Map, Value};

struct ElectrostaticResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}
struct ElectrostaticPeakFieldOperator {
    descriptor: OperatorDescriptor,
}
struct MagnetostaticResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}
struct MagnetostaticPeakFieldOperator {
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

impl JsonOperator for MagnetostaticResultDiagnosticsOperator {
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
            extract_magnetostatic_result_diagnostics(input.payload, input.config),
        )
    }
}

impl JsonOperator for MagnetostaticPeakFieldOperator {
    type Input = WorkflowOperatorEnvelope;
    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }
    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        let result: SolveMagnetostaticPlaneQuad2dResult = serde_json::from_value(input.payload)
            .map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.magnetostatic_peak_field expects a magnetostatic quad result: {error}"
                ),
            })?;
        let peak_element = result
            .elements
            .iter()
            .max_by(|left, right| {
                left.magnetic_field_strength_magnitude
                    .partial_cmp(&right.magnetic_field_strength_magnitude)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: "extract.magnetostatic_peak_field expects at least one element"
                    .to_string(),
            })?;
        let diagnostics = extract_magnetostatic_result_diagnostics(
            serde_json::to_value(&result).map_err(|error| OperatorSdkError::Handler {
                operator_id: self.descriptor.id.clone(),
                message: format!(
                    "extract.magnetostatic_peak_field could not serialize diagnostics payload: {error}"
                ),
            })?,
            serde_json::Value::Null,
        )
        .map_err(|error| OperatorSdkError::Handler {
            operator_id: self.descriptor.id.clone(),
            message: format!("extract.magnetostatic_peak_field diagnostics failed: {error}"),
        })?;

        run_summary_only(
            &self.descriptor.id,
            Ok(merge_summary_objects(
                diagnostics,
                magnetostatic_peak_summary(&result, peak_element),
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
        .register_json(MagnetostaticResultDiagnosticsOperator {
            descriptor: descriptor("extract.magnetostatic_result_diagnostics"),
        })
        .expect("extract.magnetostatic_result_diagnostics should register");
    registry
        .register_json(MagnetostaticPeakFieldOperator {
            descriptor: descriptor("extract.magnetostatic_peak_field"),
        })
        .expect("extract.magnetostatic_peak_field should register");
    register_cfd_extract_extensions(registry);
    register_transport_extract_extensions(registry);
    register_thermal_extract_extensions(registry);
}

pub fn register_workflow_transform_extensions(registry: &mut OperatorRegistry) {
    register_acoustic_transform_extensions(registry);
    register_modal_transform_extensions(registry);
    register_dynamic_transform_extensions(registry);
    register_thermal_transform_extensions(registry);
    register_structural_transform_extensions(registry);
    register_electrostatic_transform_extensions(registry);
    register_magnetostatic_transform_extensions(registry);
    register_cfd_transform_extensions(registry);
    register_transport_transform_extensions(registry);
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
