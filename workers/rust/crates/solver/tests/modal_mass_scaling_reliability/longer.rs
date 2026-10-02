use super::*;

const LONG_SEGMENTS: usize = 80;

// Independent 80- and 120-digit banded LDL inertia bisection of K-lambda*M.
const ROOTS: [(usize, f64); 9] = [
    (0, 3.017539808750544e-7),
    (1, 1.1842589597842897e-5),
    (2, 9.276402725565059e-5),
    (3, 3.557971963900987e-4),
    (4, 9.708758245053636e-4),
    (5, 2.1628755568757146e-3),
    (9, 1.9074331222057785e-2),
    (19, 3.252305299727169e-1),
    (159, 1.5853323061113574e2),
];

fn check(result: &SolveModalFrame2dResult, length: f64, count: usize) {
    complete::check_spectrum_for_segments(result, length, count, LONG_SEGMENTS, &ROOTS);
}

#[test]
fn longer_bending_chains_resolve_low_higher_and_complete_modes() {
    for count in [6, 20, 2 * LONG_SEGMENTS] {
        let result = solve_modal_frame_2d(&bending_chain_with_segments(LONG_SEGMENTS, 1.0, count))
            .unwrap_or_else(|error| panic!("modes={count}: {error}"));
        check(&result, 1.0, count);
    }
}

#[test]
fn longer_bending_modes_preserve_coordinate_and_member_order() {
    for length in [1.0, 1e14, 1e-10] {
        let mut input = bending_chain_with_segments(LONG_SEGMENTS, length, 20);
        input.nodes.reverse();
        for element in &mut input.elements {
            let (i, j) = (element.node_i, element.node_j);
            element.node_i = LONG_SEGMENTS - j;
            element.node_j = LONG_SEGMENTS - i;
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
        check(&result, length, 20);
    }
}

#[test]
fn longer_bending_references_have_independent_inertia_brackets() {
    for (index, root) in ROOTS {
        assert_eq!(
            complete::reference::count_below(LONG_SEGMENTS, root * (1.0 - 1e-6)),
            index
        );
        assert_eq!(
            complete::reference::count_below(LONG_SEGMENTS, root * (1.0 + 1e-6)),
            index + 1
        );
    }
}

#[test]
fn unresolved_longer_spectra_fail_without_partial_modes_and_replay() {
    let error = solve_modal_frame_2d(&bending_chain_with_segments(128, 1.0, 6)).unwrap_err();
    assert!(
        error.contains("refinement did not converge within 4 steps"),
        "{error}"
    );
    check(
        &solve_modal_frame_2d(&bending_chain_with_segments(LONG_SEGMENTS, 1.0, 6)).unwrap(),
        1.0,
        6,
    );
}
