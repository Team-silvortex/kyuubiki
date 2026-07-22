pub(crate) fn frame_local_stiffness(
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

pub(super) fn frame_thermal_uniform_vector(
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    average_temperature_delta: f64,
) -> [f64; 6] {
    let thermal_force = youngs_modulus * area * thermal_expansion * average_temperature_delta;
    [-thermal_force, 0.0, 0.0, thermal_force, 0.0, 0.0]
}

pub(super) fn frame_thermal_gradient_vector(
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

pub(crate) fn frame_transform(c: f64, s: f64) -> [[f64; 6]; 6] {
    [
        [c, s, 0.0, 0.0, 0.0, 0.0],
        [-s, c, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, c, s, 0.0],
        [0.0, 0.0, 0.0, -s, c, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    ]
}

pub(crate) fn transform_frame_stiffness(
    local_stiffness: &[[f64; 6]; 6],
    transform: &[[f64; 6]; 6],
) -> [[f64; 6]; 6] {
    let transform_t = transpose_6x6(transform);
    let left = multiply_matrix_6x6_6x6(&transform_t, local_stiffness);
    multiply_matrix_6x6_6x6(&left, transform)
}

pub(super) fn transpose_6x6(input: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut output = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
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

pub(super) fn multiply_matrix_vector_6x6(matrix: &[[f64; 6]; 6], vector: &[f64; 6]) -> [f64; 6] {
    let mut output = [0.0; 6];
    for row in 0..6 {
        output[row] = (0..6).map(|index| matrix[row][index] * vector[index]).sum();
    }
    output
}

pub(super) fn subtract_vector_6(lhs: &[f64; 6], rhs: &[f64; 6]) -> [f64; 6] {
    let mut output = [0.0; 6];
    for index in 0..6 {
        output[index] = lhs[index] - rhs[index];
    }
    output
}

pub(super) fn add_vector_6(lhs: &[f64; 6], rhs: &[f64; 6]) -> [f64; 6] {
    let mut output = [0.0; 6];
    for index in 0..6 {
        output[index] = lhs[index] + rhs[index];
    }
    output
}
