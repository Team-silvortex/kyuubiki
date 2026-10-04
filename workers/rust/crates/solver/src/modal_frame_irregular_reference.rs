use super::*;

#[path = "modal_frame_wide_spectrum_reference.rs"]
mod spectral;
use spectral::*;

#[path = "modal_frame_irregular_tests.rs"]
mod tests;

struct IrregularReference {
    segments: usize,
    coefficients: Vec<Vec<f64>>,
    mass: Vec<f64>,
    links: Vec<(usize, usize, f64)>,
}

impl IrregularReference {
    fn new(segments: usize, relative_spring: f64) -> Self {
        assert!((4..=16).contains(&segments));
        let width = 2 * segments;
        let size = 3 * width;
        let base = BendingReference::unit_inertia(segments)
            .first_mode()
            .0
            .rounded();
        let mut coefficients = vec![vec![0.0; size]; size];
        let mut mass = vec![0.0; size];
        for branch in 0..3 {
            for node in 1..=segments {
                let i = branch * width + 2 * (node - 1);
                let left = stiffness(branch, node - 1);
                let right = if node < segments {
                    stiffness(branch, node)
                } else {
                    0.0
                };
                coefficients[i][i] = 12.0 * left + 12.0 * right;
                coefficients[i + 1][i + 1] = 4.0 * left + 4.0 * right;
                coefficients[i][i + 1] = -6.0 * left + 6.0 * right;
                coefficients[i + 1][i] = coefficients[i][i + 1];
                if node < segments {
                    for (row, column, value) in [
                        (i, i + 2, -12.0),
                        (i, i + 3, 6.0),
                        (i + 1, i + 2, -6.0),
                        (i + 1, i + 3, 2.0),
                    ] {
                        coefficients[row][column] = value * right;
                        coefficients[column][row] = value * right;
                    }
                }
                let m =
                    4.0_f64.powi(branch as i32) * if (node + branch) % 3 == 0 { 4.0 } else { 1.0 };
                mass[i] = m;
                mass[i + 1] = m;
            }
        }
        let mut links = Vec::new();
        for (pair, (left, right)) in [(0, 1), (1, 2), (2, 0)].into_iter().enumerate() {
            for node in (2..=segments).step_by(2) {
                let target = (node + pair + 1) % segments + 1;
                for component in 0..2 {
                    let a = left * width + 2 * (node - 1) + component;
                    let b = right * width + 2 * (target - 1) + component;
                    let spring = base
                        * relative_spring
                        * (1 + node % 3) as f64
                        * if component == 0 { 1.0 } else { 0.25 };
                    links.push((a, b, spring));
                    coefficients[a][a] += spring;
                    coefficients[b][b] += spring;
                    coefficients[a][b] -= spring;
                    coefficients[b][a] -= spring;
                }
            }
        }
        Self {
            segments,
            coefficients,
            mass,
            links,
        }
    }

    fn system(&self, order: &[usize]) -> ReducedSparseModalSystem {
        let size = self.mass.len();
        assert_eq!(order.len(), size);
        let mut inverse = vec![usize::MAX; size];
        for (new, &old) in order.iter().enumerate() {
            assert!(old < size && inverse[old] == usize::MAX);
            inverse[old] = new;
        }
        // Independent element assembly, not a copy of the closed-entry oracle.
        let element = [
            [12.0, 6.0, -12.0, 6.0],
            [6.0, 4.0, -6.0, 2.0],
            [-12.0, -6.0, 12.0, -6.0],
            [6.0, 2.0, -6.0, 4.0],
        ];
        let width = 2 * self.segments;
        let mut matrix = SparseMatrix::new(size);
        for branch in 0..3 {
            for segment in 0..self.segments {
                for (row, entries) in element.iter().enumerate() {
                    for (column, &entry) in entries.iter().enumerate() {
                        let (row, column) = (2 * segment + row, 2 * segment + column);
                        if row >= 2 && column >= 2 {
                            add_at(
                                &mut matrix,
                                inverse[branch * width + row - 2],
                                inverse[branch * width + column - 2],
                                entry * stiffness(branch, segment),
                            );
                        }
                    }
                }
            }
        }
        for &(a, b, spring) in &self.links {
            let (a, b) = (inverse[a], inverse[b]);
            for (i, j, value) in [
                (a, a, spring),
                (b, b, spring),
                (a, b, -spring),
                (b, a, -spring),
            ] {
                add_at(&mut matrix, i, j, value);
            }
        }
        let mass: Vec<_> = order.iter().map(|&i| self.mass[i]).collect();
        reduce_sparse_modal_system(&matrix, &mass, &[]).unwrap()
    }

    fn normalized(&self) -> Vec<Vec<Wide>> {
        self.coefficients
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, &a)| {
                        Wide::from(a)
                            .mul(Wide::from(self.mass[i].sqrt().recip()))
                            .mul(Wide::from(self.mass[j].sqrt().recip()))
                    })
                    .collect()
            })
            .collect()
    }

    fn pairs(&self) -> Vec<(Wide, Vec<Wide>)> {
        spectrum(&self.normalized(), 40).unwrap()
    }

    fn residual(&self, value: f64, vector: &[f64], physical: bool) -> f64 {
        let mut square = Wide::default();
        let mut applied_square = Wide::default();
        let mut target_square = Wide::default();
        for (i, row) in self.coefficients.iter().enumerate() {
            let d = self.mass[i].sqrt().recip();
            let applied = row
                .iter()
                .zip(vector)
                .enumerate()
                .fold(Wide::default(), |sum, (j, (&a, &v))| {
                    let weight = if physical {
                        1.0
                    } else {
                        self.mass[j].sqrt().recip()
                    };
                    sum.add(Wide::from(a).mul(Wide::from(v)).mul(Wide::from(weight)))
                })
                .mul(Wide::from(d));
            let target = Wide::from(value)
                .mul(Wide::from(vector[i]))
                .mul(Wide::from(if physical { self.mass[i] * d } else { 1.0 }));
            let delta = applied.sub(target);
            square = square.add(delta.mul(delta));
            applied_square = applied_square.add(applied.mul(applied));
            target_square = target_square.add(target.mul(target));
        }
        square.rounded().sqrt() / applied_square.rounded().max(target_square.rounded()).sqrt()
    }
}

fn stiffness(branch: usize, segment: usize) -> f64 {
    4.0_f64.powi(branch as i32) * (1.0 + 0.125 * ((2 * segment + branch) % 5) as f64)
}
