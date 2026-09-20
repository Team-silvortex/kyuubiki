pub(super) use crate::scalar_plane_kernel::{
    ScalarQuad as HeatPlaneQuadComputed, ScalarTriangle as HeatPlaneTriangleComputed,
    scalar_gradient as plane_triangle_scalar_gradient,
};
use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
};

pub(super) fn precompute_heat_plane_triangle_element(
    request: &SolveHeatPlaneTriangle2dRequest,
    element: &HeatPlaneTriangleElementInput,
) -> Result<HeatPlaneTriangleComputed, String> {
    crate::scalar_plane_kernel::triangle(
        [
            point(&request.nodes[element.node_i]),
            point(&request.nodes[element.node_j]),
            point(&request.nodes[element.node_k]),
        ],
        element.thickness,
        element.conductivity,
        "heat plane",
    )
}

pub(super) fn precompute_heat_plane_quad_element(
    request: &SolveHeatPlaneQuad2dRequest,
    element: &HeatPlaneQuadElementInput,
) -> Result<HeatPlaneQuadComputed, String> {
    precompute_heat_plane_quad_from_coordinates(
        [
            point(&request.nodes[element.node_i]),
            point(&request.nodes[element.node_j]),
            point(&request.nodes[element.node_k]),
            point(&request.nodes[element.node_l]),
        ],
        element.thickness,
        element.conductivity,
    )
}

pub(super) fn precompute_heat_plane_quad_from_coordinates(
    points: [[f64; 2]; 4],
    thickness: f64,
    conductivity: f64,
) -> Result<HeatPlaneQuadComputed, String> {
    crate::scalar_plane_kernel::quad(points, thickness, conductivity, "heat plane")
}

fn point(node: &HeatPlaneNodeInput) -> [f64; 2] {
    [node.x, node.y]
}
