use crate::catalog::describe_built_in_operator;
use crate::magnetostatic_quality::score_magnetostatic_quality;
use crate::operator_sdk_runtime::{run_summary_only, WorkflowOperatorEnvelope};
use crate::workflow_guard_transforms::{
    benchmark_magnetostatic_pair, evaluate_magnetostatic_guard,
};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateMagnetostaticGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkMagnetostaticPairOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreMagnetostaticQualityOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateMagnetostaticGuardOperator {
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
            evaluate_magnetostatic_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkMagnetostaticPairOperator {
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
            benchmark_magnetostatic_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ScoreMagnetostaticQualityOperator {
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
            score_magnetostatic_quality(input.payload, input.config),
        )
    }
}

pub(super) fn register_magnetostatic_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateMagnetostaticGuardOperator {
            descriptor: descriptor("transform.evaluate_magnetostatic_guard"),
        })
        .expect("transform.evaluate_magnetostatic_guard should register");
    registry
        .register_json(BenchmarkMagnetostaticPairOperator {
            descriptor: descriptor("transform.benchmark_magnetostatic_pair"),
        })
        .expect("transform.benchmark_magnetostatic_pair should register");
    registry
        .register_json(ScoreMagnetostaticQualityOperator {
            descriptor: descriptor("transform.score_magnetostatic_quality"),
        })
        .expect("transform.score_magnetostatic_quality should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
