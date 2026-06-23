use crate::electrostatic_plane_2d_validation::{
    validate_electrostatic_plane_quad_request, validate_electrostatic_plane_triangle_request,
};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
};
use kyuubiki_protocol::{
    ElectrostaticPlaneNodeResult, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneQuadElementResult, ElectrostaticPlaneTriangleElementInput,
    ElectrostaticPlaneTriangleElementResult, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneQuad2dResult, SolveElectrostaticPlaneTriangle2dRequest,
    SolveElectrostaticPlaneTriangle2dResult,
};

pub fn solve_electrostatic_plane_triangle_2d(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
) -> Result<SolveElectrostaticPlaneTriangle2dResult, String> {
    validate_electrostatic_plane_triangle_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_electrostatic_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        source_vector[index] = node.charge_density;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let map = [element.node_i, element.node_j, element.node_k];
        for row in 0..3 {
            for column in 0..3 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    computed.stiffness[row][column],
                );
            }
        }
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_potential.then_some((index, node.potential)))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_source, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_potentials = solve_spd_system(&reduced_stiffness, &reduced_source)?;

    let mut potentials = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        potentials[dof] = reduced_potentials[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ElectrostaticPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            potential: potentials[index],
            charge_density: node.charge_density,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_potentials = [
                potentials[element.node_i],
                potentials[element.node_j],
                potentials[element.node_k],
            ];
            let gradient = plane_triangle_scalar_gradient(
                &computed.gradient_x,
                &computed.gradient_y,
                &element_potentials,
            );
            let electric_field_x = -gradient[0];
            let electric_field_y = -gradient[1];
            let electric_flux_density_x = element.permittivity * electric_field_x;
            let electric_flux_density_y = element.permittivity * electric_field_y;

            ElectrostaticPlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                average_potential: element_potentials.iter().sum::<f64>() / 3.0,
                potential_gradient_x: gradient[0],
                potential_gradient_y: gradient[1],
                electric_field_x,
                electric_field_y,
                electric_field_magnitude: (electric_field_x * electric_field_x
                    + electric_field_y * electric_field_y)
                    .sqrt(),
                electric_flux_density_x,
                electric_flux_density_y,
                electric_flux_density_magnitude: (electric_flux_density_x
                    * electric_flux_density_x
                    + electric_flux_density_y * electric_flux_density_y)
                    .sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let max_potential = nodes
        .iter()
        .map(|node| node.potential.abs())
        .fold(0.0_f64, f64::max);
    let max_electric_field = elements
        .iter()
        .map(|element| element.electric_field_magnitude.abs())
        .fold(0.0_f64, f64::max);
    let max_flux_density = elements
        .iter()
        .map(|element| element.electric_flux_density_magnitude.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveElectrostaticPlaneTriangle2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_potential,
        max_electric_field,
        max_flux_density,
    })
}

pub fn solve_electrostatic_plane_quad_2d(
    request: &SolveElectrostaticPlaneQuad2dRequest,
) -> Result<SolveElectrostaticPlaneQuad2dResult, String> {
    validate_electrostatic_plane_quad_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_electrostatic_plane_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        source_vector[index] = node.charge_density;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let triangles = [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ];

        for (nodes, triangle) in triangles {
            let map = [nodes[0], nodes[1], nodes[2]];
            for row in 0..3 {
                for column in 0..3 {
                    add_at(
                        &mut global_stiffness,
                        map[row],
                        map[column],
                        triangle.stiffness[row][column],
                    );
                }
            }
        }
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_potential.then_some((index, node.potential)))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_source, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_potentials = solve_spd_system(&reduced_stiffness, &reduced_source)?;

    let mut potentials = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        potentials[dof] = reduced_potentials[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ElectrostaticPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            potential: potentials[index],
            charge_density: node.charge_density,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_potentials = [
                potentials[element.node_i],
                potentials[element.node_j],
                potentials[element.node_k],
            ];
            let second_potentials = [
                potentials[element.node_i],
                potentials[element.node_k],
                potentials[element.node_l],
            ];
            let first_gradient = plane_triangle_scalar_gradient(
                &computed.first.gradient_x,
                &computed.first.gradient_y,
                &first_potentials,
            );
            let second_gradient = plane_triangle_scalar_gradient(
                &computed.second.gradient_x,
                &computed.second.gradient_y,
                &second_potentials,
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };
            let potential_gradient_x = weighted(first_gradient[0], second_gradient[0]);
            let potential_gradient_y = weighted(first_gradient[1], second_gradient[1]);
            let electric_field_x = -potential_gradient_x;
            let electric_field_y = -potential_gradient_y;
            let electric_flux_density_x = element.permittivity * electric_field_x;
            let electric_flux_density_y = element.permittivity * electric_field_y;

            ElectrostaticPlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                average_potential: (potentials[element.node_i]
                    + potentials[element.node_j]
                    + potentials[element.node_k]
                    + potentials[element.node_l])
                    / 4.0,
                potential_gradient_x,
                potential_gradient_y,
                electric_field_x,
                electric_field_y,
                electric_field_magnitude: (electric_field_x * electric_field_x
                    + electric_field_y * electric_field_y)
                    .sqrt(),
                electric_flux_density_x,
                electric_flux_density_y,
                electric_flux_density_magnitude: (electric_flux_density_x
                    * electric_flux_density_x
                    + electric_flux_density_y * electric_flux_density_y)
                    .sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let max_potential = nodes
        .iter()
        .map(|node| node.potential.abs())
        .fold(0.0_f64, f64::max);
    let max_electric_field = elements
        .iter()
        .map(|element| element.electric_field_magnitude.abs())
        .fold(0.0_f64, f64::max);
    let max_flux_density = elements
        .iter()
        .map(|element| element.electric_flux_density_magnitude.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveElectrostaticPlaneQuad2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_potential,
        max_electric_field,
        max_flux_density,
    })
}

#[derive(Debug, Clone)]
struct ElectrostaticPlaneTriangleComputed {
    stiffness: [[f64; 3]; 3],
    area: f64,
    gradient_x: [f64; 3],
    gradient_y: [f64; 3],
}

#[derive(Debug, Clone)]
struct ElectrostaticPlaneQuadComputed {
    first: ElectrostaticPlaneTriangleComputed,
    second: ElectrostaticPlaneTriangleComputed,
}

fn precompute_electrostatic_plane_triangle_element(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
    element: &ElectrostaticPlaneTriangleElementInput,
) -> Result<ElectrostaticPlaneTriangleComputed, String> {
    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let node_k = &request.nodes[element.node_k];
    let signed_area = 0.5
        * ((node_j.x - node_i.x) * (node_k.y - node_i.y)
            - (node_k.x - node_i.x) * (node_j.y - node_i.y));
    let area = signed_area.abs();
    if area <= 1.0e-12 {
        return Err("electrostatic plane triangle element area must be positive".to_string());
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

    let scale = element.permittivity * element.thickness * area;
    let mut stiffness = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            stiffness[row][column] = scale
                * ((gradient_x[row] * gradient_x[column]) + (gradient_y[row] * gradient_y[column]));
        }
    }

    Ok(ElectrostaticPlaneTriangleComputed {
        stiffness,
        area,
        gradient_x,
        gradient_y,
    })
}

fn precompute_electrostatic_plane_quad_element(
    request: &SolveElectrostaticPlaneQuad2dRequest,
    element: &ElectrostaticPlaneQuadElementInput,
) -> Result<ElectrostaticPlaneQuadComputed, String> {
    let first = ElectrostaticPlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        permittivity: element.permittivity,
    };
    let second = ElectrostaticPlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        permittivity: element.permittivity,
    };
    let triangle_request = SolveElectrostaticPlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    };

    Ok(ElectrostaticPlaneQuadComputed {
        first: precompute_electrostatic_plane_triangle_element(&triangle_request, &first)?,
        second: precompute_electrostatic_plane_triangle_element(&triangle_request, &second)?,
    })
}

fn plane_triangle_scalar_gradient(
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
