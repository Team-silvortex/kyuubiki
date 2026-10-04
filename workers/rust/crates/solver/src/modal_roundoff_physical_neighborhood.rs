use super::{budget::Checks, wide_factor::dot, wide_factor::less};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) const PASSES: usize = 4;
pub(super) const MAX_CERTIFICATES: usize = 26;

pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
}

impl Plan {
    pub(super) fn new(size: usize, width: usize) -> Result<Self, String> {
        super::budget::Plan::new(size)?;
        if !(1..=4).contains(&width) {
            return Err("physical neighborhood width must be 1..=4".into());
        }
        let square = size * size;
        let payload_bytes = 4 * square * std::mem::size_of::<Wide>() + 4096 * size;
        let component_visits = 4 * square * size + 2048 * square + 65536 * size;
        if payload_bytes > 8 * 1024 * 1024 || component_visits > 250_000_000 {
            return Err("physical neighborhood exceeds its separate proposal budget".into());
        }
        Ok(Self {
            payload_bytes,
            component_visits,
        })
    }
}

// Test-only all-coordinate proposals, without fine-coordinate elimination.
pub(super) struct Neighborhood {
    columns: Vec<Vec<Wide>>,
    gram: Vec<Vec<Wide>>,
    families: Vec<Vec<Vec<usize>>>,
}

fn input(value: Wide) -> bool {
    super::wide_factor::bounded(value)
        && (value.high == 0.0 || (1e-50..=1e50).contains(&value.high.abs()))
}

impl Neighborhood {
    pub(super) fn prepare(matrix: Vec<Vec<Wide>>, width: usize) -> Result<Self, String> {
        let size = matrix.len();
        Plan::new(size, width)?;
        if matrix
            .iter()
            .any(|r| r.len() != size || r.iter().any(|&v| !input(v)))
        {
            return Err("physical neighborhood requires bounded square directions".into());
        }
        let columns: Vec<Vec<_>> = (0..size)
            .map(|j| matrix.iter().map(|row| row[j]).collect())
            .collect();
        let mut gram = vec![vec![Wide::default(); size]; size];
        for i in 0..size {
            for j in 0..=i {
                gram[i][j] = dot(&columns[i], &columns[j])?;
                gram[j][i] = gram[i][j];
            }
        }
        let mut families = vec![(0..size).map(|i| vec![i]).collect()];
        if width >= 2 {
            let rounded: Vec<Vec<_>> = columns
                .iter()
                .map(|c| c.iter().map(|v| v.rounded()).collect())
                .collect();
            let pairs = super::coupled_pairs(&rounded)?;
            families.push(pairs.iter().map(|&(a, b)| vec![a, b]).collect());
            if width >= 3 {
                let width = width.min(size);
                let mut groups: Vec<Vec<_>> = (0..=size - width)
                    .map(|i| (i..i + width).collect())
                    .collect();
                for root in 0..size {
                    let mut neighbors: Vec<_> = pairs
                        .iter()
                        .filter_map(|&(a, b)| {
                            if a == root {
                                Some(b)
                            } else if b == root {
                                Some(a)
                            } else {
                                None
                            }
                        })
                        .map(|i| {
                            let denominator =
                                gram[root][root].rounded().sqrt() * gram[i][i].rounded().sqrt();
                            (gram[root][i].rounded().abs() / denominator, i)
                        })
                        .collect();
                    neighbors.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
                    if neighbors.len() >= width - 1 {
                        let mut group = vec![root];
                        group.extend(neighbors.into_iter().take(width - 1).map(|(_, i)| i));
                        group.sort_unstable();
                        groups.push(group);
                    }
                    checkpoint(SolverStage::ModalVectorScan, root)?;
                }
                groups.sort_unstable();
                groups.dedup();
                families.push(groups);
            }
        }
        if families.iter().map(Vec::len).sum::<usize>() > 7 * size {
            return Err("physical neighborhood exceeded its group cap".into());
        }
        Ok(Self {
            columns,
            gram,
            families,
        })
    }

    pub(super) fn correct(
        &self,
        seed: &[f64],
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        if seed.len() != self.columns.len()
            || seed.iter().any(|&v| !input(Wide { high: v, low: 0.0 }))
            || !tolerance.is_finite()
            || tolerance <= 0.0
        {
            return Err(
                "physical neighborhood requires a bounded seed and positive tolerance".into(),
            );
        }
        let mut checks = Checks::new();
        let mut retained = seed.to_vec();
        let (mut relative, mut residual) = self.measure(&retained, &mut checked, &mut checks)?;
        for pass in 0..PASSES {
            if relative <= tolerance {
                break;
            }
            checkpoint(SolverStage::ModalIteration, pass)?;
            let before = relative;
            for groups in &self.families {
                for reverse in [false, true] {
                    let mut candidate = retained.clone();
                    let mut predicted = residual.clone();
                    for visit in 0..groups.len() {
                        let index = if reverse {
                            groups.len() - 1 - visit
                        } else {
                            visit
                        };
                        self.choose(&groups[index], &mut candidate, &mut predicted)?;
                    }
                    let (next, fresh) = self.measure(&candidate, &mut checked, &mut checks)?;
                    if next < relative {
                        retained = candidate;
                        relative = next;
                        residual = fresh;
                    }
                    if relative <= tolerance {
                        break;
                    }
                }
                if relative <= tolerance {
                    break;
                }
            }
            if relative >= before {
                break;
            }
        }
        let (relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            return Err(format!(
                "physical neighborhood did not reach its unchanged residual gate (relative={relative:e})"
            ));
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(retained)
    }

    fn measure(
        &self,
        vector: &[f64],
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
    ) -> Result<(f64, Vec<Wide>), String> {
        if vector.iter().any(|&v| !input(Wide { high: v, low: 0.0 })) {
            return Err("physical neighborhood candidate exceeds its range".into());
        }
        checks.next()?;
        let (relative, residual) = checked(vector)?;
        if !relative.is_finite()
            || relative < 0.0
            || residual.len() != vector.len()
            || residual.iter().any(|&v| !input(Wide { high: v, low: 0.0 }))
        {
            return Err("physical neighborhood requires a finite matching certificate".into());
        }
        Ok((relative, residual.into_iter().map(Wide::from).collect()))
    }

    fn choose(
        &self,
        group: &[usize],
        vector: &mut [f64],
        residual: &mut [Wide],
    ) -> Result<(), String> {
        let mut products = Vec::with_capacity(group.len());
        let mut deltas = Vec::with_capacity(group.len());
        for &i in group {
            products.push(dot(&self.columns[i], residual)?);
            deltas.push(
                [vector[i], vector[i].next_up(), vector[i].next_down()].map(|v| {
                    if input(Wide { high: v, low: 0.0 }) {
                        Some(Wide::from(v).sub(Wide::from(vector[i])))
                    } else {
                        None
                    }
                }),
            );
        }
        // The constant residual square cancels in comparisons; retain wide cross terms.
        let mut best = Wide::default();
        let mut retained = vec![Wide::default(); group.len()];
        for trial in 1..3_usize.pow(group.len() as u32) {
            let mut encoded = trial;
            let mut delta = Vec::with_capacity(group.len());
            for options in &deltas {
                if let Some(value) = options[encoded % 3] {
                    delta.push(value);
                }
                encoded /= 3;
            }
            if delta.len() != group.len() {
                continue;
            }
            let mut score = Wide::default();
            for (i, &change) in delta.iter().enumerate() {
                score = score.add(change.mul(products[i]).mul(Wide::from(2.0)));
                for (j, &other) in delta.iter().enumerate() {
                    score = score.add(change.mul(other).mul(self.gram[group[i]][group[j]]));
                }
            }
            if less(score, best) {
                best = score;
                retained = delta;
            }
            checkpoint(SolverStage::ModalVectorScan, trial)?;
        }
        for (&column, &delta) in group.iter().zip(&retained) {
            let next = Wide::from(vector[column]).add(delta).rounded();
            if !input(Wide {
                high: next,
                low: 0.0,
            }) {
                return Err("physical neighborhood rounded coordinate exceeds its range".into());
            }
            for (row, (r, &a)) in residual.iter_mut().zip(&self.columns[column]).enumerate() {
                *r = r.add(a.mul(delta));
                if !super::wide_factor::bounded(*r) {
                    return Err("physical neighborhood prediction exceeds its range".into());
                }
                checkpoint_chunk(SolverStage::ModalVectorUpdate, row + 1, self.columns.len())?;
            }
            vector[column] = next;
        }
        Ok(())
    }
}

#[path = "modal_roundoff_physical_neighborhood_control_tests.rs"]
mod control_tests;
