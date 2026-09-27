pub(crate) fn frame_local_stiffness(
    area: f64,
    youngs_modulus: f64,
    moment_of_inertia: f64,
    length: f64,
) -> [[f64; 6]; 6] {
    let axial = youngs_modulus * area / length;
    let flexural = youngs_modulus * moment_of_inertia;
    let bending = flexural / length;
    let coupling = bending / length;
    let transverse = coupling / length;

    [
        [axial, 0.0, 0.0, -axial, 0.0, 0.0],
        [
            0.0,
            12.0 * transverse,
            6.0 * coupling,
            0.0,
            -12.0 * transverse,
            6.0 * coupling,
        ],
        [
            0.0,
            6.0 * coupling,
            4.0 * bending,
            0.0,
            -6.0 * coupling,
            2.0 * bending,
        ],
        [-axial, 0.0, 0.0, axial, 0.0, 0.0],
        [
            0.0,
            -12.0 * transverse,
            -6.0 * coupling,
            0.0,
            12.0 * transverse,
            -6.0 * coupling,
        ],
        [
            0.0,
            6.0 * coupling,
            2.0 * bending,
            0.0,
            -6.0 * coupling,
            4.0 * bending,
        ],
    ]
}

pub(crate) fn frame_local_geometric_stiffness(
    compressive_force: f64,
    length: f64,
) -> [[f64; 6]; 6] {
    let l2 = length * length;
    let factor = compressive_force / (30.0 * length);
    let mut stiffness = [[0.0; 6]; 6];
    let bending = [
        [36.0, 3.0 * length, -36.0, 3.0 * length],
        [3.0 * length, 4.0 * l2, -3.0 * length, -l2],
        [-36.0, -3.0 * length, 36.0, -3.0 * length],
        [3.0 * length, -l2, -3.0 * length, 4.0 * l2],
    ];
    let bending_dofs = [1, 2, 4, 5];
    for row in 0..4 {
        for column in 0..4 {
            stiffness[bending_dofs[row]][bending_dofs[column]] = bending[row][column] * factor;
        }
    }
    stiffness
}

pub(crate) fn frame_dof_map(node_i: usize, node_j: usize) -> [usize; 6] {
    [
        node_i * 3,
        node_i * 3 + 1,
        node_i * 3 + 2,
        node_j * 3,
        node_j * 3 + 1,
        node_j * 3 + 2,
    ]
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
    for (row, input_row) in input.iter().enumerate() {
        for (column, value) in input_row.iter().enumerate() {
            output[column][row] = *value;
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
