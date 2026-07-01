use crate::catalog::describe_built_in_operator;
use crate::modal_quality::score_modal_quality;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::workflow_guard_transforms::{benchmark_modal_pair, evaluate_modal_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateModalGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkModalPairOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreModalQualityOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateModalGuardOperator {
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
            evaluate_modal_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkModalPairOperator {
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
            benchmark_modal_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ScoreModalQualityOperator {
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
            score_modal_quality(input.payload, input.config),
        )
    }
}

pub(super) fn register_modal_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateModalGuardOperator {
            descriptor: descriptor("transform.evaluate_modal_guard"),
        })
        .expect("transform.evaluate_modal_guard should register");
    registry
        .register_json(BenchmarkModalPairOperator {
            descriptor: descriptor("transform.benchmark_modal_pair"),
        })
        .expect("transform.benchmark_modal_pair should register");
    registry
        .register_json(ScoreModalQualityOperator {
            descriptor: descriptor("transform.score_modal_quality"),
        })
        .expect("transform.score_modal_quality should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
