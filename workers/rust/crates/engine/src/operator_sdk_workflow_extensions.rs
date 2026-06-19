use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{run_summary_only, WorkflowOperatorEnvelope};
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
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct ElectrostaticResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}

struct ThermalResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}

struct ThermoResultDiagnosticsOperator {
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
        .register_json(ThermalResultDiagnosticsOperator {
            descriptor: descriptor("extract.thermal_result_diagnostics"),
        })
        .expect("extract.thermal_result_diagnostics should register");
    registry
        .register_json(ThermoResultDiagnosticsOperator {
            descriptor: descriptor("extract.thermo_result_diagnostics"),
        })
        .expect("extract.thermo_result_diagnostics should register");
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
