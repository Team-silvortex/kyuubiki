use kyuubiki_protocol::{
    BucklingModeDirectionAssessment, Frame2dElementInput, Frame2dNodeInput,
    SolveBucklingFrame2dRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_buckling_frame_2d;

const ELEMENTS_PER_COLUMN: usize = 100;
const LENGTH: f64 = 3.2;
const YOUNGS_MODULUS: f64 = 205.0e9;
const INERTIA: f64 = 7.4e-6;
const REFERENCE_FORCE: f64 = 100_000.0;

#[test]
fn sparse_block_iteration_retains_a_repeated_first_frame_mode() {
    let result = solve_buckling_frame_2d(&twin_column_request())
        .expect("symmetric twin columns should retain both first modes");
    let expected_factor = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * INERTIA
        / (LENGTH.powi(2) * REFERENCE_FORCE);

    assert!(result.free_dofs.len() > 512);
    assert_eq!(result.modes.len(), 2);
    assert_relative(result.modes[0].load_factor, expected_factor, 1.0e-6);
    assert_relative(result.modes[1].load_factor, expected_factor, 1.0e-6);
    assert_relative(
        result.modes[1].load_factor,
        result.modes[0].load_factor,
        1.0e-8,
    );
    assert_eq!(
        result.modes[0].direction_assessment,
        BucklingModeDirectionAssessment::Clustered
    );
    assert_eq!(
        result.modes[1].direction_assessment,
        BucklingModeDirectionAssessment::Clustered
    );
    assert!(result.modes[0].relative_gap_to_next.unwrap() < 1.0e-8);
    assert_eq!(result.modes[1].relative_gap_to_next, None);

    let correlation = result.modes[0]
        .shape
        .iter()
        .zip(&result.modes[1].shape)
        .map(|(left, right)| left * right)
        .sum::<f64>()
        .abs();
    assert!(correlation < 1.0e-8, "mode correlation={correlation}");
    assert!(
        result
            .modes
            .iter()
            .all(|mode| mode.residual_norm.is_finite())
    );
}

fn twin_column_request() -> SolveBucklingFrame2dRequest {
    let mut nodes = Vec::with_capacity((ELEMENTS_PER_COLUMN + 1) * 2);
    let mut elements = Vec::with_capacity(ELEMENTS_PER_COLUMN * 2);
    for column in 0..2 {
        append_column(column, &mut nodes, &mut elements);
    }
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest { nodes, elements },
        mode_count: Some(2),
    }
}

fn append_column(
    column: usize,
    nodes: &mut Vec<Frame2dNodeInput>,
    elements: &mut Vec<Frame2dElementInput>,
) {
    let node_offset = nodes.len();
    let segment = LENGTH / ELEMENTS_PER_COLUMN as f64;
    for index in 0..=ELEMENTS_PER_COLUMN {
        let endpoint = index == 0 || index == ELEMENTS_PER_COLUMN;
        nodes.push(Frame2dNodeInput {
            id: format!("c{column}-n{index}"),
            x: column as f64 * 2.0,
            y: index as f64 * segment,
            fix_x: endpoint,
            fix_y: index == 0,
            fix_rz: false,
            load_x: 0.0,
            load_y: if index == ELEMENTS_PER_COLUMN {
                -REFERENCE_FORCE
            } else {
                0.0
            },
            moment_z: 0.0,
        });
    }
    for index in 0..ELEMENTS_PER_COLUMN {
        elements.push(Frame2dElementInput {
            id: format!("c{column}-e{index}"),
            node_i: node_offset + index,
            node_j: node_offset + index + 1,
            area: 0.01,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: INERTIA,
            section_modulus: 1.0e-4,
        });
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
