use super::jacobi_eigenpairs;
use crate::linear_algebra::stable_l2_norm;

fn basis(size: usize) -> Vec<Vec<f64>> {
    assert!(size.is_power_of_two());
    (0..size)
        .map(|column| {
            (0..size)
                .map(|row| {
                    let sign = if (row & column).count_ones() % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    };
                    sign / (size as f64).sqrt()
                })
                .collect()
        })
        .collect()
}

fn from_spectrum(basis: &[Vec<f64>], values: &[f64]) -> Vec<Vec<f64>> {
    assert_eq!(basis.len(), values.len());
    assert!(basis.iter().all(|vector| vector.len() == basis.len()));
    (0..basis.len())
        .map(|row| {
            (0..basis.len())
                .map(|column| {
                    basis
                        .iter()
                        .zip(values)
                        .map(|(v, value)| value * v[row] * v[column])
                        .sum()
                })
                .collect()
        })
        .collect()
}

fn projector(vectors: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let size = vectors[0].len();
    assert!(vectors.iter().all(|vector| vector.len() == size));
    (0..size)
        .map(|row| {
            (0..size)
                .map(|column| vectors.iter().map(|v| v[row] * v[column]).sum())
                .collect()
        })
        .collect()
}

fn distance(left: &[Vec<f64>], right: &[Vec<f64>]) -> f64 {
    assert_eq!(left.len(), right.len());
    assert!(left.iter().zip(right).all(|(a, b)| a.len() == b.len()));
    stable_l2_norm(
        left.iter()
            .flatten()
            .zip(right.iter().flatten())
            .map(|(a, b)| a - b),
    )
}

fn check_spectrum(matrix: &[Vec<f64>], pairs: &[(f64, Vec<f64>)], expected: &[f64]) {
    assert_eq!(pairs.len(), expected.len());
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    for ((value, vector), reference) in pairs.iter().zip(expected) {
        assert_eq!(vector.len(), matrix.len());
        let reference_scale = if *reference == 0.0 {
            scale
        } else {
            reference.abs()
        };
        assert!(
            (value - reference).abs() / reference_scale < 1e-10,
            "{value:e} != {reference:e}"
        );
        assert!((stable_l2_norm(vector.iter().copied()) - 1.0).abs() < 1e-12);
        let residual = stable_l2_norm(matrix.iter().zip(vector).map(|(row, component)| {
            row.iter().zip(vector).map(|(a, b)| a * b).sum::<f64>() - value * component
        }));
        assert!(
            residual / reference_scale < 1e-10,
            "residual={residual:e}, value={value:e}"
        );
    }
    assert!(pairs.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    for (index, (_, vector)) in pairs.iter().enumerate() {
        for (_, other) in pairs.iter().skip(index + 1) {
            assert!(
                vector
                    .iter()
                    .zip(other)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    .abs()
                    < 1e-12
            );
        }
    }
}

#[test]
fn repeated_signed_and_zero_clusters_keep_their_full_subspaces() {
    let basis = basis(8);
    let values = [-3.0, -3.0, 0.0, 0.0, 1.0, 1.0, 4.0, 8.0];
    let matrix = from_spectrum(&basis, &values);
    let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
    check_spectrum(&matrix, &pairs, &values);
    for first in [0, 2, 4] {
        let modes: Vec<_> = pairs[first..first + 2]
            .iter()
            .map(|(_, v)| v.clone())
            .collect();
        assert!(distance(&projector(&modes), &projector(&basis[first..first + 2])) < 1e-10);
    }
}

#[test]
fn repeated_dense_modes_retain_multiplicity_and_the_analytic_projector() {
    for size in [4, 8, 16, 32] {
        let basis = basis(size);
        let count = size / 2;
        let values: Vec<_> = (0..size)
            .map(|i| if i < count { 1.0 } else { 4.0 + i as f64 })
            .collect();
        let matrix = from_spectrum(&basis, &values);
        let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
        check_spectrum(&matrix, &pairs, &values);
        let modes: Vec<_> = pairs[..count].iter().map(|(_, v)| v.clone()).collect();
        assert!(distance(&projector(&modes), &projector(&basis[..count])) < 1e-10);
    }
}

#[test]
fn clustered_dense_modes_keep_small_splittings_and_the_combined_subspace() {
    for gap in [1e-5, 1e-8, 1e-10] {
        let basis = basis(8);
        let values = [1.0, 1.0, 1.0 + gap, 1.0 + gap, 4.0, 7.0, 9.0, 13.0];
        let matrix = from_spectrum(&basis, &values);
        let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
        check_spectrum(&matrix, &pairs, &values);
        assert!(((pairs[2].0 - pairs[1].0) / gap - 1.0).abs() < 2e-4);
        let modes: Vec<_> = pairs[..4].iter().map(|(_, v)| v.clone()).collect();
        assert!(distance(&projector(&modes), &projector(&basis[..4])) < 1e-10);
    }
}

#[test]
fn repeated_subspaces_survive_common_scaling_coordinate_permutation_and_signs() {
    let basis = basis(8);
    let values = [1.0, 1.0, 1.0, 1.0, 4.0, 7.0, 9.0, 13.0];
    for scale in [1e-200, 1.0, 1e200] {
        let order = [3, 7, 0, 2, 5, 1, 6, 4];
        let signs = [1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0];
        let moved: Vec<Vec<_>> = basis
            .iter()
            .map(|v| {
                order
                    .iter()
                    .enumerate()
                    .map(|(i, j)| signs[i] * v[*j])
                    .collect()
            })
            .collect();
        let scaled = values.map(|value| value * scale);
        let matrix = from_spectrum(&moved, &scaled);
        let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
        check_spectrum(&matrix, &pairs, &scaled);
        let modes: Vec<_> = pairs[..4].iter().map(|(_, v)| v.clone()).collect();
        assert!(distance(&projector(&modes), &projector(&moved[..4])) < 1e-10);
    }
}

#[test]
fn cluster_oracle_accepts_rotated_bases_but_detects_a_duplicated_direction() {
    let basis = basis(4);
    let rotated: Vec<Vec<_>> = [1.0, -1.0]
        .into_iter()
        .map(|sign| {
            basis[0]
                .iter()
                .zip(&basis[1])
                .map(|(a, b)| (a + sign * b) / 2.0_f64.sqrt())
                .collect()
        })
        .collect();
    let reference = projector(&basis[..2]);
    assert!(distance(&reference, &projector(&rotated)) < 1e-14);
    let duplicated = vec![basis[0].clone(), basis[0].clone()];
    assert!(distance(&reference, &projector(&duplicated)) > 1.0);
}
