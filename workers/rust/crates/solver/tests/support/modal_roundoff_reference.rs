use kyuubiki_protocol::SolveModalFrame2dResult;

// Reassemble the actual rounded geometry, not an ideal unit beam after a
// decimal coordinate conversion. No production assembly/product helper is used.
pub fn reassembled_residual(result: &SolveModalFrame2dResult) -> f64 {
    let request = &result.input;
    let size = request.nodes.len() * 2;
    let mut rows = vec![std::collections::BTreeMap::<usize, f64>::new(); size];
    let mut mass = vec![0.0; size];
    for element in &request.elements {
        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        let rotation = element.youngs_modulus * element.moment_of_inertia / length;
        let cross = rotation / length;
        let translation = cross / length;
        let local = [
            [
                12.0 * translation,
                6.0 * cross,
                -12.0 * translation,
                6.0 * cross,
            ],
            [6.0 * cross, 4.0 * rotation, -6.0 * cross, 2.0 * rotation],
            [
                -12.0 * translation,
                -6.0 * cross,
                12.0 * translation,
                -6.0 * cross,
            ],
            [6.0 * cross, 2.0 * rotation, -6.0 * cross, 4.0 * rotation],
        ];
        let map = [
            2 * element.node_i,
            2 * element.node_i + 1,
            2 * element.node_j,
            2 * element.node_j + 1,
        ];
        for (i, row) in local.iter().enumerate() {
            for (j, &a) in row.iter().enumerate() {
                *rows[map[i]].entry(map[j]).or_default() += a;
            }
        }
        let total = element.density * element.area * length;
        for node in [element.node_i, element.node_j] {
            mass[2 * node] += total / 2.0;
            mass[2 * node + 1] += total * (length * length / 24.0);
        }
    }
    let mode = &result.modes[0];
    let vector: Vec<_> = mode
        .shape
        .chunks_exact(3)
        .flat_map(|v| [v[1], v[2]])
        .collect();
    let (mut residual, mut force_norm, mut target_norm) = (0.0_f64, 0.0_f64, 0.0_f64);
    for dof in &result.free_dofs {
        let i = 2 * (dof / 3) + dof % 3 - 1;
        let (mut sum, mut tail) = (0.0_f64, 0.0_f64);
        for (&j, &a) in &rows[i] {
            let product = a * vector[j];
            let next = sum + product;
            tail += if sum.abs() >= product.abs() {
                (sum - next) + product
            } else {
                (product - next) + sum
            };
            tail += a.mul_add(vector[j], -product);
            sum = next;
        }
        let force = (sum + tail) / mass[i].sqrt();
        let target = mode.eigenvalue_rad_s_squared * mass[i].sqrt() * vector[i];
        residual = residual.hypot(force - target);
        force_norm = force_norm.hypot(force);
        target_norm = target_norm.hypot(target);
    }
    residual / force_norm.max(target_norm)
}
