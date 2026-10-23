use kyuubiki_protocol::{
    MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput, SolveMagnetostaticBar1dRequest,
};
use kyuubiki_solver::{solve_magnetostatic_bar_1d, solve_magnetostatic_bar_1d_owned};

#[test]
fn magnetic_prescribed_cancellation_retains_small_response_on_paths_and_junctions() {
    let delta = 2.0_f64.powi(-27);
    for power in [-600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        for sign in [-1.0, 1.0] {
            for arms in [1, 4] {
                for reverse in [false, true] {
                    let request = cancellation_model(scale, sign, arms, reverse);
                    let expected = sign * delta * delta / (arms as f64 + delta);
                    let borrowed = solve_magnetostatic_bar_1d(&request).unwrap();
                    let owned = solve_magnetostatic_bar_1d_owned(request).unwrap();
                    assert_eq!(
                        serde_json::to_value(&borrowed).unwrap(),
                        serde_json::to_value(owned).unwrap()
                    );
                    let actual = borrowed.nodes[0].magnetic_potential;
                    assert!(
                        (actual / expected - 1.0).abs() < 1.0e-12,
                        "{actual:e} != {expected:e}"
                    );
                }
            }
        }
    }
}

fn cancellation_model(
    scale: f64,
    sign: f64,
    arms: usize,
    reverse: bool,
) -> SolveMagnetostaticBar1dRequest {
    let delta = 2.0_f64.powi(-27);
    SolveMagnetostaticBar1dRequest {
        nodes: (0..=arms)
            .map(|index| MagnetostaticBar1dNodeInput {
                id: format!("n{index}"),
                x: index as f64,
                fix_magnetic_potential: index != 0,
                magnetic_potential: if index == 1 {
                    -sign * (1.0 - delta)
                } else {
                    0.0
                },
                magnetomotive_source: if index == 0 { sign * scale } else { 0.0 },
            })
            .collect(),
        elements: (1..=arms)
            .map(|index| MagnetostaticBar1dElementInput {
                id: format!("e{index}"),
                node_i: if reverse { index } else { 0 },
                node_j: if reverse { 0 } else { index },
                area: 1.0,
                permeability: if index == 1 {
                    scale * (1.0 + delta)
                } else {
                    scale * index as f64
                },
            })
            .collect(),
    }
}

#[test]
fn magnetic_sparse_boundary_preserves_constant_subnormal_potential_on_a_four_arm_junction() {
    for bits in [1, 3, 7] {
        for sign in [-1.0, 1.0] {
            for reverse in [false, true] {
                let value = sign * f64::from_bits(bits);
                let request = SolveMagnetostaticBar1dRequest {
                    nodes: (0..=4)
                        .map(|index| MagnetostaticBar1dNodeInput {
                            id: format!("n{index}"),
                            x: index as f64,
                            fix_magnetic_potential: index != 0,
                            magnetic_potential: if index == 0 { 0.0 } else { value },
                            magnetomotive_source: 0.0,
                        })
                        .collect(),
                    elements: (1..=4)
                        .map(|index| MagnetostaticBar1dElementInput {
                            id: format!("e{index}"),
                            node_i: if reverse { index } else { 0 },
                            node_j: if reverse { 0 } else { index },
                            area: 1.0,
                            permeability: 0.25 * index as f64,
                        })
                        .collect(),
                };
                verify_constant_field(request, value);
            }
        }
    }
}

#[test]
fn magnetic_boundary_range_keeps_constant_subnormal_potential_and_zero_field() {
    for bits in [1, 3, 7] {
        for sign in [-1.0, 1.0] {
            for reverse in [false, true] {
                let value = sign * f64::from_bits(bits);
                verify_constant_field(model(value, 0.5, 3, reverse), value);
            }
        }
    }
}

#[test]
fn magnetic_boundary_range_recovers_an_overflowing_load_with_finite_potential() {
    for sign in [-1.0, 1.0] {
        for reverse in [false, true] {
            let value = sign * 2.0_f64.powi(1022);
            verify_constant_field(model(value, 8.0, 2, reverse), value);
        }
    }
}

fn verify_constant_field(request: SolveMagnetostaticBar1dRequest, value: f64) {
    let result = solve_magnetostatic_bar_1d(&request).unwrap();
    let owned = solve_magnetostatic_bar_1d_owned(request).unwrap();
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        serde_json::to_value(owned).unwrap()
    );
    assert!(
        result
            .nodes
            .iter()
            .all(|node| node.magnetic_potential == value)
    );
    assert_eq!(result.max_magnetic_potential, value.abs());
    for element in &result.elements {
        assert_eq!(element.average_magnetic_potential, value);
        assert_eq!(element.magnetic_potential_gradient, 0.0);
        assert_eq!(element.magnetic_field_strength, 0.0);
        assert_eq!(element.magnetic_flux_density, 0.0);
        assert_eq!(element.stored_energy, 0.0);
    }
    assert_eq!(result.total_stored_energy, 0.0);
}

fn model(
    value: f64,
    permeability: f64,
    count: usize,
    reverse: bool,
) -> SolveMagnetostaticBar1dRequest {
    SolveMagnetostaticBar1dRequest {
        nodes: (0..count)
            .map(|index| MagnetostaticBar1dNodeInput {
                id: format!("n{index}"),
                x: index as f64,
                fix_magnetic_potential: index == 0 || (count == 3 && index == 2),
                magnetic_potential: if index == 0 || index == 2 { value } else { 0.0 },
                magnetomotive_source: 0.0,
            })
            .collect(),
        elements: (1..count)
            .map(|index| MagnetostaticBar1dElementInput {
                id: format!("e{index}"),
                node_i: if reverse { index } else { index - 1 },
                node_j: if reverse { index - 1 } else { index },
                area: 1.0,
                permeability,
            })
            .collect(),
    }
}
