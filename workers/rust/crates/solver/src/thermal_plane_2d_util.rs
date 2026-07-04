use kyuubiki_protocol::{
    PlaneNodeInput, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
};

pub(crate) fn build_force_vector(request: &SolveThermalPlaneTriangle2dRequest) -> Vec<f64> {
    build_force_vector_from_nodes(&request.nodes)
}

pub(crate) fn build_quad_force_vector(request: &SolveThermalPlaneQuad2dRequest) -> Vec<f64> {
    build_force_vector_from_nodes(&request.nodes)
}

fn build_force_vector_from_nodes(nodes: &[kyuubiki_protocol::ThermalPlaneNodeInput]) -> Vec<f64> {
    let mut force_vector = vec![0.0; nodes.len() * 2];
    for (index, node) in nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }
    force_vector
}

pub(crate) fn to_plane_nodes(request: &SolveThermalPlaneTriangle2dRequest) -> Vec<PlaneNodeInput> {
    request
        .nodes
        .iter()
        .map(|node| PlaneNodeInput {
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            fix_x: node.fix_x,
            fix_y: node.fix_y,
            load_x: node.load_x,
            load_y: node.load_y,
        })
        .collect()
}

pub(crate) fn to_triangle_request(
    request: &SolveThermalPlaneQuad2dRequest,
) -> SolveThermalPlaneTriangle2dRequest {
    SolveThermalPlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    }
}

pub(crate) fn triangle_dof_map(node_i: usize, node_j: usize, node_k: usize) -> [usize; 6] {
    [
        node_i * 2,
        node_i * 2 + 1,
        node_j * 2,
        node_j * 2 + 1,
        node_k * 2,
        node_k * 2 + 1,
    ]
}

pub(crate) fn triangle_displacements(
    displacements: &[f64],
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> [f64; 6] {
    let map = triangle_dof_map(node_i, node_j, node_k);
    std::array::from_fn(|index| displacements[map[index]])
}
