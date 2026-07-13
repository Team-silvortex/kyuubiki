use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::workflow_coupled_readiness::evaluate_coupled_readiness;
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct EvaluateCoupledReadinessOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for EvaluateCoupledReadinessOperator {
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
            evaluate_coupled_readiness(input.payload, input.config),
        )
    }
}

pub(super) fn register_coupled_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(EvaluateCoupledReadinessOperator {
            descriptor: descriptor("transform.evaluate_coupled_readiness"),
        })
        .expect("transform.evaluate_coupled_readiness should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id).unwrap_or_else(|| {
        panic!("built-in descriptor missing for workflow extension operator {operator_id}")
    })
}
