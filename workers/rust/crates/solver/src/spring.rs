use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system, solve_tridiagonal_system,
};
use crate::spring_validation::{
    validate_spring_1d_request, validate_spring_2d_request, validate_spring_3d_request,
};
use kyuubiki_protocol::{
    SolveSpring1dRequest, SolveSpring1dResult, SolveSpring2dRequest, SolveSpring2dResult,
    SolveSpring3dRequest, SolveSpring3dResult, Spring1dElementResult, Spring1dNodeResult,
    Spring2dElementResult, Spring2dNodeResult, Spring3dElementResult, Spring3dNodeResult,
};

pub fn solve_spring_1d(request: &SolveSpring1dRequest) -> Result<SolveSpring1dResult, String> {
    validate_spring_1d_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index] = node.load_x;
    }

    for element in &request.elements {
        let map = [element.node_i, element.node_j];
        let local_stiffness = [
            [element.stiffness, -element.stiffness],
            [-element.stiffness, element.stiffness],
        ];

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
        .filter_map(|(index, node)| if node.fix_x { Some(index) } else { None })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_tridiagonal_system(&reduced_stiffness, &reduced_force)
        .unwrap_or_else(|| solve_spd_system(&reduced_stiffness, &reduced_force))?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| Spring1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            ux: displacements[index],
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let extension = displacements[element.node_j] - displacements[element.node_i];
            let force = element.stiffness * extension;
            let strain_energy = 0.5 * force * extension;

            Spring1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length: (node_j.x - node_i.x).abs(),
                extension,
                force,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.ux.abs())
        .fold(0.0_f64, f64::max);
    let max_force = elements
        .iter()
        .map(|element| element.force.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveSpring1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_force,
        total_strain_energy,
    })
}

pub fn solve_spring_2d(request: &SolveSpring2dRequest) -> Result<SolveSpring2dResult, String> {
    validate_spring_2d_request(request)?;

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
        let k = element.stiffness;

        let local = [
            [c * c, c * s, -c * c, -c * s],
            [c * s, s * s, -c * s, -s * s],
            [-c * c, -c * s, c * c, c * s],
            [-c * s, -s * s, c * s, s * s],
        ];

        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
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
        .map(|(index, node)| Spring2dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            ux: displacements[index * 2],
            uy: displacements[index * 2 + 1],
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
            let extension = (ux_j - ux_i) * c + (uy_j - uy_i) * s;
            let force = element.stiffness * extension;
            let strain_energy = 0.5 * force * extension;

            Spring2dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                extension,
                force,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy).sqrt())
        .fold(0.0_f64, f64::max);
    let max_force = elements
        .iter()
        .map(|element| element.force.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveSpring2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_force,
        total_strain_energy,
    })
}

pub fn solve_spring_3d(request: &SolveSpring3dRequest) -> Result<SolveSpring3dResult, String> {
    validate_spring_3d_request(request)?;

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
        let k = element.stiffness;

        let local = [
            [l * l, l * m, l * n, -l * l, -l * m, -l * n],
            [l * m, m * m, m * n, -l * m, -m * m, -m * n],
            [l * n, m * n, n * n, -l * n, -m * n, -n * n],
            [-l * l, -l * m, -l * n, l * l, l * m, l * n],
            [-l * m, -m * m, -m * n, l * m, m * m, m * n],
            [-l * n, -m * n, -n * n, l * n, m * n, n * n],
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
        .map(|(index, node)| Spring3dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            z: node.z,
            ux: displacements[index * 3],
            uy: displacements[index * 3 + 1],
            uz: displacements[index * 3 + 2],
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
            let extension = (ux_j - ux_i) * l + (uy_j - uy_i) * m + (uz_j - uz_i) * n;
            let force = element.stiffness * extension;
            let strain_energy = 0.5 * force * extension;

            Spring3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                extension,
                force,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt())
        .fold(0.0_f64, f64::max);
    let max_force = elements
        .iter()
        .map(|element| element.force.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveSpring3dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_force,
        total_strain_energy,
    })
}
