use super::*;
use kyuubiki_protocol::SolveModalFrame3dResult;
use kyuubiki_solver::solve_modal_frame_3d;

#[test]
fn published_planar_modes_preserve_bits_and_residual_after_json_round_trip() {
    for (segments, roots) in [(96, polished::ROOTS_96), (100, polished::ROOTS_100)] {
        for count in [6, 20, 2 * segments] {
            let model = bending_chain_with_segments(segments, 1.0, count);
            let encoded_model = serde_json::to_vec(&model).unwrap();
            let decoded_model = serde_json::from_slice(&encoded_model).unwrap();
            assert_eq!(model, decoded_model);
            let original = solve_modal_frame_2d(&decoded_model).unwrap();
            let encoded = serde_json::to_vec(&original).unwrap();
            let decoded: SolveModalFrame2dResult = serde_json::from_slice(&encoded).unwrap();
            assert!(decoded == original, "JSON changed planar result fields");
            for (mode, expected_mode) in decoded.modes.iter().zip(&original.modes) {
                assert_eq!(
                    mode.eigenvalue_rad_s_squared.to_bits(),
                    expected_mode.eigenvalue_rad_s_squared.to_bits()
                );
                for (actual, expected) in mode.shape.iter().zip(&expected_mode.shape) {
                    assert_eq!(actual.to_bits(), expected.to_bits(), "mode {}", mode.index);
                }
            }
            complete::check_spectrum_for_segments(&decoded, 1.0, count, segments, &roots);
            published::check_published_modes(&decoded, segments);
        }
    }
}

#[test]
fn published_spatial_modes_preserve_repeated_pairs_after_json_round_trip() {
    for segments in [66, 100] {
        let model = complete::spatial::model(segments, 1.0);
        let original = solve_modal_frame_3d(&model).unwrap();
        let encoded = serde_json::to_vec(&original).unwrap();
        let decoded: SolveModalFrame3dResult = serde_json::from_slice(&encoded).unwrap();
        assert!(decoded == original, "JSON changed spatial result fields");
        for (mode, expected) in decoded.modes.iter().zip(&original.modes) {
            assert_eq!(
                mode.eigenvalue_rad_s_squared.to_bits(),
                expected.eigenvalue_rad_s_squared.to_bits()
            );
            for (actual, expected) in mode.shape.iter().zip(&expected.shape) {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
            let relative = published_reference::unit_bending_residual(
                segments,
                mode.eigenvalue_rad_s_squared,
                &mode.shape,
                true,
            );
            assert!(
                relative <= 1e-8,
                "spatial JSON mode {}: {relative:e}",
                mode.index
            );
        }
    }
}
