use kyuubiki_protocol::{
    BucklingBeam1dElementInput, BucklingBeam1dNodeInput, BucklingModeDirectionAssessment,
    SolveBucklingBeam1dRequest,
};
use kyuubiki_solver::solve_buckling_beam_1d;

const ELEMENTS_PER_COLUMN: usize = 300;
const LENGTH: f64 = 3.2;
const YOUNGS_MODULUS: f64 = 205.0e9;
const INERTIA: f64 = 7.4e-6;
const REFERENCE_FORCE: f64 = 100_000.0;

#[test]
fn sparse_single_mode_request_resolves_nearly_repeated_beam_factors() {
    let request = paired_column_request();
    let result = solve_buckling_beam_1d(&request)
        .expect("oversampled single-mode solve should resolve the softer column");
    let expected_factor = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * INERTIA
        / (LENGTH.powi(2) * REFERENCE_FORCE);

    assert!(result.free_dofs.len() > 512);
    assert_eq!(result.modes.len(), 1);
    assert_relative(result.minimum_load_factor, expected_factor, 1.0e-6);
    assert_eq!(
        result.modes[0].direction_assessment,
        BucklingModeDirectionAssessment::Unassessed
    );
    assert_eq!(result.modes[0].relative_gap_to_next, None);

    let split = (ELEMENTS_PER_COLUMN + 1) * 2;
    let softer_norm = l2_norm(&result.modes[0].shape[..split]);
    let stiffer_norm = l2_norm(&result.modes[0].shape[split..]);
    assert!(softer_norm > 0.99, "softer column mode norm={softer_norm}");
    assert!(
        stiffer_norm < 0.05,
        "stiffer column mode norm={stiffer_norm}"
    );
    assert!(result.modes[0].residual_norm.is_finite());
}

fn paired_column_request() -> SolveBucklingBeam1dRequest {
    let mut nodes = Vec::with_capacity((ELEMENTS_PER_COLUMN + 1) * 2);
    let mut elements = Vec::with_capacity(ELEMENTS_PER_COLUMN * 2);
    append_column(0, 1.0, &mut nodes, &mut elements);
    append_column(1, 1.000_001, &mut nodes, &mut elements);
    SolveBucklingBeam1dRequest {
        nodes,
        elements,
        mode_count: Some(1),
    }
}

fn append_column(
    column: usize,
    stiffness_scale: f64,
    nodes: &mut Vec<BucklingBeam1dNodeInput>,
    elements: &mut Vec<BucklingBeam1dElementInput>,
) {
    let node_offset = nodes.len();
    let segment = LENGTH / ELEMENTS_PER_COLUMN as f64;
    for index in 0..=ELEMENTS_PER_COLUMN {
        nodes.push(BucklingBeam1dNodeInput {
            id: format!("c{column}-n{index}"),
            x: index as f64 * segment,
            fix_y: index == 0 || index == ELEMENTS_PER_COLUMN,
            fix_rz: false,
        });
    }
    for index in 0..ELEMENTS_PER_COLUMN {
        elements.push(BucklingBeam1dElementInput {
            id: format!("c{column}-e{index}"),
            node_i: node_offset + index,
            node_j: node_offset + index + 1,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: INERTIA * stiffness_scale,
            reference_compressive_force: REFERENCE_FORCE,
        });
    }
}

fn l2_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
