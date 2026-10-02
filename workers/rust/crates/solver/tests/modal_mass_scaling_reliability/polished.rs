use super::*;

// Independent 80/120-digit physical K-lambda*M inertia bisection, not continuum roots.
pub(super) const ROOTS_96: [(usize, f64); 9] = [
    (0, 1.4553085447431156e-7),
    (1, 5.712730198911023e-6),
    (2, 4.47606463734338e-5),
    (3, 1.7174163867530644e-4),
    (4, 4.688419676264506e-4),
    (5, 1.045002999787545e-3),
    (9, 9.241836936192287e-3),
    (19, 1.5949592609683072e-1),
    (191, 1.5853323061113574e2),
];
pub(super) const ROOTS_100: [(usize, f64); 9] = [
    (0, 1.236075027143164e-7),
    (1, 4.852330686025576e-6),
    (2, 3.8021072825043554e-5),
    (3, 1.458920056234989e-4),
    (4, 3.983_054_168_160_999e-4),
    (5, 8.878653418209562e-4),
    (9, 7.856_089_927_861_69e-3),
    (19, 1.3587583669754427e-1),
    (199, 1.5853323061113574e2),
];

#[test]
fn polished_bending_spectra_match_low_high_and_complete_discrete_roots() {
    for (segments, roots) in [(96, ROOTS_96), (100, ROOTS_100)] {
        for count in [6, 20, 2 * segments] {
            let result =
                solve_modal_frame_2d(&bending_chain_with_segments(segments, 1.0, count)).unwrap();
            complete::check_spectrum_for_segments(&result, 1.0, count, segments, &roots);
            published::check_published_modes(&result, segments);
        }
    }
}

#[test]
fn polished_bending_modes_preserve_coordinate_scaling_and_reversed_numbering() {
    for (segments, roots) in [(96, ROOTS_96), (100, ROOTS_100)] {
        for length in [1.0, 1e14, 1e-10] {
            let mut input = bending_chain_with_segments(segments, length, 20);
            input.nodes.reverse();
            for element in &mut input.elements {
                let (i, j) = (element.node_i, element.node_j);
                element.node_i = segments - j;
                element.node_j = segments - i;
            }
            input.elements.reverse();
            let mut result = solve_modal_frame_2d_owned(input).unwrap();
            for mode in &mut result.modes {
                mode.shape = mode
                    .shape
                    .chunks_exact(3)
                    .rev()
                    .flatten()
                    .copied()
                    .collect();
            }
            complete::check_spectrum_for_segments(&result, length, 20, segments, &roots);
            if length == 1.0 {
                published::check_published_modes(&result, segments);
            }
        }
    }
}

#[test]
fn polished_bending_roots_have_independent_physical_inertia_brackets() {
    for (segments, roots) in [(96, ROOTS_96), (100, ROOTS_100)] {
        for (index, root) in roots {
            assert_eq!(
                complete::reference::count_below(segments, root * (1.0 - 1e-6)),
                index
            );
            assert_eq!(
                complete::reference::count_below(segments, root * (1.0 + 1e-6)),
                index + 1
            );
        }
    }
}

#[test]
fn polished_spatial_bending_preserves_repeated_modes_and_mass_orthogonality() {
    let roots: Vec<_> = ROOTS_100.iter().take(3).map(|(_, value)| *value).collect();
    for length in [1.0, 1e14, 1e-10] {
        complete::spatial::check_spatial_bending(100, length, &roots);
    }
}

#[test]
fn polished_bending_reuses_one_inverse_and_cancels_inside_the_polish_product() {
    let factors = Rc::new(Cell::new(0));
    let observed = factors.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || solve_modal_frame_2d(&bending_chain_with_segments(100, 1.0, 20)),
    )
    .unwrap();
    assert_eq!(factors.get(), 1);
    let started = Rc::new(Cell::new(false));
    let observed = started.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalValidation && point.completed_steps == 0 {
                observed.set(true);
            }
            if observed.get()
                && point.stage == SolverStage::SparseMatvec
                && point.completed_steps == 64
            {
                cancel.request_cancel();
            }
        },
        || {
            let result = solve_modal_frame_2d(&bending_chain_with_segments(100, 1.0, 20));
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(started.get());
    assert!(error.contains("cancel"));
    let replay = solve_modal_frame_2d(&bending_chain_with_segments(100, 1.0, 20)).unwrap();
    complete::check_spectrum_for_segments(&replay, 1.0, 20, 100, &ROOTS_100);
}
