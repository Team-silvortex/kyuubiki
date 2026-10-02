use super::*;
use kyuubiki_protocol::SolveModalFrame3dRequest;
use kyuubiki_solver::solve_modal_frame_3d;

pub(crate) fn model(segments: usize, length: f64) -> SolveModalFrame3dRequest {
    let mut value = serde_json::to_value(bending_chain_with_segments(segments, length, 6)).unwrap();
    for node in value["nodes"].as_array_mut().unwrap() {
        node["z"] = json!(0.0);
        node["fix_z"] = node["fix_y"].clone();
        node["fix_ry"] = node["fix_rz"].clone();
        node["fix_rx"] = json!(true);
        for key in ["load_z", "moment_x", "moment_y"] {
            node[key] = json!(0.0);
        }
    }
    for element in value["elements"].as_array_mut().unwrap() {
        element["shear_modulus"] = json!(0.4 * length.powi(3));
        for key in [
            "moment_of_inertia_y",
            "moment_of_inertia_z",
            "torsion_constant",
        ] {
            element[key] = json!(1.0);
        }
        element.as_object_mut().unwrap().remove("moment_of_inertia");
        element.as_object_mut().unwrap().remove("section_modulus");
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn spatial_slender_complete_modes_retain_repeated_bending_directions_and_mass_orthogonality() {
    for length in [1.0, 1e14, 1e-10] {
        check_spatial_bending(SEGMENTS, length, &ROOTS);
    }
}

pub(crate) fn check_spatial_bending(segments: usize, length: f64, roots: &[f64]) {
    let result = solve_modal_frame_3d(&model(segments, length)).unwrap();
    assert_eq!(result.free_dofs.len(), 4 * segments);
    assert_eq!(result.modes.len(), 6);
    assert!(
        result
            .modes
            .windows(2)
            .all(|pair| pair[0].eigenvalue_rad_s_squared <= pair[1].eigenvalue_rad_s_squared)
    );
    let mut directions = Vec::new();
    for (index, mode) in result.modes.iter().enumerate() {
        if length == 1.0 {
            let relative = super::super::published_reference::unit_bending_residual(
                segments,
                mode.eigenvalue_rad_s_squared,
                &mode.shape,
                true,
            );
            assert!(
                relative <= 1e-8,
                "spatial published mode {index}: {relative:e}"
            );
        }
        assert!((mode.eigenvalue_rad_s_squared / roots[index / 2] - 1.0).abs() < 2e-8);
        assert_eq!(mode.shape.len(), 6 * (segments + 1));
        assert!((mode.shape.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 1e-10);
        let mut vector = Vec::new();
        for (node, values) in mode.shape.chunks_exact(6).enumerate() {
            assert!(values.iter().all(|v| v.is_finite()));
            assert_eq!(values[0], 0.0);
            assert_eq!(values[3], 0.0);
            if node == 0 {
                assert!(values.iter().all(|v| *v == 0.0));
                continue;
            }
            let mass: f64 = if node == segments { 0.5 } else { 1.0 };
            for component in [1, 2] {
                vector.push(values[component] * mass.sqrt());
            }
            for component in [4, 5] {
                vector.push((values[component] * length) * (mass / 12.0).sqrt());
            }
        }
        let norm = vector.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(norm.is_finite() && norm > 0.0);
        vector.iter_mut().for_each(|v| *v /= norm);
        directions.push(vector);
    }
    for (i, vector) in directions.iter().enumerate() {
        for other in &directions[..i] {
            assert!(
                vector
                    .iter()
                    .zip(other)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    .abs()
                    < 1e-10
            );
        }
    }
}
