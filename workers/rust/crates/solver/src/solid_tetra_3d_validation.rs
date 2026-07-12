use kyuubiki_protocol::{SolidTetra3dElementInput, SolveSolidTetra3dRequest};

pub(crate) fn validate_request(request: &SolveSolidTetra3dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("solid tetra 3d model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("solid tetra 3d model must define at least one element".to_string());
    }
    if constrained_dof_count(request) < 6 {
        return Err(
            "solid tetra 3d model must restrain at least six degrees of freedom".to_string(),
        );
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite() && node.z.is_finite()) {
            return Err(format!(
                "solid tetra 3d node {index} coordinates must be finite"
            ));
        }
        if !(node.load_x.is_finite() && node.load_y.is_finite() && node.load_z.is_finite()) {
            return Err(format!("solid tetra 3d node {index} loads must be finite"));
        }
    }
    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn constrained_dof_count(request: &SolveSolidTetra3dRequest) -> usize {
    request
        .nodes
        .iter()
        .map(|node| node.fix_x as usize + node.fix_y as usize + node.fix_z as usize)
        .sum()
}

fn validate_element(
    request: &SolveSolidTetra3dRequest,
    element: &SolidTetra3dElementInput,
) -> Result<(), String> {
    let node_indices = [
        element.node_a,
        element.node_b,
        element.node_c,
        element.node_d,
    ];
    for index in node_indices {
        if index >= request.nodes.len() {
            return Err(format!(
                "solid tetra element {} references missing node {}",
                element.id, index
            ));
        }
    }
    for left in 0..node_indices.len() {
        for right in (left + 1)..node_indices.len() {
            if node_indices[left] == node_indices[right] {
                return Err(format!(
                    "solid tetra element {} must reference four distinct nodes",
                    element.id
                ));
            }
        }
    }
    if !element.youngs_modulus.is_finite() {
        return Err(format!(
            "solid tetra element {} youngs_modulus must be finite",
            element.id
        ));
    }
    if element.youngs_modulus <= 0.0 {
        return Err(format!(
            "solid tetra element {} must have positive youngs_modulus",
            element.id
        ));
    }
    if !element.poisson_ratio.is_finite() {
        return Err(format!(
            "solid tetra element {} poisson_ratio must be finite",
            element.id
        ));
    }
    if !(element.poisson_ratio > -1.0 && element.poisson_ratio < 0.5) {
        return Err(format!(
            "solid tetra element {} must have poisson_ratio in (-1, 0.5)",
            element.id
        ));
    }
    validate_positive_volume(request, element)
}

fn validate_positive_volume(
    request: &SolveSolidTetra3dRequest,
    element: &SolidTetra3dElementInput,
) -> Result<(), String> {
    let points = [
        element.node_a,
        element.node_b,
        element.node_c,
        element.node_d,
    ]
    .map(|index| {
        let node = &request.nodes[index];
        [node.x, node.y, node.z]
    });
    let volume = tetra_volume(points);
    if !(volume.is_finite() && volume > 1.0e-18) {
        return Err(format!(
            "solid tetra element {} has zero volume",
            element.id
        ));
    }
    Ok(())
}

fn tetra_volume(points: [[f64; 3]; 4]) -> f64 {
    let ax = points[1][0] - points[0][0];
    let ay = points[1][1] - points[0][1];
    let az = points[1][2] - points[0][2];
    let bx = points[2][0] - points[0][0];
    let by = points[2][1] - points[0][1];
    let bz = points[2][2] - points[0][2];
    let cx = points[3][0] - points[0][0];
    let cy = points[3][1] - points[0][1];
    let cz = points[3][2] - points[0][2];
    let triple = ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx);
    triple.abs() / 6.0
}
