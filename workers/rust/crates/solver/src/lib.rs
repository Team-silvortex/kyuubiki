use kyuubiki_protocol::{
    ElementResult, Job, JobStatus, NodeResult, ProgressEvent, SolveBarRequest, SolveBarResult,
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
    use super::{MockSolver, solve_bar_1d};
    use kyuubiki_protocol::{Job, JobStatus, SolveBarRequest};

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
}
