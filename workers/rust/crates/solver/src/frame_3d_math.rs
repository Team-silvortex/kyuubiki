pub(super) fn frame3d_thermal_uniform_vector(
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

pub(super) fn frame3d_thermal_gradient_vector(
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

pub(super) fn frame3d_rotation(
    dx: f64,
    dy: f64,
    dz: f64,
    length: f64,
) -> Result<[[f64; 3]; 3], String> {
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

pub(super) fn frame3d_local_stiffness(
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

pub(super) fn frame3d_transform(rotation: &[[f64; 3]; 3]) -> [[f64; 12]; 12] {
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

pub(super) fn transform_frame3d_stiffness(
    local_stiffness: &[[f64; 12]; 12],
    transform: &[[f64; 12]; 12],
) -> [[f64; 12]; 12] {
    let transform_t = transpose_12x12(transform);
    let left = multiply_matrix_12x12_12x12(&transform_t, local_stiffness);
    multiply_matrix_12x12_12x12(&left, transform)
}

pub(super) fn frame3d_dof_map(node_i: usize, node_j: usize) -> [usize; 12] {
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

pub(super) fn transpose_12x12(input: &[[f64; 12]; 12]) -> [[f64; 12]; 12] {
    let mut output = [[0.0; 12]; 12];
    for row in 0..12 {
        for column in 0..12 {
            output[column][row] = input[row][column];
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

pub(super) fn multiply_matrix_vector_12x12(
    matrix: &[[f64; 12]; 12],
    vector: &[f64; 12],
) -> [f64; 12] {
    let mut output = [0.0; 12];
    for row in 0..12 {
        output[row] = (0..12)
            .map(|index| matrix[row][index] * vector[index])
            .sum();
    }
    output
}

pub(super) fn subtract_vector_12(lhs: &[f64; 12], rhs: &[f64; 12]) -> [f64; 12] {
    let mut output = [0.0; 12];
    for index in 0..12 {
        output[index] = lhs[index] - rhs[index];
    }
    output
}

pub(super) fn add_vector_12(lhs: &[f64; 12], rhs: &[f64; 12]) -> [f64; 12] {
    let mut output = [0.0; 12];
    for index in 0..12 {
        output[index] = lhs[index] + rhs[index];
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
