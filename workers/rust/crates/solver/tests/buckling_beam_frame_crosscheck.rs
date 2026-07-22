use kyuubiki_protocol::{
    BucklingBeam1dElementInput, BucklingBeam1dNodeInput, Frame2dElementInput, Frame2dNodeInput,
    SolveBucklingBeam1dRequest, SolveBucklingFrame2dRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::{solve_buckling_beam_1d, solve_buckling_frame_2d};

const ELEMENTS: usize = 16;
const LENGTH: f64 = 3.2;
const YOUNGS_MODULUS: f64 = 205.0e9;
const INERTIA: f64 = 7.4e-6;
const REFERENCE_FORCE: f64 = 100_000.0;

#[test]
fn beam_and_preloaded_frame_agree_on_the_first_column_buckling_mode() {
    let beam = solve_buckling_beam_1d(&beam_request()).expect("beam column should solve");
    let frame = solve_buckling_frame_2d(&frame_request()).expect("frame column should solve");

    assert_relative(frame.minimum_load_factor, beam.minimum_load_factor, 1.0e-9);
    let beam_shape = &beam.modes[0].shape;
    let mut frame_bending_shape = (0..=ELEMENTS)
        .flat_map(|node| {
            [
                -frame.modes[0].shape[node * 3],
                frame.modes[0].shape[node * 3 + 2],
            ]
        })
        .collect::<Vec<_>>();
    normalize(&mut frame_bending_shape);
    let correlation = beam_shape
        .iter()
        .zip(frame_bending_shape)
        .map(|(left, right)| left * right)
        .sum::<f64>()
        .abs();
    assert!(correlation > 1.0 - 1.0e-9, "correlation={correlation}");
}

fn beam_request() -> SolveBucklingBeam1dRequest {
    let segment = LENGTH / ELEMENTS as f64;
    SolveBucklingBeam1dRequest {
        nodes: (0..=ELEMENTS)
            .map(|index| BucklingBeam1dNodeInput {
                id: format!("bn{index}"),
                x: index as f64 * segment,
                fix_y: index == 0 || index == ELEMENTS,
                fix_rz: false,
            })
            .collect(),
        elements: (0..ELEMENTS)
            .map(|index| BucklingBeam1dElementInput {
                id: format!("be{index}"),
                node_i: index,
                node_j: index + 1,
                youngs_modulus: YOUNGS_MODULUS,
                moment_of_inertia: INERTIA,
                reference_compressive_force: REFERENCE_FORCE,
            })
            .collect(),
        mode_count: Some(1),
    }
}

fn frame_request() -> SolveBucklingFrame2dRequest {
    let segment = LENGTH / ELEMENTS as f64;
    let nodes = (0..=ELEMENTS)
        .map(|index| Frame2dNodeInput {
            id: format!("fn{index}"),
            x: 0.0,
            y: index as f64 * segment,
            fix_x: index == 0 || index == ELEMENTS,
            fix_y: index == 0,
            fix_rz: false,
            load_x: 0.0,
            load_y: if index == ELEMENTS {
                -REFERENCE_FORCE
            } else {
                0.0
            },
            moment_z: 0.0,
        })
        .collect();
    let elements = (0..ELEMENTS)
        .map(|index| Frame2dElementInput {
            id: format!("fe{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: INERTIA,
            section_modulus: 1.0e-4,
        })
        .collect();
    SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest { nodes, elements },
        mode_count: Some(1),
    }
}

fn normalize(values: &mut [f64]) {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    values.iter_mut().for_each(|value| *value /= norm);
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
