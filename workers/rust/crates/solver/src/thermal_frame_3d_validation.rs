use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
};

pub(crate) fn validate_request(request: &SolveThermalFrame3dRequest) -> Result<(), String> {
    checkpoint(SolverStage::ElementPrecompute, 0)?;
    if request.nodes.len() < 2 {
        return Err("thermal 3d frame must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal 3d frame must define at least one element".to_string());
    }
    let count = request.nodes.len()
        + request.elements.len()
        + request.directional_springs.len()
        + request.directional_rotational_springs.len()
        + request.directional_constraints.len()
        + request.directional_rotational_constraints.len();
    let mut completed = 0;
    let mut constrained =
        request.directional_constraints.len() + request.directional_rotational_constraints.len();
    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() || !node.z.is_finite() {
            return Err(format!(
                "thermal 3d frame node {index} has invalid coordinates"
            ));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.load_z.is_finite() {
            return Err(format!("thermal 3d frame node {index} has invalid load"));
        }
        if !node.moment_x.is_finite() || !node.moment_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("thermal 3d frame node {index} has invalid moment"));
        }
        if !node.temperature_delta.is_finite() {
            return Err("thermal 3d frame node temperature_delta must be finite".to_string());
        }
        validate_nodal_constraint_loads(index, node)?;
        constrained += [
            node.fix_x,
            node.fix_y,
            node.fix_z,
            node.fix_rx,
            node.fix_ry,
            node.fix_rz,
        ]
        .into_iter()
        .filter(|v| *v)
        .count();
        completed += 1;
        checkpoint_chunk(SolverStage::ElementPrecompute, completed, count)?;
    }

    if constrained < 6 {
        return Err("thermal 3d frame must restrain at least six degrees of freedom".to_string());
    }
    for element in &request.elements {
        validate_element(request, element)?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementPrecompute, completed, count)?;
    }
    for spring in &request.directional_springs {
        validate_directional_spring(
            spring.node,
            spring.direction,
            spring.stiffness,
            request.nodes.len(),
            "directional spring",
        )?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementPrecompute, completed, count)?;
    }
    for spring in &request.directional_rotational_springs {
        validate_directional_spring(
            spring.node,
            spring.direction,
            spring.stiffness,
            request.nodes.len(),
            "directional rotational spring",
        )?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementPrecompute, completed, count)?;
    }
    for constraint in &request.directional_constraints {
        validate_directional_constraint(
            constraint.node,
            constraint.direction,
            request.nodes.len(),
            "directional constraint",
        )?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementPrecompute, completed, count)?;
    }
    for constraint in &request.directional_rotational_constraints {
        validate_directional_constraint(
            constraint.node,
            constraint.direction,
            request.nodes.len(),
            "directional rotational constraint",
        )?;
        completed += 1;
        checkpoint_chunk(SolverStage::ElementPrecompute, completed, count)?;
    }
    Ok(())
}

fn validate_nodal_constraint_loads(
    index: usize,
    node: &ThermalFrame3dNodeInput,
) -> Result<(), String> {
    for (fixed, value, load, degree) in [
        (node.fix_x, node.load_x, "load_x", "x"),
        (node.fix_y, node.load_y, "load_y", "y"),
        (node.fix_z, node.load_z, "load_z", "z"),
        (node.fix_rx, node.moment_x, "moment_x", "rx"),
        (node.fix_ry, node.moment_y, "moment_y", "ry"),
        (node.fix_rz, node.moment_z, "moment_z", "rz"),
    ] {
        if fixed && value != 0.0 {
            return Err(format!(
                "thermal 3d frame node {index} ({}) applies non-zero {load} to fixed {degree} degree of freedom; move the load to an unconstrained degree of freedom",
                node.id
            ));
        }
    }
    Ok(())
}

fn validate_directional_constraint(
    node: usize,
    direction: [f64; 3],
    node_count: usize,
    kind: &str,
) -> Result<(), String> {
    if node >= node_count {
        return Err(format!(
            "thermal 3d frame {kind} references an out-of-range node"
        ));
    }
    normalized_direction(direction).map_err(|error| format!("thermal 3d frame {kind} {error}"))?;
    Ok(())
}

fn validate_directional_spring(
    node: usize,
    direction: [f64; 3],
    stiffness: f64,
    node_count: usize,
    kind: &str,
) -> Result<(), String> {
    if node >= node_count {
        return Err(format!(
            "thermal 3d frame {kind} references an out-of-range node"
        ));
    }
    if !(stiffness.is_finite() && stiffness > 0.0) {
        return Err(format!(
            "thermal 3d frame {kind} stiffness must be positive"
        ));
    }
    normalized_direction(direction).map_err(|error| format!("thermal 3d frame {kind} {error}"))?;
    Ok(())
}

fn validate_element(
    request: &SolveThermalFrame3dRequest,
    element: &ThermalFrame3dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("thermal 3d frame element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("thermal 3d frame element cannot connect a node to itself".to_string());
    }
    validate_positive_frame_properties(element)?;
    if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
        return Err("thermal 3d frame element thermal_expansion must be non-negative".to_string());
    }
    if !(element.section_depth_y.is_finite() && element.section_depth_y > 0.0) {
        return Err("thermal 3d frame element section_depth_y must be positive".to_string());
    }
    if !(element.section_depth_z.is_finite() && element.section_depth_z > 0.0) {
        return Err("thermal 3d frame element section_depth_z must be positive".to_string());
    }
    if !element.temperature_gradient_y.is_finite() {
        return Err("thermal 3d frame element temperature_gradient_y must be finite".to_string());
    }
    if !element.temperature_gradient_z.is_finite() {
        return Err("thermal 3d frame element temperature_gradient_z must be finite".to_string());
    }

    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let dx = node_j.x - node_i.x;
    let dy = node_j.y - node_i.y;
    let dz = node_j.z - node_i.z;
    let length = dx.hypot(dy).hypot(dz);
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("3d frame element length must be positive".to_string());
    }
    frame3d_rotation_with_local_y(dx, dy, dz, length, element.local_y_axis)?;
    Ok(())
}

fn validate_positive_frame_properties(element: &ThermalFrame3dElementInput) -> Result<(), String> {
    for (label, value) in [
        ("area", element.area),
        ("youngs_modulus", element.youngs_modulus),
        ("shear_modulus", element.shear_modulus),
        ("torsion_constant", element.torsion_constant),
        ("moment_of_inertia_y", element.moment_of_inertia_y),
        ("moment_of_inertia_z", element.moment_of_inertia_z),
        ("section_modulus_y", element.section_modulus_y),
        ("section_modulus_z", element.section_modulus_z),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(format!("thermal 3d frame element {label} must be positive"));
        }
    }
    Ok(())
}

use crate::frame_3d_math::{frame3d_rotation_with_local_y, normalized_direction};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
