use kyuubiki_protocol::{
    ElementResult, Job, JobStatus, NodeResult, PlaneNodeResult, PlaneTriangleElementResult,
    ProgressEvent, SolveBarRequest, SolveBarResult, SolvePlaneTriangle2dRequest,
    SolvePlaneTriangle2dResult, SolveTruss2dRequest, SolveTruss2dResult, TrussElementResult,
    TrussNodeResult,
};

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

pub fn solve_bar_1d(request: &SolveBarRequest) -> Result<SolveBarResult, String> {
    validate_request(request)?;

    let node_count = request.elements + 1;
    let element_length = request.length / request.elements as f64;
    let stiffness = request.youngs_modulus * request.area / element_length;
    let mut global_stiffness = zero_matrix(node_count);
    let mut force_vector = vec![0.0; node_count];
    force_vector[node_count - 1] = request.tip_force;

    for index in 0..request.elements {
        add_at(&mut global_stiffness, index, index, stiffness);
        add_at(&mut global_stiffness, index, index + 1, -stiffness);
        add_at(&mut global_stiffness, index + 1, index, -stiffness);
        add_at(&mut global_stiffness, index + 1, index + 1, stiffness);
    }

    let reduced_stiffness = global_stiffness
        .iter()
        .skip(1)
        .map(|row| row.iter().skip(1).copied().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let reduced_force = force_vector.iter().skip(1).copied().collect::<Vec<_>>();
    let reduced_displacements = solve_linear_system(reduced_stiffness, reduced_force)?;

    let mut displacements = Vec::with_capacity(node_count);
    displacements.push(0.0);
    displacements.extend(reduced_displacements);

    let nodes = displacements
        .iter()
        .enumerate()
        .map(|(index, displacement)| NodeResult {
            index,
            x: request.length * index as f64 / request.elements as f64,
            displacement: *displacement,
        })
        .collect::<Vec<_>>();

    let elements = (0..request.elements)
        .map(|index| {
            let left = &nodes[index];
            let right = &nodes[index + 1];
            let strain = (right.displacement - left.displacement) / element_length;
            let stress = request.youngs_modulus * strain;

            ElementResult {
                index,
                x1: left.x,
                x2: right.x,
                strain,
                stress,
                axial_force: stress * request.area,
            }
        })
        .collect::<Vec<_>>();

    let reaction_force = global_stiffness[0]
        .iter()
        .zip(displacements.iter())
        .map(|(stiffness_ij, displacement)| stiffness_ij * displacement)
        .sum::<f64>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement.abs())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveBarResult {
        input: request.clone(),
        nodes,
        elements,
        tip_displacement: *displacements.last().unwrap_or(&0.0),
        reaction_force,
        max_displacement,
        max_stress,
    })
}

pub fn solve_truss_2d(request: &SolveTruss2dRequest) -> Result<SolveTruss2dResult, String> {
    validate_truss_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = zero_matrix(dof_count);
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

    let free = (0..dof_count)
        .filter(|dof| !constrained.contains(dof))
        .collect::<Vec<_>>();

    let reduced_stiffness = free
        .iter()
        .map(|&row| {
            free.iter()
                .map(|&column| global_stiffness[row][column])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let reduced_force = free
        .iter()
        .map(|&row| force_vector[row])
        .collect::<Vec<_>>();
    let reduced_displacements = solve_linear_system(reduced_stiffness, reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| TrussNodeResult {
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
            let axial_extension = (ux_j - ux_i) * c + (uy_j - uy_i) * s;
            let strain = axial_extension / length;
            let stress = element.youngs_modulus * strain;

            TrussElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                strain,
                stress,
                axial_force: stress * element.area,
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

    validate_small_displacement_truss(request, max_displacement)?;

    Ok(SolveTruss2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
    })
}

fn validate_small_displacement_truss(
    request: &SolveTruss2dRequest,
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
            "truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

pub fn solve_plane_triangle_2d(
    request: &SolvePlaneTriangle2dRequest,
) -> Result<SolvePlaneTriangle2dResult, String> {
    validate_plane_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = zero_matrix(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for element in &request.elements {
        let (element_stiffness, _area, _b, _d) = triangle_element_data(request, element)?;
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
                    element_stiffness[row][column],
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

    let free = (0..dof_count)
        .filter(|dof| !constrained.contains(dof))
        .collect::<Vec<_>>();

    let reduced_stiffness = free
        .iter()
        .map(|&row| {
            free.iter()
                .map(|&column| global_stiffness[row][column])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let reduced_force = free
        .iter()
        .map(|&row| force_vector[row])
        .collect::<Vec<_>>();
    let reduced_displacements = solve_linear_system(reduced_stiffness, reduced_force)?;

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
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let (_k, area, b_matrix, d_matrix) = triangle_element_data(request, element)?;

            let element_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
                displacements[element.node_k * 2],
                displacements[element.node_k * 2 + 1],
            ];

            let strain = multiply_matrix_vector_3x6(&b_matrix, &element_displacements);
            let stress = multiply_matrix_vector_3x3(&d_matrix, &strain);
            let von_mises = ((stress[0] * stress[0]) - (stress[0] * stress[1])
                + (stress[1] * stress[1])
                + 3.0 * stress[2] * stress[2])
                .sqrt();

            Ok(PlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area,
                strain_x: strain[0],
                strain_y: strain[1],
                gamma_xy: strain[2],
                stress_x: stress[0],
                stress_y: stress[1],
                tau_xy: stress[2],
                von_mises,
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

fn validate_request(request: &SolveBarRequest) -> Result<(), String> {
    if !(request.length.is_finite() && request.length > 0.0) {
        return Err("length must be a positive finite number".to_string());
    }

    if !(request.area.is_finite() && request.area > 0.0) {
        return Err("area must be a positive finite number".to_string());
    }

    if !(request.youngs_modulus.is_finite() && request.youngs_modulus > 0.0) {
        return Err("youngs_modulus must be a positive finite number".to_string());
    }

    if request.elements == 0 {
        return Err("elements must be a positive integer".to_string());
    }

    if !request.tip_force.is_finite() {
        return Err("tip_force must be a finite number".to_string());
    }

    Ok(())
}

fn validate_truss_request(request: &SolveTruss2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("truss must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("truss must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("truss must include at least one support".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("truss element references an out-of-range node".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("truss element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("truss element youngs_modulus must be positive".to_string());
        }
    }

    Ok(())
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

fn signed_triangle_area(
    node_i: &kyuubiki_protocol::PlaneNodeInput,
    node_j: &kyuubiki_protocol::PlaneNodeInput,
    node_k: &kyuubiki_protocol::PlaneNodeInput,
) -> f64 {
    0.5 * ((node_j.x - node_i.x) * (node_k.y - node_i.y)
        - (node_k.x - node_i.x) * (node_j.y - node_i.y))
}

fn get_planar_bounds(points: &[(f64, f64)]) -> (f64, f64) {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);

    (max_x - min_x, max_y - min_y)
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

fn zero_matrix(size: usize) -> Vec<Vec<f64>> {
    vec![vec![0.0; size]; size]
}

fn add_at(matrix: &mut [Vec<f64>], row: usize, column: usize, value: f64) {
    matrix[row][column] += value;
}

fn solve_linear_system(matrix: Vec<Vec<f64>>, vector: Vec<f64>) -> Result<Vec<f64>, String> {
    let size = vector.len();

    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err("matrix dimensions do not match vector".to_string());
    }

    let mut augmented = matrix
        .into_iter()
        .zip(vector)
        .map(|(mut row, value)| {
            row.push(value);
            row
        })
        .collect::<Vec<_>>();

    for pivot in 0..size {
        let max_row = (pivot..size)
            .max_by(|&left, &right| {
                augmented[left][pivot]
                    .abs()
                    .partial_cmp(&augmented[right][pivot].abs())
                    .expect("finite pivot comparisons")
            })
            .expect("pivot range should not be empty");

        augmented.swap(pivot, max_row);

        let pivot_value = augmented[pivot][pivot];
        if pivot_value.abs() < 1.0e-12 {
            return Err("system is singular".to_string());
        }

        for column in pivot..=size {
            augmented[pivot][column] /= pivot_value;
        }

        for row in 0..size {
            if row == pivot {
                continue;
            }

            let factor = augmented[row][pivot];
            for column in pivot..=size {
                augmented[row][column] -= factor * augmented[pivot][column];
            }
        }
    }

    Ok(augmented.into_iter().map(|row| row[size]).collect())
}

#[cfg(test)]
mod tests {
    use super::{MockSolver, solve_bar_1d, solve_plane_triangle_2d, solve_truss_2d};
    use kyuubiki_protocol::{
        Job, JobStatus, PlaneNodeInput, PlaneTriangleElementInput, SolveBarRequest,
        SolvePlaneTriangle2dRequest, SolveTruss2dRequest, TrussElementInput, TrussNodeInput,
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
}
