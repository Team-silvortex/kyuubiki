use std::collections::BTreeMap;

use crate::solver_control::{SolverStage, checkpoint_chunk};

pub(crate) fn validate_restraints(
    node_count: usize,
    edges: impl Iterator<Item = (usize, usize)>,
    point: impl Fn(usize) -> [f64; 3],
    fixed: impl Fn(usize) -> [bool; 6],
    space: bool,
) -> Result<(), String> {
    let mut parent: Vec<_> = (0..node_count).collect();
    let mut used = vec![false; node_count];
    for (index, (i, j)) in edges.enumerate() {
        checkpoint_chunk(SolverStage::ElementPrecompute, index, node_count)?;
        used[i] = true;
        used[j] = true;
        let left = root(&mut parent, i);
        let right = root(&mut parent, j);
        parent[left] = right;
    }
    let mut components = BTreeMap::<usize, Vec<usize>>::new();
    for (index, used) in used.into_iter().enumerate() {
        checkpoint_chunk(SolverStage::ElementPrecompute, index, node_count)?;
        if !used {
            return Err(format!(
                "modal frame node {index} is an orphan without element mass"
            ));
        }
        components
            .entry(root(&mut parent, index))
            .or_default()
            .push(index);
    }
    let dimension = if space { 6 } else { 3 };
    for nodes in components.values() {
        let origin = point(nodes[0]);
        let mut scale = 0.0_f64;
        for (index, &node) in nodes.iter().enumerate() {
            checkpoint_chunk(SolverStage::ElementPrecompute, index, nodes.len())?;
            for (coordinate, origin) in point(node).into_iter().zip(origin) {
                let relative = coordinate - origin;
                if !relative.is_finite() {
                    return Err("modal frame component coordinate span is not representable".into());
                }
                scale = scale.max(relative.abs());
            }
        }
        if scale <= 0.0 {
            return Err("modal frame component has zero coordinate span".into());
        }
        let mut basis = [[0.0; 6]; 6];
        let mut rank = 0;
        for &node in nodes {
            let [x, y, z] = std::array::from_fn(|axis| (point(node)[axis] - origin[axis]) / scale);
            let rows = if space {
                [
                    [1.0, 0.0, 0.0, 0.0, z, -y],
                    [0.0, 1.0, 0.0, -z, 0.0, x],
                    [0.0, 0.0, 1.0, y, -x, 0.0],
                    [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                ]
            } else {
                [
                    [1.0, 0.0, -y, 0.0, 0.0, 0.0],
                    [0.0, 1.0, x, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                    [0.0; 6],
                    [0.0; 6],
                    [0.0; 6],
                ]
            };
            for (row, fixed) in rows.into_iter().zip(fixed(node)) {
                if fixed {
                    rank += add_row(&mut basis, row, dimension);
                }
            }
            if rank == dimension {
                break;
            }
        }
        if rank != dimension {
            return Err(format!(
                "modal frame component containing node {} has unrestrained rigid-body motion (restraint rank {rank}/{dimension})",
                nodes[0]
            ));
        }
    }
    Ok(())
}

fn root(parent: &mut [usize], mut index: usize) -> usize {
    while parent[index] != index {
        parent[index] = parent[parent[index]];
        index = parent[index];
    }
    index
}

fn add_row(basis: &mut [[f64; 6]; 6], mut row: [f64; 6], dimension: usize) -> usize {
    for pivot in 0..dimension {
        if row[pivot].abs() <= 1e-10 {
            continue;
        }
        if basis[pivot][pivot] == 0.0 {
            let divisor = row[pivot];
            for value in &mut row[pivot..dimension] {
                *value /= divisor;
            }
            basis[pivot] = row;
            return 1;
        }
        let factor = row[pivot];
        for (value, basis_value) in row[pivot..dimension]
            .iter_mut()
            .zip(&basis[pivot][pivot..dimension])
        {
            *value -= factor * basis_value;
        }
    }
    0
}
