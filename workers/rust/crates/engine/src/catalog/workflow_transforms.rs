use crate::catalog::workflow_transform_diagnostics::workflow_diagnostics_transform_descriptors;
use crate::catalog::workflow_transform_physics::workflow_physics_transform_descriptors;
use crate::catalog::workflow_transform_summary::workflow_summary_transform_descriptors;
use kyuubiki_protocol::OperatorDescriptor;

pub(super) fn workflow_transform_descriptors() -> Vec<OperatorDescriptor> {
    let mut descriptors = workflow_summary_transform_descriptors();
    descriptors.extend(workflow_physics_transform_descriptors());
    descriptors.extend(workflow_diagnostics_transform_descriptors());
    descriptors
}
