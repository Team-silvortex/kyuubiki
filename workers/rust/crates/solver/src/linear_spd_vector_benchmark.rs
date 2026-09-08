use super::*;
use std::hint::black_box;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
enum Kernel {
    RhsScale,
    Normalize,
    Copy,
    Dot,
    Norm,
    Update,
    Residual,
    Direction,
    Scale,
}

#[derive(Clone)]
struct Workspace {
    a: Vec<f64>,
    b: Vec<f64>,
    x: Vec<f64>,
    r: Vec<f64>,
    scratch: Vec<f64>,
    scalar: f64,
}

impl Workspace {
    fn new(size: usize) -> Self {
        Self {
            a: (0..size).map(|i| 1.0 + (i % 17) as f64 / 17.0).collect(),
            b: (0..size).map(|i| 0.125 + (i % 13) as f64 / 13.0).collect(),
            x: vec![0.0; size],
            r: vec![1.0; size],
            scratch: Vec::new(),
            scalar: 0.0,
        }
    }

    fn step(&mut self, kernel: Kernel, reference: bool) -> Result<(), String> {
        let Self {
            a,
            b,
            x,
            r,
            scratch,
            scalar,
        } = self;
        match kernel {
            Kernel::RhsScale => {
                *scalar = if reference {
                    a.iter().map(|v| v.abs()).fold(0.0, f64::max)
                } else {
                    rhs_scale(a)?
                }
            }
            Kernel::Normalize => {
                *scratch = if reference {
                    a.iter().map(|v| v / 7.0).collect()
                } else {
                    normalize_rhs(a, 7.0)?
                }
            }
            Kernel::Copy => {
                if reference {
                    x.clone_from(a);
                } else {
                    copy_direction(a, x)?;
                }
            }
            Kernel::Dot => {
                *scalar = if reference {
                    reference_dot(a, b)
                } else {
                    dot(a, b)?
                }
            }
            Kernel::Norm => {
                *scalar = if reference {
                    reference_norm(a)
                } else {
                    l2_norm(a)?
                }
            }
            Kernel::Update => {
                *scalar = if reference {
                    reference_update(x, r, a, b, 0.25)
                } else {
                    update_solution_residual(x, r, a, b, 0.25)?
                }
            }
            Kernel::Residual => {
                if reference {
                    let mut sum = 0.0;
                    for index in 0..a.len() {
                        r[index] = a[index] / 7.0 - b[index];
                        sum += r[index] * r[index];
                    }
                    *scalar = sum;
                } else {
                    *scalar = update_residual(a, 7.0, b, r)?;
                }
            }
            Kernel::Direction => {
                if reference {
                    for index in 0..x.len() {
                        x[index] = b[index] + 0.125 * x[index];
                    }
                } else {
                    update_direction(x, b, 0.125)?;
                }
            }
            Kernel::Scale => {
                if reference {
                    for value in x {
                        *value *= 1.0001;
                    }
                } else {
                    *x = rescale_solution(std::mem::take(x), 1.0001)?;
                }
            }
        }
        Ok(())
    }

    fn check(&self, reference: &Self) {
        same_bits(&[self.scalar], &[reference.scalar]);
        same_bits(&self.x, &reference.x);
        same_bits(&self.r, &reference.r);
        same_bits(&self.scratch, &reference.scratch);
    }
}

#[test]
#[ignore = "release-mode paired vector microbenchmark; run explicitly with --ignored --nocapture"]
fn paired_pcg_vector_control_overhead() {
    const SIZE: usize = 1_000_000;
    const REPEATS: usize = 20;
    const SAMPLES: usize = 9;
    let mut seed = Workspace::new(SIZE);
    seed.x.clone_from(&seed.a);
    for kernel in [
        Kernel::RhsScale,
        Kernel::Normalize,
        Kernel::Copy,
        Kernel::Dot,
        Kernel::Norm,
        Kernel::Update,
        Kernel::Residual,
        Kernel::Direction,
        Kernel::Scale,
    ] {
        let mut expected = seed.clone();
        for _ in 0..REPEATS {
            expected.step(kernel, true).unwrap();
        }
        let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
        for sample in 0..SAMPLES + 3 {
            for rotated in 0..3 {
                let mode = (sample + rotated) % 3;
                let mut workspace = seed.clone();
                let mut run = || -> Result<f64, String> {
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        black_box(&mut workspace).step(black_box(kernel), mode == 0)?;
                    }
                    Ok(start.elapsed().as_secs_f64() * 1000.0 / REPEATS as f64)
                };
                let elapsed = if mode == 2 {
                    with_solver_control(&SolverControl::default(), run).unwrap()
                } else {
                    run().unwrap()
                };
                workspace.check(&expected);
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
        eprintln!(
            "pcg_vector kernel={kernel:?} elements={SIZE} samples={SAMPLES} repeats={REPEATS} baseline_ms={:.6} unscoped_ms={:.6} controlled_ms={:.6}",
            medians[0], medians[1], medians[2]
        );
    }
}
