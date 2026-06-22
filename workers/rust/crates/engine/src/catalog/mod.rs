mod builtins;
mod descriptors;
mod reporting;

use kyuubiki_protocol::OperatorDescriptor;

pub fn built_in_operator_descriptors() -> Vec<OperatorDescriptor> {
    builtins::built_in_operator_descriptors()
}

pub fn describe_built_in_operator(id: &str) -> Option<OperatorDescriptor> {
    built_in_operator_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.id == id)
}
