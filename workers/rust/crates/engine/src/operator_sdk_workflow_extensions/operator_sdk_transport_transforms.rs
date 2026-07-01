use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::transport_diagnostics::extract_transport_result_diagnostics;
use crate::workflow_guard_transforms::{benchmark_transport_pair, evaluate_transport_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct TransportResultDiagnosticsOperator {
    descriptor: OperatorDescriptor,
}

struct EvaluateTransportGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkTransportPairOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for TransportResultDiagnosticsOperator {
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
            extract_transport_result_diagnostics(input.payload, input.config),
        )
    }
}

impl JsonOperator for EvaluateTransportGuardOperator {
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
            evaluate_transport_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkTransportPairOperator {
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
            benchmark_transport_pair(input.payload, input.config),
        )
    }
}

pub(super) fn register_transport_extract_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(TransportResultDiagnosticsOperator {
            descriptor: descriptor("extract.transport_result_diagnostics"),
        })
        .expect("extract.transport_result_diagnostics should register");
}

pub(super) fn register_transport_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateTransportGuardOperator {
            descriptor: descriptor("transform.evaluate_transport_guard"),
        })
        .expect("transform.evaluate_transport_guard should register");
    registry
        .register_json(BenchmarkTransportPairOperator {
            descriptor: descriptor("transform.benchmark_transport_pair"),
        })
        .expect("transform.benchmark_transport_pair should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
