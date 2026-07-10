use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use kyuubiki_protocol::{
    SolveThermalTruss2dRequest, SolveThermalTruss2dResult, SolveThermalTruss3dRequest,
    SolveThermalTruss3dResult, ThermalTruss2dElementResult, ThermalTruss2dNodeResult,
    ThermalTruss3dElementResult, ThermalTruss3dNodeResult,
};

pub fn solve_thermal_truss_2d(
    request: &SolveThermalTruss2dRequest,
) -> Result<SolveThermalTruss2dResult, String> {
    validate_thermal_truss_2d_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let k = element.youngs_modulus * element.area / length;
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let thermal_force = element.youngs_modulus
            * element.area
            * element.thermal_expansion
            * average_temperature_delta;

        let local = [
            [c * c, c * s, -c * c, -c * s],
            [c * s, s * s, -c * s, -s * s],
            [-c * c, -c * s, c * c, c * s],
            [-c * s, -s * s, c * s, s * s],
        ];
        let equivalent_load = [
            -thermal_force * c,
            -thermal_force * s,
            thermal_force * c,
            thermal_force * s,
        ];

        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
            force_vector[map[row]] += equivalent_load[row];

            for column in 0..4 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    k * local[row][column],
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
        .map(|(index, node)| ThermalTruss2dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
            temperature_delta: node.temperature_delta,
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

            let ux_i = displacements[element.node_i * 2];
            let uy_i = displacements[element.node_i * 2 + 1];
            let ux_j = displacements[element.node_j * 2];
            let uy_j = displacements[element.node_j * 2 + 1];
            let average_temperature_delta =
                0.5 * (node_i.temperature_delta + node_j.temperature_delta);
            let total_strain = ((ux_j - ux_i) * c + (uy_j - uy_i) * s) / length;
            let thermal_strain = element.thermal_expansion * average_temperature_delta;
            let mechanical_strain = total_strain - thermal_strain;
            let stress = element.youngs_modulus * mechanical_strain;
            let strain_energy_density = 0.5 * stress * mechanical_strain;

            ThermalTruss2dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature_delta,
                thermal_strain,
                mechanical_strain,
                total_strain,
                stress,
                axial_force: stress * element.area,
                strain_energy_density,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy).sqrt())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max);
    let max_axial_force = elements
        .iter()
        .map(|element| element.axial_force.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum();
    let max_strain_energy_density = elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max);

    validate_small_displacement_thermal_truss_2d(request, max_displacement)?;

    Ok(SolveThermalTruss2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        total_strain_energy,
        max_strain_energy_density,
    })
}

pub fn solve_thermal_truss_3d(
    request: &SolveThermalTruss3dRequest,
) -> Result<SolveThermalTruss3dResult, String> {
    validate_thermal_truss_3d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 3] = node.load_x;
        force_vector[index * 3 + 1] = node.load_y;
        force_vector[index * 3 + 2] = node.load_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let l = dx / length;
        let m = dy / length;
        let n = dz / length;
        let k = element.youngs_modulus * element.area / length;
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let thermal_force = element.youngs_modulus
            * element.area
            * element.thermal_expansion
            * average_temperature_delta;

        let local = [
            [l * l, l * m, l * n, -l * l, -l * m, -l * n],
            [l * m, m * m, m * n, -l * m, -m * m, -m * n],
            [l * n, m * n, n * n, -l * n, -m * n, -n * n],
            [-l * l, -l * m, -l * n, l * l, l * m, l * n],
            [-l * m, -m * m, -m * n, l * m, m * m, m * n],
            [-l * n, -m * n, -n * n, l * n, m * n, n * n],
        ];
        let equivalent_load = [
            -thermal_force * l,
            -thermal_force * m,
            -thermal_force * n,
            thermal_force * l,
            thermal_force * m,
            thermal_force * n,
        ];

        let map = [
            element.node_i * 3,
            element.node_i * 3 + 1,
            element.node_i * 3 + 2,
            element.node_j * 3,
            element.node_j * 3 + 1,
            element.node_j * 3 + 2,
        ];

        for row in 0..6 {
            force_vector[map[row]] += equivalent_load[row];

            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    k * local[row][column],
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
            if node.fix_z {
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
        .map(|(index, node)| ThermalTruss3dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            z: node.z,
            ux: displacements[index * 3],
            uy: displacements[index * 3 + 1],
            uz: displacements[index * 3 + 2],
            temperature_delta: node.temperature_delta,
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
            let l = dx / length;
            let m = dy / length;
            let n = dz / length;

            let ux_i = displacements[element.node_i * 3];
            let uy_i = displacements[element.node_i * 3 + 1];
            let uz_i = displacements[element.node_i * 3 + 2];
            let ux_j = displacements[element.node_j * 3];
            let uy_j = displacements[element.node_j * 3 + 1];
            let uz_j = displacements[element.node_j * 3 + 2];
            let average_temperature_delta =
                0.5 * (node_i.temperature_delta + node_j.temperature_delta);
            let total_strain = ((ux_j - ux_i) * l + (uy_j - uy_i) * m + (uz_j - uz_i) * n) / length;
            let thermal_strain = element.thermal_expansion * average_temperature_delta;
            let mechanical_strain = total_strain - thermal_strain;
            let stress = element.youngs_modulus * mechanical_strain;
            let strain_energy_density = 0.5 * stress * mechanical_strain;

            ThermalTruss3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature_delta,
                thermal_strain,
                mechanical_strain,
                total_strain,
                stress,
                axial_force: stress * element.area,
                strain_energy_density,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max);
    let max_axial_force = elements
        .iter()
        .map(|element| element.axial_force.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum();
    let max_strain_energy_density = elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max);

    validate_small_displacement_thermal_truss_3d(request, max_displacement)?;

    Ok(SolveThermalTruss3dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        total_strain_energy,
        max_strain_energy_density,
    })
}

fn validate_thermal_truss_2d_request(request: &SolveThermalTruss2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal truss must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal truss must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal truss must include at least one support".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal truss node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("thermal truss element references an out-of-range node".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("thermal truss element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal truss element youngs_modulus must be positive".to_string());
        }

        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err("thermal truss element thermal_expansion must be non-negative".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        if length <= 1.0e-12 {
            return Err("thermal truss element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_thermal_truss_3d_request(request: &SolveThermalTruss3dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("3d thermal truss must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d thermal truss must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_z)
    });
    if constrained_dofs < 6 {
        return Err("3d thermal truss must restrain at least six degrees of freedom".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("3d thermal truss node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d thermal truss element references an out-of-range node".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("3d thermal truss element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("3d thermal truss element youngs_modulus must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "3d thermal truss element thermal_expansion must be non-negative".to_string(),
            );
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2)
            + (node_j.y - node_i.y).powi(2)
            + (node_j.z - node_i.z).powi(2))
        .sqrt();
        if length <= 1.0e-12 {
            return Err("3d thermal truss element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_small_displacement_thermal_truss_2d(
    request: &SolveThermalTruss2dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    let bounds = get_planar_bounds(
        &request
            .nodes
            .iter()
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>(),
    );
    let characteristic_length = bounds.0.max(bounds.1).max(1.0e-9);

    if max_displacement > characteristic_length * 0.25 {
        return Err(
            "thermal truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

fn validate_small_displacement_thermal_truss_3d(
    request: &SolveThermalTruss3dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    let characteristic_length = get_spatial_bounds(
        &request
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>(),
    );

    if max_displacement > characteristic_length * 0.25 {
        return Err(
            "3d thermal truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

fn get_planar_bounds(points: &[(f64, f64)]) -> (f64, f64) {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);

    (max_x - min_x, max_y - min_y)
}

fn get_spatial_bounds(points: &[(f64, f64, f64)]) -> f64 {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);
    let min_z = points.iter().map(|point| point.2).fold(0.0_f64, f64::min);
    let max_z = points.iter().map(|point| point.2).fold(1.0_f64, f64::max);

    (max_x - min_x).max(max_y - min_y).max(max_z - min_z)
}
