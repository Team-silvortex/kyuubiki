mod bridges;
mod builtins;
mod descriptors;
mod reporting;
mod workflow_transform_diagnostics;
mod workflow_transform_physics;
mod workflow_transform_summary;
mod workflow_transforms;

use kyuubiki_protocol::OperatorDescriptor;

pub fn built_in_operator_descriptors() -> Vec<OperatorDescriptor> {
    builtins::built_in_operator_descriptors()
}

pub fn describe_built_in_operator(id: &str) -> Option<OperatorDescriptor> {
    built_in_operator_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
}
