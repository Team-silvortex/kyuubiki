use kyuubiki_protocol::{
    BucklingBeam1dElementInput, BucklingBeam1dNodeInput, Frame2dElementInput, Frame2dNodeInput,
    SolveBucklingBeam1dRequest, SolveBucklingFrame2dRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_buckling_beam_1d, solve_buckling_frame_2d};

fn beam(elements: usize) -> SolveBucklingBeam1dRequest {
    SolveBucklingBeam1dRequest {
        nodes: (0..=elements)
            .map(|index| BucklingBeam1dNodeInput {
                id: format!("n{index}"),
                x: 3.2 * index as f64 / elements as f64,
                fix_y: index == 0 || index == elements,
                fix_rz: false,
            })
            .collect(),
        elements: (0..elements)
            .map(|index| BucklingBeam1dElementInput {
                id: format!("e{index}"),
                node_i: index,
                node_j: index + 1,
                youngs_modulus: 205.0e9,
                moment_of_inertia: 7.4e-6,
                reference_compressive_force: 100_000.0,
            })
            .collect(),
        mode_count: Some(3),
    }
}

fn frame(elements: usize) -> SolveBucklingFrame2dRequest {
    let reference = beam(elements);
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest {
            nodes: reference
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| Frame2dNodeInput {
                    id: node.id.clone(),
                    x: node.x,
                    y: 0.0,
                    fix_x: index == 0,
                    fix_y: node.fix_y,
                    fix_rz: false,
                    load_x: if index == elements { -100_000.0 } else { 0.0 },
                    load_y: 0.0,
                    moment_z: 0.0,
                })
                .collect(),
            elements: reference
                .elements
                .iter()
                .map(|element| Frame2dElementInput {
                    id: element.id.clone(),
                    node_i: element.node_i,
                    node_j: element.node_j,
                    area: 0.01,
                    youngs_modulus: element.youngs_modulus,
                    moment_of_inertia: element.moment_of_inertia,
                    section_modulus: 1.0e-4,
                })
                .collect(),
        },
        mode_count: Some(3),
    }
}

fn relative(actual: f64, expected: f64, tolerance: f64) {
    assert!(actual.is_finite() && expected.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < tolerance,
        "{actual} vs {expected}"
    );
}

#[test]
fn reversing_individual_beam_elements_preserves_critical_loads_and_shapes() {
    let input = beam(8);
    let baseline = solve_buckling_beam_1d(&input).unwrap();
    for reversed in [vec![2], vec![1, 3, 5, 7], (0..8).collect()] {
        let mut changed = input.clone();
        for index in reversed {
            let element = &mut changed.elements[index];
            std::mem::swap(&mut element.node_i, &mut element.node_j);
        }
        let result = solve_buckling_beam_1d(&changed).unwrap();
        for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
            relative(actual.load_factor, expected.load_factor, 1.0e-9);
            let alignment = actual
                .shape
                .iter()
                .zip(&expected.shape)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .abs();
            relative(alignment, 1.0, 1.0e-9);
        }
    }
}

#[test]
fn scaling_frame_stiffness_and_preload_together_preserves_compression_and_factors() {
    let input = frame(8);
    let baseline = solve_buckling_frame_2d(&input).unwrap();
    for scale in [1.0e-20, 1.0e20] {
        let mut changed = input.clone();
        for node in &mut changed.frame.nodes {
            node.load_x *= scale;
        }
        for element in &mut changed.frame.elements {
            element.youngs_modulus *= scale;
        }
        let result = solve_buckling_frame_2d(&changed).unwrap();
        for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
            relative(actual.load_factor, expected.load_factor, 1.0e-8);
        }
        for (actual, expected) in result
            .element_preloads
            .iter()
            .zip(&baseline.element_preloads)
        {
            assert!(actual.active_in_geometric_stiffness);
            relative(
                actual.reference_compressive_force / scale,
                expected.reference_compressive_force,
                1.0e-10,
            );
        }
    }
}

#[test]
fn heterogeneous_beam_is_invariant_to_node_element_and_endpoint_order() {
    let mut input = beam(8);
    for (index, element) in input.elements.iter_mut().enumerate() {
        element.moment_of_inertia *= 1.0 + index as f64 * 0.2;
        element.reference_compressive_force *= 1.0 + index as f64 * 0.05;
    }
    let baseline = solve_buckling_beam_1d(&input).unwrap();
    let last = input.nodes.len() - 1;
    input.nodes.reverse();
    input.elements.reverse();
    for (index, element) in input.elements.iter_mut().enumerate() {
        element.node_i = last - element.node_i;
        element.node_j = last - element.node_j;
        if index % 2 == 0 {
            std::mem::swap(&mut element.node_i, &mut element.node_j);
        }
    }
    let result = solve_buckling_beam_1d(&input).unwrap();
    for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
        relative(actual.load_factor, expected.load_factor, 1e-9);
        let alignment = (0..=last)
            .flat_map(|node| (0..2).map(move |dof| (node, dof)))
            .map(|(node, dof)| {
                actual.shape[2 * (last - node) + dof] * expected.shape[2 * node + dof]
            })
            .sum::<f64>()
            .abs();
        relative(alignment, 1.0, 1e-9);
    }
}

#[test]
fn reflecting_beam_coordinates_reverses_rotation_but_not_the_critical_load() {
    let mut input = beam(8);
    let baseline = solve_buckling_beam_1d(&input).unwrap();
    for node in &mut input.nodes {
        node.x = -node.x;
    }
    let result = solve_buckling_beam_1d(&input).unwrap();
    for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
        relative(actual.load_factor, expected.load_factor, 1e-9);
        let alignment = actual
            .shape
            .iter()
            .zip(&expected.shape)
            .enumerate()
            .map(|(dof, (a, b))| a * b * if dof % 2 == 0 { 1.0 } else { -1.0 })
            .sum::<f64>()
            .abs();
        relative(alignment, 1.0, 1e-9);
    }
}

#[test]
fn sparse_mixed_orientation_beam_retains_the_euler_limit() {
    let mut input = beam(280);
    input.mode_count = Some(1);
    for element in input.elements.iter_mut().step_by(2) {
        std::mem::swap(&mut element.node_i, &mut element.node_j);
    }
    let result = solve_buckling_beam_1d(&input).unwrap();
    assert!(result.free_dofs.len() > 512);
    let critical = std::f64::consts::PI.powi(2) * 205e9 * 7.4e-6 / 3.2_f64.powi(2);
    relative(result.minimum_load_factor * 100_000.0, critical, 1e-6);
    relative(
        result.modes[0]
            .shape
            .iter()
            .map(|value| value * value)
            .sum::<f64>(),
        1.0,
        1e-12,
    );
}

#[test]
fn oblique_frame_with_reversed_members_preserves_cantilever_modes_and_preload() {
    let mut input = frame(8);
    input.frame.nodes[0].fix_rz = true;
    input.frame.nodes[8].fix_y = false;
    let baseline = solve_buckling_frame_2d(&input).unwrap();
    let (s, c) = 0.63_f64.sin_cos();
    for node in &mut input.frame.nodes {
        node.y = s * node.x;
        node.x *= c;
        node.load_y = s * node.load_x;
        node.load_x *= c;
    }
    for element in input.frame.elements.iter_mut().step_by(2) {
        std::mem::swap(&mut element.node_i, &mut element.node_j);
    }
    let result = solve_buckling_frame_2d(&input).unwrap();
    for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
        relative(actual.load_factor, expected.load_factor, 1e-8);
        let alignment = actual
            .shape
            .chunks_exact(3)
            .zip(expected.shape.chunks_exact(3))
            .map(|(a, b)| a[0] * (c * b[0] - s * b[1]) + a[1] * (s * b[0] + c * b[1]) + a[2] * b[2])
            .sum::<f64>()
            .abs();
        relative(alignment, 1.0, 1e-8);
    }
    for preload in &result.element_preloads {
        assert!(preload.active_in_geometric_stiffness);
        relative(preload.reference_compressive_force, 100_000.0, 1e-10);
    }
}

#[test]
fn scaling_only_frame_preload_changes_the_factor_not_the_critical_force() {
    let input = frame(8);
    let baseline = solve_buckling_frame_2d(&input).unwrap();
    for scale in [1e-20, 1e20] {
        let mut scaled = input.clone();
        for node in &mut scaled.frame.nodes {
            node.load_x *= scale;
        }
        let result = solve_buckling_frame_2d(&scaled).unwrap();
        for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
            relative(actual.load_factor * scale, expected.load_factor, 1e-8);
        }
    }
}

#[test]
fn a_strongly_loaded_component_does_not_hide_an_independent_tiny_compression() {
    let mut input = frame(4);
    let baseline = solve_buckling_frame_2d(&input).unwrap();
    let mut weak = input.frame.clone();
    let node_offset = input.frame.nodes.len();
    for node in &mut weak.nodes {
        node.id = format!("weak-{}", node.id);
        node.y = 2.0;
        node.load_x *= 1e-20;
    }
    for element in &mut weak.elements {
        element.id = format!("weak-{}", element.id);
        element.node_i += node_offset;
        element.node_j += node_offset;
    }
    input.frame.nodes.extend(weak.nodes);
    input.frame.elements.extend(weak.elements);
    let result = solve_buckling_frame_2d(&input).unwrap();
    for preload in &result.element_preloads {
        assert!(preload.active_in_geometric_stiffness, "{}", preload.id);
        let expected = if preload.id.starts_with("weak-") {
            1e-15
        } else {
            100_000.0
        };
        relative(preload.reference_compressive_force, expected, 1e-10);
    }
    for (actual, expected) in result.modes.iter().zip(&baseline.modes) {
        relative(actual.load_factor, expected.load_factor, 1e-8);
    }
}

#[test]
fn rigid_translation_and_disconnected_free_nodes_fail_without_poisoning_replay() {
    let input = beam(8);
    let mut rotations_only = input.clone();
    for node in &mut rotations_only.nodes {
        node.fix_rz = node.fix_y;
        node.fix_y = false;
    }
    assert!(solve_buckling_beam_1d(&rotations_only).is_err());
    let mut orphan = input.clone();
    orphan.nodes.push(BucklingBeam1dNodeInput {
        id: "unconnected".into(),
        x: 10.0,
        fix_y: false,
        fix_rz: false,
    });
    assert!(solve_buckling_beam_1d(&orphan).is_err());
    let baseline = solve_buckling_beam_1d(&input).unwrap();
    let replay = solve_buckling_beam_1d(&input).unwrap();
    assert_eq!(baseline.modes, replay.modes);
}

#[test]
fn zero_and_tensile_preloads_remain_invalid_at_small_scales() {
    for force in [0.0, 1e-15, 100_000.0] {
        let mut input = frame(4);
        input.frame.nodes[4].load_x = force;
        let error = solve_buckling_frame_2d(&input).unwrap_err();
        assert!(error.contains("no compressive member force"), "{error}");
    }
    assert!(solve_buckling_frame_2d(&frame(4)).is_ok());
}

#[test]
fn nonfinite_element_assembly_is_an_indexed_error_before_eigensolving() {
    let mut input = beam(4);
    input.elements[2].youngs_modulus = 1e308;
    input.elements[2].moment_of_inertia = 2.0;
    let error = solve_buckling_beam_1d(&input).unwrap_err();
    assert!(
        error.contains("element 2") && error.contains("finite"),
        "{error}"
    );
    assert!(solve_buckling_beam_1d(&beam(4)).is_ok());
}

#[test]
fn assembly_eigensolve_and_mode_recovery_cancel_inside_both_operators() {
    for is_frame in [false, true] {
        let solve = || {
            if is_frame {
                solve_buckling_frame_2d(&frame(8))
                    .map(|result| serde_json::to_value(result).unwrap())
            } else {
                solve_buckling_beam_1d(&beam(8)).map(|result| serde_json::to_value(result).unwrap())
            }
        };
        let baseline = solve().unwrap();
        for stage in [
            SolverStage::ElementAssembly,
            SolverStage::ModalSweep,
            SolverStage::ResultFreeDofs,
            SolverStage::ResultTotals,
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps > 0 {
                        cancel.request_cancel();
                    }
                },
                || {
                    let result = solve();
                    assert!(
                        result.is_err(),
                        "frame={is_frame}, ignored cancellation at {stage:?}"
                    );
                    result
                },
            )
            .unwrap_err();
            assert!(error.contains("cancelled"), "{error}");
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            assert_eq!(solve().unwrap(), baseline);
        }
    }
}
