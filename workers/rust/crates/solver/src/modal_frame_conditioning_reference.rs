// Test-only oracle: independent assembly/factors, not interval certification.
pub(super) use crate::modal_test_wide::Wide;

pub(super) struct BendingReference {
    stiffness: Vec<Vec<Wide>>,
    inverse_mass: Vec<f64>,
}

impl BendingReference {
    pub(super) fn unit_inertia(segments: usize) -> Self {
        let mut reference = Self::new(segments);
        reference.inverse_mass.fill(1.0);
        reference
    }

    pub(super) fn new(segments: usize) -> Self {
        assert!((1..=128).contains(&segments));
        let size = 2 * segments;
        let mut stiffness = vec![vec![Wide::default(); size]; size];
        // Independently assemble K from integer beam entries, not a production dump.
        let element = [
            [12.0, 6.0, -12.0, 6.0],
            [6.0, 4.0, -6.0, 2.0],
            [-12.0, -6.0, 12.0, -6.0],
            [6.0, 2.0, -6.0, 4.0],
        ];
        for segment in 0..segments {
            for (row, entries) in element.iter().enumerate() {
                for (column, &entry) in entries.iter().enumerate() {
                    let (row, column) = (2 * segment + row, 2 * segment + column);
                    if row >= 2 && column >= 2 {
                        stiffness[row - 2][column - 2] =
                            stiffness[row - 2][column - 2].add(Wide::from(entry));
                    }
                }
            }
        }
        let inverse_mass = (0..size)
            .map(|i| {
                let mass: f64 = (if i % 2 == 0 { 1.0 } else { 1.0 / 12.0 })
                    * if i >= size - 2 { 0.5 } else { 1.0 };
                mass.sqrt().recip()
            })
            .collect();
        Self {
            stiffness,
            inverse_mass,
        }
    }

    pub(super) fn apply(&self, vector: &[Wide]) -> Vec<Wide> {
        assert_eq!(vector.len(), self.inverse_mass.len());
        let scaled: Vec<_> = vector
            .iter()
            .zip(&self.inverse_mass)
            .map(|(&v, &d)| v.mul(Wide::from(d)))
            .collect();
        self.physical_apply(&scaled)
            .into_iter()
            .zip(&self.inverse_mass)
            .map(|(v, &d)| v.mul(Wide::from(d)))
            .collect()
    }

    fn physical_apply(&self, vector: &[Wide]) -> Vec<Wide> {
        (0..vector.len())
            .map(|row| {
                (row.saturating_sub(3)..(row + 4).min(vector.len()))
                    .fold(Wide::default(), |sum, column| {
                        sum.add(self.stiffness[row][column].mul(vector[column]))
                    })
            })
            .collect()
    }

    pub(super) fn residual(&self, value: Wide, vector: &[Wide]) -> f64 {
        let applied = self.apply(vector);
        let target: Vec<_> = vector.iter().map(|&v| value.mul(v)).collect();
        let residual = norm(applied.iter().zip(&target).map(|(&a, &b)| a.sub(b)));
        residual / norm(applied.iter().copied()).max(norm(target.iter().copied()))
    }

    pub(super) fn physical_residual(&self, value: Wide, shape: &[Wide], mass: &[f64]) -> f64 {
        assert_eq!(shape.len(), self.inverse_mass.len());
        assert_eq!(mass.len(), shape.len());
        let applied: Vec<_> = self
            .physical_apply(shape)
            .into_iter()
            .zip(&self.inverse_mass)
            .map(|(v, &d)| v.mul(Wide::from(d)))
            .collect();
        let target: Vec<_> = shape
            .iter()
            .zip(mass)
            .zip(&self.inverse_mass)
            .map(|((&v, &m), &d)| value.mul(v).mul(Wide::from(m)).mul(Wide::from(d)))
            .collect();
        norm(applied.iter().zip(&target).map(|(&a, &b)| a.sub(b)))
            / norm(applied.iter().copied()).max(norm(target.iter().copied()))
    }

    pub(super) fn direction_error(&self, vector: &[Wide], reference: &[Wide]) -> f64 {
        assert_eq!(vector.len(), self.inverse_mass.len());
        let scale = dot(vector, reference).div(dot(reference, reference));
        norm(
            vector
                .iter()
                .zip(reference)
                .map(|(&a, &b)| a.sub(b.mul(scale))),
        ) / norm(vector.iter().copied())
    }

    pub(super) fn rounded_matrix_residual(&self, value: Wide, vector: &[Wide]) -> f64 {
        let applied: Vec<_> = (0..vector.len())
            .map(|row| {
                (row.saturating_sub(3)..(row + 4).min(vector.len())).fold(
                    Wide::default(),
                    |sum, column| {
                        let entry = self.stiffness[row][column]
                            .mul(Wide::from(self.inverse_mass[row]))
                            .mul(Wide::from(self.inverse_mass[column]));
                        sum.add(Wide::from(entry.rounded()).mul(vector[column]))
                    },
                )
            })
            .collect();
        let target: Vec<_> = vector.iter().map(|&v| value.mul(v)).collect();
        norm(applied.iter().zip(&target).map(|(&a, &b)| a.sub(b)))
            / norm(applied.iter().copied()).max(norm(target.iter().copied()))
    }

    pub(super) fn first_mode(&self) -> (Wide, Vec<Wide>) {
        let size = self.inverse_mass.len();
        let (lower, diagonal) = factor(&self.stiffness);
        // Match the exact operator defined by the stored f64 inverse-mass factors.
        // This separates product evaluation error from matrix-representation error.
        let mass: Vec<_> = self
            .inverse_mass
            .iter()
            .map(|&d| Wide::from(1.0).div(Wide::from(d).mul(Wide::from(d))))
            .collect();
        let mut physical = vec![Wide::from(1.0); size];
        for _ in 0..32 {
            let rhs: Vec<_> = physical
                .iter()
                .zip(&mass)
                .map(|(&v, &m)| v.mul(m))
                .collect();
            physical = solve(&lower, &diagonal, &rhs);
            let maximum = physical
                .iter()
                .map(|v| v.rounded().abs())
                .fold(0.0_f64, f64::max);
            let power = Wide::from(2.0_f64.powi(-maximum.log2().floor() as i32));
            for value in &mut physical {
                *value = value.mul(power);
            }
        }
        let applied = self.physical_apply(&physical);
        let numerator = dot(&physical, &applied);
        let denominator = physical
            .iter()
            .zip(&mass)
            .fold(Wide::default(), |sum, (&v, &m)| sum.add(v.mul(v).mul(m)));
        let value = numerator.div(denominator);
        let vector = physical
            .into_iter()
            .zip(&self.inverse_mass)
            .map(|(v, &d)| v.div(Wide::from(d)))
            .collect();
        (value, vector)
    }
}

fn factor(matrix: &[Vec<Wide>]) -> (Vec<Vec<Wide>>, Vec<Wide>) {
    let size = matrix.len();
    let mut lower = vec![vec![Wide::default(); size]; size];
    let mut diagonal = vec![Wide::default(); size];
    for row in 0..size {
        lower[row][row] = Wide::from(1.0);
        for column in row.saturating_sub(3)..row {
            let mut entry = matrix[row][column];
            for (k, &pivot) in diagonal
                .iter()
                .enumerate()
                .take(column)
                .skip(row.saturating_sub(3))
            {
                entry = entry.sub(lower[row][k].mul(lower[column][k]).mul(pivot));
            }
            lower[row][column] = entry.div(diagonal[column]);
        }
        let mut entry = matrix[row][row];
        for (k, &pivot) in diagonal
            .iter()
            .enumerate()
            .take(row)
            .skip(row.saturating_sub(3))
        {
            entry = entry.sub(lower[row][k].mul(lower[row][k]).mul(pivot));
        }
        assert!(entry.rounded() > 0.0);
        diagonal[row] = entry;
    }
    (lower, diagonal)
}

fn solve(lower: &[Vec<Wide>], diagonal: &[Wide], rhs: &[Wide]) -> Vec<Wide> {
    let size = rhs.len();
    let mut result = rhs.to_vec();
    for row in 0..size {
        for column in row.saturating_sub(3)..row {
            result[row] = result[row].sub(lower[row][column].mul(result[column]));
        }
    }
    for (value, &d) in result.iter_mut().zip(diagonal) {
        *value = value.div(d);
    }
    for row in (0..size).rev() {
        for column in row + 1..(row + 4).min(size) {
            result[row] = result[row].sub(lower[column][row].mul(result[column]));
        }
    }
    result
}

fn dot(left: &[Wide], right: &[Wide]) -> Wide {
    assert_eq!(left.len(), right.len());
    left.iter()
        .zip(right)
        .fold(Wide::default(), |sum, (&a, &b)| sum.add(a.mul(b)))
}

fn norm(values: impl Iterator<Item = Wide>) -> f64 {
    values.fold(0.0_f64, |sum, v| sum.hypot(v.rounded()))
}

#[test]
fn wide_reference_preserves_known_sum_product_and_division_tails() {
    let one = Wide::from(1.0);
    let small = Wide::from(2.0_f64.powi(-54));
    assert_eq!(one.add(small).sub(one).rounded(), small.rounded());
    let delta = Wide::from(2.0_f64.powi(-27));
    assert_eq!(
        one.add(delta).mul(one.sub(delta)).sub(one).rounded(),
        -2.0_f64.powi(-54)
    );
    let third = one.div(Wide::from(3.0));
    assert!(third.mul(Wide::from(3.0)).sub(one).rounded().abs() < 1e-31);
    assert!(third.low != 0.0);
}

#[test]
fn independent_banded_reference_matches_existing_discrete_roots() {
    for (segments, expected) in [(80, 3.017539808750544e-7), (100, 1.236075027143164e-7)] {
        let reference = BendingReference::new(segments);
        let (value, vector) = reference.first_mode();
        assert!((value.rounded() / expected - 1.0).abs() < 2e-14);
        assert!(reference.residual(value, &vector) < 1e-20);
    }
}

#[test]
fn independent_banded_factor_recovers_known_physical_solutions() {
    for segments in [1, 3, 80, 128] {
        let reference = BendingReference::new(segments);
        let expected: Vec<_> = (0..2 * segments)
            .map(|i| Wide::from((i as f64 + 1.0) / 512.0))
            .collect();
        let rhs = reference.physical_apply(&expected);
        let (lower, diagonal) = factor(&reference.stiffness);
        let actual = solve(&lower, &diagonal, &rhs);
        let relative = norm(actual.iter().zip(&expected).map(|(&a, &b)| a.sub(b)))
            / norm(expected.iter().copied());
        assert!(
            relative < 1e-21,
            "segments={segments} relative={relative:e}"
        );
    }
}
