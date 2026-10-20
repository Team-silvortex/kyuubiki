use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use kyuubiki_protocol::{
    NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput, SolveContactGap1dRequest,
    SolveNonlinearSpring1dRequest,
};

pub(crate) fn validate_request(request: &SolveNonlinearSpring1dRequest) -> Result<(), String> {
    validate_springs(&request.nodes, &request.elements, request.tolerance)
}

fn validate_springs(
    nodes: &[NonlinearSpring1dNodeInput],
    elements: &[NonlinearSpring1dElementInput],
    tolerance: Option<f64>,
) -> Result<(), String> {
    checkpoint(SolverStage::ElementPrecompute, 0)?;
    if nodes.len() < 2 {
        return Err("nonlinear spring model must define at least two nodes".to_string());
    }
    if elements.is_empty() {
        return Err("nonlinear spring model must define at least one element".to_string());
    }
    if tolerance.is_some_and(|value| !(value.is_finite() && value > 0.0)) {
        return Err("nonlinear spring tolerance must be positive".to_string());
    }

    let mut has_restraint = false;
    for (index, node) in nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.load_x.is_finite()) {
            return Err(format!(
                "nonlinear spring node {index} coordinates and loads must be finite"
            ));
        }
        has_restraint |= node.fix_x;
        checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, nodes.len())?;
    }
    if !has_restraint {
        return Err(
            "nonlinear spring model must restrain at least one degree of freedom".to_string(),
        );
    }

    for (index, element) in elements.iter().enumerate() {
        if element.node_i >= nodes.len() || element.node_j >= nodes.len() {
            return Err("nonlinear spring element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("nonlinear spring element must connect two distinct nodes".to_string());
        }
        if !(element.stiffness.is_finite() && element.stiffness > 0.0) {
            return Err("nonlinear spring element stiffness must be positive".to_string());
        }
        if !element.cubic_stiffness.is_finite() {
            return Err("nonlinear spring cubic stiffness must be finite".to_string());
        }
        // k and c have different units; k + c at an arbitrary unit extension
        // cannot certify the displacement-dependent tangent or material stability.
        let length = (nodes[element.node_j].x - nodes[element.node_i].x).abs();
        if !(length.is_finite() && length > 0.0) {
            return Err("nonlinear spring element length must be positive".to_string());
        }
        checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, elements.len())?;
    }

    validate_restraints(nodes, elements)
}

pub(crate) fn validate_contact_request(request: &SolveContactGap1dRequest) -> Result<(), String> {
    validate_springs(&request.nodes, &request.elements, request.tolerance)?;

    if request.contacts.is_empty() {
        return Err("contact gap model must define at least one contact".to_string());
    }

    for (index, contact) in request.contacts.iter().enumerate() {
        if contact.node >= request.nodes.len() {
            return Err("contact gap references an out-of-range node".to_string());
        }
        if !(contact.gap.is_finite() && contact.gap >= 0.0) {
            return Err("contact gap must be finite and non-negative".to_string());
        }
        if !(contact.normal_stiffness.is_finite() && contact.normal_stiffness > 0.0) {
            return Err("contact normal stiffness must be positive".to_string());
        }
        checkpoint_chunk(
            SolverStage::ElementPrecompute,
            index + 1,
            request.contacts.len(),
        )?;
    }

    Ok(())
}

fn validate_restraints(
    nodes: &[NonlinearSpring1dNodeInput],
    elements: &[NonlinearSpring1dElementInput],
) -> Result<(), String> {
    // The load path starts at zero displacement with all non-negative gaps inactive.
    // A contact cannot anchor that initial tangent; each spring component needs fix_x.
    checkpoint(SolverStage::ConstraintIndex, 0)?;
    let mut parent = Vec::with_capacity(nodes.len());
    let mut anchored = Vec::with_capacity(nodes.len());
    let mut rank = vec![0_u8; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        parent.push(index);
        anchored.push(node.fix_x);
        checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, nodes.len())?;
    }

    checkpoint(SolverStage::ConstraintMap, 0)?;
    for (index, element) in elements.iter().enumerate() {
        let mut left = find_root(&mut parent, element.node_i);
        let mut right = find_root(&mut parent, element.node_j);
        if left != right {
            if rank[left] < rank[right] {
                std::mem::swap(&mut left, &mut right);
            }
            parent[right] = left;
            anchored[left] |= anchored[right];
            if rank[left] == rank[right] {
                rank[left] += 1;
            }
        }
        checkpoint_chunk(SolverStage::ConstraintMap, index + 1, elements.len())?;
    }

    checkpoint(SolverStage::ConstraintReduce, 0)?;
    for (index, node) in nodes.iter().enumerate() {
        if !anchored[find_root(&mut parent, index)] {
            return Err(format!(
                "nonlinear spring component containing node {index} ({}) has unrestrained rigid-body translation; each component must include a fixed node before load stepping (inactive gap contacts are not restraints)",
                node.id,
            ));
        }
        checkpoint_chunk(SolverStage::ConstraintReduce, index + 1, nodes.len())?;
    }
    Ok(())
}

fn find_root(parent: &mut [usize], mut node: usize) -> usize {
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}
