use kyuubiki_protocol::{
    Beam1dElementResult, Beam1dNodeResult, ElectrostaticPlaneNodeResult,
    ElectrostaticPlaneQuadElementInput, ElectrostaticPlaneQuadElementResult,
    ElectrostaticPlaneTriangleElementInput, ElectrostaticPlaneTriangleElementResult,
    Frame2dElementResult, Frame2dNodeResult, Frame3dElementResult, Frame3dNodeResult,
    HeatPlaneNodeInput, HeatPlaneNodeResult, HeatPlaneQuadElementInput, HeatPlaneQuadElementResult,
    HeatPlaneTriangleElementInput, HeatPlaneTriangleElementResult, Job, JobStatus, PlaneNodeResult,
    PlaneQuadElementInput, PlaneQuadElementResult, PlaneTriangleElementInput,
    PlaneTriangleElementResult, ProgressEvent, SolveBeam1dRequest, SolveBeam1dResult,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneQuad2dResult,
    SolveElectrostaticPlaneTriangle2dRequest, SolveElectrostaticPlaneTriangle2dResult,
    SolveFrame2dRequest, SolveFrame2dResult, SolveFrame3dRequest, SolveFrame3dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult, SolvePlaneQuad2dRequest, SolvePlaneQuad2dResult,
    SolvePlaneTriangle2dRequest, SolvePlaneTriangle2dResult, SolveThermalBeam1dRequest,
    SolveThermalBeam1dResult, SolveThermalFrame2dRequest, SolveThermalFrame2dResult,
    SolveThermalFrame3dRequest, SolveThermalFrame3dResult, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneQuad2dResult, SolveThermalPlaneTriangle2dRequest,
    SolveThermalPlaneTriangle2dResult, SolveTorsion1dRequest, SolveTorsion1dResult,
    ThermalBeam1dElementResult, ThermalBeam1dNodeResult, ThermalFrame2dElementResult,
    ThermalFrame2dNodeResult, ThermalFrame3dElementResult, ThermalFrame3dNodeResult,
    ThermalPlaneNodeResult, ThermalPlaneQuadElementInput, ThermalPlaneQuadElementResult,
    ThermalPlaneTriangleElementInput, ThermalPlaneTriangleElementResult, Torsion1dElementResult,
    Torsion1dNodeResult,
};

mod bar_1d;
mod linear_algebra;
mod spring;
mod thermal_truss;
mod truss;

pub use bar_1d::{
    solve_bar_1d, solve_electrostatic_bar_1d, solve_heat_bar_1d, solve_thermal_bar_1d,
};
use linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, reduce_sparse_system_with_prescribed,
    solve_spd_system,
};
pub use spring::{solve_spring_1d, solve_spring_2d, solve_spring_3d};
pub use thermal_truss::{solve_thermal_truss_2d, solve_thermal_truss_3d};
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

pub fn solve_beam_1d(request: &SolveBeam1dRequest) -> Result<SolveBeam1dResult, String> {
    validate_beam_1d_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_y;
        force_vector[index * 2 + 1] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let local_stiffness =
            beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
        let equivalent_load = beam_uniform_load_vector(length, element.distributed_load_y);
        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
            force_vector[map[row]] += equivalent_load[row];
        }

        for row in 0..4 {
            for column in 0..4 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    local_stiffness[row][column],
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
            if node.fix_y {
                dofs.push(index * 2);
            }
            if node.fix_rz {
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
        .map(|(index, node)| {
            let uy = displacements[index * 2];
            let rz = displacements[index * 2 + 1];

            Beam1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                uy,
                rz,
                displacement_magnitude: uy.abs(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let local_stiffness =
                beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
            let local_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
            ];
            let equivalent_load = beam_uniform_load_vector(length, element.distributed_load_y);
            let local_forces = subtract_vector_4(
                &multiply_matrix_vector_4x4(&local_stiffness, &local_displacements),
                &equivalent_load,
            );
            let max_bending_stress =
                local_forces[1].abs().max(local_forces[3].abs()) / element.section_modulus;

            Beam1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                shear_force_i: local_forces[0],
                moment_i: local_forces[1],
                shear_force_j: local_forces[2],
                moment_j: local_forces[3],
                max_bending_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_bending_stress)
        .fold(0.0_f64, f64::max);

    Ok(SolveBeam1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
    })
}

pub fn solve_thermal_beam_1d(
    request: &SolveThermalBeam1dRequest,
) -> Result<SolveThermalBeam1dResult, String> {
    validate_thermal_beam_1d_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_y;
        force_vector[index * 2 + 1] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let local_stiffness =
            beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
        let equivalent_load = add_vector_4(
            &beam_uniform_load_vector(length, element.distributed_load_y),
            &beam_thermal_gradient_vector(
                element.youngs_modulus,
                element.moment_of_inertia,
                element.thermal_expansion,
                element.section_depth,
                element.temperature_gradient_y,
            ),
        );
        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
            force_vector[map[row]] += equivalent_load[row];
        }

        for row in 0..4 {
            for column in 0..4 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    local_stiffness[row][column],
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
            if node.fix_y {
                dofs.push(index * 2);
            }
            if node.fix_rz {
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
        .map(|(index, node)| {
            let uy = displacements[index * 2];
            let rz = displacements[index * 2 + 1];

            ThermalBeam1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                uy,
                rz,
                displacement_magnitude: uy.abs(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let local_stiffness =
                beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
            let local_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
            ];
            let equivalent_load = add_vector_4(
                &beam_uniform_load_vector(length, element.distributed_load_y),
                &beam_thermal_gradient_vector(
                    element.youngs_modulus,
                    element.moment_of_inertia,
                    element.thermal_expansion,
                    element.section_depth,
                    element.temperature_gradient_y,
                ),
            );
            let local_forces = subtract_vector_4(
                &multiply_matrix_vector_4x4(&local_stiffness, &local_displacements),
                &equivalent_load,
            );
            let max_bending_stress =
                local_forces[1].abs().max(local_forces[3].abs()) / element.section_modulus;

            ThermalBeam1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                temperature_gradient_y: element.temperature_gradient_y,
                thermal_curvature: element.thermal_expansion * element.temperature_gradient_y
                    / element.section_depth,
                shear_force_i: local_forces[0],
                moment_i: local_forces[1],
                shear_force_j: local_forces[2],
                moment_j: local_forces[3],
                max_bending_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_bending_stress)
        .fold(0.0_f64, f64::max);
    let max_temperature_gradient = elements
        .iter()
        .map(|element| element.temperature_gradient_y.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveThermalBeam1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        max_temperature_gradient,
    })
}

pub fn solve_torsion_1d(request: &SolveTorsion1dRequest) -> Result<SolveTorsion1dResult, String> {
    validate_torsion_1d_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut torque_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        torque_vector[index] = node.torque_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let stiffness = element.shear_modulus * element.polar_moment / length;
        let map = [element.node_i, element.node_j];
        let local_stiffness = [[stiffness, -stiffness], [-stiffness, stiffness]];

        for row in 0..2 {
            for column in 0..2 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    local_stiffness[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_rz.then_some(index))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_torque, free) =
        reduce_sparse_system(&global_stiffness, &torque_vector, &constrained);
    let reduced_rotations = solve_spd_system(&reduced_stiffness, &reduced_torque)?;

    let mut rotations = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        rotations[dof] = reduced_rotations[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| Torsion1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            rz: rotations[index],
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let twist = rotations[element.node_j] - rotations[element.node_i];
            let torque = element.shear_modulus * element.polar_moment * twist / length;
            let shear_stress = torque.abs() / element.section_modulus;

            Torsion1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                twist,
                torque,
                shear_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_torque = elements
        .iter()
        .map(|element| element.torque.abs())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.shear_stress)
        .fold(0.0_f64, f64::max);

    Ok(SolveTorsion1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_rotation,
        max_torque,
        max_stress,
    })
}

pub fn solve_frame_3d(request: &SolveFrame3dRequest) -> Result<SolveFrame3dResult, String> {
    validate_frame_3d_request(request)?;

    let dof_count = request.nodes.len() * 6;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 6] = node.load_x;
        force_vector[index * 6 + 1] = node.load_y;
        force_vector[index * 6 + 2] = node.load_z;
        force_vector[index * 6 + 3] = node.moment_x;
        force_vector[index * 6 + 4] = node.moment_y;
        force_vector[index * 6 + 5] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let rotation = frame3d_rotation(dx, dy, dz, length)?;
        let local_stiffness = frame3d_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.shear_modulus,
            element.torsion_constant,
            element.moment_of_inertia_y,
            element.moment_of_inertia_z,
            length,
        );
        let transform = frame3d_transform(&rotation);
        let global_element_stiffness = transform_frame3d_stiffness(&local_stiffness, &transform);
        let map = frame3d_dof_map(element.node_i, element.node_j);

        for row in 0..12 {
            for column in 0..12 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
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
                dofs.push(index * 6);
            }
            if node.fix_y {
                dofs.push(index * 6 + 1);
            }
            if node.fix_z {
                dofs.push(index * 6 + 2);
            }
            if node.fix_rx {
                dofs.push(index * 6 + 3);
            }
            if node.fix_ry {
                dofs.push(index * 6 + 4);
            }
            if node.fix_rz {
                dofs.push(index * 6 + 5);
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
        .map(|(index, node)| {
            let ux = displacements[index * 6];
            let uy = displacements[index * 6 + 1];
            let uz = displacements[index * 6 + 2];
            let rx = displacements[index * 6 + 3];
            let ry = displacements[index * 6 + 4];
            let rz = displacements[index * 6 + 5];

            Frame3dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                ux,
                uy,
                uz,
                rx,
                ry,
                rz,
                displacement_magnitude: (ux * ux + uy * uy + uz * uz).sqrt(),
                rotation_magnitude: (rx * rx + ry * ry + rz * rz).sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let dz = node_j.z - node_i.z;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let rotation = frame3d_rotation(dx, dy, dz, length)
                .expect("validated 3d frame element should define a stable local axis");
            let local_stiffness = frame3d_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.shear_modulus,
                element.torsion_constant,
                element.moment_of_inertia_y,
                element.moment_of_inertia_z,
                length,
            );
            let transform = frame3d_transform(&rotation);
            let map = frame3d_dof_map(element.node_i, element.node_j);
            let global_displacements = [
                displacements[map[0]],
                displacements[map[1]],
                displacements[map[2]],
                displacements[map[3]],
                displacements[map[4]],
                displacements[map[5]],
                displacements[map[6]],
                displacements[map[7]],
                displacements[map[8]],
                displacements[map[9]],
                displacements[map[10]],
                displacements[map[11]],
            ];
            let local_displacements =
                multiply_matrix_vector_12x12(&transform, &global_displacements);
            let local_forces = multiply_matrix_vector_12x12(&local_stiffness, &local_displacements);
            let axial_stress = local_forces[0].abs().max(local_forces[6].abs()) / element.area;
            let bending_stress_y =
                local_forces[4].abs().max(local_forces[10].abs()) / element.section_modulus_y;
            let bending_stress_z =
                local_forces[5].abs().max(local_forces[11].abs()) / element.section_modulus_z;
            let max_bending_stress = bending_stress_y + bending_stress_z;
            let max_combined_stress = axial_stress + max_bending_stress;

            Frame3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                axial_force_i: local_forces[0],
                shear_force_y_i: local_forces[1],
                shear_force_z_i: local_forces[2],
                torsion_i: local_forces[3],
                moment_y_i: local_forces[4],
                moment_z_i: local_forces[5],
                axial_force_j: local_forces[6],
                shear_force_y_j: local_forces[7],
                shear_force_z_j: local_forces[8],
                torsion_j: local_forces[9],
                moment_y_j: local_forces[10],
                moment_z_j: local_forces[11],
                axial_stress,
                max_bending_stress,
                max_combined_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rotation_magnitude)
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| {
            [
                element.moment_y_i.abs(),
                element.moment_z_i.abs(),
                element.moment_y_j.abs(),
                element.moment_z_j.abs(),
            ]
        })
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_combined_stress)
        .fold(0.0_f64, f64::max);

    Ok(SolveFrame3dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
    })
}

pub fn solve_plane_triangle_2d(
    request: &SolvePlaneTriangle2dRequest,
) -> Result<SolvePlaneTriangle2dResult, String> {
    validate_plane_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_plane_triangle_element(request, element))
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

        for row in 0..6 {
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
        .map(|(index, node)| PlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
            displacement_magnitude: (displacements[index * 2].powi(2)
                + displacements[index * 2 + 1].powi(2))
            .sqrt(),
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

            let strain = multiply_matrix_vector_3x6(&computed.b_matrix, &element_displacements);
            let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &strain);
            let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

            Ok::<PlaneTriangleElementResult, String>(PlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                strain_x: strain[0],
                strain_y: strain[1],
                gamma_xy: strain[2],
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

    Ok(SolvePlaneTriangle2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
    })
}

pub fn solve_plane_quad_2d(
    request: &SolvePlaneQuad2dRequest,
) -> Result<SolvePlaneQuad2dResult, String> {
    validate_plane_quad_request(request)?;

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
        .map(|element| precompute_plane_quad_element(request, element))
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

            for row in 0..6 {
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
        .map(|(index, node)| PlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
            displacement_magnitude: (displacements[index * 2].powi(2)
                + displacements[index * 2 + 1].powi(2))
            .sqrt(),
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

            let first_state = plane_triangle_state(&computed.first, &first_displacements);
            let second_state = plane_triangle_state(&computed.second, &second_displacements);
            let total_area = computed.first.area + computed.second.area;

            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };

            PlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                strain_x: weighted(first_state.strain[0], second_state.strain[0]),
                strain_y: weighted(first_state.strain[1], second_state.strain[1]),
                gamma_xy: weighted(first_state.strain[2], second_state.strain[2]),
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

    Ok(SolvePlaneQuad2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
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

pub fn solve_frame_2d(request: &SolveFrame2dRequest) -> Result<SolveFrame2dResult, String> {
    validate_frame_2d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 3] = node.load_x;
        force_vector[index * 3 + 1] = node.load_y;
        force_vector[index * 3 + 2] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let local_stiffness = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let transform = frame_transform(c, s);
        let global_element_stiffness = transform_frame_stiffness(&local_stiffness, &transform);
        let map = [
            element.node_i * 3,
            element.node_i * 3 + 1,
            element.node_i * 3 + 2,
            element.node_j * 3,
            element.node_j * 3 + 1,
            element.node_j * 3 + 2,
        ];

        for row in 0..6 {
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
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
                dofs.push(index * 3);
            }
            if node.fix_y {
                dofs.push(index * 3 + 1);
            }
            if node.fix_rz {
                dofs.push(index * 3 + 2);
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
        .map(|(index, node)| {
            let ux = displacements[index * 3];
            let uy = displacements[index * 3 + 1];
            let rz = displacements[index * 3 + 2];

            Frame2dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                rz,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let length = (dx * dx + dy * dy).sqrt();
            let c = dx / length;
            let s = dy / length;
            let local_stiffness = frame_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.moment_of_inertia,
                length,
            );
            let transform = frame_transform(c, s);
            let global_displacements = [
                displacements[element.node_i * 3],
                displacements[element.node_i * 3 + 1],
                displacements[element.node_i * 3 + 2],
                displacements[element.node_j * 3],
                displacements[element.node_j * 3 + 1],
                displacements[element.node_j * 3 + 2],
            ];
            let local_displacements = multiply_matrix_vector_6x6(&transform, &global_displacements);
            let local_forces = multiply_matrix_vector_6x6(&local_stiffness, &local_displacements);
            let axial_stress = local_forces[0].abs().max(local_forces[3].abs()) / element.area;
            let bending_stress =
                local_forces[2].abs().max(local_forces[5].abs()) / element.section_modulus;
            let max_combined_stress = axial_stress + bending_stress;

            Frame2dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                axial_force_i: local_forces[0],
                shear_force_i: local_forces[1],
                moment_i: local_forces[2],
                axial_force_j: local_forces[3],
                shear_force_j: local_forces[4],
                moment_j: local_forces[5],
                axial_stress,
                max_bending_stress: bending_stress,
                max_combined_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_combined_stress)
        .fold(0.0_f64, f64::max);

    Ok(SolveFrame2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
    })
}

pub fn solve_thermal_frame_2d(
    request: &SolveThermalFrame2dRequest,
) -> Result<SolveThermalFrame2dResult, String> {
    validate_thermal_frame_2d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 3] = node.load_x;
        force_vector[index * 3 + 1] = node.load_y;
        force_vector[index * 3 + 2] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let local_stiffness = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let transform = frame_transform(c, s);
        let transform_t = transpose_6x6(&transform);
        let global_element_stiffness = transform_frame_stiffness(&local_stiffness, &transform);
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let equivalent_local = add_vector_6(
            &frame_thermal_uniform_vector(
                element.area,
                element.youngs_modulus,
                element.thermal_expansion,
                average_temperature_delta,
            ),
            &frame_thermal_gradient_vector(
                element.youngs_modulus,
                element.moment_of_inertia,
                element.thermal_expansion,
                element.section_depth,
                element.temperature_gradient_y,
            ),
        );
        let equivalent_global = multiply_matrix_vector_6x6(&transform_t, &equivalent_local);
        let map = [
            element.node_i * 3,
            element.node_i * 3 + 1,
            element.node_i * 3 + 2,
            element.node_j * 3,
            element.node_j * 3 + 1,
            element.node_j * 3 + 2,
        ];

        for row in 0..6 {
            force_vector[map[row]] += equivalent_global[row];
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
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
                dofs.push(index * 3);
            }
            if node.fix_y {
                dofs.push(index * 3 + 1);
            }
            if node.fix_rz {
                dofs.push(index * 3 + 2);
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
        .map(|(index, node)| {
            let ux = displacements[index * 3];
            let uy = displacements[index * 3 + 1];
            let rz = displacements[index * 3 + 2];

            ThermalFrame2dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                rz,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
                temperature_delta: node.temperature_delta,
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let length = (dx * dx + dy * dy).sqrt();
            let c = dx / length;
            let s = dy / length;
            let local_stiffness = frame_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.moment_of_inertia,
                length,
            );
            let transform = frame_transform(c, s);
            let global_displacements = [
                displacements[element.node_i * 3],
                displacements[element.node_i * 3 + 1],
                displacements[element.node_i * 3 + 2],
                displacements[element.node_j * 3],
                displacements[element.node_j * 3 + 1],
                displacements[element.node_j * 3 + 2],
            ];
            let local_displacements = multiply_matrix_vector_6x6(&transform, &global_displacements);
            let average_temperature_delta =
                0.5 * (node_i.temperature_delta + node_j.temperature_delta);
            let equivalent_local = add_vector_6(
                &frame_thermal_uniform_vector(
                    element.area,
                    element.youngs_modulus,
                    element.thermal_expansion,
                    average_temperature_delta,
                ),
                &frame_thermal_gradient_vector(
                    element.youngs_modulus,
                    element.moment_of_inertia,
                    element.thermal_expansion,
                    element.section_depth,
                    element.temperature_gradient_y,
                ),
            );
            let local_forces = subtract_vector_6(
                &multiply_matrix_vector_6x6(&local_stiffness, &local_displacements),
                &equivalent_local,
            );
            let thermal_strain = element.thermal_expansion * average_temperature_delta;
            let total_strain = (local_displacements[3] - local_displacements[0]) / length;
            let mechanical_strain = total_strain - thermal_strain;
            let axial_stress = local_forces[0].abs().max(local_forces[3].abs()) / element.area;
            let bending_stress =
                local_forces[2].abs().max(local_forces[5].abs()) / element.section_modulus;
            let max_combined_stress = axial_stress + bending_stress;

            ThermalFrame2dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature_delta,
                thermal_strain,
                mechanical_strain,
                total_strain,
                temperature_gradient_y: element.temperature_gradient_y,
                thermal_curvature: element.thermal_expansion * element.temperature_gradient_y
                    / element.section_depth,
                axial_force_i: local_forces[0],
                shear_force_i: local_forces[1],
                moment_i: local_forces[2],
                axial_force_j: local_forces[3],
                shear_force_j: local_forces[4],
                moment_j: local_forces[5],
                axial_stress,
                max_bending_stress: bending_stress,
                max_combined_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_combined_stress)
        .fold(0.0_f64, f64::max);
    let max_axial_force = elements
        .iter()
        .flat_map(|element| [element.axial_force_i.abs(), element.axial_force_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_gradient = elements
        .iter()
        .map(|element| element.temperature_gradient_y.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveThermalFrame2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        max_temperature_gradient,
    })
}

pub fn solve_thermal_frame_3d(
    request: &SolveThermalFrame3dRequest,
) -> Result<SolveThermalFrame3dResult, String> {
    validate_thermal_frame_3d_request(request)?;

    let dof_count = request.nodes.len() * 6;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 6] = node.load_x;
        force_vector[index * 6 + 1] = node.load_y;
        force_vector[index * 6 + 2] = node.load_z;
        force_vector[index * 6 + 3] = node.moment_x;
        force_vector[index * 6 + 4] = node.moment_y;
        force_vector[index * 6 + 5] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let rotation = frame3d_rotation(dx, dy, dz, length)?;
        let local_stiffness = frame3d_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.shear_modulus,
            element.torsion_constant,
            element.moment_of_inertia_y,
            element.moment_of_inertia_z,
            length,
        );
        let transform = frame3d_transform(&rotation);
        let global_element_stiffness = transform_frame3d_stiffness(&local_stiffness, &transform);
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let equivalent_local = add_vector_12(
            &frame3d_thermal_uniform_vector(
                element.area,
                element.youngs_modulus,
                element.thermal_expansion,
                average_temperature_delta,
            ),
            &frame3d_thermal_gradient_vector(
                element.youngs_modulus,
                element.moment_of_inertia_y,
                element.moment_of_inertia_z,
                element.thermal_expansion,
                element.section_depth_y,
                element.section_depth_z,
                element.temperature_gradient_y,
                element.temperature_gradient_z,
            ),
        );
        let equivalent_global =
            multiply_matrix_vector_12x12(&transpose_12x12(&transform), &equivalent_local);
        let map = frame3d_dof_map(element.node_i, element.node_j);

        for row in 0..12 {
            force_vector[map[row]] += equivalent_global[row];
            for column in 0..12 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
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
                dofs.push(index * 6);
            }
            if node.fix_y {
                dofs.push(index * 6 + 1);
            }
            if node.fix_z {
                dofs.push(index * 6 + 2);
            }
            if node.fix_rx {
                dofs.push(index * 6 + 3);
            }
            if node.fix_ry {
                dofs.push(index * 6 + 4);
            }
            if node.fix_rz {
                dofs.push(index * 6 + 5);
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
        .map(|(index, node)| {
            let ux = displacements[index * 6];
            let uy = displacements[index * 6 + 1];
            let uz = displacements[index * 6 + 2];
            let rx = displacements[index * 6 + 3];
            let ry = displacements[index * 6 + 4];
            let rz = displacements[index * 6 + 5];

            ThermalFrame3dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                ux,
                uy,
                uz,
                rx,
                ry,
                rz,
                displacement_magnitude: (ux * ux + uy * uy + uz * uz).sqrt(),
                rotation_magnitude: (rx * rx + ry * ry + rz * rz).sqrt(),
                temperature_delta: node.temperature_delta,
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let dz = node_j.z - node_i.z;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let rotation = frame3d_rotation(dx, dy, dz, length)
                .expect("validated thermal 3d frame element should define a stable local axis");
            let local_stiffness = frame3d_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.shear_modulus,
                element.torsion_constant,
                element.moment_of_inertia_y,
                element.moment_of_inertia_z,
                length,
            );
            let transform = frame3d_transform(&rotation);
            let map = frame3d_dof_map(element.node_i, element.node_j);
            let global_displacements = [
                displacements[map[0]],
                displacements[map[1]],
                displacements[map[2]],
                displacements[map[3]],
                displacements[map[4]],
                displacements[map[5]],
                displacements[map[6]],
                displacements[map[7]],
                displacements[map[8]],
                displacements[map[9]],
                displacements[map[10]],
                displacements[map[11]],
            ];
            let local_displacements =
                multiply_matrix_vector_12x12(&transform, &global_displacements);
            let average_temperature_delta =
                0.5 * (node_i.temperature_delta + node_j.temperature_delta);
            let equivalent_local = add_vector_12(
                &frame3d_thermal_uniform_vector(
                    element.area,
                    element.youngs_modulus,
                    element.thermal_expansion,
                    average_temperature_delta,
                ),
                &frame3d_thermal_gradient_vector(
                    element.youngs_modulus,
                    element.moment_of_inertia_y,
                    element.moment_of_inertia_z,
                    element.thermal_expansion,
                    element.section_depth_y,
                    element.section_depth_z,
                    element.temperature_gradient_y,
                    element.temperature_gradient_z,
                ),
            );
            let local_forces = subtract_vector_12(
                &multiply_matrix_vector_12x12(&local_stiffness, &local_displacements),
                &equivalent_local,
            );
            let thermal_strain = element.thermal_expansion * average_temperature_delta;
            let total_strain = (local_displacements[6] - local_displacements[0]) / length;
            let mechanical_strain = total_strain - thermal_strain;
            let axial_stress = local_forces[0].abs().max(local_forces[6].abs()) / element.area;
            let bending_stress_y =
                local_forces[4].abs().max(local_forces[10].abs()) / element.section_modulus_y;
            let bending_stress_z =
                local_forces[5].abs().max(local_forces[11].abs()) / element.section_modulus_z;
            let max_bending_stress = bending_stress_y + bending_stress_z;
            let max_combined_stress = axial_stress + max_bending_stress;

            ThermalFrame3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature_delta,
                thermal_strain,
                mechanical_strain,
                total_strain,
                temperature_gradient_y: element.temperature_gradient_y,
                temperature_gradient_z: element.temperature_gradient_z,
                thermal_curvature_y: element.thermal_expansion * element.temperature_gradient_y
                    / element.section_depth_y,
                thermal_curvature_z: element.thermal_expansion * element.temperature_gradient_z
                    / element.section_depth_z,
                axial_force_i: local_forces[0],
                shear_force_y_i: local_forces[1],
                shear_force_z_i: local_forces[2],
                torsion_i: local_forces[3],
                moment_y_i: local_forces[4],
                moment_z_i: local_forces[5],
                axial_force_j: local_forces[6],
                shear_force_y_j: local_forces[7],
                shear_force_z_j: local_forces[8],
                torsion_j: local_forces[9],
                moment_y_j: local_forces[10],
                moment_z_j: local_forces[11],
                axial_stress,
                max_bending_stress,
                max_combined_stress,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rotation_magnitude)
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| {
            [
                element.moment_y_i.abs(),
                element.moment_z_i.abs(),
                element.moment_y_j.abs(),
                element.moment_z_j.abs(),
            ]
        })
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_combined_stress)
        .fold(0.0_f64, f64::max);
    let max_axial_force = elements
        .iter()
        .flat_map(|element| [element.axial_force_i.abs(), element.axial_force_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_gradient = elements
        .iter()
        .flat_map(|element| {
            [
                element.temperature_gradient_y.abs(),
                element.temperature_gradient_z.abs(),
            ]
        })
        .fold(0.0_f64, f64::max);

    Ok(SolveThermalFrame3dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        max_temperature_gradient,
    })
}

fn validate_plane_request(request: &SolvePlaneTriangle2dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("plane model must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("plane model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane model must include at least one support".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
        {
            return Err("plane element references an out-of-range node".to_string());
        }

        if !(element.thickness.is_finite() && element.thickness > 0.0) {
            return Err("plane element thickness must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("plane element youngs_modulus must be positive".to_string());
        }

        if !(element.poisson_ratio.is_finite()
            && element.poisson_ratio > -1.0
            && element.poisson_ratio < 0.5)
        {
            return Err("plane element poisson_ratio must be between -1.0 and 0.5".to_string());
        }

        let area = signed_triangle_area(
            &request.nodes[element.node_i],
            &request.nodes[element.node_j],
            &request.nodes[element.node_k],
        )
        .abs();

        if area <= 1.0e-12 {
            return Err("plane element area must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_plane_quad_request(request: &SolvePlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("plane quad model must define at least four nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("plane quad model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane quad model must include at least one support".to_string());
    }

    for element in &request.elements {
        let indices = [
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ];

        if indices.iter().any(|&index| index >= request.nodes.len()) {
            return Err("plane quad element references an out-of-range node".to_string());
        }

        let unique_count = indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if unique_count < 4 {
            return Err("plane quad element must reference four distinct nodes".to_string());
        }

        if !(element.thickness.is_finite() && element.thickness > 0.0) {
            return Err("plane quad element thickness must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("plane quad element youngs_modulus must be positive".to_string());
        }

        if !(element.poisson_ratio.is_finite()
            && element.poisson_ratio > -1.0
            && element.poisson_ratio < 0.5)
        {
            return Err(
                "plane quad element poisson_ratio must be between -1.0 and 0.5".to_string(),
            );
        }

        let first_area = signed_triangle_area(
            &request.nodes[element.node_i],
            &request.nodes[element.node_j],
            &request.nodes[element.node_k],
        )
        .abs();
        let second_area = signed_triangle_area(
            &request.nodes[element.node_i],
            &request.nodes[element.node_k],
            &request.nodes[element.node_l],
        )
        .abs();

        if first_area <= 1.0e-12 || second_area <= 1.0e-12 {
            return Err(
                "plane quad element must decompose into positive-area triangles".to_string(),
            );
        }
    }

    Ok(())
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

fn validate_frame_2d_request(request: &SolveFrame2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("2d frame must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("2d frame must define at least one element".to_string());
    }

    if !request
        .nodes
        .iter()
        .any(|node| node.fix_x || node.fix_y || node.fix_rz)
    {
        return Err("2d frame must include at least one support".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_rz)
    });
    if constrained_dofs < 3 {
        return Err("2d frame must restrain at least three degrees of freedom".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("2d frame element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("2d frame element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("2d frame element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("2d frame element youngs_modulus must be positive".to_string());
        }
        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("2d frame element moment_of_inertia must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("2d frame element section_modulus must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2) + (node_j.y - node_i.y).powi(2)).sqrt();
        if length <= 1.0e-12 {
            return Err("2d frame element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_thermal_frame_2d_request(request: &SolveThermalFrame2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal frame must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal frame must define at least one element".to_string());
    }

    if !request
        .nodes
        .iter()
        .any(|node| node.fix_x || node.fix_y || node.fix_rz)
    {
        return Err("thermal frame must include at least one support".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal frame node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("thermal frame element references an out-of-range node".to_string());
        }

        if element.node_i == element.node_j {
            return Err("thermal frame element cannot connect a node to itself".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("thermal frame element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal frame element youngs_modulus must be positive".to_string());
        }

        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("thermal frame element moment_of_inertia must be positive".to_string());
        }

        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("thermal frame element section_modulus must be positive".to_string());
        }

        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err("thermal frame element thermal_expansion must be non-negative".to_string());
        }

        if !(element.section_depth.is_finite() && element.section_depth > 0.0) {
            return Err("thermal frame element section_depth must be positive".to_string());
        }

        if !element.temperature_gradient_y.is_finite() {
            return Err("thermal frame element temperature_gradient_y must be finite".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        if !(length.is_finite() && length > 0.0) {
            return Err("thermal frame element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_thermal_frame_3d_request(request: &SolveThermalFrame3dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal 3d frame must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal 3d frame must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x)
            + usize::from(node.fix_y)
            + usize::from(node.fix_z)
            + usize::from(node.fix_rx)
            + usize::from(node.fix_ry)
            + usize::from(node.fix_rz)
    });
    if constrained_dofs < 6 {
        return Err("thermal 3d frame must restrain at least six degrees of freedom".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal 3d frame node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("thermal 3d frame element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("thermal 3d frame element cannot connect a node to itself".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("thermal 3d frame element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal 3d frame element youngs_modulus must be positive".to_string());
        }
        if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
            return Err("thermal 3d frame element shear_modulus must be positive".to_string());
        }
        if !(element.torsion_constant.is_finite() && element.torsion_constant > 0.0) {
            return Err("thermal 3d frame element torsion_constant must be positive".to_string());
        }
        if !(element.moment_of_inertia_y.is_finite() && element.moment_of_inertia_y > 0.0) {
            return Err(
                "thermal 3d frame element moment_of_inertia_y must be positive".to_string(),
            );
        }
        if !(element.moment_of_inertia_z.is_finite() && element.moment_of_inertia_z > 0.0) {
            return Err(
                "thermal 3d frame element moment_of_inertia_z must be positive".to_string(),
            );
        }
        if !(element.section_modulus_y.is_finite() && element.section_modulus_y > 0.0) {
            return Err("thermal 3d frame element section_modulus_y must be positive".to_string());
        }
        if !(element.section_modulus_z.is_finite() && element.section_modulus_z > 0.0) {
            return Err("thermal 3d frame element section_modulus_z must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "thermal 3d frame element thermal_expansion must be non-negative".to_string(),
            );
        }
        if !(element.section_depth_y.is_finite() && element.section_depth_y > 0.0) {
            return Err("thermal 3d frame element section_depth_y must be positive".to_string());
        }
        if !(element.section_depth_z.is_finite() && element.section_depth_z > 0.0) {
            return Err("thermal 3d frame element section_depth_z must be positive".to_string());
        }
        if !element.temperature_gradient_y.is_finite() {
            return Err(
                "thermal 3d frame element temperature_gradient_y must be finite".to_string(),
            );
        }
        if !element.temperature_gradient_z.is_finite() {
            return Err(
                "thermal 3d frame element temperature_gradient_z must be finite".to_string(),
            );
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        frame3d_rotation(dx, dy, dz, length)?;
    }

    Ok(())
}

fn validate_beam_1d_request(request: &SolveBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d beam must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("1d beam must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_y) + usize::from(node.fix_rz)
    });
    if constrained_dofs < 2 {
        return Err("1d beam must restrain at least two degrees of freedom".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d beam element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d beam element must connect two distinct nodes".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("1d beam element youngs_modulus must be positive".to_string());
        }
        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("1d beam element moment_of_inertia must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("1d beam element section_modulus must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if length <= 1.0e-12 {
            return Err("1d beam element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_thermal_beam_1d_request(request: &SolveThermalBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal beam requires at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal beam requires at least one element".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("thermal beam node {index} has invalid x"));
        }
        if !node.load_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("thermal beam node {index} has invalid load"));
        }
    }

    for (index, element) in request.elements.iter().enumerate() {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err(format!(
                "thermal beam element {index} references an unknown node"
            ));
        }

        if element.node_i == element.node_j {
            return Err(format!(
                "thermal beam element {index} must connect two distinct nodes"
            ));
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if length <= f64::EPSILON {
            return Err(format!("thermal beam element {index} has zero length"));
        }

        if element.youngs_modulus <= 0.0
            || element.moment_of_inertia <= 0.0
            || element.section_modulus <= 0.0
            || element.section_depth <= 0.0
        {
            return Err(format!(
                "thermal beam element {index} must have positive stiffness and section properties"
            ));
        }

        if !element.thermal_expansion.is_finite()
            || !element.distributed_load_y.is_finite()
            || !element.temperature_gradient_y.is_finite()
        {
            return Err(format!(
                "thermal beam element {index} has invalid thermal load data"
            ));
        }
    }

    Ok(())
}

fn validate_torsion_1d_request(request: &SolveTorsion1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d torsion model must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("1d torsion model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_rz) {
        return Err("1d torsion model must include at least one rotational support".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d torsion element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d torsion element must connect two distinct nodes".to_string());
        }
        if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
            return Err("1d torsion element shear_modulus must be positive".to_string());
        }
        if !(element.polar_moment.is_finite() && element.polar_moment > 0.0) {
            return Err("1d torsion element polar_moment must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("1d torsion element section_modulus must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if length <= 1.0e-12 {
            return Err("1d torsion element length must be positive".to_string());
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

fn validate_frame_3d_request(request: &SolveFrame3dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("3d frame must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d frame must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x)
            + usize::from(node.fix_y)
            + usize::from(node.fix_z)
            + usize::from(node.fix_rx)
            + usize::from(node.fix_ry)
            + usize::from(node.fix_rz)
    });
    if constrained_dofs < 6 {
        return Err("3d frame must restrain at least six degrees of freedom".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d frame element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("3d frame element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("3d frame element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("3d frame element youngs_modulus must be positive".to_string());
        }
        if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
            return Err("3d frame element shear_modulus must be positive".to_string());
        }
        if !(element.torsion_constant.is_finite() && element.torsion_constant > 0.0) {
            return Err("3d frame element torsion_constant must be positive".to_string());
        }
        if !(element.moment_of_inertia_y.is_finite() && element.moment_of_inertia_y > 0.0) {
            return Err("3d frame element moment_of_inertia_y must be positive".to_string());
        }
        if !(element.moment_of_inertia_z.is_finite() && element.moment_of_inertia_z > 0.0) {
            return Err("3d frame element moment_of_inertia_z must be positive".to_string());
        }
        if !(element.section_modulus_y.is_finite() && element.section_modulus_y > 0.0) {
            return Err("3d frame element section_modulus_y must be positive".to_string());
        }
        if !(element.section_modulus_z.is_finite() && element.section_modulus_z > 0.0) {
            return Err("3d frame element section_modulus_z must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        if length <= 1.0e-12 {
            return Err("3d frame element length must be positive".to_string());
        }

        frame3d_rotation(dx, dy, dz, length)?;
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
struct PlaneTriangleComputed {
    stiffness: [[f64; 6]; 6],
    area: f64,
    b_matrix: [[f64; 6]; 3],
    d_matrix: [[f64; 3]; 3],
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
struct PlaneQuadComputed {
    first: PlaneTriangleComputed,
    second: PlaneTriangleComputed,
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

fn precompute_plane_triangle_element(
    request: &SolvePlaneTriangle2dRequest,
    element: &kyuubiki_protocol::PlaneTriangleElementInput,
) -> Result<PlaneTriangleComputed, String> {
    let (stiffness, area, b_matrix, d_matrix) = triangle_element_data(request, element)?;

    Ok(PlaneTriangleComputed {
        stiffness,
        area,
        b_matrix,
        d_matrix,
    })
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

fn precompute_plane_quad_element(
    request: &SolvePlaneQuad2dRequest,
    element: &PlaneQuadElementInput,
) -> Result<PlaneQuadComputed, String> {
    let first = PlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    let second = PlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };

    let triangle_request = SolvePlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    };

    Ok(PlaneQuadComputed {
        first: precompute_plane_triangle_element(&triangle_request, &first)?,
        second: precompute_plane_triangle_element(&triangle_request, &second)?,
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
struct PlaneTriangleState {
    strain: [f64; 3],
    stress: [f64; 3],
    principal_stress_1: f64,
    principal_stress_2: f64,
    max_in_plane_shear: f64,
    von_mises: f64,
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

fn plane_triangle_state(
    computed: &PlaneTriangleComputed,
    element_displacements: &[f64; 6],
) -> PlaneTriangleState {
    let strain = multiply_matrix_vector_3x6(&computed.b_matrix, element_displacements);
    let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &strain);
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    PlaneTriangleState {
        strain,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
    }
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

fn frame_local_stiffness(
    area: f64,
    youngs_modulus: f64,
    moment_of_inertia: f64,
    length: f64,
) -> [[f64; 6]; 6] {
    let axial = youngs_modulus * area / length;
    let flexural = youngs_modulus * moment_of_inertia;
    let l2 = length * length;
    let l3 = l2 * length;

    [
        [axial, 0.0, 0.0, -axial, 0.0, 0.0],
        [
            0.0,
            12.0 * flexural / l3,
            6.0 * flexural / l2,
            0.0,
            -12.0 * flexural / l3,
            6.0 * flexural / l2,
        ],
        [
            0.0,
            6.0 * flexural / l2,
            4.0 * flexural / length,
            0.0,
            -6.0 * flexural / l2,
            2.0 * flexural / length,
        ],
        [-axial, 0.0, 0.0, axial, 0.0, 0.0],
        [
            0.0,
            -12.0 * flexural / l3,
            -6.0 * flexural / l2,
            0.0,
            12.0 * flexural / l3,
            -6.0 * flexural / l2,
        ],
        [
            0.0,
            6.0 * flexural / l2,
            2.0 * flexural / length,
            0.0,
            -6.0 * flexural / l2,
            4.0 * flexural / length,
        ],
    ]
}

fn frame_thermal_uniform_vector(
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    average_temperature_delta: f64,
) -> [f64; 6] {
    let thermal_force = youngs_modulus * area * thermal_expansion * average_temperature_delta;
    [-thermal_force, 0.0, 0.0, thermal_force, 0.0, 0.0]
}

fn beam_local_stiffness(youngs_modulus: f64, moment_of_inertia: f64, length: f64) -> [[f64; 4]; 4] {
    let flexural = youngs_modulus * moment_of_inertia;
    let l2 = length * length;
    let l3 = l2 * length;

    [
        [
            12.0 * flexural / l3,
            6.0 * flexural / l2,
            -12.0 * flexural / l3,
            6.0 * flexural / l2,
        ],
        [
            6.0 * flexural / l2,
            4.0 * flexural / length,
            -6.0 * flexural / l2,
            2.0 * flexural / length,
        ],
        [
            -12.0 * flexural / l3,
            -6.0 * flexural / l2,
            12.0 * flexural / l3,
            -6.0 * flexural / l2,
        ],
        [
            6.0 * flexural / l2,
            2.0 * flexural / length,
            -6.0 * flexural / l2,
            4.0 * flexural / length,
        ],
    ]
}

fn beam_uniform_load_vector(length: f64, distributed_load_y: f64) -> [f64; 4] {
    let l2 = length * length;

    [
        distributed_load_y * length / 2.0,
        distributed_load_y * l2 / 12.0,
        distributed_load_y * length / 2.0,
        -distributed_load_y * l2 / 12.0,
    ]
}

fn beam_thermal_gradient_vector(
    youngs_modulus: f64,
    moment_of_inertia: f64,
    thermal_expansion: f64,
    section_depth: f64,
    temperature_gradient_y: f64,
) -> [f64; 4] {
    let thermal_curvature = thermal_expansion * temperature_gradient_y / section_depth;
    let thermal_moment = youngs_modulus * moment_of_inertia * thermal_curvature;

    [0.0, -thermal_moment, 0.0, thermal_moment]
}

fn frame_thermal_gradient_vector(
    youngs_modulus: f64,
    moment_of_inertia: f64,
    thermal_expansion: f64,
    section_depth: f64,
    temperature_gradient_y: f64,
) -> [f64; 6] {
    let thermal_curvature = thermal_expansion * temperature_gradient_y / section_depth;
    let thermal_moment = youngs_modulus * moment_of_inertia * thermal_curvature;
    [0.0, 0.0, -thermal_moment, 0.0, 0.0, thermal_moment]
}

fn frame3d_thermal_uniform_vector(
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    average_temperature_delta: f64,
) -> [f64; 12] {
    let thermal_force = youngs_modulus * area * thermal_expansion * average_temperature_delta;
    [
        -thermal_force,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        thermal_force,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ]
}

fn frame3d_thermal_gradient_vector(
    youngs_modulus: f64,
    moment_of_inertia_y: f64,
    moment_of_inertia_z: f64,
    thermal_expansion: f64,
    section_depth_y: f64,
    section_depth_z: f64,
    temperature_gradient_y: f64,
    temperature_gradient_z: f64,
) -> [f64; 12] {
    let thermal_curvature_y = thermal_expansion * temperature_gradient_y / section_depth_y;
    let thermal_curvature_z = thermal_expansion * temperature_gradient_z / section_depth_z;
    let thermal_moment_z = youngs_modulus * moment_of_inertia_z * thermal_curvature_y;
    let thermal_moment_y = youngs_modulus * moment_of_inertia_y * thermal_curvature_z;

    [
        0.0,
        0.0,
        0.0,
        0.0,
        -thermal_moment_y,
        -thermal_moment_z,
        0.0,
        0.0,
        0.0,
        0.0,
        thermal_moment_y,
        thermal_moment_z,
    ]
}

fn frame_transform(c: f64, s: f64) -> [[f64; 6]; 6] {
    [
        [c, s, 0.0, 0.0, 0.0, 0.0],
        [-s, c, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, c, s, 0.0],
        [0.0, 0.0, 0.0, -s, c, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    ]
}

fn transform_frame_stiffness(
    local_stiffness: &[[f64; 6]; 6],
    transform: &[[f64; 6]; 6],
) -> [[f64; 6]; 6] {
    let transform_t = transpose_6x6(transform);
    let left = multiply_matrix_6x6_6x6(&transform_t, local_stiffness);
    multiply_matrix_6x6_6x6(&left, transform)
}

fn frame3d_rotation(dx: f64, dy: f64, dz: f64, length: f64) -> Result<[[f64; 3]; 3], String> {
    if length <= 1.0e-12 {
        return Err("3d frame element length must be positive".to_string());
    }

    let local_x = [dx / length, dy / length, dz / length];
    let reference = if local_x[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };

    let mut local_y = cross3(reference, local_x);
    let local_y_norm = norm3(local_y);
    if local_y_norm <= 1.0e-12 {
        return Err("3d frame element orientation is ill-defined".to_string());
    }
    local_y = scale3(local_y, 1.0 / local_y_norm);
    let local_z = cross3(local_x, local_y);

    Ok([local_x, local_y, local_z])
}

fn frame3d_local_stiffness(
    area: f64,
    youngs_modulus: f64,
    shear_modulus: f64,
    torsion_constant: f64,
    moment_of_inertia_y: f64,
    moment_of_inertia_z: f64,
    length: f64,
) -> [[f64; 12]; 12] {
    let axial = youngs_modulus * area / length;
    let torsion = shear_modulus * torsion_constant / length;

    let by1 = 12.0 * youngs_modulus * moment_of_inertia_y / length.powi(3);
    let by2 = 6.0 * youngs_modulus * moment_of_inertia_y / length.powi(2);
    let by3 = 4.0 * youngs_modulus * moment_of_inertia_y / length;
    let by4 = 2.0 * youngs_modulus * moment_of_inertia_y / length;

    let bz1 = 12.0 * youngs_modulus * moment_of_inertia_z / length.powi(3);
    let bz2 = 6.0 * youngs_modulus * moment_of_inertia_z / length.powi(2);
    let bz3 = 4.0 * youngs_modulus * moment_of_inertia_z / length;
    let bz4 = 2.0 * youngs_modulus * moment_of_inertia_z / length;

    let mut k = [[0.0; 12]; 12];

    k[0][0] = axial;
    k[0][6] = -axial;
    k[6][0] = -axial;
    k[6][6] = axial;

    k[3][3] = torsion;
    k[3][9] = -torsion;
    k[9][3] = -torsion;
    k[9][9] = torsion;

    let yz_idx = [1usize, 5usize, 7usize, 11usize];
    let yz_vals = [
        [bz1, bz2, -bz1, bz2],
        [bz2, bz3, -bz2, bz4],
        [-bz1, -bz2, bz1, -bz2],
        [bz2, bz4, -bz2, bz3],
    ];
    for row in 0..4 {
        for column in 0..4 {
            k[yz_idx[row]][yz_idx[column]] = yz_vals[row][column];
        }
    }

    let zy_idx = [2usize, 4usize, 8usize, 10usize];
    let zy_vals = [
        [by1, -by2, -by1, -by2],
        [-by2, by3, by2, by4],
        [-by1, by2, by1, by2],
        [-by2, by4, by2, by3],
    ];
    for row in 0..4 {
        for column in 0..4 {
            k[zy_idx[row]][zy_idx[column]] = zy_vals[row][column];
        }
    }

    k
}

fn frame3d_transform(rotation: &[[f64; 3]; 3]) -> [[f64; 12]; 12] {
    let mut transform = [[0.0; 12]; 12];
    for block in 0..4 {
        let offset = block * 3;
        for row in 0..3 {
            for column in 0..3 {
                transform[offset + row][offset + column] = rotation[row][column];
            }
        }
    }
    transform
}

fn transform_frame3d_stiffness(
    local_stiffness: &[[f64; 12]; 12],
    transform: &[[f64; 12]; 12],
) -> [[f64; 12]; 12] {
    let transform_t = transpose_12x12(transform);
    let left = multiply_matrix_12x12_12x12(&transform_t, local_stiffness);
    multiply_matrix_12x12_12x12(&left, transform)
}

fn frame3d_dof_map(node_i: usize, node_j: usize) -> [usize; 12] {
    [
        node_i * 6,
        node_i * 6 + 1,
        node_i * 6 + 2,
        node_i * 6 + 3,
        node_i * 6 + 4,
        node_i * 6 + 5,
        node_j * 6,
        node_j * 6 + 1,
        node_j * 6 + 2,
        node_j * 6 + 3,
        node_j * 6 + 4,
        node_j * 6 + 5,
    ]
}

fn transpose_6x6(input: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut output = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            output[column][row] = input[row][column];
        }
    }
    output
}

fn transpose_12x12(input: &[[f64; 12]; 12]) -> [[f64; 12]; 12] {
    let mut output = [[0.0; 12]; 12];
    for row in 0..12 {
        for column in 0..12 {
            output[column][row] = input[row][column];
        }
    }
    output
}

fn multiply_matrix_6x6_6x6(lhs: &[[f64; 6]; 6], rhs: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut output = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            output[row][column] = (0..6)
                .map(|index| lhs[row][index] * rhs[index][column])
                .sum();
        }
    }
    output
}

fn multiply_matrix_12x12_12x12(lhs: &[[f64; 12]; 12], rhs: &[[f64; 12]; 12]) -> [[f64; 12]; 12] {
    let mut output = [[0.0; 12]; 12];
    for row in 0..12 {
        for column in 0..12 {
            output[row][column] = (0..12)
                .map(|index| lhs[row][index] * rhs[index][column])
                .sum();
        }
    }
    output
}

fn multiply_matrix_vector_6x6(matrix: &[[f64; 6]; 6], vector: &[f64; 6]) -> [f64; 6] {
    let mut output = [0.0; 6];
    for row in 0..6 {
        output[row] = (0..6).map(|index| matrix[row][index] * vector[index]).sum();
    }
    output
}

fn multiply_matrix_vector_12x12(matrix: &[[f64; 12]; 12], vector: &[f64; 12]) -> [f64; 12] {
    let mut output = [0.0; 12];
    for row in 0..12 {
        output[row] = (0..12)
            .map(|index| matrix[row][index] * vector[index])
            .sum();
    }
    output
}

fn multiply_matrix_vector_4x4(matrix: &[[f64; 4]; 4], vector: &[f64; 4]) -> [f64; 4] {
    let mut output = [0.0; 4];
    for row in 0..4 {
        output[row] = (0..4).map(|index| matrix[row][index] * vector[index]).sum();
    }
    output
}

fn cross3(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [
        lhs[1] * rhs[2] - lhs[2] * rhs[1],
        lhs[2] * rhs[0] - lhs[0] * rhs[2],
        lhs[0] * rhs[1] - lhs[1] * rhs[0],
    ]
}

fn norm3(vector: [f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn scale3(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn subtract_vector_4(lhs: &[f64; 4], rhs: &[f64; 4]) -> [f64; 4] {
    let mut output = [0.0; 4];
    for index in 0..4 {
        output[index] = lhs[index] - rhs[index];
    }
    output
}

fn add_vector_4(lhs: &[f64; 4], rhs: &[f64; 4]) -> [f64; 4] {
    let mut output = [0.0; 4];
    for index in 0..4 {
        output[index] = lhs[index] + rhs[index];
    }
    output
}

fn subtract_vector_6(lhs: &[f64; 6], rhs: &[f64; 6]) -> [f64; 6] {
    let mut output = [0.0; 6];
    for index in 0..6 {
        output[index] = lhs[index] - rhs[index];
    }
    output
}

fn add_vector_6(lhs: &[f64; 6], rhs: &[f64; 6]) -> [f64; 6] {
    let mut output = [0.0; 6];
    for index in 0..6 {
        output[index] = lhs[index] + rhs[index];
    }
    output
}

fn subtract_vector_12(lhs: &[f64; 12], rhs: &[f64; 12]) -> [f64; 12] {
    let mut output = [0.0; 12];
    for index in 0..12 {
        output[index] = lhs[index] - rhs[index];
    }
    output
}

fn add_vector_12(lhs: &[f64; 12], rhs: &[f64; 12]) -> [f64; 12] {
    let mut output = [0.0; 12];
    for index in 0..12 {
        output[index] = lhs[index] + rhs[index];
    }
    output
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
