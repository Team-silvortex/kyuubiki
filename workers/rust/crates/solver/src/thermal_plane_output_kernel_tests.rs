use super::{build_thermal_plane_nodes, weighted_average};
use kyuubiki_protocol::ThermalPlaneNodeInput;

#[test]
fn thermal_means_preserve_constants_cancellation_and_unequal_weights() {
    for value in [f64::MAX, -f64::MAX, 1.0, 0.0, f64::from_bits(1)] {
        assert_eq!(weighted_average([value; 3], [1.0; 3]), value);
        assert_eq!(weighted_average([value; 4], [0.1, 0.2, 0.3, 0.4]), value);
    }
    for values in [[1e308, 1.0, -1e308], [-1e308, 1e308, 1.0]] {
        assert!((weighted_average(values, [1.0; 3]) - 1.0 / 3.0).abs() < 1e-14);
    }
    assert!(
        (weighted_average([10.0, 20.0, 30.0, 40.0], [0.1, 0.2, 0.3, 0.4]) - 30.0).abs() < 1e-14
    );
}

#[test]
fn thermal_mean_guards_cannot_turn_invalid_fields_into_a_finite_average() {
    for values in [[f64::NAN; 3], [f64::INFINITY; 3], [1.0, f64::NAN, -1.0]] {
        assert!(weighted_average(values, [1.0; 3]).is_nan());
    }
    for weights in [
        [0.0; 3],
        [-1.0, 1.0, 1.0],
        [f64::INFINITY; 3],
        [f64::NAN; 3],
    ] {
        assert!(weighted_average([0.0; 3], weights).is_nan());
    }
}

#[test]
fn thermal_node_recovery_preserves_finite_norms_and_rejects_nonfinite_values() {
    let nodes = [ThermalPlaneNodeInput {
        id: "norm-node".into(),
        x: 0.0,
        y: 0.0,
        fix_x: false,
        fix_y: false,
        load_x: 0.0,
        load_y: 0.0,
        temperature_delta: 0.0,
    }];
    for scale in [1e-200, 1.0, 1e200] {
        let result = build_thermal_plane_nodes(&nodes, &[3.0 * scale, 4.0 * scale]).unwrap();
        assert!((result[0].displacement_magnitude / scale - 5.0).abs() < 1e-14);
    }
    for displacements in [[f64::MAX; 2], [f64::NAN, 1.0], [f64::INFINITY, 1.0]] {
        let error = build_thermal_plane_nodes(&nodes, &displacements).unwrap_err();
        assert!(
            error.contains("norm-node") && error.contains("representable"),
            "{error}"
        );
    }
}
