use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use crate::solver_preparation_tests::{heat_quad, heat_triangle, thermal_nodes};
use kyuubiki_protocol::*;

fn thermal_quad(n: usize) -> SolveThermalPlaneQuad2dRequest {
    let heat = heat_quad(n);
    SolveThermalPlaneQuad2dRequest {
        nodes: thermal_nodes(heat.nodes),
        elements: heat
            .elements
            .into_iter()
            .map(|e| ThermalPlaneQuadElementInput {
                id: e.id,
                node_i: e.node_i,
                node_j: e.node_j,
                node_k: e.node_k,
                node_l: e.node_l,
                thickness: e.thickness,
                youngs_modulus: 1e9,
                poisson_ratio: 0.25,
                thermal_expansion: 1e-5,
            })
            .collect(),
    }
}

fn thermal_triangle(n: usize) -> SolveThermalPlaneTriangle2dRequest {
    let heat = heat_triangle(n);
    SolveThermalPlaneTriangle2dRequest {
        nodes: thermal_nodes(heat.nodes),
        elements: heat
            .elements
            .into_iter()
            .map(|e| ThermalPlaneTriangleElementInput {
                id: e.id,
                node_i: e.node_i,
                node_j: e.node_j,
                node_k: e.node_k,
                thickness: e.thickness,
                youngs_modulus: 1e9,
                poisson_ratio: 0.25,
                thermal_expansion: 1e-5,
            })
            .collect(),
    }
}

pub(super) fn interrupted<T>(
    stage: SolverStage,
    steps: usize,
    run: impl FnOnce() -> Result<T, String>,
) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == steps as u64 {
                cancel.request_cancel();
            }
        },
        || {
            let raw = run();
            assert!(
                raw.is_err(),
                "raw result builder ignored {stage:?} after {steps}"
            );
            raw
        },
    );
    assert!(result.err().unwrap().starts_with("solver cancelled"));
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, steps as u64);
}

const RESULT_STAGES: [SolverStage; 6] = [
    SolverStage::ResultFreeDofs,
    SolverStage::ResultNodes,
    SolverStage::ResultElements,
    SolverStage::ResultNodeSummary,
    SolverStage::ResultElementSummary,
    SolverStage::ResultTotals,
];

fn summary_steps(stage: SolverStage, triangle: bool) -> usize {
    match stage {
        SolverStage::ResultNodeSummary => 169,
        SolverStage::ResultElementSummary => {
            if triangle {
                288
            } else {
                144
            }
        }
        _ => 64,
    }
}

#[test]
fn heat_quad_postprocessing_cancels_without_publishing_a_partial_field() {
    let request = heat_quad(12);
    let before = crate::solve_heat_plane_quad_2d(&request).unwrap();
    for stage in RESULT_STAGES {
        interrupted(stage, summary_steps(stage, false), || {
            crate::solve_heat_plane_quad_2d(&request)
        });
        assert_eq!(
            crate::solve_heat_plane_quad_2d_owned(request.clone()).unwrap(),
            before
        );
    }
    interrupted(SolverStage::ResultPrescribed, 26, || {
        crate::solve_heat_plane_quad_2d(&request)
    });
    let profile = crate::profile_heat_plane_quad_2d(&request).unwrap();
    assert_eq!(profile.result, before);
    for node in before.nodes {
        assert!((node.temperature - (100.0 - 80.0 * node.x)).abs() < 1e-6);
    }
}

#[test]
fn heat_triangle_postprocessing_cancels_without_publishing_a_partial_field() {
    let request = heat_triangle(12);
    let before = crate::solve_heat_plane_triangle_2d(&request).unwrap();
    for stage in RESULT_STAGES {
        interrupted(stage, summary_steps(stage, true), || {
            crate::solve_heat_plane_triangle_2d(&request)
        });
        assert_eq!(
            crate::solve_heat_plane_triangle_2d_owned(request.clone()).unwrap(),
            before
        );
    }
    interrupted(SolverStage::ResultPrescribed, 26, || {
        crate::solve_heat_plane_triangle_2d(&request)
    });
    for node in before.nodes {
        assert!((node.temperature - (100.0 - 80.0 * node.x)).abs() < 1e-6);
    }
}

#[test]
fn thermal_quad_postprocessing_cancellation_preserves_the_next_expansion() {
    let request = thermal_quad(12);
    let before = crate::solve_thermal_plane_quad_2d(&request).unwrap();
    for stage in RESULT_STAGES
        .into_iter()
        .chain([SolverStage::ResultRhsNorm])
    {
        interrupted(stage, summary_steps(stage, false), || {
            crate::solve_thermal_plane_quad_2d(&request)
        });
        assert_eq!(
            crate::solve_thermal_plane_quad_2d_owned(request.clone()).unwrap(),
            before
        );
    }
    assert_eq!(
        crate::profile_thermal_plane_quad_2d_with_options(&request, Default::default())
            .unwrap()
            .result,
        before
    );
    for node in before.nodes {
        assert!((node.ux - node.x * 2e-4).abs() < 1e-10);
        assert!((node.uy - node.y * 2e-4).abs() < 1e-10);
    }
}

#[test]
fn thermal_triangle_postprocessing_cancellation_preserves_the_next_expansion() {
    let request = thermal_triangle(12);
    let before = crate::solve_thermal_plane_triangle_2d(&request).unwrap();
    for stage in RESULT_STAGES
        .into_iter()
        .chain([SolverStage::ResultRhsNorm])
    {
        interrupted(stage, summary_steps(stage, true), || {
            crate::solve_thermal_plane_triangle_2d(&request)
        });
        assert_eq!(
            crate::solve_thermal_plane_triangle_2d_owned(request.clone()).unwrap(),
            before
        );
    }
    assert_eq!(
        crate::profile_thermal_plane_triangle_2d_with_options(&request, Default::default())
            .unwrap()
            .result,
        before
    );
    for node in before.nodes {
        assert!((node.ux - node.x * 2e-4).abs() < 1e-10);
        assert!((node.uy - node.y * 2e-4).abs() < 1e-10);
    }
}

#[test]
#[ignore = "retained old/new physical result comparison; run explicitly with --ignored --nocapture"]
fn retained_postprocess_physical_reference() {
    for n in [1, 7, 12] {
        for (name, value) in [
            (
                "heat_quad",
                serde_json::to_value(crate::solve_heat_plane_quad_2d(&heat_quad(n)).unwrap())
                    .unwrap(),
            ),
            (
                "heat_triangle",
                serde_json::to_value(
                    crate::solve_heat_plane_triangle_2d(&heat_triangle(n)).unwrap(),
                )
                .unwrap(),
            ),
            (
                "thermal_quad",
                serde_json::to_value(crate::solve_thermal_plane_quad_2d(&thermal_quad(n)).unwrap())
                    .unwrap(),
            ),
            (
                "thermal_triangle",
                serde_json::to_value(
                    crate::solve_thermal_plane_triangle_2d(&thermal_triangle(n)).unwrap(),
                )
                .unwrap(),
            ),
        ] {
            println!(
                "postprocess_reference {name} {n} {}",
                serde_json::to_string(&value).unwrap()
            );
        }
    }
}

#[test]
fn owned_short_results_poll_entry_and_terminal_nodes_and_elements() {
    for n in [1, 7] {
        for (stage, steps) in [
            (SolverStage::ResultNodes, 0),
            (SolverStage::ResultNodes, (n + 1) * (n + 1)),
            (SolverStage::ResultElements, 0),
            (SolverStage::ResultElements, n * n),
            (SolverStage::ResultNodeSummary, (n + 1) * (n + 1)),
            (SolverStage::ResultTotals, n * n),
        ] {
            interrupted(stage, steps, || {
                crate::solve_heat_plane_quad_2d_owned(heat_quad(n))
            });
            interrupted(stage, steps, || {
                crate::solve_thermal_plane_quad_2d_owned(thermal_quad(n))
            });
            let triangle_steps = if matches!(
                stage,
                SolverStage::ResultElements | SolverStage::ResultTotals
            ) {
                steps * 2
            } else {
                steps
            };
            interrupted(stage, triangle_steps, || {
                crate::solve_heat_plane_triangle_2d_owned(heat_triangle(n))
            });
            interrupted(stage, triangle_steps, || {
                crate::solve_thermal_plane_triangle_2d_owned(thermal_triangle(n))
            });
        }
    }
    interrupted(SolverStage::ResultFreeDofs, 0, || {
        crate::solve_heat_plane_quad_2d_owned(heat_quad(1))
    });
}

#[test]
fn later_thermal_summary_invocations_remain_cancellable() {
    use std::{cell::Cell, rc::Rc};
    for stage in [
        SolverStage::ResultNodeSummary,
        SolverStage::ResultElementSummary,
    ] {
        for triangle in [false, true] {
            let steps = summary_steps(stage, triangle) as u64;
            let control = SolverControl::default();
            let cancel = control.clone();
            let invocations = Rc::new(Cell::new(0));
            let observed = invocations.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage {
                        if point.completed_steps == 0 {
                            observed.set(observed.get() + 1);
                        }
                        if observed.get() == 2 && point.completed_steps == steps {
                            cancel.request_cancel();
                        }
                    }
                },
                || {
                    let raw = if triangle {
                        crate::profile_thermal_plane_triangle_2d_with_options(
                            &thermal_triangle(12),
                            Default::default(),
                        )
                        .map(|_| ())
                    } else {
                        crate::profile_thermal_plane_quad_2d_with_options(
                            &thermal_quad(12),
                            Default::default(),
                        )
                        .map(|_| ())
                    };
                    assert!(raw.is_err(), "late summary returned a successful profile");
                    raw
                },
            );
            assert!(result.err().unwrap().starts_with("solver cancelled"));
            assert_eq!(invocations.get(), 2);
            let point = control.last_checkpoint().unwrap();
            assert_eq!(point.stage, stage);
            assert_eq!(point.completed_steps, steps);
        }
    }
}
