pub(crate) fn rigid_body_restraint_rank<P, F>(node_indices: &[usize], point: P, fixed: F) -> usize
where
    P: Fn(usize) -> [f64; 3],
    F: Fn(usize) -> [bool; 3],
{
    let mut centroid = [0.0; 3];
    for &index in node_indices {
        let position = point(index);
        for axis in 0..3 {
            centroid[axis] += position[axis];
        }
    }
    centroid = centroid.map(|value| value / node_indices.len() as f64);
    let scale = node_indices
        .iter()
        .map(|&index| {
            let position = point(index);
            ((position[0] - centroid[0]).powi(2)
                + (position[1] - centroid[1]).powi(2)
                + (position[2] - centroid[2]).powi(2))
            .sqrt()
        })
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);
    let mut rows = Vec::<[f64; 6]>::new();
    for &index in node_indices {
        let position = point(index);
        let x = (position[0] - centroid[0]) / scale;
        let y = (position[1] - centroid[1]) / scale;
        let z = (position[2] - centroid[2]) / scale;
        let restraints = fixed(index);
        if restraints[0] {
            rows.push([1.0, 0.0, 0.0, 0.0, z, -y]);
        }
        if restraints[1] {
            rows.push([0.0, 1.0, 0.0, -z, 0.0, x]);
        }
        if restraints[2] {
            rows.push([0.0, 0.0, 1.0, y, -x, 0.0]);
        }
    }
    matrix_rank(&mut rows)
}

fn matrix_rank(rows: &mut [[f64; 6]]) -> usize {
    let mut rank = 0;
    for column in 0..6 {
        let Some(pivot) = (rank..rows.len()).max_by(|&left, &right| {
            rows[left][column]
                .abs()
                .total_cmp(&rows[right][column].abs())
        }) else {
            break;
        };
        if rows[pivot][column].abs() <= 1.0e-10 {
            continue;
        }
        rows.swap(rank, pivot);
        let divisor = rows[rank][column];
        for value in &mut rows[rank][column..] {
            *value /= divisor;
        }
        for row in 0..rows.len() {
            if row == rank {
                continue;
            }
            let factor = rows[row][column];
            for entry in column..6 {
                rows[row][entry] -= factor * rows[rank][entry];
            }
        }
        rank += 1;
    }
    rank
}
