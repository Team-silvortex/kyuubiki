use super::*;
use std::hint::black_box;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
enum Kernel {
    ValidateRhs,
    ValidateMatrix,
    Diagonal,
    Magnitude,
    RhsScale,
    MatrixScale,
    Regularize,
}

enum Output {
    Unit,
    Scalar(f64),
    Vector(Vec<f64>),
    Matrix(SparseMatrix),
}

impl Output {
    fn check(&self, expected: &Self) {
        match (self, expected) {
            (Self::Unit, Self::Unit) => {}
            (Self::Scalar(actual), Self::Scalar(expected)) => same_bits(&[*actual], &[*expected]),
            (Self::Vector(actual), Self::Vector(expected)) => same_bits(actual, expected),
            (Self::Matrix(actual), Self::Matrix(expected)) => same_matrix(actual, expected),
            _ => panic!("benchmark output kind changed"),
        }
    }
}

fn execute(
    kernel: Kernel,
    old: bool,
    matrix: &SparseMatrix,
    rhs: &[f64],
    factors: &[f64],
) -> Result<Output, String> {
    Ok(match kernel {
        Kernel::ValidateRhs => {
            let empty = SparseMatrix::new(0);
            if old {
                reference::validate(&empty, rhs)?;
            } else {
                scaling::validate_sparse_system_finite(&empty, rhs)?;
            }
            Output::Unit
        }
        Kernel::ValidateMatrix => {
            if old {
                reference::validate(matrix, &[])?;
            } else {
                scaling::validate_sparse_system_finite(matrix, &[])?;
            }
            Output::Unit
        }
        Kernel::Diagonal => Output::Vector(if old {
            reference::diagonal_scaling(matrix)
        } else {
            scaling::diagonal_sparse_scaling(matrix)?
        }),
        Kernel::Magnitude => Output::Scalar(if old {
            reference::diagonal_magnitude(matrix, factors)
        } else {
            scaling::average_scaled_diagonal_magnitude(matrix, factors)?
        }),
        Kernel::RhsScale => Output::Vector(if old {
            reference::scale_vector(rhs, factors)
        } else {
            scaling::scale_sparse_rhs(rhs, factors)?
        }),
        Kernel::MatrixScale => Output::Matrix(if old {
            reference::scale_matrix(matrix, factors)
        } else {
            scaling::scale_sparse_matrix(matrix, factors)?
        }),
        Kernel::Regularize => Output::Matrix(if old {
            reference::regularize(matrix, 0.01)
        } else {
            scaling::regularize_sparse_diagonal(matrix, 0.01)?
        }),
    })
}

#[test]
#[ignore = "release-mode paired scaling/validation benchmark; run explicitly with --ignored --nocapture"]
fn paired_sparse_scaling_control_overhead() {
    const REPEATS: usize = 8;
    const SAMPLES: usize = 9;
    for kernel in [
        Kernel::ValidateRhs,
        Kernel::ValidateMatrix,
        Kernel::Diagonal,
        Kernel::Magnitude,
        Kernel::RhsScale,
        Kernel::MatrixScale,
        Kernel::Regularize,
    ] {
        let size = if matches!(kernel, Kernel::MatrixScale | Kernel::Regularize) {
            100_000
        } else {
            1_000_000
        };
        let (matrix, rhs) = system(size, false, false);
        let factors = reference::diagonal_scaling(&matrix);
        let expected = execute(kernel, true, &matrix, &rhs, &factors).unwrap();
        let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
        for sample in 0..SAMPLES + 3 {
            for rotated in 0..3 {
                let mode = (sample + rotated) % 3;
                let mut last = Output::Unit;
                let mut run = || -> Result<f64, String> {
                    let started = Instant::now();
                    for _ in 0..REPEATS {
                        last = black_box(execute(
                            black_box(kernel),
                            mode == 0,
                            black_box(&matrix),
                            black_box(&rhs),
                            black_box(&factors),
                        )?);
                    }
                    Ok(started.elapsed().as_secs_f64() * 1000.0 / REPEATS as f64)
                };
                let elapsed = if mode == 2 {
                    with_solver_control(&SolverControl::default(), run).unwrap()
                } else {
                    run().unwrap()
                };
                last.check(&expected);
                if sample >= 3 {
                    samples[mode].push(elapsed);
                }
            }
        }
        let medians: Vec<_> = samples
            .iter_mut()
            .map(|samples| {
                samples.sort_by(f64::total_cmp);
                samples[SAMPLES / 2]
            })
            .collect();
        let vector_only = matches!(kernel, Kernel::ValidateRhs | Kernel::RhsScale);
        eprintln!(
            "sparse_scaling kernel={kernel:?} rows={} rhs_elements={} nnz={} samples={SAMPLES} repeats={REPEATS} baseline_ms={:.6} unscoped_ms={:.6} controlled_ms={:.6}",
            if vector_only { 0 } else { size },
            if vector_only { rhs.len() } else { 0 },
            if vector_only {
                0
            } else {
                matrix.non_zero_count()
            },
            medians[0],
            medians[1],
            medians[2]
        );
    }
}
