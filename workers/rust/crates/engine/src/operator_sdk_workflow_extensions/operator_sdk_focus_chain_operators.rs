use crate::catalog::describe_built_in_operator;
use crate::operator_sdk_runtime::{run_summary_only, WorkflowOperatorEnvelope};
use crate::workflow_focus_chain::{
    compose_focus_bridge_request, compose_focus_chain_input, execute_focus_bridge_execution,
    resolve_focus_bridge_execution, select_focus_payload,
};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct SelectFocusPayloadOperator {
    descriptor: OperatorDescriptor,
}
struct ComposeFocusChainInputOperator {
    descriptor: OperatorDescriptor,
}
struct ComposeFocusBridgeRequestOperator {
    descriptor: OperatorDescriptor,
}
struct ResolveFocusBridgeExecutionOperator {
    descriptor: OperatorDescriptor,
}
struct ExecuteFocusBridgeExecutionOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for SelectFocusPayloadOperator {
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
            select_focus_payload(input.payload, input.config),
        )
    }
}

impl JsonOperator for ComposeFocusChainInputOperator {
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
            compose_focus_chain_input(input.payload, input.config),
        )
    }
}

impl JsonOperator for ComposeFocusBridgeRequestOperator {
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
            compose_focus_bridge_request(input.payload, input.config),
        )
    }
}

impl JsonOperator for ResolveFocusBridgeExecutionOperator {
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
            resolve_focus_bridge_execution(input.payload, input.config),
        )
    }
}

impl JsonOperator for ExecuteFocusBridgeExecutionOperator {
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
            execute_focus_bridge_execution(input.payload, input.config),
        )
    }
}

pub(super) fn register_focus_chain_transform_extensions(registry: &mut OperatorRegistry) {
    registry
        .register_json(SelectFocusPayloadOperator {
            descriptor: descriptor("transform.select_focus_payload"),
        })
        .expect("transform.select_focus_payload should register");
    registry
        .register_json(ComposeFocusChainInputOperator {
            descriptor: descriptor("transform.compose_focus_chain_input"),
        })
        .expect("transform.compose_focus_chain_input should register");
    registry
        .register_json(ComposeFocusBridgeRequestOperator {
            descriptor: descriptor("transform.compose_focus_bridge_request"),
        })
        .expect("transform.compose_focus_bridge_request should register");
    registry
        .register_json(ResolveFocusBridgeExecutionOperator {
            descriptor: descriptor("transform.resolve_focus_bridge_execution"),
        })
        .expect("transform.resolve_focus_bridge_execution should register");
    registry
        .register_json(ExecuteFocusBridgeExecutionOperator {
            descriptor: descriptor("transform.execute_focus_bridge_execution"),
        })
        .expect("transform.execute_focus_bridge_execution should register");
}

fn descriptor(operator_id: &str) -> OperatorDescriptor {
    describe_built_in_operator(operator_id)
        .unwrap_or_else(|| panic!("missing built-in descriptor for {operator_id}"))
}
