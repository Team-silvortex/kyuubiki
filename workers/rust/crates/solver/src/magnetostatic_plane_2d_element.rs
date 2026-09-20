pub(super) use crate::scalar_plane_kernel::{
    ScalarQuad as QuadComputed, ScalarTriangle as TriangleComputed, scalar_gradient,
};
use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneTriangleElementInput, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneTriangle2dRequest,
};

pub(super) fn precompute_triangle_element(
    request: &SolveMagnetostaticPlaneTriangle2dRequest,
    element: &MagnetostaticPlaneTriangleElementInput,
) -> Result<TriangleComputed, String> {
    crate::scalar_plane_kernel::triangle(
        [
            point(&request.nodes[element.node_i]),
            point(&request.nodes[element.node_j]),
            point(&request.nodes[element.node_k]),
        ],
        element.thickness,
        1.0 / element.permeability,
        "magnetostatic plane",
    )
}

pub(super) fn precompute_quad_element(
    request: &SolveMagnetostaticPlaneQuad2dRequest,
    element: &MagnetostaticPlaneQuadElementInput,
) -> Result<QuadComputed, String> {
    crate::scalar_plane_kernel::quad(
        [
            point(&request.nodes[element.node_i]),
            point(&request.nodes[element.node_j]),
            point(&request.nodes[element.node_k]),
            point(&request.nodes[element.node_l]),
        ],
        element.thickness,
        1.0 / element.permeability,
        "magnetostatic plane",
    )
}

fn point(node: &MagnetostaticPlaneNodeInput) -> [f64; 2] {
    [node.x, node.y]
}
