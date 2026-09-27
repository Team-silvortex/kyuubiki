use super::*;

#[test]
fn near_parallel_section_hint_still_builds_a_right_handed_orthonormal_triad() {
    for delta in [[1.0, 1.0, 0.0], [1.0, 2.0, 3.0], [-4.0, 2.0, -1.0]] {
        for epsilon in [1e-3, 1e-6, 1e-9, 1e-11] {
            let axis = [delta[0] + epsilon, delta[1] - epsilon, delta[2]];
            let r = frame3d_rotation_with_local_y(
                delta[0],
                delta[1],
                delta[2],
                norm3(delta),
                Some(axis),
            )
            .unwrap();
            for i in 0..3 {
                for j in 0..3 {
                    let expected = if i == j { 1.0 } else { 0.0 };
                    let actual = dot3(r[i], r[j]);
                    assert!(
                        (actual - expected).abs() < 8e-15,
                        "delta={delta:?} epsilon={epsilon:e}, R*R^T[{i},{j}]={actual:e}"
                    );
                }
            }
            assert!((dot3(cross3(r[0], r[1]), r[2]) - 1.0).abs() < 8e-15);
        }
    }
}

#[test]
fn section_hint_cannot_give_an_infinitesimal_rigid_rotation_elastic_force() {
    let delta = [1.0, 2.0, 3.0];
    let omega = [0.3, -0.2, 0.1];
    let tip_translation = cross3(omega, delta);
    let length = norm3(delta);
    let stiffness = frame3d_local_stiffness(2.0, 1000.0, 400.0, 0.5, 2.0, 1.0, length);
    let mut rigid_motion = [0.0; 12];
    rigid_motion[3..6].copy_from_slice(&omega);
    rigid_motion[6..9].copy_from_slice(&tip_translation);
    rigid_motion[9..12].copy_from_slice(&omega);
    for epsilon in [1e-3, 1e-6, 1e-9, 1e-11] {
        let rotation = frame3d_rotation_with_local_y(
            1.0,
            2.0,
            3.0,
            length,
            Some([1.0 + epsilon, 2.0 - epsilon, 3.0]),
        )
        .unwrap();
        let local = multiply_matrix_vector_12x12(&frame3d_transform(&rotation), &rigid_motion);
        let forces = multiply_matrix_vector_12x12(&stiffness, &local);
        for force in forces {
            assert!(
                force.abs() < 1e-10,
                "epsilon={epsilon:e}, spurious rigid-mode force={force:e}"
            );
        }
    }
}

#[test]
fn default_section_axes_keep_the_reference_convention_at_poles_and_branch_boundary() {
    for (delta, expected) in [
        (
            [1.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        ),
        (
            [0.0, 0.0, 1.0],
            [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        ),
        (
            [0.0, 0.0, -1.0],
            [[0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        ),
    ] {
        assert_eq!(
            frame3d_rotation(delta[0], delta[1], delta[2], 1.0).unwrap(),
            expected
        );
    }
    for z in [
        0.9 - 1e-12_f64,
        0.9,
        0.9 + 1e-12,
        -0.9 - 1e-12,
        -0.9,
        -0.9 + 1e-12,
    ] {
        let x = (1.0 - z * z).sqrt();
        for scale in [1e-10, 1.0, 1e150] {
            let length = (x * scale).hypot(z * scale);
            let r = frame3d_rotation(x * scale, 0.0, z * scale, length).unwrap();
            for i in 0..3 {
                for j in 0..3 {
                    let expected = if i == j { 1.0 } else { 0.0 };
                    assert!((dot3(r[i], r[j]) - expected).abs() < 8e-15);
                }
            }
            // Retain the original dz/length branch, including floating-point boundary ties.
            if (z * scale / length).abs() < 0.9 {
                assert!(r[1][1] > 1.0 - 1e-15, "z={z}, scale={scale}, r={r:?}");
            } else {
                assert!(r[1][1].abs() < 1e-15, "z={z}, scale={scale}, r={r:?}");
            }
        }
    }
}

#[test]
fn normalized_parallelism_floor_remains_independent_of_hint_magnitude() {
    for scale in [1e-10, 1.0, 1e200, 1e308] {
        for epsilon in [0.0, 1e-13] {
            let result = frame3d_rotation_with_local_y(
                1.0,
                1.0,
                0.0,
                2.0_f64.sqrt(),
                Some([scale, scale * (1.0 + epsilon), 0.0]),
            );
            assert!(
                result
                    .unwrap_err()
                    .contains("local_y_axis must not be parallel")
            );
        }
        let r = frame3d_rotation_with_local_y(
            1.0,
            1.0,
            0.0,
            2.0_f64.sqrt(),
            Some([scale, scale * (1.0 + 4e-12), 0.0]),
        )
        .unwrap();
        assert!(dot3(r[0], r[1]).abs() < 8e-15);
        assert!(r[1][0] < 0.0 && r[1][1] > 0.0 && r[2][2] > 0.0);
    }
}
