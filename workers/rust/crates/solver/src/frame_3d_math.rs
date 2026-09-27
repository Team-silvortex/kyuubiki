pub(super) fn normalized_direction(direction: [f64; 3]) -> Result<[f64; 3], String> {
    if direction.iter().any(|v| !v.is_finite()) {
        return Err("direction must be finite".into());
    }
    let scale = direction.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Err("direction must be non-zero".into());
    }
    let scaled = direction.map(|v| v / scale);
    let norm = norm3(scaled);
    if scale <= 1e-12 && scale * norm <= 1e-12 {
        return Err("direction must be non-zero".into());
    }
    Ok(scaled.map(|v| v / norm))
}

pub(super) fn frame3d_rotation(
    dx: f64,
    dy: f64,
    dz: f64,
    length: f64,
) -> Result<[[f64; 3]; 3], String> {
    frame3d_rotation_with_local_y(dx, dy, dz, length, None)
}

pub(super) fn frame3d_rotation_with_local_y(
    dx: f64,
    dy: f64,
    dz: f64,
    length: f64,
    local_y_axis: Option<[f64; 3]>,
) -> Result<[[f64; 3]; 3], String> {
    if !length.is_finite() || length <= 1.0e-12 {
        return Err("3d frame element length must be positive".to_string());
    }

    let local_x = normalized_direction([dx, dy, dz])
        .map_err(|_| "3d frame element length must be positive".to_string())?;
    let mut local_y = if let Some(axis) = local_y_axis {
        if axis.iter().any(|component| !component.is_finite()) {
            return Err("3d frame local_y_axis must be finite".to_string());
        }
        let mut axis = normalized_direction(axis).map_err(|_| {
            "3d frame local_y_axis must not be parallel to the element axis".to_string()
        })?;
        // A single projection can amplify axial roundoff when the hint is
        // nearly parallel. Remove that component again before normalization.
        for _ in 0..2 {
            let projection = dot3(axis, local_x);
            for component in 0..3 {
                axis[component] = (-projection).mul_add(local_x[component], axis[component]);
            }
        }
        axis
    } else {
        // Preserve the existing implicit roll at floating-point branch ties.
        let reference = if (dz / length).abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        cross3(reference, local_x)
    };

    let local_y_norm = norm3(local_y);
    if local_y_norm <= 1.0e-12 {
        return Err("3d frame local_y_axis must not be parallel to the element axis".to_string());
    }
    local_y = scale3(local_y, 1.0 / local_y_norm);
    let local_z = normalized_direction(cross3(local_x, local_y))?;
    // Close the right-handed triad rather than retaining normalization drift.
    let local_y = cross3(local_z, local_x);

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

    let by = youngs_modulus * moment_of_inertia_y / length;
    let by_coupling = by / length;
    let by1 = 12.0 * (by_coupling / length);
    let by2 = 6.0 * by_coupling;
    let by3 = 4.0 * by;
    let by4 = 2.0 * by;

    let bz = youngs_modulus * moment_of_inertia_z / length;
    let bz_coupling = bz / length;
    let bz1 = 12.0 * (bz_coupling / length);
    let bz2 = 6.0 * bz_coupling;
    let bz3 = 4.0 * bz;
    let bz4 = 2.0 * bz;

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
    for (row, input_row) in input.iter().enumerate() {
        for (column, value) in input_row.iter().enumerate() {
            output[column][row] = *value;
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

fn cross3(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [
        lhs[1] * rhs[2] - lhs[2] * rhs[1],
        lhs[2] * rhs[0] - lhs[0] * rhs[2],
        lhs[0] * rhs[1] - lhs[1] * rhs[0],
    ]
}

fn norm3(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

fn dot3(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    lhs[0] * rhs[0] + lhs[1] * rhs[1] + lhs[2] * rhs[2]
}

fn scale3(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

#[cfg(test)]
#[path = "frame_3d_orientation_tests.rs"]
mod orientation_tests;
