use std::borrow::Cow;

use crate::electrostatic_plane_2d_element::{
    plane_triangle_scalar_gradient, precompute_electrostatic_plane_quad_element,
    precompute_electrostatic_plane_triangle_element,
};
use crate::electrostatic_plane_2d_validation::{
    validate_electrostatic_plane_quad_request, validate_electrostatic_plane_triangle_request,
};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed,
    solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use kyuubiki_protocol::{
    ElectrostaticPlaneNodeResult, ElectrostaticPlaneQuadElementResult,
    ElectrostaticPlaneTriangleElementResult, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneQuad2dResult, SolveElectrostaticPlaneTriangle2dRequest,
    SolveElectrostaticPlaneTriangle2dResult,
};

pub fn solve_electrostatic_plane_triangle_2d(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
) -> Result<SolveElectrostaticPlaneTriangle2dResult, String> {
    solve_electrostatic_plane_triangle_2d_internal(
        Cow::Borrowed(request),
        SpdSolveOptions::default(),
    )
}

pub fn solve_electrostatic_plane_triangle_2d_owned(
    request: SolveElectrostaticPlaneTriangle2dRequest,
) -> Result<SolveElectrostaticPlaneTriangle2dResult, String> {
    solve_electrostatic_plane_triangle_2d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_electrostatic_plane_triangle_2d_with_options(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveElectrostaticPlaneTriangle2dResult, String> {
    solve_electrostatic_plane_triangle_2d_internal(Cow::Borrowed(request), options)
}

fn solve_electrostatic_plane_triangle_2d_internal(
    request: Cow<'_, SolveElectrostaticPlaneTriangle2dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveElectrostaticPlaneTriangle2dResult, String> {
    validate_electrostatic_plane_triangle_request(request.as_ref())?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_electrostatic_plane_triangle_element(request.as_ref(), element))
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
    let reduced_potentials =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_source, options)?
            .solution;

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
            let electric_field_magnitude =
                (electric_field_x * electric_field_x + electric_field_y * electric_field_y).sqrt();
            let electric_flux_density_magnitude = (electric_flux_density_x
                * electric_flux_density_x
                + electric_flux_density_y * electric_flux_density_y)
                .sqrt();
            let electric_energy_density =
                0.5 * element.permittivity * electric_field_magnitude * electric_field_magnitude;
            let stored_energy = electric_energy_density * computed.area * element.thickness;

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
                electric_field_magnitude,
                electric_flux_density_x,
                electric_flux_density_y,
                electric_flux_density_magnitude,
                electric_energy_density,
                stored_energy,
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
    let total_stored_energy = elements.iter().map(|element| element.stored_energy).sum();
    let max_electric_energy_density = elements
        .iter()
        .map(|element| element.electric_energy_density)
        .fold(0.0_f64, f64::max);

    Ok(SolveElectrostaticPlaneTriangle2dResult {
        input: request.into_owned(),
        nodes,
        elements,
        max_potential,
        max_electric_field,
        max_flux_density,
        max_electric_energy_density,
        total_stored_energy,
    })
}

pub fn solve_electrostatic_plane_quad_2d(
    request: &SolveElectrostaticPlaneQuad2dRequest,
) -> Result<SolveElectrostaticPlaneQuad2dResult, String> {
    solve_electrostatic_plane_quad_2d_internal(Cow::Borrowed(request), SpdSolveOptions::default())
}

pub fn solve_electrostatic_plane_quad_2d_owned(
    request: SolveElectrostaticPlaneQuad2dRequest,
) -> Result<SolveElectrostaticPlaneQuad2dResult, String> {
    solve_electrostatic_plane_quad_2d_internal(Cow::Owned(request), SpdSolveOptions::default())
}

pub fn solve_electrostatic_plane_quad_2d_with_options(
    request: &SolveElectrostaticPlaneQuad2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveElectrostaticPlaneQuad2dResult, String> {
    solve_electrostatic_plane_quad_2d_internal(Cow::Borrowed(request), options)
}

fn solve_electrostatic_plane_quad_2d_internal(
    request: Cow<'_, SolveElectrostaticPlaneQuad2dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveElectrostaticPlaneQuad2dResult, String> {
    validate_electrostatic_plane_quad_request(request.as_ref())?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_electrostatic_plane_quad_element(request.as_ref(), element))
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
    let reduced_potentials =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_source, options)?
            .solution;

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
            let electric_field_magnitude =
                (electric_field_x * electric_field_x + electric_field_y * electric_field_y).sqrt();
            let electric_flux_density_magnitude = (electric_flux_density_x
                * electric_flux_density_x
                + electric_flux_density_y * electric_flux_density_y)
                .sqrt();
            let electric_energy_density =
                0.5 * element.permittivity * electric_field_magnitude * electric_field_magnitude;
            let stored_energy = electric_energy_density * total_area * element.thickness;

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
                electric_field_magnitude,
                electric_flux_density_x,
                electric_flux_density_y,
                electric_flux_density_magnitude,
                electric_energy_density,
                stored_energy,
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
    let total_stored_energy = elements.iter().map(|element| element.stored_energy).sum();
    let max_electric_energy_density = elements
        .iter()
        .map(|element| element.electric_energy_density)
        .fold(0.0_f64, f64::max);

    Ok(SolveElectrostaticPlaneQuad2dResult {
        input: request.into_owned(),
        nodes,
        elements,
        max_potential,
        max_electric_field,
        max_flux_density,
        max_electric_energy_density,
        total_stored_energy,
    })
}
