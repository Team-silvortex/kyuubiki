use crate::catalog::describe_built_in_operator;
use crate::dynamic_quality::score_dynamic_quality;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::workflow_guard_transforms::{benchmark_dynamic_pair, evaluate_dynamic_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateDynamicGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkDynamicPairOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreDynamicQualityOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateDynamicGuardOperator {
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
            evaluate_dynamic_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkDynamicPairOperator {
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
            benchmark_dynamic_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ScoreDynamicQualityOperator {
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
            score_dynamic_quality(input.payload, input.config),
        )
    }
}

pub(super) fn register_dynamic_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateDynamicGuardOperator {
            descriptor: descriptor("transform.evaluate_dynamic_guard"),
        })
        .expect("transform.evaluate_dynamic_guard should register");
    registry
        .register_json(BenchmarkDynamicPairOperator {
            descriptor: descriptor("transform.benchmark_dynamic_pair"),
        })
        .expect("transform.benchmark_dynamic_pair should register");
    registry
        .register_json(ScoreDynamicQualityOperator {
            descriptor: descriptor("transform.score_dynamic_quality"),
        })
        .expect("transform.score_dynamic_quality should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
