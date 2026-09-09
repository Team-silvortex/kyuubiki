use super::*;
use crate::solver_control::{SolverControl, with_solver_control};
use crate::thermal_plane_2d_results::build_thermal_plane_nodes;
use kyuubiki_protocol::{ThermalPlaneNodeInput, ThermalPlaneNodeResult};
use std::{hint::black_box, time::Instant};

#[derive(Clone, Copy, Debug)]
enum Kernel {
    Nodes,
    Maximum,
    Sum,
    Restore,
}

enum Output {
    Nodes(Vec<ThermalPlaneNodeResult>),
    Scalar(f64),
    Vector(Vec<f64>),
}

impl Output {
    fn check(&self, expected: &Self) {
        match (self, expected) {
            (Self::Nodes(a), Self::Nodes(b)) => assert_eq!(a, b),
            (Self::Scalar(a), Self::Scalar(b)) => same_bits(*a, *b),
            (Self::Vector(a), Self::Vector(b)) => {
                assert_eq!(a.len(), b.len());
                for (&a, &b) in a.iter().zip(b) {
                    same_bits(a, b);
                }
            }
            _ => panic!("benchmark result type changed"),
        }
    }
}

struct Input {
    values: Vec<f64>,
    nodes: Vec<ThermalPlaneNodeInput>,
    prescribed: Vec<(usize, f64)>,
    free: Vec<usize>,
}

fn execute(kernel: Kernel, old: bool, input: &Input) -> Output {
    let values = &input.values;
    match kernel {
        Kernel::Nodes => Output::Nodes(if old {
            input
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| {
                    let ux = values[index * 2];
                    let uy = values[index * 2 + 1];
                    ThermalPlaneNodeResult {
                        index,
                        id: node.id.clone(),
                        x: node.x,
                        y: node.y,
                        ux,
                        uy,
                        displacement_magnitude: (ux * ux + uy * uy).sqrt(),
                        temperature_delta: node.temperature_delta,
                    }
                })
                .collect()
        } else {
            build_thermal_plane_nodes(&input.nodes, values).unwrap()
        }),
        Kernel::Maximum => Output::Scalar(if old {
            values.iter().copied().fold(0.0_f64, f64::max)
        } else {
            max_results(SolverStage::ResultNodeSummary, values, |v| *v).unwrap()
        }),
        Kernel::Sum => Output::Scalar(if old {
            values.iter().copied().sum::<f64>()
        } else {
            fold_results(
                SolverStage::ResultTotals,
                values.iter().copied(),
                -0.0,
                |a, b| a + b,
            )
            .unwrap()
        }),
        Kernel::Restore => Output::Vector(if old {
            let mut full = vec![0.0; values.len()];
            for &(index, value) in &input.prescribed {
                full[index] = value;
            }
            for (index, &dof) in input.free.iter().enumerate() {
                full[dof] = values[index];
            }
            full
        } else {
            restore_solution(
                values.len(),
                &input.prescribed,
                &input.free,
                &values[..input.free.len()],
            )
            .unwrap()
        }),
    }
}

#[test]
#[ignore = "release-mode paired result-build benchmark; run explicitly with --ignored --nocapture"]
fn paired_postprocess_control_overhead() {
    const REPEATS: usize = 8;
    const SAMPLES: usize = 9;
    for kernel in [Kernel::Nodes, Kernel::Maximum, Kernel::Sum, Kernel::Restore] {
        let node_count = if matches!(kernel, Kernel::Nodes) {
            100_000
        } else {
            0
        };
        let value_count = if node_count > 0 {
            node_count * 2
        } else {
            1_000_000
        };
        let input = Input {
            values: (0..value_count).map(|i| (i % 257) as f64 * 0.125).collect(),
            nodes: (0..node_count)
                .map(|i| ThermalPlaneNodeInput {
                    id: format!("n{i}"),
                    x: (i % 1000) as f64,
                    y: (i / 1000) as f64,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 20.0,
                })
                .collect(),
            prescribed: if matches!(kernel, Kernel::Restore) {
                (0..value_count / 2).map(|i| (2 * i, -(i as f64))).collect()
            } else {
                vec![]
            },
            free: if matches!(kernel, Kernel::Restore) {
                (0..value_count / 2).map(|i| 2 * i + 1).collect()
            } else {
                vec![]
            },
        };
        let expected = execute(kernel, true, &input);
        let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| vec![]);
        for sample in 0..SAMPLES + 3 {
            for offset in 0..3 {
                let mode = (sample + offset) % 3;
                let mut last = Output::Scalar(0.0);
                let mut run = || -> Result<f64, String> {
                    let started = Instant::now();
                    for _ in 0..REPEATS {
                        last = black_box(execute(black_box(kernel), mode == 0, black_box(&input)));
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
        let median: Vec<_> = samples
            .iter_mut()
            .map(|s| {
                s.sort_by(f64::total_cmp);
                s[SAMPLES / 2]
            })
            .collect();
        eprintln!(
            "postprocess kernel={kernel:?} node_count={node_count} value_count={value_count} samples={SAMPLES} repeats={REPEATS} baseline_ms={:.6} unscoped_ms={:.6} controlled_ms={:.6}",
            median[0], median[1], median[2]
        );
    }
}
