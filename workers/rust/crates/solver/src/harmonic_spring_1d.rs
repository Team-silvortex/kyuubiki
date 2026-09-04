use crate::dynamic_spring_1d_validation::validate_harmonic_request;
use kyuubiki_protocol::{
    HarmonicSpring1dElementResponse, HarmonicSpring1dFrequencyResult, HarmonicSpring1dNodeResponse,
    SolveHarmonicSpring1dRequest, SolveHarmonicSpring1dResult,
};
use std::f64::consts::PI;

const MAX_DENSE_HARMONIC_DOFS: usize = 2_048;
const MIN_COMPLEX_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-12;
const MAX_COMPLEX_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-8;

#[derive(Clone, Copy, Debug, Default)]
struct Complex {
    re: f64,
    im: f64,
}

pub fn solve_harmonic_spring_1d(
    request: &SolveHarmonicSpring1dRequest,
) -> Result<SolveHarmonicSpring1dResult, String> {
    validate_harmonic_request(request)?;

    let count = request.nodes.len();
    let mut free = Vec::new();
    let mut reduced_index = vec![None; count];
    for (index, node) in request.nodes.iter().enumerate() {
        if !node.fix_x {
            reduced_index[index] = Some(free.len());
            free.push(index);
        }
    }
    if free.len() > MAX_DENSE_HARMONIC_DOFS {
        return Err(format!(
            "harmonic spring 1d has {} free degrees of freedom; the dense frequency solver supports at most {MAX_DENSE_HARMONIC_DOFS}",
            free.len()
        ));
    }

    let frequencies = request
        .frequencies_hz
        .iter()
        .map(|&frequency_hz| solve_frequency(request, frequency_hz, &free, &reduced_index))
        .collect::<Result<Vec<_>, _>>()?;

    let peak = frequencies
        .iter()
        .max_by(|a, b| a.max_displacement.total_cmp(&b.max_displacement))
        .ok_or_else(|| "harmonic spring 1d requires at least one frequency".to_string())?;

    Ok(SolveHarmonicSpring1dResult {
        input: request.clone(),
        max_displacement: frequencies
            .iter()
            .map(|result| result.max_displacement)
            .fold(0.0_f64, f64::max),
        max_velocity: frequencies
            .iter()
            .map(|result| result.max_velocity)
            .fold(0.0_f64, f64::max),
        max_acceleration: frequencies
            .iter()
            .map(|result| result.max_acceleration)
            .fold(0.0_f64, f64::max),
        max_force: frequencies
            .iter()
            .map(|result| result.max_force)
            .fold(0.0_f64, f64::max),
        peak_frequency_hz: peak.frequency_hz,
        frequencies,
    })
}

fn solve_frequency(
    request: &SolveHarmonicSpring1dRequest,
    frequency_hz: f64,
    free: &[usize],
    reduced_index: &[Option<usize>],
) -> Result<HarmonicSpring1dFrequencyResult, String> {
    let omega = 2.0 * PI * frequency_hz;
    let omega_squared = omega * omega;
    if !(omega.is_finite() && omega_squared.is_finite()) {
        return Err("harmonic spring 1d angular frequency exceeds the numeric range".to_string());
    }
    let (matrix, rhs) =
        assemble_reduced_dynamic_stiffness(request, omega, omega_squared, free, reduced_index);
    let solved = solve_complex_system(matrix, rhs)?;
    let mut displacement = vec![Complex::default(); request.nodes.len()];
    for (index, &dof) in free.iter().enumerate() {
        displacement[dof] = solved[index];
    }
    validate_harmonic_equilibrium(request, omega, omega_squared, &displacement, reduced_index)?;

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let amplitude = displacement[index].amplitude();
            HarmonicSpring1dNodeResponse {
                index,
                id: node.id.clone(),
                displacement_amplitude: amplitude,
                displacement_phase_deg: displacement[index].phase_deg(),
                velocity_amplitude: omega * amplitude,
                acceleration_amplitude: omega * omega * amplitude,
            }
        })
        .collect::<Vec<_>>();
    let elements = harmonic_elements(request, &displacement, omega);

    Ok(HarmonicSpring1dFrequencyResult {
        frequency_hz,
        angular_frequency: omega,
        max_displacement: nodes
            .iter()
            .map(|node| node.displacement_amplitude)
            .fold(0.0_f64, f64::max),
        max_velocity: nodes
            .iter()
            .map(|node| node.velocity_amplitude)
            .fold(0.0_f64, f64::max),
        max_acceleration: nodes
            .iter()
            .map(|node| node.acceleration_amplitude)
            .fold(0.0_f64, f64::max),
        max_force: elements
            .iter()
            .map(|element| element.force_amplitude)
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
    })
}

fn harmonic_elements(
    request: &SolveHarmonicSpring1dRequest,
    displacement: &[Complex],
    omega: f64,
) -> Vec<HarmonicSpring1dElementResponse> {
    request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let extension = displacement[element.node_j] - displacement[element.node_i];
            let force = extension
                * Complex {
                    re: element.stiffness,
                    im: omega * element.damping,
                };
            HarmonicSpring1dElementResponse {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                extension_amplitude: extension.amplitude(),
                force_amplitude: force.amplitude(),
            }
        })
        .collect()
}

fn assemble_reduced_dynamic_stiffness(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    omega_squared: f64,
    free: &[usize],
    reduced_index: &[Option<usize>],
) -> (Vec<Vec<Complex>>, Vec<Complex>) {
    let mut matrix = vec![vec![Complex::default(); free.len()]; free.len()];
    let mut rhs = vec![Complex::default(); free.len()];
    for (row, &node) in free.iter().enumerate() {
        matrix[row][row].re = -omega_squared * request.nodes[node].mass;
        rhs[row] = Complex::real(request.nodes[node].load_x);
    }
    for element in &request.elements {
        let coefficient = Complex {
            re: element.stiffness,
            im: omega * element.damping,
        };
        let first = reduced_index[element.node_i];
        let second = reduced_index[element.node_j];
        if let Some(first) = first {
            matrix[first][first] = matrix[first][first] + coefficient;
        }
        if let Some(second) = second {
            matrix[second][second] = matrix[second][second] + coefficient;
        }
        if let (Some(first), Some(second)) = (first, second) {
            matrix[first][second] = matrix[first][second] - coefficient;
            matrix[second][first] = matrix[second][first] - coefficient;
        }
    }
    (matrix, rhs)
}

fn solve_complex_system(
    mut matrix: Vec<Vec<Complex>>,
    mut rhs: Vec<Complex>,
) -> Result<Vec<Complex>, String> {
    let size = rhs.len();
    if size == 0 || matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err("harmonic spring 1d dynamic stiffness dimensions do not match".to_string());
    }
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || rhs.iter().any(|value| !value.is_finite())
    {
        return Err("harmonic spring 1d dynamic stiffness contains a non-finite value".to_string());
    }
    let row_scales = matrix
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| value.amplitude())
                .fold(0.0, f64::max)
        })
        .collect::<Vec<_>>();
    if row_scales.contains(&0.0) {
        return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
    }
    for ((row, rhs), row_scale) in matrix.iter_mut().zip(&mut rhs).zip(row_scales) {
        let scale = Complex::real(row_scale);
        for value in row {
            *value = *value / scale;
        }
        *rhs = *rhs / scale;
    }
    let relative_pivot_floor = 32.0 * f64::EPSILON * size as f64;

    for pivot in 0..size {
        let best = (pivot..size)
            .max_by(|&a, &b| {
                matrix[a][pivot]
                    .amplitude()
                    .total_cmp(&matrix[b][pivot].amplitude())
            })
            .expect("pivot range is non-empty");
        matrix.swap(pivot, best);
        rhs.swap(pivot, best);
        if matrix[pivot][pivot].amplitude() <= relative_pivot_floor {
            return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
        }

        for row in (pivot + 1)..size {
            let factor = matrix[row][pivot] / matrix[pivot][pivot];
            if !factor.is_finite() {
                return Err("harmonic spring 1d dynamic stiffness elimination diverged".to_string());
            }
            matrix[row][pivot] = Complex::default();
            for column in (pivot + 1)..size {
                matrix[row][column] = matrix[row][column] - factor * matrix[pivot][column];
                if !matrix[row][column].is_finite() {
                    return Err(
                        "harmonic spring 1d dynamic stiffness elimination diverged".to_string()
                    );
                }
            }
            rhs[row] = rhs[row] - factor * rhs[pivot];
            if !rhs[row].is_finite() {
                return Err("harmonic spring 1d dynamic stiffness elimination diverged".to_string());
            }
        }
    }

    let mut result = vec![Complex::default(); size];
    for row in (0..size).rev() {
        let mut sum = rhs[row];
        for (column, value) in result.iter().enumerate().skip(row + 1) {
            sum = sum - matrix[row][column] * *value;
        }
        result[row] = sum / matrix[row][row];
        if !result[row].is_finite() {
            return Err(
                "harmonic spring 1d dynamic stiffness back substitution diverged".to_string(),
            );
        }
    }
    Ok(result)
}

fn validate_harmonic_equilibrium(
    request: &SolveHarmonicSpring1dRequest,
    omega: f64,
    omega_squared: f64,
    displacement: &[Complex],
    reduced_index: &[Option<usize>],
) -> Result<(), String> {
    let mut dynamic_scale = request
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| reduced_index[*index].is_some())
        .map(|(_, node)| (omega_squared * node.mass).abs())
        .fold(0.0_f64, f64::max);
    for element in &request.elements {
        if reduced_index[element.node_i].is_some() || reduced_index[element.node_j].is_some() {
            dynamic_scale = dynamic_scale.max(
                Complex {
                    re: element.stiffness,
                    im: omega * element.damping,
                }
                .amplitude(),
            );
        }
    }
    if !(dynamic_scale.is_finite() && dynamic_scale > 0.0) {
        return Err("harmonic spring 1d dynamic stiffness has invalid scale".to_string());
    }

    let free_count = reduced_index.iter().flatten().count();
    let mut residuals = vec![Complex::default(); free_count];
    let mut equation_scales = vec![0.0_f64; free_count];
    let scale = Complex::real(dynamic_scale);
    for (node_index, node) in request.nodes.iter().enumerate() {
        let Some(row) = reduced_index[node_index] else {
            continue;
        };
        let rhs = Complex::real(node.load_x) / scale;
        let inertia = Complex::real(omega_squared * node.mass) / scale * displacement[node_index];
        residuals[row] = rhs + inertia;
        equation_scales[row] = rhs.amplitude() + inertia.amplitude();
    }
    for element in &request.elements {
        let coefficient = Complex {
            re: element.stiffness,
            im: omega * element.damping,
        } / scale;
        let extension = displacement[element.node_j] - displacement[element.node_i];
        let element_force = coefficient * extension;
        let force_scale = coefficient.amplitude()
            * (displacement[element.node_i].amplitude() + displacement[element.node_j].amplitude());
        if let Some(row) = reduced_index[element.node_i] {
            residuals[row] = residuals[row] + element_force;
            equation_scales[row] += force_scale;
        }
        if let Some(row) = reduced_index[element.node_j] {
            residuals[row] = residuals[row] - element_force;
            equation_scales[row] += force_scale;
        }
    }

    let tolerance = (128.0 * f64::EPSILON * free_count as f64).clamp(
        MIN_COMPLEX_BACKWARD_ERROR_TOLERANCE,
        MAX_COMPLEX_BACKWARD_ERROR_TOLERANCE,
    );
    let mut maximum = 0.0_f64;
    for (residual, equation_scale) in residuals.into_iter().zip(equation_scales) {
        if !(residual.is_finite() && equation_scale.is_finite()) {
            return Err("harmonic spring 1d dynamic stiffness residual is non-finite".to_string());
        }
        let relative = if equation_scale == 0.0 {
            0.0
        } else {
            residual.amplitude() / equation_scale
        };
        maximum = maximum.max(relative);
    }
    if maximum > tolerance {
        return Err(format!(
            "harmonic spring 1d dynamic stiffness failed backward-error validation ({maximum:.6e})"
        ));
    }
    Ok(())
}

impl Complex {
    fn real(value: f64) -> Self {
        Self { re: value, im: 0.0 }
    }

    fn amplitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    fn phase_deg(self) -> f64 {
        self.im.atan2(self.re).to_degrees()
    }
}

impl std::ops::Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl std::ops::Div for Complex {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let scale = rhs.re.abs().max(rhs.im.abs());
        let rhs_re = rhs.re / scale;
        let rhs_im = rhs.im / scale;
        let denominator = rhs_re * rhs_re + rhs_im * rhs_im;
        let left_re = self.re / scale;
        let left_im = self.im / scale;
        Self {
            re: (left_re * rhs_re + left_im * rhs_im) / denominator,
            im: (left_im * rhs_re - left_re * rhs_im) / denominator,
        }
    }
}
