use kyuubiki_protocol::{
    BucklingBeam1dElementInput, BucklingBeam1dNodeInput, SolveBucklingBeam1dRequest,
};
use kyuubiki_solver::solve_buckling_beam_1d;

#[test]
fn pinned_column_converges_to_euler_critical_load() {
    let length: f64 = 3.2;
    let youngs_modulus = 205.0e9;
    let inertia = 7.4e-6;
    let reference_force = 100_000.0;
    let expected = std::f64::consts::PI.powi(2) * youngs_modulus * inertia / length.powi(2);
    let mut errors = Vec::new();

    for elements in [1, 2, 4, 8, 16] {
        let result = solve_buckling_beam_1d(&column_request(
            elements,
            length,
            youngs_modulus,
            inertia,
            reference_force,
        ))
        .expect("pinned Euler column should solve");
        let critical = result.minimum_load_factor * reference_force;
        errors.push((critical - expected).abs());
        assert!(result.modes[0].residual_norm.is_finite());
        assert_eq!(result.modes[0].shape.len(), (elements + 1) * 2);
    }

    assert!(errors.windows(2).all(|pair| pair[1] < pair[0]));
    assert!(
        errors[4] / expected < 1.0e-5,
        "relative error={}",
        errors[4] / expected
    );
}

#[test]
fn buckling_factor_tracks_stiffness_length_and_reference_force() {
    let baseline = solve_buckling_beam_1d(&column_request(8, 2.0, 200.0e9, 5.0e-6, 80_000.0))
        .expect("baseline column should solve")
        .minimum_load_factor;
    let stiffer = solve_buckling_beam_1d(&column_request(8, 2.0, 400.0e9, 5.0e-6, 80_000.0))
        .expect("stiffer column should solve")
        .minimum_load_factor;
    let longer = solve_buckling_beam_1d(&column_request(8, 4.0, 200.0e9, 5.0e-6, 80_000.0))
        .expect("longer column should solve")
        .minimum_load_factor;
    let stronger_reference =
        solve_buckling_beam_1d(&column_request(8, 2.0, 200.0e9, 5.0e-6, 160_000.0))
            .expect("stronger reference load should solve")
            .minimum_load_factor;

    assert_close(stiffer / baseline, 2.0);
    assert_close(longer / baseline, 0.25);
    assert_close(stronger_reference / baseline, 0.5);
}

#[test]
fn multiple_modes_are_sorted_normalized_and_equilibrated() {
    let mut request = column_request(8, 3.0, 210.0e9, 8.0e-6, 100_000.0);
    request.mode_count = Some(3);
    let result = solve_buckling_beam_1d(&request).expect("multi-mode column should solve");

    assert_eq!(result.modes.len(), 3);
    assert!(
        result
            .modes
            .windows(2)
            .all(|pair| pair[0].load_factor < pair[1].load_factor)
    );
    for mode in result.modes {
        let norm = mode
            .shape
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1.0e-10);
        assert!(mode.residual_norm.is_finite());
    }
}

fn column_request(
    element_count: usize,
    length: f64,
    youngs_modulus: f64,
    inertia: f64,
    reference_force: f64,
) -> SolveBucklingBeam1dRequest {
    let segment = length / element_count as f64;
    SolveBucklingBeam1dRequest {
        nodes: (0..=element_count)
            .map(|index| BucklingBeam1dNodeInput {
                id: format!("n{index}"),
                x: index as f64 * segment,
                fix_y: index == 0 || index == element_count,
                fix_rz: false,
            })
            .collect(),
        elements: (0..element_count)
            .map(|index| BucklingBeam1dElementInput {
                id: format!("e{index}"),
                node_i: index,
                node_j: index + 1,
                youngs_modulus,
                moment_of_inertia: inertia,
                reference_compressive_force: reference_force,
            })
            .collect(),
        mode_count: Some(1),
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 2.0e-7 * expected.abs().max(1.0));
}
