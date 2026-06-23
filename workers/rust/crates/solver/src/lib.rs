use kyuubiki_protocol::{
    ElectrostaticPlaneNodeResult, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneQuadElementResult, ElectrostaticPlaneTriangleElementInput,
    ElectrostaticPlaneTriangleElementResult, HeatPlaneNodeInput, HeatPlaneNodeResult,
    HeatPlaneQuadElementInput, HeatPlaneQuadElementResult, HeatPlaneTriangleElementInput,
    HeatPlaneTriangleElementResult, Job, JobStatus, PlaneTriangleElementInput, ProgressEvent,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneQuad2dResult,
    SolveElectrostaticPlaneTriangle2dRequest, SolveElectrostaticPlaneTriangle2dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult, SolvePlaneTriangle2dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneQuad2dResult, SolveThermalPlaneTriangle2dRequest,
    SolveThermalPlaneTriangle2dResult, ThermalPlaneNodeResult, ThermalPlaneQuadElementInput,
    ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementInput,
    ThermalPlaneTriangleElementResult,
};

mod bar_1d;
mod beam_1d;
mod frame_2d;
mod frame_2d_math;
mod frame_3d;
mod frame_3d_math;
mod linear_algebra;
mod plane_2d;
mod plane_2d_math;
mod spring;
mod thermal_frame_3d;
mod thermal_truss;
mod torsion_1d;
mod truss;

pub use bar_1d::{
    solve_bar_1d, solve_electrostatic_bar_1d, solve_heat_bar_1d, solve_thermal_bar_1d,
};
pub use beam_1d::{solve_beam_1d, solve_thermal_beam_1d};
pub use frame_2d::{solve_frame_2d, solve_thermal_frame_2d};
pub use frame_3d::solve_frame_3d;
use linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, reduce_sparse_system_with_prescribed,
    solve_spd_system,
};
pub use plane_2d::{solve_plane_quad_2d, solve_plane_triangle_2d};
pub use spring::{solve_spring_1d, solve_spring_2d, solve_spring_3d};
pub use thermal_frame_3d::solve_thermal_frame_3d;
pub use thermal_truss::{solve_thermal_truss_2d, solve_thermal_truss_3d};
pub use torsion_1d::solve_torsion_1d;
pub use truss::{solve_truss_2d, solve_truss_3d};

pub struct MockSolver {
    step_count: u64,
}

impl MockSolver {
    pub fn new(step_count: u64) -> Self {
        Self {
            step_count: step_count.max(1),
        }
    }

    pub fn solve(&self, job: &Job) -> Vec<ProgressEvent> {
        let mut events = Vec::with_capacity((self.step_count + 1) as usize);

        for step in 1..=self.step_count {
            let progress = step as f32 / self.step_count as f32;
            let mut event = ProgressEvent::new(job.job_id.clone(), JobStatus::Solving, progress);
            event.iteration = Some(step);
            event.residual = Some(1.0 / (step as f64 + 1.0));
            event.peak_memory = Some(512 + step * 32);
            event.message = Some(format!("mock solve step {step}/{}", self.step_count));
            events.push(event);
        }

        events.push(ProgressEvent::new(
            job.job_id.clone(),
            JobStatus::Completed,
            1.0,
        ));

        events
    }
}

pub fn solve_heat_plane_triangle_2d(
    request: &SolveHeatPlaneTriangle2dRequest,
) -> Result<SolveHeatPlaneTriangle2dResult, String> {
    validate_heat_plane_triangle_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_heat_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        heat_vector[index] = node.heat_load;
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
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed);
    let reduced_temperatures = solve_spd_system(&reduced_stiffness, &reduced_heat)?;

    let mut temperatures = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        temperatures[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        temperatures[dof] = reduced_temperatures[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| HeatPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            temperature: temperatures[index],
            heat_load: node.heat_load,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_temperatures = [
                temperatures[element.node_i],
                temperatures[element.node_j],
                temperatures[element.node_k],
            ];
            let gradient = plane_triangle_scalar_gradient(
                &computed.gradient_x,
                &computed.gradient_y,
                &element_temperatures,
            );
            let heat_flux_x = -element.conductivity * gradient[0];
            let heat_flux_y = -element.conductivity * gradient[1];

            HeatPlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                average_temperature: element_temperatures.iter().sum::<f64>() / 3.0,
                temperature_gradient_x: gradient[0],
                temperature_gradient_y: gradient[1],
                heat_flux_x,
                heat_flux_y,
                heat_flux_magnitude: (heat_flux_x * heat_flux_x + heat_flux_y * heat_flux_y).sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let max_temperature = nodes
        .iter()
        .map(|node| node.temperature.abs())
        .fold(0.0_f64, f64::max);
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux_magnitude.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveHeatPlaneTriangle2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_temperature,
        max_heat_flux,
    })
}

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

pub fn solve_heat_plane_quad_2d(
    request: &SolveHeatPlaneQuad2dRequest,
) -> Result<SolveHeatPlaneQuad2dResult, String> {
    solve_heat_plane_quad_2d_internal(request, false).map(|profile| profile.result)
}

#[derive(Debug, Clone)]
pub struct HeatPlaneQuadMemoryStage {
    pub label: &'static str,
    pub rss_kib: u64,
}

#[derive(Debug, Clone)]
pub struct HeatPlaneQuadProfile {
    pub result: SolveHeatPlaneQuad2dResult,
    pub memory_stages: Vec<HeatPlaneQuadMemoryStage>,
}

pub fn profile_heat_plane_quad_2d(
    request: &SolveHeatPlaneQuad2dRequest,
) -> Result<HeatPlaneQuadProfile, String> {
    solve_heat_plane_quad_2d_internal(request, true)
}

fn solve_heat_plane_quad_2d_internal(
    request: &SolveHeatPlaneQuad2dRequest,
    collect_memory_stages: bool,
) -> Result<HeatPlaneQuadProfile, String> {
    validate_heat_plane_quad_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];
    let mut memory_stages = Vec::new();
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_heat_plane_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;
    push_heat_plane_quad_memory_stage(&mut memory_stages, collect_memory_stages, "precompute");

    for (index, node) in request.nodes.iter().enumerate() {
        heat_vector[index] = node.heat_load;
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
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();

    push_heat_plane_quad_memory_stage(&mut memory_stages, collect_memory_stages, "assemble_global");
    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed);
    push_heat_plane_quad_memory_stage(&mut memory_stages, collect_memory_stages, "reduce_system");
    let reduced_temperatures = solve_spd_system(&reduced_stiffness, &reduced_heat)?;
    push_heat_plane_quad_memory_stage(&mut memory_stages, collect_memory_stages, "solve_system");

    let mut temperatures = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        temperatures[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        temperatures[dof] = reduced_temperatures[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| HeatPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            temperature: temperatures[index],
            heat_load: node.heat_load,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_temperatures = [
                temperatures[element.node_i],
                temperatures[element.node_j],
                temperatures[element.node_k],
            ];
            let second_temperatures = [
                temperatures[element.node_i],
                temperatures[element.node_k],
                temperatures[element.node_l],
            ];
            let first_gradient = plane_triangle_scalar_gradient(
                &computed.first.gradient_x,
                &computed.first.gradient_y,
                &first_temperatures,
            );
            let second_gradient = plane_triangle_scalar_gradient(
                &computed.second.gradient_x,
                &computed.second.gradient_y,
                &second_temperatures,
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };
            let heat_flux_x =
                -element.conductivity * weighted(first_gradient[0], second_gradient[0]);
            let heat_flux_y =
                -element.conductivity * weighted(first_gradient[1], second_gradient[1]);

            HeatPlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                average_temperature: (temperatures[element.node_i]
                    + temperatures[element.node_j]
                    + temperatures[element.node_k]
                    + temperatures[element.node_l])
                    / 4.0,
                temperature_gradient_x: weighted(first_gradient[0], second_gradient[0]),
                temperature_gradient_y: weighted(first_gradient[1], second_gradient[1]),
                heat_flux_x,
                heat_flux_y,
                heat_flux_magnitude: (heat_flux_x * heat_flux_x + heat_flux_y * heat_flux_y).sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let max_temperature = nodes
        .iter()
        .map(|node| node.temperature.abs())
        .fold(0.0_f64, f64::max);
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux_magnitude.abs())
        .fold(0.0_f64, f64::max);

    push_heat_plane_quad_memory_stage(&mut memory_stages, collect_memory_stages, "assemble");

    Ok(HeatPlaneQuadProfile {
        result: SolveHeatPlaneQuad2dResult {
            input: request.clone(),
            nodes,
            elements,
            max_temperature,
            max_heat_flux,
        },
        memory_stages,
    })
}

pub fn solve_thermal_plane_triangle_2d(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> Result<SolveThermalPlaneTriangle2dResult, String> {
    validate_thermal_plane_triangle_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_thermal_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
            element.node_k * 2,
            element.node_k * 2 + 1,
        ];
        let equivalent_load = thermal_plane_triangle_equivalent_load(
            &computed.b_matrix,
            &computed.d_matrix,
            computed.area,
            element.thickness,
            element.thermal_expansion,
            computed.average_temperature_delta,
        );

        for row in 0..6 {
            force_vector[map[row]] += equivalent_load[row];
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    computed.stiffness[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 2);
            }
            if node.fix_y {
                dofs.push(index * 2 + 1);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ThermalPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
            displacement_magnitude: (displacements[index * 2].powi(2)
                + displacements[index * 2 + 1].powi(2))
            .sqrt(),
            temperature_delta: node.temperature_delta,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
                displacements[element.node_k * 2],
                displacements[element.node_k * 2 + 1],
            ];

            let total_strain =
                multiply_matrix_vector_3x6(&computed.b_matrix, &element_displacements);
            let thermal_strain = [
                element.thermal_expansion * computed.average_temperature_delta,
                element.thermal_expansion * computed.average_temperature_delta,
                0.0,
            ];
            let mechanical_strain = subtract_vector_3(&total_strain, &thermal_strain);
            let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &mechanical_strain);
            let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

            Ok::<ThermalPlaneTriangleElementResult, String>(ThermalPlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                average_temperature_delta: computed.average_temperature_delta,
                thermal_strain: thermal_strain[0],
                mechanical_strain_x: mechanical_strain[0],
                mechanical_strain_y: mechanical_strain[1],
                total_strain_x: total_strain[0],
                total_strain_y: total_strain[1],
                gamma_xy: total_strain[2],
                stress_x: stress[0],
                stress_y: stress[1],
                tau_xy: stress[2],
                principal_stress_1: derived.principal_stress_1,
                principal_stress_2: derived.principal_stress_2,
                max_in_plane_shear: derived.max_in_plane_shear,
                von_mises: derived.von_mises,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let max_displacement = nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy).sqrt())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveThermalPlaneTriangle2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
        max_temperature_delta,
    })
}

pub fn solve_thermal_plane_quad_2d(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<SolveThermalPlaneQuad2dResult, String> {
    validate_thermal_plane_quad_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_thermal_plane_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

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
            let map = [
                nodes[0] * 2,
                nodes[0] * 2 + 1,
                nodes[1] * 2,
                nodes[1] * 2 + 1,
                nodes[2] * 2,
                nodes[2] * 2 + 1,
            ];
            let equivalent_load = thermal_plane_triangle_equivalent_load(
                &triangle.b_matrix,
                &triangle.d_matrix,
                triangle.area,
                element.thickness,
                element.thermal_expansion,
                triangle.average_temperature_delta,
            );

            for row in 0..6 {
                force_vector[map[row]] += equivalent_load[row];
                for column in 0..6 {
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

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 2);
            }
            if node.fix_y {
                dofs.push(index * 2 + 1);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ThermalPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
            displacement_magnitude: (displacements[index * 2].powi(2)
                + displacements[index * 2 + 1].powi(2))
            .sqrt(),
            temperature_delta: node.temperature_delta,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
                displacements[element.node_k * 2],
                displacements[element.node_k * 2 + 1],
            ];
            let second_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_k * 2],
                displacements[element.node_k * 2 + 1],
                displacements[element.node_l * 2],
                displacements[element.node_l * 2 + 1],
            ];

            let first_state = thermal_plane_triangle_state(
                &computed.first,
                &first_displacements,
                element.thermal_expansion,
            );
            let second_state = thermal_plane_triangle_state(
                &computed.second,
                &second_displacements,
                element.thermal_expansion,
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };

            ThermalPlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                average_temperature_delta: weighted(
                    computed.first.average_temperature_delta,
                    computed.second.average_temperature_delta,
                ),
                thermal_strain: weighted(first_state.thermal_strain, second_state.thermal_strain),
                mechanical_strain_x: weighted(
                    first_state.mechanical_strain[0],
                    second_state.mechanical_strain[0],
                ),
                mechanical_strain_y: weighted(
                    first_state.mechanical_strain[1],
                    second_state.mechanical_strain[1],
                ),
                total_strain_x: weighted(first_state.total_strain[0], second_state.total_strain[0]),
                total_strain_y: weighted(first_state.total_strain[1], second_state.total_strain[1]),
                gamma_xy: weighted(first_state.total_strain[2], second_state.total_strain[2]),
                stress_x: weighted(first_state.stress[0], second_state.stress[0]),
                stress_y: weighted(first_state.stress[1], second_state.stress[1]),
                tau_xy: weighted(first_state.stress[2], second_state.stress[2]),
                principal_stress_1: weighted(
                    first_state.principal_stress_1,
                    second_state.principal_stress_1,
                ),
                principal_stress_2: weighted(
                    first_state.principal_stress_2,
                    second_state.principal_stress_2,
                ),
                max_in_plane_shear: weighted(
                    first_state.max_in_plane_shear,
                    second_state.max_in_plane_shear,
                ),
                von_mises: weighted(first_state.von_mises, second_state.von_mises),
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy).sqrt())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveThermalPlaneQuad2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
        max_temperature_delta,
    })
}

fn validate_thermal_plane_triangle_request(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("thermal plane model must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal plane model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal plane model must include at least one support".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal plane node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
        {
            return Err("thermal plane element references an out-of-range node".to_string());
        }
        if !(element.thickness.is_finite() && element.thickness > 0.0) {
            return Err("thermal plane element thickness must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal plane element youngs_modulus must be positive".to_string());
        }
        if !(element.poisson_ratio.is_finite()
            && element.poisson_ratio > -1.0
            && element.poisson_ratio < 0.5)
        {
            return Err(
                "thermal plane element poisson_ratio must be between -1.0 and 0.5".to_string(),
            );
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err("thermal plane element thermal_expansion must be non-negative".to_string());
        }

        let ni = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_i].id.clone(),
            x: request.nodes[element.node_i].x,
            y: request.nodes[element.node_i].y,
            fix_x: request.nodes[element.node_i].fix_x,
            fix_y: request.nodes[element.node_i].fix_y,
            load_x: request.nodes[element.node_i].load_x,
            load_y: request.nodes[element.node_i].load_y,
        };
        let nj = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_j].id.clone(),
            x: request.nodes[element.node_j].x,
            y: request.nodes[element.node_j].y,
            fix_x: request.nodes[element.node_j].fix_x,
            fix_y: request.nodes[element.node_j].fix_y,
            load_x: request.nodes[element.node_j].load_x,
            load_y: request.nodes[element.node_j].load_y,
        };
        let nk = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_k].id.clone(),
            x: request.nodes[element.node_k].x,
            y: request.nodes[element.node_k].y,
            fix_x: request.nodes[element.node_k].fix_x,
            fix_y: request.nodes[element.node_k].fix_y,
            load_x: request.nodes[element.node_k].load_x,
            load_y: request.nodes[element.node_k].load_y,
        };
        let area = signed_triangle_area(&ni, &nj, &nk).abs();
        if area <= 1.0e-12 {
            return Err("thermal plane element area must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_thermal_plane_quad_request(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("thermal plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal plane quad model must include at least one support".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal plane quad node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        let indices = [
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ];
        if indices.iter().any(|&index| index >= request.nodes.len()) {
            return Err("thermal plane quad element references an out-of-range node".to_string());
        }
        let unique_count = indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if unique_count < 4 {
            return Err(
                "thermal plane quad element must reference four distinct nodes".to_string(),
            );
        }
        if !(element.thickness.is_finite() && element.thickness > 0.0) {
            return Err("thermal plane quad element thickness must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal plane quad element youngs_modulus must be positive".to_string());
        }
        if !(element.poisson_ratio.is_finite()
            && element.poisson_ratio > -1.0
            && element.poisson_ratio < 0.5)
        {
            return Err(
                "thermal plane quad element poisson_ratio must be between -1.0 and 0.5".to_string(),
            );
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "thermal plane quad element thermal_expansion must be non-negative".to_string(),
            );
        }

        let to_plane_node = |index: usize| kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[index].id.clone(),
            x: request.nodes[index].x,
            y: request.nodes[index].y,
            fix_x: request.nodes[index].fix_x,
            fix_y: request.nodes[index].fix_y,
            load_x: request.nodes[index].load_x,
            load_y: request.nodes[index].load_y,
        };
        let first_area = signed_triangle_area(
            &to_plane_node(element.node_i),
            &to_plane_node(element.node_j),
            &to_plane_node(element.node_k),
        )
        .abs();
        let second_area = signed_triangle_area(
            &to_plane_node(element.node_i),
            &to_plane_node(element.node_k),
            &to_plane_node(element.node_l),
        )
        .abs();
        if first_area <= 1.0e-12 || second_area <= 1.0e-12 {
            return Err(
                "thermal plane quad element must decompose into positive-area triangles"
                    .to_string(),
            );
        }
    }

    Ok(())
}

fn validate_heat_plane_triangle_request(
    request: &SolveHeatPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("heat plane triangle model must define at least three nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("heat plane triangle model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_temperature) {
        return Err(
            "heat plane triangle model must include at least one temperature support".to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("heat plane triangle node coordinates must be finite".to_string());
        }
        if !node.temperature.is_finite() {
            return Err("heat plane triangle node temperature must be finite".to_string());
        }
        if !node.heat_load.is_finite() {
            return Err("heat plane triangle node heat_load must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
        {
            return Err("heat plane triangle element references an out-of-range node".to_string());
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("heat plane triangle thickness must be positive".to_string());
        }
        if !element.conductivity.is_finite() || element.conductivity <= 0.0 {
            return Err("heat plane triangle conductivity must be positive".to_string());
        }
        let ni = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_i].id.clone(),
            x: request.nodes[element.node_i].x,
            y: request.nodes[element.node_i].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let nj = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_j].id.clone(),
            x: request.nodes[element.node_j].x,
            y: request.nodes[element.node_j].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let nk = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_k].id.clone(),
            x: request.nodes[element.node_k].x,
            y: request.nodes[element.node_k].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let area = signed_triangle_area(&ni, &nj, &nk).abs();
        if area <= 1.0e-12 {
            return Err("heat plane triangle element area must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_electrostatic_plane_triangle_request(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err(
            "electrostatic plane triangle model must define at least three nodes".to_string(),
        );
    }
    if request.elements.is_empty() {
        return Err(
            "electrostatic plane triangle model must define at least one element".to_string(),
        );
    }
    if !request.nodes.iter().any(|node| node.fix_potential) {
        return Err(
            "electrostatic plane triangle model must include at least one potential support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("electrostatic plane triangle node coordinates must be finite".to_string());
        }
        if !node.potential.is_finite() {
            return Err("electrostatic plane triangle node potential must be finite".to_string());
        }
        if !node.charge_density.is_finite() {
            return Err(
                "electrostatic plane triangle node charge_density must be finite".to_string(),
            );
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
        {
            return Err(
                "electrostatic plane triangle element references an out-of-range node".to_string(),
            );
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("electrostatic plane triangle thickness must be positive".to_string());
        }
        if !element.permittivity.is_finite() || element.permittivity <= 0.0 {
            return Err("electrostatic plane triangle permittivity must be positive".to_string());
        }
        let ni = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_i].id.clone(),
            x: request.nodes[element.node_i].x,
            y: request.nodes[element.node_i].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let nj = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_j].id.clone(),
            x: request.nodes[element.node_j].x,
            y: request.nodes[element.node_j].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let nk = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_k].id.clone(),
            x: request.nodes[element.node_k].x,
            y: request.nodes[element.node_k].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let area = signed_triangle_area(&ni, &nj, &nk).abs();
        if area <= 1.0e-12 {
            return Err("electrostatic plane triangle element area must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_electrostatic_plane_quad_request(
    request: &SolveElectrostaticPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("electrostatic plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("electrostatic plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_potential) {
        return Err(
            "electrostatic plane quad model must include at least one potential support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("electrostatic plane quad node coordinates must be finite".to_string());
        }
        if !node.potential.is_finite() {
            return Err("electrostatic plane quad node potential must be finite".to_string());
        }
        if !node.charge_density.is_finite() {
            return Err("electrostatic plane quad node charge_density must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
            || element.node_l >= request.nodes.len()
        {
            return Err(
                "electrostatic plane quad element references an out-of-range node".to_string(),
            );
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("electrostatic plane quad thickness must be positive".to_string());
        }
        if !element.permittivity.is_finite() || element.permittivity <= 0.0 {
            return Err("electrostatic plane quad permittivity must be positive".to_string());
        }

        let to_node = |index: usize| kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[index].id.clone(),
            x: request.nodes[index].x,
            y: request.nodes[index].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let ni = to_node(element.node_i);
        let nj = to_node(element.node_j);
        let nk = to_node(element.node_k);
        let nl = to_node(element.node_l);
        let first_area = signed_triangle_area(&ni, &nj, &nk).abs();
        let second_area = signed_triangle_area(&ni, &nk, &nl).abs();
        if first_area <= 1.0e-12 || second_area <= 1.0e-12 {
            return Err("electrostatic plane quad triangles must have positive area".to_string());
        }
    }

    Ok(())
}

fn validate_heat_plane_quad_request(request: &SolveHeatPlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("heat plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("heat plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_temperature) {
        return Err(
            "heat plane quad model must include at least one temperature support".to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("heat plane quad node coordinates must be finite".to_string());
        }
        if !node.temperature.is_finite() {
            return Err("heat plane quad node temperature must be finite".to_string());
        }
        if !node.heat_load.is_finite() {
            return Err("heat plane quad node heat_load must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
            || element.node_l >= request.nodes.len()
        {
            return Err("heat plane quad element references an out-of-range node".to_string());
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("heat plane quad thickness must be positive".to_string());
        }
        if !element.conductivity.is_finite() || element.conductivity <= 0.0 {
            return Err("heat plane quad conductivity must be positive".to_string());
        }

        let to_node = |index: usize| kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[index].id.clone(),
            x: request.nodes[index].x,
            y: request.nodes[index].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let ni = to_node(element.node_i);
        let nj = to_node(element.node_j);
        let nk = to_node(element.node_k);
        let nl = to_node(element.node_l);
        let first_area = signed_triangle_area(&ni, &nj, &nk).abs();
        let second_area = signed_triangle_area(&ni, &nk, &nl).abs();
        if first_area <= 1.0e-12 || second_area <= 1.0e-12 {
            return Err("heat plane quad triangles must have positive area".to_string());
        }
    }

    Ok(())
}

fn triangle_element_data(
    request: &SolvePlaneTriangle2dRequest,
    element: &kyuubiki_protocol::PlaneTriangleElementInput,
) -> Result<([[f64; 6]; 6], f64, [[f64; 6]; 3], [[f64; 3]; 3]), String> {
    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let node_k = &request.nodes[element.node_k];

    let signed_area = signed_triangle_area(node_i, node_j, node_k);
    let area = signed_area.abs();

    if area <= 1.0e-12 {
        return Err("plane element area must be positive".to_string());
    }

    let b1 = node_j.y - node_k.y;
    let b2 = node_k.y - node_i.y;
    let b3 = node_i.y - node_j.y;
    let c1 = node_k.x - node_j.x;
    let c2 = node_i.x - node_k.x;
    let c3 = node_j.x - node_i.x;
    let factor = 1.0 / (2.0 * area);

    let b_matrix = [
        [b1 * factor, 0.0, b2 * factor, 0.0, b3 * factor, 0.0],
        [0.0, c1 * factor, 0.0, c2 * factor, 0.0, c3 * factor],
        [
            c1 * factor,
            b1 * factor,
            c2 * factor,
            b2 * factor,
            c3 * factor,
            b3 * factor,
        ],
    ];

    let e = element.youngs_modulus;
    let nu = element.poisson_ratio;
    let coeff = e / (1.0 - nu * nu);
    let d_matrix = [
        [coeff, coeff * nu, 0.0],
        [coeff * nu, coeff, 0.0],
        [0.0, 0.0, coeff * (1.0 - nu) * 0.5],
    ];

    let bt = transpose_3x6(&b_matrix);
    let bt_d = multiply_matrix_6x3_3x3(&bt, &d_matrix);
    let mut stiffness = multiply_matrix_6x3_3x6(&bt_d, &b_matrix);
    let scale = element.thickness * area;

    for row in 0..6 {
        for column in 0..6 {
            stiffness[row][column] *= scale;
        }
    }

    Ok((stiffness, area, b_matrix, d_matrix))
}

#[derive(Debug, Clone)]
struct ThermalPlaneTriangleComputed {
    stiffness: [[f64; 6]; 6],
    area: f64,
    b_matrix: [[f64; 6]; 3],
    d_matrix: [[f64; 3]; 3],
    average_temperature_delta: f64,
}

#[derive(Debug, Clone)]
struct ThermalPlaneQuadComputed {
    first: ThermalPlaneTriangleComputed,
    second: ThermalPlaneTriangleComputed,
}

#[derive(Debug, Clone)]
struct HeatPlaneTriangleComputed {
    stiffness: [[f64; 3]; 3],
    area: f64,
    gradient_x: [f64; 3],
    gradient_y: [f64; 3],
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

#[derive(Debug, Clone)]
struct HeatPlaneQuadComputed {
    first: HeatPlaneTriangleComputed,
    second: HeatPlaneTriangleComputed,
}

fn precompute_thermal_plane_triangle_element(
    request: &SolveThermalPlaneTriangle2dRequest,
    element: &ThermalPlaneTriangleElementInput,
) -> Result<ThermalPlaneTriangleComputed, String> {
    let plane_request = SolvePlaneTriangle2dRequest {
        nodes: request
            .nodes
            .iter()
            .map(|node| kyuubiki_protocol::PlaneNodeInput {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_x: node.fix_x,
                fix_y: node.fix_y,
                load_x: node.load_x,
                load_y: node.load_y,
            })
            .collect(),
        elements: vec![],
    };
    let plane_element = PlaneTriangleElementInput {
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    let (stiffness, area, b_matrix, d_matrix) =
        triangle_element_data(&plane_request, &plane_element)?;
    let average_temperature_delta = (request.nodes[element.node_i].temperature_delta
        + request.nodes[element.node_j].temperature_delta
        + request.nodes[element.node_k].temperature_delta)
        / 3.0;

    Ok(ThermalPlaneTriangleComputed {
        stiffness,
        area,
        b_matrix,
        d_matrix,
        average_temperature_delta,
    })
}

fn precompute_heat_plane_triangle_element(
    request: &SolveHeatPlaneTriangle2dRequest,
    element: &HeatPlaneTriangleElementInput,
) -> Result<HeatPlaneTriangleComputed, String> {
    precompute_heat_plane_triangle_element_from_nodes(&request.nodes, element)
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

fn precompute_thermal_plane_quad_element(
    request: &SolveThermalPlaneQuad2dRequest,
    element: &ThermalPlaneQuadElementInput,
) -> Result<ThermalPlaneQuadComputed, String> {
    let first = ThermalPlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
        thermal_expansion: element.thermal_expansion,
    };
    let second = ThermalPlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
        thermal_expansion: element.thermal_expansion,
    };
    let triangle_request = SolveThermalPlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    };

    Ok(ThermalPlaneQuadComputed {
        first: precompute_thermal_plane_triangle_element(&triangle_request, &first)?,
        second: precompute_thermal_plane_triangle_element(&triangle_request, &second)?,
    })
}

fn precompute_heat_plane_quad_element(
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

#[derive(Debug, Clone)]
struct ThermalPlaneTriangleState {
    total_strain: [f64; 3],
    mechanical_strain: [f64; 3],
    thermal_strain: f64,
    stress: [f64; 3],
    principal_stress_1: f64,
    principal_stress_2: f64,
    max_in_plane_shear: f64,
    von_mises: f64,
}

#[derive(Debug, Clone, Copy)]
struct PlanarStressMetrics {
    principal_stress_1: f64,
    principal_stress_2: f64,
    max_in_plane_shear: f64,
    von_mises: f64,
}

fn thermal_plane_triangle_state(
    computed: &ThermalPlaneTriangleComputed,
    element_displacements: &[f64; 6],
    thermal_expansion: f64,
) -> ThermalPlaneTriangleState {
    let total_strain = multiply_matrix_vector_3x6(&computed.b_matrix, element_displacements);
    let thermal_strain = thermal_expansion * computed.average_temperature_delta;
    let thermal_vector = [thermal_strain, thermal_strain, 0.0];
    let mechanical_strain = subtract_vector_3(&total_strain, &thermal_vector);
    let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &mechanical_strain);
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    ThermalPlaneTriangleState {
        total_strain,
        mechanical_strain,
        thermal_strain,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
    }
}

fn derive_planar_stress_metrics(sigma_x: f64, sigma_y: f64, tau_xy: f64) -> PlanarStressMetrics {
    let center = 0.5 * (sigma_x + sigma_y);
    let radius = (((0.5 * (sigma_x - sigma_y)).powi(2)) + tau_xy.powi(2)).sqrt();
    let principal_stress_1 = center + radius;
    let principal_stress_2 = center - radius;
    let max_in_plane_shear = radius;
    let von_mises =
        ((sigma_x * sigma_x) - (sigma_x * sigma_y) + (sigma_y * sigma_y) + 3.0 * tau_xy * tau_xy)
            .sqrt();

    PlanarStressMetrics {
        principal_stress_1,
        principal_stress_2,
        max_in_plane_shear,
        von_mises,
    }
}

fn signed_triangle_area(
    node_i: &kyuubiki_protocol::PlaneNodeInput,
    node_j: &kyuubiki_protocol::PlaneNodeInput,
    node_k: &kyuubiki_protocol::PlaneNodeInput,
) -> f64 {
    0.5 * ((node_j.x - node_i.x) * (node_k.y - node_i.y)
        - (node_k.x - node_i.x) * (node_j.y - node_i.y))
}

fn push_heat_plane_quad_memory_stage(
    stages: &mut Vec<HeatPlaneQuadMemoryStage>,
    enabled: bool,
    label: &'static str,
) {
    if !enabled {
        return;
    }

    stages.push(HeatPlaneQuadMemoryStage {
        label,
        rss_kib: current_rss_kib(),
    });
}

fn current_rss_kib() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm")
            && let Some(resident_pages) = statm.split_whitespace().nth(1)
            && let Ok(resident_pages) = resident_pages.parse::<u64>()
        {
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
            if page_size > 0 {
                return resident_pages * page_size as u64 / 1024;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if status == 0 {
            let usage = unsafe { usage.assume_init() };
            return (usage.ru_maxrss as u64) / 1024;
        }
    }

    0
}

fn transpose_3x6(input: &[[f64; 6]; 3]) -> [[f64; 3]; 6] {
    let mut output = [[0.0; 3]; 6];

    for row in 0..3 {
        for column in 0..6 {
            output[column][row] = input[row][column];
        }
    }

    output
}

fn multiply_matrix_6x3_3x3(lhs: &[[f64; 3]; 6], rhs: &[[f64; 3]; 3]) -> [[f64; 3]; 6] {
    let mut output = [[0.0; 3]; 6];

    for row in 0..6 {
        for column in 0..3 {
            output[row][column] = (0..3)
                .map(|index| lhs[row][index] * rhs[index][column])
                .sum();
        }
    }

    output
}

fn multiply_matrix_6x3_3x6(lhs: &[[f64; 3]; 6], rhs: &[[f64; 6]; 3]) -> [[f64; 6]; 6] {
    let mut output = [[0.0; 6]; 6];

    for row in 0..6 {
        for column in 0..6 {
            output[row][column] = (0..3)
                .map(|index| lhs[row][index] * rhs[index][column])
                .sum();
        }
    }

    output
}

fn multiply_matrix_vector_3x6(matrix: &[[f64; 6]; 3], vector: &[f64; 6]) -> [f64; 3] {
    let mut output = [0.0; 3];

    for row in 0..3 {
        output[row] = (0..6).map(|index| matrix[row][index] * vector[index]).sum();
    }

    output
}

fn multiply_matrix_vector_3x3(matrix: &[[f64; 3]; 3], vector: &[f64; 3]) -> [f64; 3] {
    let mut output = [0.0; 3];

    for row in 0..3 {
        output[row] = (0..3).map(|index| matrix[row][index] * vector[index]).sum();
    }

    output
}

fn subtract_vector_3(left: &[f64; 3], right: &[f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn thermal_plane_triangle_equivalent_load(
    b_matrix: &[[f64; 6]; 3],
    d_matrix: &[[f64; 3]; 3],
    area: f64,
    thickness: f64,
    thermal_expansion: f64,
    average_temperature_delta: f64,
) -> [f64; 6] {
    let thermal_strain = [
        thermal_expansion * average_temperature_delta,
        thermal_expansion * average_temperature_delta,
        0.0,
    ];
    let thermal_stress = multiply_matrix_vector_3x3(d_matrix, &thermal_strain);
    let bt = transpose_3x6(b_matrix);
    let mut equivalent_load = [0.0; 6];

    for row in 0..6 {
        equivalent_load[row] = (0..3)
            .map(|index| bt[row][index] * thermal_stress[index])
            .sum::<f64>()
            * thickness
            * area;
    }

    equivalent_load
}

#[cfg(test)]
mod tests {
    use super::{
        MockSolver, solve_bar_1d, solve_beam_1d, solve_electrostatic_bar_1d,
        solve_electrostatic_plane_quad_2d, solve_electrostatic_plane_triangle_2d, solve_frame_2d,
        solve_frame_3d, solve_heat_bar_1d, solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d,
        solve_plane_quad_2d, solve_plane_triangle_2d, solve_spring_1d, solve_spring_2d,
        solve_spring_3d, solve_thermal_bar_1d, solve_thermal_beam_1d, solve_thermal_frame_2d,
        solve_thermal_frame_3d, solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d,
        solve_thermal_truss_2d, solve_thermal_truss_3d, solve_torsion_1d, solve_truss_2d,
        solve_truss_3d,
    };
    use kyuubiki_protocol::{
        Beam1dElementInput, Beam1dNodeInput, ElectrostaticBar1dElementInput,
        ElectrostaticBar1dNodeInput, ElectrostaticPlaneNodeInput,
        ElectrostaticPlaneQuadElementInput, ElectrostaticPlaneTriangleElementInput,
        Frame2dElementInput, Frame2dNodeInput, Frame3dElementInput, Frame3dNodeInput,
        HeatBar1dElementInput, HeatBar1dNodeInput, HeatPlaneNodeInput, HeatPlaneQuadElementInput,
        HeatPlaneTriangleElementInput, Job, JobStatus, PlaneNodeInput, PlaneQuadElementInput,
        PlaneTriangleElementInput, SolveBarRequest, SolveBeam1dRequest,
        SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
        SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
        SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
        SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest, SolveSpring1dRequest,
        SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBar1dRequest,
        SolveThermalBeam1dRequest, SolveThermalFrame2dRequest, SolveThermalFrame3dRequest,
        SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
        SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, SolveTorsion1dRequest,
        SolveTruss2dRequest, SolveTruss3dRequest, Spring1dElementInput, Spring1dNodeInput,
        Spring2dElementInput, Spring2dNodeInput, Spring3dElementInput, Spring3dNodeInput,
        ThermalBar1dElementInput, ThermalBar1dNodeInput, ThermalBeam1dElementInput,
        ThermalBeam1dNodeInput, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
        ThermalPlaneNodeInput, ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
        ThermalTruss2dElementInput, ThermalTruss2dNodeInput, ThermalTruss3dElementInput,
        ThermalTruss3dNodeInput, Torsion1dElementInput, Torsion1dNodeInput, Truss3dElementInput,
        Truss3dNodeInput, TrussElementInput, TrussNodeInput,
    };

    #[test]
    fn emits_solving_events_and_completion() {
        let solver = MockSolver::new(3);
        let job = Job::new("job-1", "project-1", "case-1");

        let events = solver.solve(&job);

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].stage, JobStatus::Solving);
        assert_eq!(events[2].progress, 1.0);
        assert_eq!(events[3].stage, JobStatus::Completed);
    }

    #[test]
    fn solves_a_one_element_tensile_bar() {
        let result = solve_bar_1d(&SolveBarRequest {
            length: 1.0,
            area: 0.01,
            youngs_modulus: 210.0e9,
            elements: 1,
            tip_force: 1000.0,
        })
        .expect("solver should succeed");

        assert!((result.tip_displacement - 4.761904761904762e-7).abs() < 1.0e-12);
        assert!((result.max_stress - 100_000.0).abs() < 1.0e-6);
        assert!((result.reaction_force + 1000.0).abs() < 1.0e-6);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
    }

    #[test]
    fn rejects_invalid_requests() {
        let error = solve_bar_1d(&SolveBarRequest {
            length: 0.0,
            area: 0.01,
            youngs_modulus: 210.0e9,
            elements: 1,
            tip_force: 1000.0,
        })
        .expect_err("invalid request should fail");

        assert!(error.contains("length"));
    }

    #[test]
    fn solves_a_small_thermal_bar_1d_with_restrained_expansion() {
        let result = solve_thermal_bar_1d(&SolveThermalBar1dRequest {
            nodes: vec![
                ThermalBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_x: true,
                    load_x: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_x: true,
                    load_x: 0.0,
                    temperature_delta: 40.0,
                },
            ],
            elements: vec![ThermalBar1dElementInput {
                id: "tb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            }],
        })
        .expect("thermal bar should solve");

        assert!(result.max_displacement.abs() < 1.0e-12);
        assert!(result.max_stress > 1.0e8);
        assert!(result.max_axial_force > 1.0e6);
        assert_eq!(result.max_temperature_delta, 40.0);
        assert!(result.elements[0].stress < 0.0);
    }

    #[test]
    fn solves_a_small_heat_bar_1d_gradient() {
        let result = solve_heat_bar_1d(&SolveHeatBar1dRequest {
            nodes: vec![
                HeatBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_temperature: true,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatBar1dElementInput {
                id: "hb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                conductivity: 50.0,
            }],
        })
        .expect("heat bar should solve");

        assert_eq!(result.nodes[0].temperature, 100.0);
        assert_eq!(result.nodes[1].temperature, 0.0);
        assert!((result.elements[0].temperature_gradient + 100.0).abs() < 1.0e-9);
        assert!((result.elements[0].heat_flux - 5_000.0).abs() < 1.0e-6);
        assert_eq!(result.max_temperature, 100.0);
        assert!((result.max_heat_flux - 5_000.0).abs() < 1.0e-6);
    }

    #[test]
    fn solves_a_small_electrostatic_bar_1d_gradient() {
        let result = solve_electrostatic_bar_1d(&SolveElectrostaticBar1dRequest {
            nodes: vec![
                ElectrostaticBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                ElectrostaticBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![ElectrostaticBar1dElementInput {
                id: "eb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                permittivity: 2.0,
            }],
        })
        .expect("electrostatic bar should solve");

        assert_eq!(result.nodes[0].potential, 10.0);
        assert_eq!(result.nodes[1].potential, 0.0);
        assert!((result.elements[0].potential_gradient + 10.0).abs() < 1.0e-9);
        assert!((result.elements[0].electric_field - 10.0).abs() < 1.0e-9);
        assert!((result.elements[0].electric_flux_density - 20.0).abs() < 1.0e-9);
        assert_eq!(result.max_potential, 10.0);
        assert!((result.max_electric_field - 10.0).abs() < 1.0e-9);
        assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
    }

    #[test]
    fn solves_a_small_electrostatic_plane_triangle_2d_patch() {
        let result =
            solve_electrostatic_plane_triangle_2d(&SolveElectrostaticPlaneTriangle2dRequest {
                nodes: vec![
                    ElectrostaticPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_potential: true,
                        potential: 0.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                ],
                elements: vec![ElectrostaticPlaneTriangleElementInput {
                    id: "ep0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.05,
                    permittivity: 2.0,
                }],
            })
            .expect("electrostatic plane triangle should solve");

        assert_eq!(result.nodes[0].potential, 10.0);
        assert_eq!(result.nodes[1].potential, 0.0);
        assert_eq!(result.nodes[2].potential, 10.0);
        assert!((result.elements[0].potential_gradient_x + 10.0).abs() < 1.0e-9);
        assert!(result.elements[0].potential_gradient_y.abs() < 1.0e-9);
        assert!((result.elements[0].electric_field_x - 10.0).abs() < 1.0e-9);
        assert!(result.elements[0].electric_field_y.abs() < 1.0e-9);
        assert!((result.elements[0].electric_flux_density_x - 20.0).abs() < 1.0e-9);
        assert_eq!(result.max_potential, 10.0);
        assert!((result.max_electric_field - 10.0).abs() < 1.0e-9);
        assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
    }

    #[test]
    fn solves_a_small_electrostatic_plane_quad_2d_patch() {
        let result = solve_electrostatic_plane_quad_2d(&SolveElectrostaticPlaneQuad2dRequest {
            nodes: vec![
                ElectrostaticPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![ElectrostaticPlaneQuadElementInput {
                id: "epq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.05,
                permittivity: 2.0,
            }],
        })
        .expect("electrostatic plane quad should solve");

        assert_eq!(result.nodes[0].potential, 10.0);
        assert_eq!(result.nodes[1].potential, 0.0);
        assert_eq!(result.nodes[2].potential, 0.0);
        assert_eq!(result.nodes[3].potential, 10.0);
        assert!((result.elements[0].potential_gradient_x + 10.0).abs() < 1.0e-9);
        assert!(result.elements[0].potential_gradient_y.abs() < 1.0e-9);
        assert!((result.elements[0].electric_field_x - 10.0).abs() < 1.0e-9);
        assert!(result.elements[0].electric_field_y.abs() < 1.0e-9);
        assert!((result.elements[0].electric_flux_density_x - 20.0).abs() < 1.0e-9);
        assert_eq!(result.max_potential, 10.0);
        assert!((result.max_electric_field - 10.0).abs() < 1.0e-9);
        assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
    }

    #[test]
    fn solves_a_small_heat_plane_triangle_2d_patch() {
        let result = solve_heat_plane_triangle_2d(&SolveHeatPlaneTriangle2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneTriangleElementInput {
                id: "hp0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 10.0,
            }],
        })
        .expect("heat plane triangle should solve");

        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.max_temperature, 100.0);
        assert!(result.max_heat_flux > 0.0);
    }

    #[test]
    fn solves_a_small_heat_plane_quad_2d_patch() {
        let result = solve_heat_plane_quad_2d(&SolveHeatPlaneQuad2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneQuadElementInput {
                id: "hq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                conductivity: 10.0,
            }],
        })
        .expect("heat plane quad should solve");

        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.max_temperature, 100.0);
        assert!(result.max_heat_flux > 0.0);
    }

    #[test]
    fn solves_a_small_thermal_truss_2d_with_restrained_expansion() {
        let result = solve_thermal_truss_2d(&SolveThermalTruss2dRequest {
            nodes: vec![
                ThermalTruss2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalTruss2dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
            ],
            elements: vec![ThermalTruss2dElementInput {
                id: "tt0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            }],
        })
        .expect("thermal truss 2d should solve");

        assert!(result.max_displacement.abs() < 1.0e-12);
        assert!(result.max_stress > 1.0e8);
        assert!(result.max_axial_force > 1.0e6);
        assert_eq!(result.max_temperature_delta, 40.0);
        assert!(result.elements[0].stress < 0.0);
    }

    #[test]
    fn solves_a_small_thermal_truss_3d_with_restrained_expansion() {
        let result = solve_thermal_truss_3d(&SolveThermalTruss3dRequest {
            nodes: vec![
                ThermalTruss3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalTruss3dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalTruss3dNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    temperature_delta: 0.0,
                },
            ],
            elements: vec![ThermalTruss3dElementInput {
                id: "tt0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            }],
        })
        .expect("thermal truss 3d should solve");

        assert!(result.max_displacement.abs() < 1.0e-12);
        assert!(result.max_stress > 1.0e8);
        assert!(result.max_axial_force > 1.0e6);
        assert_eq!(result.max_temperature_delta, 40.0);
        assert!(result.elements[0].stress < 0.0);
    }

    #[test]
    fn solves_a_small_beam_1d_cantilever() {
        let result = solve_beam_1d(&SolveBeam1dRequest {
            nodes: vec![
                Beam1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                Beam1dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    fix_y: false,
                    fix_rz: false,
                    load_y: -1000.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Beam1dElementInput {
                id: "b0".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                distributed_load_y: 0.0,
            }],
        })
        .expect("1d beam should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!((result.max_displacement - 0.0015873015873015873).abs() < 1.0e-12);
        assert!((result.max_rotation - 0.0011904761904761906).abs() < 1.0e-12);
        assert!((result.max_moment - 2000.0).abs() < 1.0e-6);
        assert!((result.max_stress - 1.25e7).abs() < 1.0e-2);
    }

    #[test]
    fn solves_a_small_thermal_beam_1d_with_restrained_gradient() {
        let result = solve_thermal_beam_1d(&SolveThermalBeam1dRequest {
            nodes: vec![
                ThermalBeam1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                ThermalBeam1dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![ThermalBeam1dElementInput {
                id: "tb0".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                distributed_load_y: 0.0,
                temperature_gradient_y: 40.0,
            }],
        })
        .expect("thermal beam should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_moment > 0.0);
        assert!(result.max_stress > 0.0);
        assert_eq!(result.max_temperature_gradient, 40.0);
    }

    #[test]
    fn solves_a_small_thermal_frame_2d_with_restrained_expansion() {
        let result = solve_thermal_frame_2d(&SolveThermalFrame2dRequest {
            nodes: vec![
                kyuubiki_protocol::ThermalFrame2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
                kyuubiki_protocol::ThermalFrame2dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ThermalFrame2dElementInput {
                id: "tf0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                temperature_gradient_y: 30.0,
            }],
        })
        .expect("thermal frame should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_axial_force > 0.0);
        assert!(result.max_moment > 0.0);
        assert!(result.max_stress > 0.0);
        assert_eq!(result.max_temperature_delta, 35.0);
        assert_eq!(result.max_temperature_gradient, 30.0);
    }

    #[test]
    fn solves_a_small_thermal_frame_3d_with_restrained_expansion() {
        let result = solve_thermal_frame_3d(&SolveThermalFrame3dRequest {
            nodes: vec![
                ThermalFrame3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    fix_rx: true,
                    fix_ry: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
                ThermalFrame3dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    fix_rx: true,
                    fix_ry: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
            ],
            elements: vec![ThermalFrame3dElementInput {
                id: "tf3-0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                shear_modulus: 80.0e9,
                torsion_constant: 5.0e-6,
                moment_of_inertia_y: 8.0e-6,
                moment_of_inertia_z: 6.0e-6,
                section_modulus_y: 1.6e-4,
                section_modulus_z: 1.2e-4,
                thermal_expansion: 12.0e-6,
                section_depth_y: 0.2,
                section_depth_z: 0.15,
                temperature_gradient_y: 30.0,
                temperature_gradient_z: 20.0,
            }],
        })
        .expect("thermal frame 3d should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_displacement.abs() < 1.0e-12);
        assert!(result.max_axial_force > 0.0);
        assert!(result.max_moment > 0.0);
        assert!(result.max_stress > 0.0);
        assert_eq!(result.max_temperature_delta, 35.0);
        assert_eq!(result.max_temperature_gradient, 30.0);
    }

    #[test]
    fn solves_a_small_beam_1d_cantilever_with_uniform_load() {
        let result = solve_beam_1d(&SolveBeam1dRequest {
            nodes: vec![
                Beam1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                Beam1dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    fix_y: false,
                    fix_rz: false,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Beam1dElementInput {
                id: "b0".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                distributed_load_y: -1000.0,
            }],
        })
        .expect("1d beam with uniform load should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!((result.max_displacement - 0.0011904761904761906).abs() < 1.0e-12);
        assert!((result.max_rotation - 0.0007936507936507938).abs() < 1.0e-12);
        assert!((result.max_moment - 2000.0).abs() < 1.0e-6);
        assert!((result.max_stress - 1.25e7).abs() < 1.0e-2);
        assert!((result.elements[0].shear_force_i - 2000.0).abs() < 1.0e-6);
        assert!((result.elements[0].moment_i - 2000.0).abs() < 1.0e-6);
        assert!(result.elements[0].shear_force_j.abs() < 1.0e-6);
        assert!(result.elements[0].moment_j.abs() < 1.0e-6);
    }

    #[test]
    fn solves_a_small_torsion_1d_shaft() {
        let result = solve_torsion_1d(&SolveTorsion1dRequest {
            nodes: vec![
                Torsion1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_rz: true,
                    torque_z: 0.0,
                },
                Torsion1dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    fix_rz: false,
                    torque_z: 1200.0,
                },
            ],
            elements: vec![Torsion1dElementInput {
                id: "t0".to_string(),
                node_i: 0,
                node_j: 1,
                shear_modulus: 80.0e9,
                polar_moment: 3.0e-6,
                section_modulus: 2.0e-4,
            }],
        })
        .expect("1d torsion shaft should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!(result.nodes[1].rz > 0.0);
        assert!((result.max_torque - 1200.0).abs() < 1.0e-6);
        assert!((result.elements[0].torque - 1200.0).abs() < 1.0e-6);
        assert!((result.max_stress - 6.0e6).abs() < 1.0e-3);
    }

    #[test]
    fn solves_a_small_spring_1d_chain() {
        let result = solve_spring_1d(&SolveSpring1dRequest {
            nodes: vec![
                Spring1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_x: true,
                    load_x: 0.0,
                },
                Spring1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_x: false,
                    load_x: 1000.0,
                },
            ],
            elements: vec![Spring1dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("1d spring should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
        assert!((result.max_force - 1000.0).abs() < 1.0e-9);
        assert!((result.elements[0].extension - 0.04).abs() < 1.0e-12);
        assert!((result.elements[0].force - 1000.0).abs() < 1.0e-9);
    }

    #[test]
    fn solves_a_small_spring_2d_chain() {
        let result = solve_spring_2d(&SolveSpring2dRequest {
            nodes: vec![
                Spring2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                Spring2dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 1000.0,
                    load_y: 0.0,
                },
            ],
            elements: vec![Spring2dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("2d spring should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
        assert!((result.max_force - 1000.0).abs() < 1.0e-9);
        assert!((result.nodes[1].ux - 0.04).abs() < 1.0e-12);
        assert!(result.nodes[1].uy.abs() < 1.0e-12);
        assert!((result.elements[0].extension - 0.04).abs() < 1.0e-12);
        assert!((result.elements[0].force - 1000.0).abs() < 1.0e-9);
    }

    #[test]
    fn solves_a_small_spring_3d_chain() {
        let result = solve_spring_3d(&SolveSpring3dRequest {
            nodes: vec![
                Spring3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Spring3dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: false,
                    fix_y: true,
                    fix_z: true,
                    load_x: 1000.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
            ],
            elements: vec![Spring3dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("3d spring should solve");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
        assert!((result.max_force - 1000.0).abs() < 1.0e-9);
        assert!((result.nodes[1].ux - 0.04).abs() < 1.0e-12);
        assert!(result.nodes[1].uy.abs() < 1.0e-12);
        assert!(result.nodes[1].uz.abs() < 1.0e-12);
        assert!((result.elements[0].extension - 0.04).abs() < 1.0e-12);
        assert!((result.elements[0].force - 1000.0).abs() < 1.0e-9);
    }

    #[test]
    fn solves_a_small_two_dimensional_truss() {
        let result = solve_truss_2d(&SolveTruss2dRequest {
            nodes: vec![
                TrussNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                TrussNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                TrussNodeInput {
                    id: "n2".to_string(),
                    x: 0.5,
                    y: 0.75,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
            ],
            elements: vec![
                TrussElementInput {
                    id: "e0".to_string(),
                    node_i: 0,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                TrussElementInput {
                    id: "e1".to_string(),
                    node_i: 1,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                TrussElementInput {
                    id: "e2".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
            ],
        })
        .expect("2d truss should solve");

        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.elements.len(), 3);
        assert!(result.max_displacement > 0.0);
        assert!(result.max_stress > 0.0);
    }

    #[test]
    fn rejects_truss_responses_that_blow_past_small_displacement_limits() {
        let error = solve_truss_2d(&SolveTruss2dRequest {
            nodes: vec![
                TrussNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                TrussNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                TrussNodeInput {
                    id: "n2".to_string(),
                    x: 0.5,
                    y: 0.75,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
            ],
            elements: vec![
                TrussElementInput {
                    id: "e0".to_string(),
                    node_i: 0,
                    node_j: 2,
                    area: 1.0e-12,
                    youngs_modulus: 70.0e9,
                },
                TrussElementInput {
                    id: "e1".to_string(),
                    node_i: 1,
                    node_j: 2,
                    area: 1.0e-12,
                    youngs_modulus: 70.0e9,
                },
                TrussElementInput {
                    id: "e2".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 1.0e-12,
                    youngs_modulus: 70.0e9,
                },
            ],
        })
        .expect_err("overly soft truss should be rejected");

        assert!(error.contains("small-deformation"));
    }

    #[test]
    fn solves_a_small_plane_triangle_patch() {
        let request = SolvePlaneTriangle2dRequest {
            nodes: vec![
                PlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
                PlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
            ],
            elements: vec![
                PlaneTriangleElementInput {
                    id: "p0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                },
                PlaneTriangleElementInput {
                    id: "p1".to_string(),
                    node_i: 0,
                    node_j: 2,
                    node_k: 3,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                },
            ],
        };

        let result = solve_plane_triangle_2d(&request).expect("plane solve should succeed");

        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 2);
        assert!(result.max_displacement > 0.0);
        assert!(result.max_stress > 0.0);
    }

    #[test]
    fn solves_a_small_plane_quad_patch() {
        let request = SolvePlaneQuad2dRequest {
            nodes: vec![
                PlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
                PlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
            ],
            elements: vec![PlaneQuadElementInput {
                id: "q0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            }],
        };

        let result = solve_plane_quad_2d(&request).expect("plane quad solve should succeed");

        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_displacement > 0.0);
        assert!(result.max_stress > 0.0);
        assert!(result.elements[0].area > 0.0);
    }

    #[test]
    fn solves_a_small_thermal_plane_triangle_patch_with_restrained_expansion() {
        let request = SolveThermalPlaneTriangle2dRequest {
            nodes: vec![
                ThermalPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
            ],
            elements: vec![ThermalPlaneTriangleElementInput {
                id: "tp0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 12.0e-6,
            }],
        };

        let result =
            solve_thermal_plane_triangle_2d(&request).expect("thermal plane triangle should solve");

        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_displacement.abs() < 1.0e-12);
        assert!(result.max_stress > 1.0e7);
        assert_eq!(result.max_temperature_delta, 40.0);
        assert!(result.elements[0].stress_x < 0.0);
        assert!(result.elements[0].mechanical_strain_x < 0.0);
        assert_eq!(result.elements[0].average_temperature_delta, 40.0);
    }

    #[test]
    fn solves_a_small_thermal_plane_quad_patch_with_restrained_expansion() {
        let request = SolveThermalPlaneQuad2dRequest {
            nodes: vec![
                ThermalPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
            ],
            elements: vec![ThermalPlaneQuadElementInput {
                id: "tq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 11.0e-6,
            }],
        };

        let result =
            solve_thermal_plane_quad_2d(&request).expect("thermal plane quad should solve");

        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_displacement.abs() < 1.0e-12);
        assert!(result.max_stress > 1.0e7);
        assert_eq!(result.max_temperature_delta, 30.0);
        assert!(result.elements[0].stress_x < 0.0);
        assert!(result.elements[0].mechanical_strain_x < 0.0);
        assert_eq!(result.elements[0].average_temperature_delta, 30.0);
    }

    #[test]
    fn solves_a_small_frame_2d_cantilever() {
        let request = SolveFrame2dRequest {
            nodes: vec![
                Frame2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                Frame2dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: false,
                    fix_rz: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Frame2dElementInput {
                id: "f0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
            }],
        };

        let result = solve_frame_2d(&request).expect("frame solve should succeed");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_displacement > 0.0);
        assert!(result.max_rotation > 0.0);
        assert!(result.max_moment > 0.0);
        assert!(result.max_stress > 0.0);

        let tip = &result.nodes[1];
        let expected_tip_uy = (1000.0 * 2.0_f64.powi(3)) / (3.0 * 210.0e9 * 8.0e-6);
        let expected_tip_rz = (1000.0 * 2.0_f64.powi(2)) / (2.0 * 210.0e9 * 8.0e-6);

        assert!((tip.uy.abs() - expected_tip_uy).abs() / expected_tip_uy < 1.0e-6);
        assert!((tip.rz.abs() - expected_tip_rz).abs() / expected_tip_rz < 1.0e-6);
    }

    #[test]
    fn solves_a_small_frame_3d_cantilever() {
        let request = SolveFrame3dRequest {
            nodes: vec![
                Frame3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    fix_rx: true,
                    fix_ry: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                },
                Frame3dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: false,
                    fix_y: false,
                    fix_z: false,
                    fix_rx: false,
                    fix_ry: false,
                    fix_rz: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Frame3dElementInput {
                id: "f0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                shear_modulus: 80.0e9,
                torsion_constant: 5.0e-6,
                moment_of_inertia_y: 8.0e-6,
                moment_of_inertia_z: 8.0e-6,
                section_modulus_y: 1.6e-4,
                section_modulus_z: 1.6e-4,
            }],
        };

        let result = solve_frame_3d(&request).expect("3d frame solve should succeed");

        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.elements.len(), 1);
        assert!(result.max_displacement > 0.0);
        assert!(result.max_rotation > 0.0);
        assert!(result.max_moment > 0.0);
        assert!(result.max_stress > 0.0);

        let tip = &result.nodes[1];
        let expected_tip_uy = (1000.0 * 2.0_f64.powi(3)) / (3.0 * 210.0e9 * 8.0e-6);
        let expected_tip_rz = (1000.0 * 2.0_f64.powi(2)) / (2.0 * 210.0e9 * 8.0e-6);

        assert!((tip.uy.abs() - expected_tip_uy).abs() / expected_tip_uy < 1.0e-6);
        assert!((tip.rz.abs() - expected_tip_rz).abs() / expected_tip_rz < 1.0e-6);
    }

    #[test]
    fn solves_a_small_three_dimensional_truss() {
        let request = SolveTruss3dRequest {
            nodes: vec![
                Truss3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Truss3dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Truss3dNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Truss3dNodeInput {
                    id: "n3".to_string(),
                    x: 0.2,
                    y: 0.2,
                    z: 1.0,
                    fix_x: false,
                    fix_y: false,
                    fix_z: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: -1000.0,
                },
            ],
            elements: vec![
                Truss3dElementInput {
                    id: "e0".to_string(),
                    node_i: 0,
                    node_j: 3,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e1".to_string(),
                    node_i: 1,
                    node_j: 3,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e2".to_string(),
                    node_i: 2,
                    node_j: 3,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e3".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e4".to_string(),
                    node_i: 1,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e5".to_string(),
                    node_i: 2,
                    node_j: 0,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
            ],
        };

        let result = solve_truss_3d(&request).expect("3d truss should solve");

        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.elements.len(), 6);
        assert!(result.max_displacement > 0.0);
        assert!(result.max_stress > 0.0);
    }
}
