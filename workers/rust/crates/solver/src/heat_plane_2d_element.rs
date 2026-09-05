use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
};

#[derive(Debug, Clone)]
pub(super) struct HeatPlaneTriangleComputed {
    pub stiffness: [[f64; 3]; 3],
    pub area: f64,
    pub gradient_x: [f64; 3],
    pub gradient_y: [f64; 3],
}

#[derive(Debug, Clone)]
pub(super) struct HeatPlaneQuadComputed {
    pub first: HeatPlaneTriangleComputed,
    pub second: HeatPlaneTriangleComputed,
}

pub(super) fn precompute_heat_plane_triangle_element(
    request: &SolveHeatPlaneTriangle2dRequest,
    element: &HeatPlaneTriangleElementInput,
) -> Result<HeatPlaneTriangleComputed, String> {
    precompute_heat_plane_triangle_element_from_nodes(&request.nodes, element)
}

pub(super) fn precompute_heat_plane_quad_element(
    request: &SolveHeatPlaneQuad2dRequest,
    element: &HeatPlaneQuadElementInput,
) -> Result<HeatPlaneQuadComputed, String> {
    let points = [
        point(&request.nodes[element.node_i]),
        point(&request.nodes[element.node_j]),
        point(&request.nodes[element.node_k]),
        point(&request.nodes[element.node_l]),
    ];
    precompute_heat_plane_quad_from_coordinates(points, element.thickness, element.conductivity)
}

pub(super) fn precompute_heat_plane_quad_from_coordinates(
    points: [[f64; 2]; 4],
    thickness: f64,
    conductivity: f64,
) -> Result<HeatPlaneQuadComputed, String> {
    Ok(HeatPlaneQuadComputed {
        first: precompute_heat_plane_triangle_from_coordinates(
            [points[0], points[1], points[2]],
            thickness,
            conductivity,
        )?,
        second: precompute_heat_plane_triangle_from_coordinates(
            [points[0], points[2], points[3]],
            thickness,
            conductivity,
        )?,
    })
}

pub(super) fn plane_triangle_scalar_gradient(
    gradient_x: &[f64; 3],
    gradient_y: &[f64; 3],
    nodal_values: &[f64; 3],
) -> [f64; 2] {
    [
        (0..3)
            .map(|index| gradient_x[index] * nodal_values[index])
            .sum(),
        (0..3)
            .map(|index| gradient_y[index] * nodal_values[index])
            .sum(),
    ]
}

fn precompute_heat_plane_triangle_element_from_nodes(
    nodes: &[HeatPlaneNodeInput],
    element: &HeatPlaneTriangleElementInput,
) -> Result<HeatPlaneTriangleComputed, String> {
    precompute_heat_plane_triangle_from_coordinates(
        [
            point(&nodes[element.node_i]),
            point(&nodes[element.node_j]),
            point(&nodes[element.node_k]),
        ],
        element.thickness,
        element.conductivity,
    )
}

fn precompute_heat_plane_triangle_from_coordinates(
    points: [[f64; 2]; 3],
    thickness: f64,
    conductivity: f64,
) -> Result<HeatPlaneTriangleComputed, String> {
    let [node_i, node_j, node_k] = points;
    let signed_area = 0.5
        * ((node_j[0] - node_i[0]) * (node_k[1] - node_i[1])
            - (node_k[0] - node_i[0]) * (node_j[1] - node_i[1]));
    let area = signed_area.abs();
    if area <= 1.0e-12 {
        return Err("heat plane triangle element area must be positive".to_string());
    }

    let twice_area = signed_area * 2.0;
    let gradient_x = [
        (node_j[1] - node_k[1]) / twice_area,
        (node_k[1] - node_i[1]) / twice_area,
        (node_i[1] - node_j[1]) / twice_area,
    ];
    let gradient_y = [
        (node_k[0] - node_j[0]) / twice_area,
        (node_i[0] - node_k[0]) / twice_area,
        (node_j[0] - node_i[0]) / twice_area,
    ];

    let scale = conductivity * thickness * area;
    let mut stiffness = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            stiffness[row][column] = scale
                * ((gradient_x[row] * gradient_x[column]) + (gradient_y[row] * gradient_y[column]));
        }
    }

    Ok(HeatPlaneTriangleComputed {
        stiffness,
        area,
        gradient_x,
        gradient_y,
    })
}

fn point(node: &HeatPlaneNodeInput) -> [f64; 2] {
    [node.x, node.y]
}
