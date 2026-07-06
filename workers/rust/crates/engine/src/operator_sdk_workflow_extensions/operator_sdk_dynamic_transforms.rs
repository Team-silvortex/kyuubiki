use crate::catalog::describe_built_in_operator;
use crate::dynamic_quality::score_dynamic_quality;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct ScoreDynamicQualityOperator {
    descriptor: OperatorDescriptor,
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
