use super::*;

#[path = "reference.rs"]
pub(super) mod reference;

#[path = "spatial.rs"]
pub(super) mod spatial;

// 80-digit banded LDL inertia bisection of the independently assembled K-lambda*M.
const ROOTS: [f64; 6] = [
    6.513218799205997e-7,
    2.5553097863515013e-5,
    2.0007498276160384e-4,
    7.669658416882003e-4,
    2.0914454533768654e-3,
    4.655568785074286e-3,
];

fn check_spectrum(result: &SolveModalFrame2dResult, length: f64, count: usize) {
    let roots: Vec<_> = ROOTS.iter().copied().enumerate().collect();
    check_spectrum_for_segments(result, length, count, SEGMENTS, &roots);
}

pub(super) fn check_spectrum_for_segments(
    result: &SolveModalFrame2dResult,
    length: f64,
    count: usize,
    segments: usize,
    roots: &[(usize, f64)],
) {
    assert_eq!(result.free_dofs.len(), 2 * segments);
    assert!((result.total_mass / segments as f64 - 1.0).abs() < 1e-12);
    assert_eq!(result.modes.len(), count);
    assert!(
        result
            .modes
            .windows(2)
            .all(|pair| pair[0].eigenvalue_rad_s_squared <= pair[1].eigenvalue_rad_s_squared)
    );
    let mut whitened = Vec::new();
    for (index, mode) in result.modes.iter().enumerate() {
        assert_eq!(mode.index, index);
        assert_eq!(mode.shape.len(), 3 * (segments + 1));
        assert!((mode.participation_norm - 1.0).abs() < 1e-12);
        assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
        assert!((mode.shape.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 1e-10);
        if let Some((_, root)) = roots.iter().find(|(mode, _)| *mode == index) {
            let error = (mode.eigenvalue_rad_s_squared / root - 1.0).abs();
            assert!(
                error < 2e-8,
                "length={length:e}, mode={index}, reference error={error:e}"
            );
        }
        let mut vector = Vec::new();
        for (node, values) in mode.shape.chunks_exact(3).enumerate() {
            assert!(values.iter().all(|v| v.is_finite()));
            assert_eq!(values[0], 0.0);
            if node == 0 {
                assert!(values.iter().all(|v| *v == 0.0));
                continue;
            }
            let mass: f64 = if node == segments { 0.5 } else { 1.0 };
            vector.push(values[1] * mass.sqrt());
            vector.push((values[2] * length) * (mass / 12.0).sqrt());
        }
        let norm = vector.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(norm.is_finite() && norm > 0.0);
        vector.iter_mut().for_each(|v| *v /= norm);
        whitened.push(vector);
    }
    for (i, vector) in whitened.iter().enumerate() {
        for other in &whitened[..i] {
            let dot = vector.iter().zip(other).map(|(a, b)| a * b).sum::<f64>();
            assert!(dot.abs() < 1e-10, "mass orthogonality={dot:e}");
        }
    }
}

#[test]
fn slender_bending_complete_spectrum_passes_the_original_residual_gate() {
    for count in [2, 6, 2 * SEGMENTS] {
        check_spectrum(
            &solve_modal_frame_2d(&bending_chain(1.0, count)).unwrap(),
            1.0,
            count,
        );
    }
    let result = solve_modal_frame_2d(&bending_chain(1.0, 6)).unwrap();
    let single = solve_modal_frame_2d(&bending_chain(1.0, 1)).unwrap();
    assert!(
        (result.modes[0].eigenvalue_rad_s_squared / single.modes[0].eigenvalue_rad_s_squared - 1.0)
            .abs()
            < 1e-7
    );
}

#[test]
fn complete_bending_modes_preserve_mass_coordinate_scaling_and_default_requests() {
    for length in [1e14, 1e-10] {
        let mut input = bending_chain(length, 6);
        input.mode_count = None;
        check_spectrum(&solve_modal_frame_2d_owned(input).unwrap(), length, 6);
    }
}

#[test]
fn complete_bending_modes_preserve_node_member_and_endpoint_order_invariance() {
    let mut input = bending_chain(1.0, 6);
    input.nodes.reverse();
    for element in &mut input.elements {
        let (i, j) = (element.node_i, element.node_j);
        element.node_i = SEGMENTS - j;
        element.node_j = SEGMENTS - i;
    }
    input.elements.reverse();
    let mut result = solve_modal_frame_2d(&input).unwrap();
    for mode in &mut result.modes {
        mode.shape = mode
            .shape
            .chunks_exact(3)
            .rev()
            .flatten()
            .copied()
            .collect();
    }
    check_spectrum(&result, 1.0, 6);
}

#[test]
fn complete_bending_refinement_reuses_one_factor_and_cancels_without_partial_modes() {
    let factors = Rc::new(Cell::new(0));
    let observed = factors.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || solve_modal_frame_2d(&bending_chain(1.0, 6)),
    )
    .unwrap();
    assert_eq!(factors.get(), 1);
    check_spectrum(&result, 1.0, 6);
    for (stage, boundary) in [
        (SolverStage::DenseFactor, 64),
        (SolverStage::DenseSubstitution, 64),
        (SolverStage::ModalIteration, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == boundary {
                    cancel.request_cancel();
                }
            },
            || {
                let result = solve_modal_frame_2d(&bending_chain(1.0, 6));
                assert!(result.is_err());
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        check_spectrum(
            &solve_modal_frame_2d(&bending_chain(1.0, 6)).unwrap(),
            1.0,
            6,
        );
    }
}
