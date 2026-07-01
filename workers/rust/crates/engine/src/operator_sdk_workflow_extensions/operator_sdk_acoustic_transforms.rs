use crate::acoustic_quality::score_acoustic_quality;
use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::workflow_guard_transforms::{benchmark_acoustic_pair, evaluate_acoustic_guard};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateAcousticGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkAcousticPairOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreAcousticQualityOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateAcousticGuardOperator {
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
            evaluate_acoustic_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkAcousticPairOperator {
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
            benchmark_acoustic_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ScoreAcousticQualityOperator {
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
            score_acoustic_quality(input.payload, input.config),
        )
    }
}

pub(super) fn register_acoustic_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateAcousticGuardOperator {
            descriptor: descriptor("transform.evaluate_acoustic_guard"),
        })
        .expect("transform.evaluate_acoustic_guard should register");
    registry
        .register_json(BenchmarkAcousticPairOperator {
            descriptor: descriptor("transform.benchmark_acoustic_pair"),
        })
        .expect("transform.benchmark_acoustic_pair should register");
    registry
        .register_json(ScoreAcousticQualityOperator {
            descriptor: descriptor("transform.score_acoustic_quality"),
        })
        .expect("transform.score_acoustic_quality should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
