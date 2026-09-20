pub(super) use crate::scalar_plane_kernel::{
    ScalarQuad as ElectrostaticPlaneQuadComputed,
    ScalarTriangle as ElectrostaticPlaneTriangleComputed,
    scalar_gradient as plane_triangle_scalar_gradient,
};
use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneTriangleElementInput, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest,
};

pub(super) fn precompute_electrostatic_plane_triangle_element(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
    element: &ElectrostaticPlaneTriangleElementInput,
) -> Result<ElectrostaticPlaneTriangleComputed, String> {
    crate::scalar_plane_kernel::triangle(
        [
            point(&request.nodes[element.node_i]),
            point(&request.nodes[element.node_j]),
            point(&request.nodes[element.node_k]),
        ],
        element.thickness,
        element.permittivity,
        "electrostatic plane",
    )
}

pub(super) fn precompute_electrostatic_plane_quad_element(
    request: &SolveElectrostaticPlaneQuad2dRequest,
    element: &ElectrostaticPlaneQuadElementInput,
) -> Result<ElectrostaticPlaneQuadComputed, String> {
    crate::scalar_plane_kernel::quad(
        [
            point(&request.nodes[element.node_i]),
            point(&request.nodes[element.node_j]),
            point(&request.nodes[element.node_k]),
            point(&request.nodes[element.node_l]),
        ],
        element.thickness,
        element.permittivity,
        "electrostatic plane",
    )
}

fn point(node: &ElectrostaticPlaneNodeInput) -> [f64; 2] {
    [node.x, node.y]
}
