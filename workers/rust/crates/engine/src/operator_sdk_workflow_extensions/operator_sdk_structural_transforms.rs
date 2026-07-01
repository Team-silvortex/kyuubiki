use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::structural_quality::score_structural_quality;
use crate::workflow_guard_transforms::{benchmark_structural_pair, evaluate_structural_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateStructuralGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkStructuralPairOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreStructuralQualityOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateStructuralGuardOperator {
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
            evaluate_structural_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkStructuralPairOperator {
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
            benchmark_structural_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ScoreStructuralQualityOperator {
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
            score_structural_quality(input.payload, input.config),
        )
    }
}

pub(super) fn register_structural_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateStructuralGuardOperator {
            descriptor: descriptor("transform.evaluate_structural_guard"),
        })
        .expect("transform.evaluate_structural_guard should register");
    registry
        .register_json(BenchmarkStructuralPairOperator {
            descriptor: descriptor("transform.benchmark_structural_pair"),
        })
        .expect("transform.benchmark_structural_pair should register");
    registry
        .register_json(ScoreStructuralQualityOperator {
            descriptor: descriptor("transform.score_structural_quality"),
        })
        .expect("transform.score_structural_quality should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
