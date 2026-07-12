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
    let first = HeatPlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        conductivity: element.conductivity,
    };
    let second = HeatPlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        conductivity: element.conductivity,
    };
    Ok(HeatPlaneQuadComputed {
        first: precompute_heat_plane_triangle_element_from_nodes(&request.nodes, &first)?,
        second: precompute_heat_plane_triangle_element_from_nodes(&request.nodes, &second)?,
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
    let node_i = &nodes[element.node_i];
    let node_j = &nodes[element.node_j];
    let node_k = &nodes[element.node_k];
    let signed_area = 0.5
        * ((node_j.x - node_i.x) * (node_k.y - node_i.y)
            - (node_k.x - node_i.x) * (node_j.y - node_i.y));
    let area = signed_area.abs();
    if area <= 1.0e-12 {
        return Err("heat plane triangle element area must be positive".to_string());
    }

    let twice_area = signed_area * 2.0;
    let gradient_x = [
        (node_j.y - node_k.y) / twice_area,
        (node_k.y - node_i.y) / twice_area,
        (node_i.y - node_j.y) / twice_area,
    ];
    let gradient_y = [
        (node_k.x - node_j.x) / twice_area,
        (node_i.x - node_k.x) / twice_area,
        (node_j.x - node_i.x) / twice_area,
    ];

    let scale = element.conductivity * element.thickness * area;
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
