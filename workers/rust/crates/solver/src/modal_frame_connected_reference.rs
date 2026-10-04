use super::tests::{physical_mass_overlap, subspace_distance, wide_overlap};
use super::*;

#[path = "modal_frame_connected_tests.rs"]
mod tests;

#[path = "modal_frame_irregular_reference.rs"]
mod irregular;

fn connected_system(
    segments: usize,
    stiffness: f64,
    spring: f64,
    order: &[usize],
) -> ReducedSparseModalSystem {
    let width = 2 * segments;
    assert_eq!(order.len(), 2 * width);
    assert!(stiffness > 0.0 && spring > 0.0);
    let mut inverse = vec![usize::MAX; order.len()];
    for (new, &old) in order.iter().enumerate() {
        assert!(old < order.len() && inverse[old] == usize::MAX);
        inverse[old] = new;
    }
    let mut matrix = SparseMatrix::new(order.len());
    let element = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for (block, scale) in [1.0, stiffness].into_iter().enumerate() {
        for segment in 0..segments {
            for (row, entries) in element.iter().enumerate() {
                for (column, &entry) in entries.iter().enumerate() {
                    let (row, column) = (2 * segment + row, 2 * segment + column);
                    if row >= 2 && column >= 2 {
                        add_at(
                            &mut matrix,
                            inverse[block * width + row - 2],
                            inverse[block * width + column - 2],
                            scale * entry,
                        );
                    }
                }
            }
        }
    }
    for i in 0..width {
        let (left, right) = (inverse[i], inverse[width + i]);
        add_at(&mut matrix, left, left, spring);
        add_at(&mut matrix, right, right, spring);
        add_at(&mut matrix, left, right, -spring);
        add_at(&mut matrix, right, left, -spring);
        assert_eq!(
            matrix
                .row_entries(left)
                .iter()
                .find(|(j, _)| *j == right)
                .unwrap()
                .1,
            -spring
        );
    }
    let mass: Vec<_> = order
        .iter()
        .map(|i| if *i < width { 1.0 } else { 4.0 })
        .collect();
    reduce_sparse_modal_system(&matrix, &mass, &[]).unwrap()
}

// Separate closed-entry assembly, never a dump of the production sparse matrix.
// Stored f64 coefficients and ideal component arithmetic are distinct references.
struct ConnectedReference {
    coefficients: Vec<Vec<f64>>,
    width: usize,
    stiffness: f64,
    spring: f64,
    root: Wide,
    direction: Vec<Wide>,
}

impl ConnectedReference {
    fn new(segments: usize, stiffness: f64, spring: f64) -> Self {
        let width = 2 * segments;
        let (root, direction) = BendingReference::unit_inertia(segments).first_mode();
        let mut coefficients = vec![vec![0.0; 2 * width]; 2 * width];
        for (block, scale) in [1.0, stiffness].into_iter().enumerate() {
            let offset = block * width;
            for node in 0..segments {
                let i = offset + 2 * node;
                let tip = node + 1 == segments;
                coefficients[i][i] = scale * if tip { 12.0 } else { 24.0 } + spring;
                coefficients[i + 1][i + 1] = scale * if tip { 4.0 } else { 8.0 } + spring;
                if tip {
                    coefficients[i][i + 1] = -6.0 * scale;
                    coefficients[i + 1][i] = -6.0 * scale;
                } else {
                    for (row, column, value) in [
                        (i, i + 2, -12.0),
                        (i, i + 3, 6.0),
                        (i + 1, i + 2, -6.0),
                        (i + 1, i + 3, 2.0),
                    ] {
                        coefficients[row][column] = value * scale;
                        coefficients[column][row] = value * scale;
                    }
                }
            }
        }
        for i in 0..width {
            coefficients[i][width + i] = -spring;
            coefficients[width + i][i] = -spring;
        }
        Self {
            coefficients,
            width,
            stiffness,
            spring,
            root,
            direction,
        }
    }

    fn pairs(&self) -> [(f64, Vec<f64>); 2] {
        let a = self.root.add(Wide::from(self.spring));
        let b = self
            .root
            .mul(Wide::from(self.stiffness / 4.0))
            .add(Wide::from(self.spring / 4.0));
        let off = Wide::from(-self.spring / 2.0);
        let delta = a.sub(b);
        let gap = wide_sqrt(delta.mul(delta).add(off.mul(off).mul(Wide::from(4.0))));
        let high = a.add(b).add(gap).mul(Wide::from(0.5));
        let low = a.mul(b).sub(off.mul(off)).div(high);
        let weights = [off, low.sub(a)];
        let norm = wide_sqrt(weights[0].mul(weights[0]).add(weights[1].mul(weights[1])));
        let weights = weights.map(|w| w.div(norm));
        let vector = |weights: [Wide; 2]| {
            weights
                .into_iter()
                .flat_map(|w| self.direction.iter().map(move |&v| v.mul(w).rounded()))
                .collect()
        };
        [
            (low.rounded(), vector(weights)),
            (
                high.rounded(),
                vector([Wide::default().sub(weights[1]), weights[0]]),
            ),
        ]
    }

    fn apply(&self, vector: &[f64], physical: bool) -> Vec<Wide> {
        assert_eq!(vector.len(), 2 * self.width);
        let d = |i| if i < self.width { 1.0 } else { 0.5 };
        self.coefficients
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .zip(vector)
                    .enumerate()
                    .fold(Wide::default(), |sum, (j, (&a, &v))| {
                        sum.add(
                            Wide::from(a)
                                .mul(Wide::from(v))
                                .mul(Wide::from(if physical { 1.0 } else { d(j) })),
                        )
                    })
                    .mul(Wide::from(d(i)))
            })
            .collect()
    }

    fn residual(&self, value: f64, vector: &[f64], physical: bool) -> f64 {
        let applied = self.apply(vector, physical);
        let target: Vec<_> = vector
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                Wide::from(value).mul(Wide::from(v)).mul(Wide::from(
                    if physical && i >= self.width {
                        2.0
                    } else {
                        1.0
                    },
                ))
            })
            .collect();
        wide_norm(applied.iter().zip(&target).map(|(&a, &b)| a.sub(b)))
            / wide_norm(applied.iter().copied()).max(wide_norm(target.iter().copied()))
    }
}

fn wide_sqrt(value: Wide) -> Wide {
    assert!(value.rounded() > 0.0);
    let mut root = Wide::from(value.rounded().sqrt());
    for _ in 0..3 {
        root = root.add(value.div(root)).mul(Wide::from(0.5));
    }
    root
}

fn wide_norm(values: impl Iterator<Item = Wide>) -> f64 {
    values
        .fold(Wide::default(), |sum, v| sum.add(v.mul(v)))
        .rounded()
        .sqrt()
}

// One generic residual-inverse proposal, with no beam labels or extra retry loop.
// A lower residual is only a proposal: final joint admission remains mandatory.
fn inverse_proposal(
    system: &ReducedSparseModalSystem,
    inverse: &crate::linear_algebra::PreparedSpdSolver,
    guard: &BasisGuard,
    value: f64,
    vector: &[f64],
) -> Result<Vec<f64>, String> {
    let private = guard.project_seed(vector)?;
    let (before, residual) = normalized_check(system, value, &private)?;
    let correction = inverse.solve(&residual)?;
    let mut candidate = Vec::with_capacity(private.len());
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    for (index, (&v, &c)) in private.iter().zip(&correction).enumerate() {
        candidate.push(v - c);
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, private.len())?;
    }
    // Inverse correction can reintroduce an accepted-mode component even below
    // the overlap gate. Deflate it before measuring the corrected subspace.
    orthogonalize(&mut candidate, &guard.previous)?;
    let (after, _) = normalized_check(system, value, &candidate)?;
    if after >= before {
        return Err("test-only inverse proposal did not lower the true residual".into());
    }
    guard.check(&candidate)?;
    checkpoint(SolverStage::ModalValidation, guard.previous.len() + 1)?;
    Ok(candidate)
}
