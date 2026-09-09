use crate::linear_algebra::{SparseMatrix, add_at, solve_spd_system_profile_with_options};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use crate::{SpdPreconditioner, SpdSolveOptions};
use kyuubiki_protocol::*;

fn interrupted<T>(stage: SolverStage, operation: impl FnOnce() -> Result<T, String>) {
    interrupted_after(stage, 64, operation);
}

fn interrupted_after<T>(
    stage: SolverStage,
    steps: u64,
    operation: impl FnOnce() -> Result<T, String>,
) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps >= steps {
                cancel.request_cancel();
            }
        },
        operation,
    );
    assert!(
        result.is_err(),
        "preparation ignored cancellation at {stage:?}"
    );
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert!(point.completed_steps >= steps);
}

fn matrix() -> SparseMatrix {
    let mut matrix = SparseMatrix::with_uniform_row_capacity(1100, 5);
    for row in 0..1100 {
        add_at(&mut matrix, row, row, 5.0);
        for offset in [1, 17] {
            if row + offset < 1100 {
                add_at(&mut matrix, row, row + offset, -1.0);
                add_at(&mut matrix, row + offset, row, -1.0);
            }
        }
    }
    matrix
}

fn cancel_setup(stage: SolverStage) {
    let matrix = matrix();
    let rhs = vec![1.0; 1100];
    let options = SpdSolveOptions {
        preconditioner: SpdPreconditioner::IncompleteCholesky,
        progress_interval: None,
    };
    let original = solve_spd_system_profile_with_options(&matrix, &rhs, options.clone())
        .unwrap()
        .solution;
    interrupted(stage, || {
        solve_spd_system_profile_with_options(&matrix, &rhs, options.clone())
    });
    assert_eq!(
        solve_spd_system_profile_with_options(&matrix, &rhs, options)
            .unwrap()
            .solution,
        original
    );
}

#[test]
fn scaled_compression_cancellation_preserves_original_matrix() {
    cancel_setup(SolverStage::SparseCompress);
}
#[test]
fn inverse_diagonal_setup_cancellation_is_not_silently_ignored() {
    cancel_setup(SolverStage::PreconditionerSetup);
}
#[test]
fn ic0_factor_cancellation_does_not_run_the_outer_solver() {
    cancel_setup(SolverStage::IncompleteCholeskyFactor);
}
#[test]
fn ic0_transpose_cancellation_discards_the_incomplete_preconditioner() {
    cancel_setup(SolverStage::IncompleteCholeskyTranspose);
}

#[test]
fn unscaled_compression_propagates_each_preparation_cancellation() {
    let matrix = matrix();
    for stage in [
        SolverStage::SparseCompress,
        SolverStage::PreconditionerSetup,
        SolverStage::IncompleteCholeskyFactor,
        SolverStage::IncompleteCholeskyTranspose,
    ] {
        interrupted(stage, || {
            matrix.compress(SpdPreconditioner::IncompleteCholesky)
        });
    }
    matrix
        .compress(SpdPreconditioner::IncompleteCholesky)
        .unwrap();
}

#[test]
fn prepared_solver_cannot_publish_a_cancelled_compression() {
    use crate::linear_algebra::PreparedSpdSolver;
    let rhs = vec![1.0; 1100];
    let before = PreparedSpdSolver::factor(matrix())
        .unwrap()
        .solve(&rhs)
        .unwrap();
    for stage in [
        SolverStage::SparseCompress,
        SolverStage::PreconditionerSetup,
    ] {
        interrupted(stage, || PreparedSpdSolver::factor(matrix()));
    }
    let prepared = PreparedSpdSolver::factor(matrix()).unwrap();
    assert_eq!(prepared.solve(&rhs).unwrap(), before);
    assert_eq!(prepared.solve(&rhs).unwrap(), before);
}

#[test]
fn ic0_transpose_prefix_sum_and_fill_both_observe_cancellation() {
    let matrix = matrix();
    for steps in [1100 + 64, 2200 + 64] {
        interrupted_after(SolverStage::IncompleteCholeskyTranspose, steps, || {
            matrix.compress(SpdPreconditioner::IncompleteCholesky)
        });
    }
    matrix
        .compress(SpdPreconditioner::IncompleteCholesky)
        .unwrap();
}

pub(super) fn heat_quad(n: usize) -> SolveHeatPlaneQuad2dRequest {
    let nodes = (0..=n)
        .flat_map(|y| {
            (0..=n).map(move |x| HeatPlaneNodeInput {
                id: format!("n{x}-{y}"),
                x: x as f64 / n as f64,
                y: y as f64 / n as f64,
                fix_temperature: x == 0 || x == n,
                temperature: if x == 0 { 100.0 } else { 20.0 },
                heat_load: 0.0,
            })
        })
        .collect();
    let elements = (0..n)
        .flat_map(|y| {
            (0..n).map(move |x| {
                let i = y * (n + 1) + x;
                HeatPlaneQuadElementInput {
                    id: format!("e{x}-{y}"),
                    node_i: i,
                    node_j: i + 1,
                    node_k: i + n + 2,
                    node_l: i + n + 1,
                    thickness: 0.02,
                    conductivity: 45.0,
                }
            })
        })
        .collect();
    SolveHeatPlaneQuad2dRequest { nodes, elements }
}

pub(super) fn heat_triangle(n: usize) -> SolveHeatPlaneTriangle2dRequest {
    let quad = heat_quad(n);
    let elements = quad
        .elements
        .iter()
        .flat_map(|e| {
            [
                HeatPlaneTriangleElementInput {
                    id: format!("{}a", e.id),
                    node_i: e.node_i,
                    node_j: e.node_j,
                    node_k: e.node_k,
                    thickness: e.thickness,
                    conductivity: e.conductivity,
                },
                HeatPlaneTriangleElementInput {
                    id: format!("{}b", e.id),
                    node_i: e.node_i,
                    node_j: e.node_k,
                    node_k: e.node_l,
                    thickness: e.thickness,
                    conductivity: e.conductivity,
                },
            ]
        })
        .collect();
    SolveHeatPlaneTriangle2dRequest {
        nodes: quad.nodes,
        elements,
    }
}

fn verify_heat(nodes: &[HeatPlaneNodeResult]) {
    for node in nodes {
        assert!((node.temperature - (100.0 - 80.0 * node.x)).abs() < 1e-6);
    }
}

#[test]
fn heat_quad_precompute_and_assembly_cancel_without_changing_the_next_field() {
    let request = heat_quad(12);
    let before = crate::solve_heat_plane_quad_2d(&request).unwrap();
    for stage in [
        SolverStage::ElementPrecompute,
        SolverStage::ElementAssembly,
        SolverStage::ConstraintMap,
        SolverStage::ConstraintReduce,
    ] {
        interrupted(stage, || crate::solve_heat_plane_quad_2d(&request));
        assert_eq!(crate::solve_heat_plane_quad_2d(&request).unwrap(), before);
    }
    verify_heat(&before.nodes);
}

#[test]
fn short_element_batches_poll_at_the_final_element() {
    let request = heat_quad(7);
    for stage in [SolverStage::ElementPrecompute, SolverStage::ElementAssembly] {
        interrupted_after(stage, 49, || crate::solve_heat_plane_quad_2d(&request));
    }
    verify_heat(&crate::solve_heat_plane_quad_2d(&request).unwrap().nodes);
}

#[test]
fn heat_triangle_precompute_and_assembly_cancel_without_changing_the_next_field() {
    let request = heat_triangle(12);
    let before = crate::solve_heat_plane_triangle_2d(&request).unwrap();
    for stage in [
        SolverStage::ElementPrecompute,
        SolverStage::ElementAssembly,
        SolverStage::ConstraintMap,
        SolverStage::ConstraintReduce,
    ] {
        interrupted(stage, || crate::solve_heat_plane_triangle_2d(&request));
        assert_eq!(
            crate::solve_heat_plane_triangle_2d(&request).unwrap(),
            before
        );
    }
    verify_heat(&before.nodes);
}

pub(super) fn thermal_nodes(heat: Vec<HeatPlaneNodeInput>) -> Vec<ThermalPlaneNodeInput> {
    heat.into_iter()
        .map(|node| ThermalPlaneNodeInput {
            id: node.id,
            x: node.x,
            y: node.y,
            fix_x: node.x == 0.0,
            fix_y: node.y == 0.0,
            load_x: 0.0,
            load_y: 0.0,
            temperature_delta: 20.0,
        })
        .collect()
}

#[test]
fn thermal_quad_preparation_cancellation_does_not_poison_the_next_expansion() {
    let heat = heat_quad(12);
    let request = SolveThermalPlaneQuad2dRequest {
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
    };
    let before = crate::solve_thermal_plane_quad_2d(&request).unwrap();
    for stage in [
        SolverStage::ElementPrecompute,
        SolverStage::ElementAssembly,
        SolverStage::ConstraintMap,
        SolverStage::ConstraintReduce,
    ] {
        interrupted(stage, || crate::solve_thermal_plane_quad_2d(&request));
        assert_eq!(
            crate::solve_thermal_plane_quad_2d(&request).unwrap(),
            before
        );
    }
    for node in before.nodes {
        assert!((node.ux - node.x * 2e-4).abs() < 1e-10);
        assert!((node.uy - node.y * 2e-4).abs() < 1e-10);
    }
}

#[test]
fn thermal_triangle_preparation_cancellation_does_not_poison_the_next_expansion() {
    let heat = heat_triangle(12);
    let request = SolveThermalPlaneTriangle2dRequest {
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
    };
    let before = crate::solve_thermal_plane_triangle_2d(&request).unwrap();
    for stage in [
        SolverStage::ElementPrecompute,
        SolverStage::ElementAssembly,
        SolverStage::ConstraintMap,
        SolverStage::ConstraintReduce,
    ] {
        interrupted(stage, || crate::solve_thermal_plane_triangle_2d(&request));
        assert_eq!(
            crate::solve_thermal_plane_triangle_2d(&request).unwrap(),
            before
        );
    }
    for node in before.nodes {
        assert!((node.ux - node.x * 2e-4).abs() < 1e-10);
        assert!((node.uy - node.y * 2e-4).abs() < 1e-10);
    }
}

#[test]
fn real_heat_ic0_preparation_cancellation_keeps_the_next_solution_analytical() {
    let request = heat_quad(40);
    let options = SpdSolveOptions {
        preconditioner: SpdPreconditioner::IncompleteCholesky,
        progress_interval: None,
    };
    for stage in [
        SolverStage::IncompleteCholeskyFactor,
        SolverStage::IncompleteCholeskyTranspose,
        SolverStage::Ic0Forward,
        SolverStage::Ic0Backward,
    ] {
        interrupted(stage, || {
            crate::profile_heat_plane_quad_2d_with_options(&request, options.clone())
        });
    }
    verify_heat(
        &crate::profile_heat_plane_quad_2d_with_options(&request, options)
            .unwrap()
            .result
            .nodes,
    );
}

#[test]
fn real_heat_sgs_application_cancellation_preserves_the_analytical_field() {
    let request = heat_quad(40);
    let options = SpdSolveOptions {
        preconditioner: SpdPreconditioner::SymmetricGaussSeidel,
        progress_interval: None,
    };
    for stage in [SolverStage::SgsForward, SolverStage::SgsBackward] {
        interrupted(stage, || {
            crate::profile_heat_plane_quad_2d_with_options(&request, options.clone())
        });
    }
    verify_heat(
        &crate::profile_heat_plane_quad_2d_with_options(&request, options)
            .unwrap()
            .result
            .nodes,
    );
}
