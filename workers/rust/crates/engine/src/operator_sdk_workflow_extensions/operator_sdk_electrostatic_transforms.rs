use crate::catalog::describe_built_in_operator;
use crate::electrostatic_quality::score_electrostatic_quality;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::workflow_guard_transforms::{
    benchmark_electrostatic_pair, evaluate_electrostatic_guard,
};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateElectrostaticGuardOperator {
    descriptor: OperatorDescriptor,
}

struct BenchmarkElectrostaticPairOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreElectrostaticQualityOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateElectrostaticGuardOperator {
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
            evaluate_electrostatic_guard(input.payload, input.config),
        )
    }
}

impl JsonOperator for BenchmarkElectrostaticPairOperator {
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
            benchmark_electrostatic_pair(input.payload, input.config),
        )
    }
}

impl JsonOperator for ScoreElectrostaticQualityOperator {
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
            score_electrostatic_quality(input.payload, input.config),
        )
    }
}

pub(super) fn register_electrostatic_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateElectrostaticGuardOperator {
            descriptor: descriptor("transform.evaluate_electrostatic_guard"),
        })
        .expect("transform.evaluate_electrostatic_guard should register");
    registry
        .register_json(BenchmarkElectrostaticPairOperator {
            descriptor: descriptor("transform.benchmark_electrostatic_pair"),
        })
        .expect("transform.benchmark_electrostatic_pair should register");
    registry
        .register_json(ScoreElectrostaticQualityOperator {
            descriptor: descriptor("transform.score_electrostatic_quality"),
        })
        .expect("transform.score_electrostatic_quality should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
