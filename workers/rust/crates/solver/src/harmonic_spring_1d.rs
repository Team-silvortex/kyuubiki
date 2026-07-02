use kyuubiki_protocol::{
    HarmonicSpring1dElementResponse, HarmonicSpring1dFrequencyResult, HarmonicSpring1dNodeResponse,
    SolveHarmonicSpring1dRequest, SolveHarmonicSpring1dResult, TransientSpring1dElementInput,
};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, Default)]
struct Complex {
    re: f64,
    im: f64,
}

pub fn solve_harmonic_spring_1d(
    request: &SolveHarmonicSpring1dRequest,
) -> Result<SolveHarmonicSpring1dResult, String> {
    validate_request(request)?;

    let count = request.nodes.len();
    let mass = request
        .nodes
        .iter()
        .map(|node| node.mass)
        .collect::<Vec<_>>();
    let stiffness = assemble_matrix(count, request, |element| element.stiffness);
    let damping = assemble_matrix(count, request, |element| element.damping);
    let force = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();
    let free = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (!node.fix_x).then_some(index))
        .collect::<Vec<_>>();

    let frequencies = request
        .frequencies_hz
        .iter()
        .map(|&frequency_hz| {
            solve_frequency(
                request,
                frequency_hz,
                &mass,
                &stiffness,
                &damping,
                &force,
                &free,
            )
        })
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
    mass: &[f64],
    stiffness: &[Vec<f64>],
    damping: &[Vec<f64>],
    force: &[f64],
    free: &[usize],
) -> Result<HarmonicSpring1dFrequencyResult, String> {
    let omega = 2.0 * PI * frequency_hz;
    let mut matrix = vec![vec![Complex::default(); free.len()]; free.len()];
    let mut rhs = vec![Complex::default(); free.len()];

    for (row_index, &row) in free.iter().enumerate() {
        rhs[row_index] = Complex::real(force[row]);
        for (column_index, &column) in free.iter().enumerate() {
            matrix[row_index][column_index] = Complex {
                re: stiffness[row][column]
                    - omega * omega * mass[row] * (row == column) as u8 as f64,
                im: omega * damping[row][column],
            };
        }
    }

    let solved = solve_complex_system(matrix, rhs)?;
    let mut displacement = vec![Complex::default(); request.nodes.len()];
    for (index, &dof) in free.iter().enumerate() {
        displacement[dof] = solved[index];
    }

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

fn assemble_matrix(
    count: usize,
    request: &SolveHarmonicSpring1dRequest,
    value: impl Fn(&TransientSpring1dElementInput) -> f64,
) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0; count]; count];
    for element in &request.elements {
        let value = value(element);
        matrix[element.node_i][element.node_i] += value;
        matrix[element.node_i][element.node_j] -= value;
        matrix[element.node_j][element.node_i] -= value;
        matrix[element.node_j][element.node_j] += value;
    }
    matrix
}

fn solve_complex_system(
    mut matrix: Vec<Vec<Complex>>,
    mut rhs: Vec<Complex>,
) -> Result<Vec<Complex>, String> {
    let size = rhs.len();
    for pivot in 0..size {
        let best = (pivot..size)
            .max_by(|&a, &b| {
                matrix[a][pivot]
                    .norm2()
                    .total_cmp(&matrix[b][pivot].norm2())
            })
            .expect("pivot range is non-empty");
        if matrix[best][pivot].norm2() <= 1.0e-24 {
            return Err("harmonic spring 1d dynamic stiffness is singular".to_string());
        }
        matrix.swap(pivot, best);
        rhs.swap(pivot, best);

        for row in (pivot + 1)..size {
            let factor = matrix[row][pivot] / matrix[pivot][pivot];
            matrix[row][pivot] = Complex::default();
            for column in (pivot + 1)..size {
                matrix[row][column] = matrix[row][column] - factor * matrix[pivot][column];
            }
            rhs[row] = rhs[row] - factor * rhs[pivot];
        }
    }

    let mut result = vec![Complex::default(); size];
    for row in (0..size).rev() {
        let mut sum = rhs[row];
        for (column, value) in result.iter().enumerate().skip(row + 1) {
            sum = sum - matrix[row][column] * *value;
        }
        result[row] = sum / matrix[row][row];
    }
    Ok(result)
}

fn validate_request(request: &SolveHarmonicSpring1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("harmonic spring 1d must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("harmonic spring 1d must define at least one element".to_string());
    }
    if request.frequencies_hz.is_empty() {
        return Err("harmonic spring 1d must include at least one frequency".to_string());
    }
    if request.nodes.iter().all(|node| node.fix_x) {
        return Err("harmonic spring 1d must leave at least one free node".to_string());
    }
    for node in &request.nodes {
        if !node.x.is_finite()
            || !node.load_x.is_finite()
            || !node.mass.is_finite()
            || node.mass <= 0.0
        {
            return Err(format!(
                "harmonic spring node {} must have finite coordinates, load, and positive mass",
                node.id
            ));
        }
    }
    for (index, frequency) in request.frequencies_hz.iter().enumerate() {
        if !frequency.is_finite() || *frequency < 0.0 {
            return Err(format!(
                "harmonic spring frequency {index} must be non-negative and finite"
            ));
        }
    }
    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveHarmonicSpring1dRequest,
    element: &TransientSpring1dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_i == element.node_j
        || !element.stiffness.is_finite()
        || element.stiffness <= 0.0
        || !element.damping.is_finite()
        || element.damping < 0.0
    {
        return Err(format!(
            "harmonic spring element {} must have valid connectivity, stiffness, and damping",
            element.id
        ));
    }
    Ok(())
}

impl Complex {
    fn real(value: f64) -> Self {
        Self { re: value, im: 0.0 }
    }

    fn amplitude(self) -> f64 {
        self.norm2().sqrt()
    }

    fn norm2(self) -> f64 {
        self.re * self.re + self.im * self.im
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
        let scale = rhs.norm2();
        Self {
            re: (self.re * rhs.re + self.im * rhs.im) / scale,
            im: (self.im * rhs.re - self.re * rhs.im) / scale,
        }
    }
}
