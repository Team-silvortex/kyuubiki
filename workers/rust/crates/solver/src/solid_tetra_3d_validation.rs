use kyuubiki_protocol::{SolidTetra3dElementInput, SolveSolidTetra3dRequest};
use std::collections::BTreeMap;

pub(crate) fn validate_request(request: &SolveSolidTetra3dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("solid tetra 3d model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("solid tetra 3d model must define at least one element".to_string());
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
    validate_mesh_topology_and_restraints(request)
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
    if !(volume.is_finite() && volume > 0.0) {
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

pub(crate) fn mesh_component_count(request: &SolveSolidTetra3dRequest) -> usize {
    mesh_components(request).len()
}

fn validate_mesh_topology_and_restraints(request: &SolveSolidTetra3dRequest) -> Result<(), String> {
    let mut referenced = vec![false; request.nodes.len()];
    for element in &request.elements {
        for node in [
            element.node_a,
            element.node_b,
            element.node_c,
            element.node_d,
        ] {
            referenced[node] = true;
        }
    }
    if let Some(index) = referenced.iter().position(|is_referenced| !is_referenced) {
        return Err(format!(
            "solid tetra 3d node {index} ({}) is not referenced by any element",
            request.nodes[index].id,
        ));
    }

    for (component_index, component) in mesh_components(request).iter().enumerate() {
        let rank = rigid_body_restraint_rank(request, component);
        if rank < 6 {
            let first_node = component[0];
            return Err(format!(
                "solid tetra component {component_index} (first node {}:{}) restrains rigid-body rank {rank}/6",
                first_node, request.nodes[first_node].id,
            ));
        }
    }
    Ok(())
}

fn mesh_components(request: &SolveSolidTetra3dRequest) -> Vec<Vec<usize>> {
    let mut parent = (0..request.nodes.len()).collect::<Vec<_>>();
    for element in &request.elements {
        let nodes = [
            element.node_a,
            element.node_b,
            element.node_c,
            element.node_d,
        ];
        for &node in &nodes[1..] {
            union(&mut parent, nodes[0], node);
        }
    }

    let mut grouped = BTreeMap::<usize, Vec<usize>>::new();
    for node in 0..request.nodes.len() {
        let root = find(&mut parent, node);
        grouped.entry(root).or_default().push(node);
    }
    let mut components = grouped.into_values().collect::<Vec<_>>();
    components.sort_by_key(|nodes| nodes[0]);
    components
}

fn find(parent: &mut [usize], node: usize) -> usize {
    if parent[node] != node {
        parent[node] = find(parent, parent[node]);
    }
    parent[node]
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find(parent, left);
    let right_root = find(parent, right);
    if left_root != right_root {
        let (root, child) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parent[child] = root;
    }
}

fn rigid_body_restraint_rank(request: &SolveSolidTetra3dRequest, component: &[usize]) -> usize {
    let centroid = component.iter().fold([0.0; 3], |mut sum, &index| {
        let node = &request.nodes[index];
        sum[0] += node.x;
        sum[1] += node.y;
        sum[2] += node.z;
        sum
    });
    let centroid = centroid.map(|value| value / component.len() as f64);
    let scale = component
        .iter()
        .map(|&index| {
            let node = &request.nodes[index];
            ((node.x - centroid[0]).powi(2)
                + (node.y - centroid[1]).powi(2)
                + (node.z - centroid[2]).powi(2))
            .sqrt()
        })
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);
    let mut rows = Vec::<[f64; 6]>::new();
    for &index in component {
        let node = &request.nodes[index];
        let x = (node.x - centroid[0]) / scale;
        let y = (node.y - centroid[1]) / scale;
        let z = (node.z - centroid[2]) / scale;
        if node.fix_x {
            rows.push([1.0, 0.0, 0.0, 0.0, z, -y]);
        }
        if node.fix_y {
            rows.push([0.0, 1.0, 0.0, -z, 0.0, x]);
        }
        if node.fix_z {
            rows.push([0.0, 0.0, 1.0, y, -x, 0.0]);
        }
    }
    matrix_rank(&mut rows)
}

fn matrix_rank(rows: &mut [[f64; 6]]) -> usize {
    let mut rank = 0;
    for column in 0..6 {
        let pivot = (rank..rows.len()).max_by(|&left, &right| {
            rows[left][column]
                .abs()
                .total_cmp(&rows[right][column].abs())
        });
        let Some(pivot) = pivot else {
            break;
        };
        if rows[pivot][column].abs() <= 1.0e-10 {
            continue;
        }
        rows.swap(rank, pivot);
        let divisor = rows[rank][column];
        for value in &mut rows[rank][column..] {
            *value /= divisor;
        }
        for row in 0..rows.len() {
            if row == rank {
                continue;
            }
            let factor = rows[row][column];
            for entry in column..6 {
                rows[row][entry] -= factor * rows[rank][entry];
            }
        }
        rank += 1;
    }
    rank
}
