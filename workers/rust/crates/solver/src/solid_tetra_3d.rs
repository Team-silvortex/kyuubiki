use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use crate::solid_tetra_3d_validation::validate_request;
use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dElementResult, SolidTetra3dNodeResult,
    SolveSolidTetra3dRequest, SolveSolidTetra3dResult,
};

pub fn solve_solid_tetra_3d(
    request: &SolveSolidTetra3dRequest,
) -> Result<SolveSolidTetra3dResult, String> {
    validate_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut stiffness = SparseMatrix::new(dof_count);
    let mut force = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force[index * 3] = node.load_x;
        force[index * 3 + 1] = node.load_y;
        force[index * 3 + 2] = node.load_z;
    }

    for element in &request.elements {
        let geometry = tetra_geometry(request, element)?;
        let d = elasticity_matrix(element.youngs_modulus, element.poisson_ratio);
        let db = multiply_6x6_6x12(&d, &geometry.b);
        let ke = multiply_12x6_6x12(&geometry.b, &db, geometry.volume);
        let map = element_dof_map(element);

        for row in 0..12 {
            for column in 0..12 {
                add_at(&mut stiffness, map[row], map[column], ke[row][column]);
            }
        }
    }

    let constrained = constrained_dofs(request);
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&stiffness, &force, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let ux = displacements[index * 3];
            let uy = displacements[index * 3 + 1];
            let uz = displacements[index * 3 + 2];
            SolidTetra3dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                ux,
                uy,
                uz,
                displacement_magnitude: (ux * ux + uy * uy + uz * uz).sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| element_result(index, request, element, &displacements))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SolveSolidTetra3dResult {
        input: request.clone(),
        total_volume: elements.iter().map(|element| element.volume).sum(),
        max_displacement: nodes
            .iter()
            .map(|node| node.displacement_magnitude)
            .fold(0.0_f64, f64::max),
        max_von_mises_stress: elements
            .iter()
            .map(|element| element.von_mises_stress)
            .fold(0.0_f64, f64::max),
        total_strain_energy: elements
            .iter()
            .map(|element| element.strain_energy_density * element.volume)
            .sum(),
        max_strain_energy_density: elements
            .iter()
            .map(|element| element.strain_energy_density.abs())
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
    })
}

struct TetraGeometry {
    volume: f64,
    b: [[f64; 12]; 6],
}

fn element_result(
    index: usize,
    request: &SolveSolidTetra3dRequest,
    element: &SolidTetra3dElementInput,
    displacements: &[f64],
) -> Result<SolidTetra3dElementResult, String> {
    let geometry = tetra_geometry(request, element)?;
    let d = elasticity_matrix(element.youngs_modulus, element.poisson_ratio);
    let map = element_dof_map(element);
    let ue = std::array::from_fn(|i| displacements[map[i]]);
    let strain = multiply_6x12_12(&geometry.b, &ue);
    let stress = multiply_6x6_6(&d, &strain);
    let von_mises = von_mises_stress(&stress);
    let strain_energy_density = strain_energy_density(&stress, &strain);

    Ok(SolidTetra3dElementResult {
        index,
        id: element.id.clone(),
        node_a: element.node_a,
        node_b: element.node_b,
        node_c: element.node_c,
        node_d: element.node_d,
        volume: geometry.volume,
        strain_x: strain[0],
        strain_y: strain[1],
        strain_z: strain[2],
        gamma_xy: strain[3],
        gamma_yz: strain[4],
        gamma_zx: strain[5],
        stress_x: stress[0],
        stress_y: stress[1],
        stress_z: stress[2],
        shear_xy: stress[3],
        shear_yz: stress[4],
        shear_zx: stress[5],
        von_mises_stress: von_mises,
        strain_energy_density,
    })
}

fn tetra_geometry(
    request: &SolveSolidTetra3dRequest,
    element: &SolidTetra3dElementInput,
) -> Result<TetraGeometry, String> {
    let points = element_nodes(request, element);
    let a = [
        [1.0, points[0].0, points[0].1, points[0].2],
        [1.0, points[1].0, points[1].1, points[1].2],
        [1.0, points[2].0, points[2].1, points[2].2],
        [1.0, points[3].0, points[3].1, points[3].2],
    ];
    let det = det4(&a);
    let volume = det.abs() / 6.0;
    if volume <= 1.0e-18 {
        return Err(format!(
            "solid tetra element {} has zero volume",
            element.id
        ));
    }
    let inverse = invert4(a)?;
    let mut b = [[0.0; 12]; 6];

    for node in 0..4 {
        let bx = inverse[1][node];
        let by = inverse[2][node];
        let bz = inverse[3][node];
        let offset = node * 3;
        b[0][offset] = bx;
        b[1][offset + 1] = by;
        b[2][offset + 2] = bz;
        b[3][offset] = by;
        b[3][offset + 1] = bx;
        b[4][offset + 1] = bz;
        b[4][offset + 2] = by;
        b[5][offset] = bz;
        b[5][offset + 2] = bx;
    }

    Ok(TetraGeometry { volume, b })
}

fn elasticity_matrix(youngs_modulus: f64, poisson_ratio: f64) -> [[f64; 6]; 6] {
    let factor = youngs_modulus / ((1.0 + poisson_ratio) * (1.0 - 2.0 * poisson_ratio));
    let normal = 1.0 - poisson_ratio;
    let shear = (1.0 - 2.0 * poisson_ratio) * 0.5;
    [
        [
            factor * normal,
            factor * poisson_ratio,
            factor * poisson_ratio,
            0.0,
            0.0,
            0.0,
        ],
        [
            factor * poisson_ratio,
            factor * normal,
            factor * poisson_ratio,
            0.0,
            0.0,
            0.0,
        ],
        [
            factor * poisson_ratio,
            factor * poisson_ratio,
            factor * normal,
            0.0,
            0.0,
            0.0,
        ],
        [0.0, 0.0, 0.0, factor * shear, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, factor * shear, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, factor * shear],
    ]
}

fn multiply_6x6_6x12(a: &[[f64; 6]; 6], b: &[[f64; 12]; 6]) -> [[f64; 12]; 6] {
    let mut result = [[0.0; 12]; 6];
    for row in 0..6 {
        for column in 0..12 {
            result[row][column] = (0..6).map(|k| a[row][k] * b[k][column]).sum();
        }
    }
    result
}

fn multiply_12x6_6x12(b: &[[f64; 12]; 6], db: &[[f64; 12]; 6], volume: f64) -> [[f64; 12]; 12] {
    let mut result = [[0.0; 12]; 12];
    for row in 0..12 {
        for column in 0..12 {
            result[row][column] = (0..6).map(|k| b[k][row] * db[k][column]).sum::<f64>() * volume;
        }
    }
    result
}

fn multiply_6x12_12(a: &[[f64; 12]; 6], vector: &[f64; 12]) -> [f64; 6] {
    std::array::from_fn(|row| (0..12).map(|column| a[row][column] * vector[column]).sum())
}

fn multiply_6x6_6(a: &[[f64; 6]; 6], vector: &[f64; 6]) -> [f64; 6] {
    std::array::from_fn(|row| (0..6).map(|column| a[row][column] * vector[column]).sum())
}

fn von_mises_stress(stress: &[f64; 6]) -> f64 {
    let sx = stress[0];
    let sy = stress[1];
    let sz = stress[2];
    let txy = stress[3];
    let tyz = stress[4];
    let tzx = stress[5];
    (0.5 * ((sx - sy).powi(2) + (sy - sz).powi(2) + (sz - sx).powi(2))
        + 3.0 * (txy * txy + tyz * tyz + tzx * tzx))
        .sqrt()
}

fn strain_energy_density(stress: &[f64; 6], strain: &[f64; 6]) -> f64 {
    0.5 * (0..6)
        .map(|index| stress[index] * strain[index])
        .sum::<f64>()
}

fn element_nodes(
    request: &SolveSolidTetra3dRequest,
    element: &SolidTetra3dElementInput,
) -> [(f64, f64, f64); 4] {
    [
        element.node_a,
        element.node_b,
        element.node_c,
        element.node_d,
    ]
    .map(|index| {
        let node = &request.nodes[index];
        (node.x, node.y, node.z)
    })
}

fn element_dof_map(element: &SolidTetra3dElementInput) -> [usize; 12] {
    [
        element.node_a * 3,
        element.node_a * 3 + 1,
        element.node_a * 3 + 2,
        element.node_b * 3,
        element.node_b * 3 + 1,
        element.node_b * 3 + 2,
        element.node_c * 3,
        element.node_c * 3 + 1,
        element.node_c * 3 + 2,
        element.node_d * 3,
        element.node_d * 3 + 1,
        element.node_d * 3 + 2,
    ]
}

fn constrained_dofs(request: &SolveSolidTetra3dRequest) -> Vec<usize> {
    request
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
        .collect()
}

fn det4(m: &[[f64; 4]; 4]) -> f64 {
    (0..4)
        .map(|column| {
            let sign = if column % 2 == 0 { 1.0 } else { -1.0 };
            sign * m[0][column] * det3(minor3(m, 0, column))
        })
        .sum()
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn minor3(m: &[[f64; 4]; 4], skip_row: usize, skip_column: usize) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    let mut row_out = 0;
    for (row, values) in m.iter().enumerate() {
        if row == skip_row {
            continue;
        }
        let mut column_out = 0;
        for (column, value) in values.iter().enumerate() {
            if column == skip_column {
                continue;
            }
            result[row_out][column_out] = *value;
            column_out += 1;
        }
        row_out += 1;
    }
    result
}

fn invert4(matrix: [[f64; 4]; 4]) -> Result<[[f64; 4]; 4], String> {
    let det = det4(&matrix);
    if det.abs() <= 1.0e-18 {
        return Err("solid tetra coordinate matrix is singular".to_string());
    }
    let mut inverse = [[0.0; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            let sign = if (row + column) % 2 == 0 { 1.0 } else { -1.0 };
            inverse[column][row] = sign * det3(minor3(&matrix, row, column)) / det;
        }
    }
    Ok(inverse)
}
