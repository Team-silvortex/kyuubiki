use super::common::assert_close;
use kyuubiki_protocol::{
    Frame2dNodeInput, Frame3dNodeInput, ModalFrame2dElementInput, ModalFrame3dElementInput,
    SolveModalFrame2dRequest, SolveModalFrame3dRequest,
};
use kyuubiki_solver::{solve_modal_frame_2d, solve_modal_frame_3d};

const DENSITY: f64 = 7850.0;
const AREA: f64 = 0.01;
const YOUNGS_MODULUS: f64 = 210.0e9;
const MOMENT_OF_INERTIA: f64 = 8.333e-6;
const SECTION_MODULUS: f64 = 1.667e-4;
const SHEAR_MODULUS: f64 = 80.0e9;
const TORSION_CONSTANT: f64 = 1.0e-5;

#[test]
fn modal_frame_2d_frequency_scaling_tracks_stiffness_and_density() {
    let base =
        solve_modal_frame_2d(&modal_2d_request(1.0, 1.0)).expect("base modal 2d frame solves");

    for case in [
        ModalScaleCase {
            stiffness_factor: 4.0,
            density_factor: 1.0,
        },
        ModalScaleCase {
            stiffness_factor: 1.0,
            density_factor: 2.25,
        },
        ModalScaleCase {
            stiffness_factor: 3.24,
            density_factor: 1.44,
        },
    ] {
        let result = solve_modal_frame_2d(&modal_2d_request(
            case.stiffness_factor,
            case.density_factor,
        ))
        .expect("scaled modal 2d frame solves");
        let frequency_scale = (case.stiffness_factor / case.density_factor).sqrt();

        assert_eq!(result.free_dofs, base.free_dofs);
        assert_close(
            result.total_mass,
            base.total_mass * case.density_factor,
            "modal 2d mass scaling",
        );
        assert_close(
            result.min_frequency_hz,
            base.min_frequency_hz * frequency_scale,
            "modal 2d min frequency scaling",
        );
        assert_close(
            result.max_frequency_hz,
            base.max_frequency_hz * frequency_scale,
            "modal 2d max frequency scaling",
        );
        assert_modal_modes_scale(&base.modes, &result.modes, frequency_scale, 6, "modal 2d");
        assert_strictly_increasing(result.modes.iter().map(|mode| mode.natural_frequency_hz));
    }
}

#[test]
fn modal_frame_3d_frequency_scaling_tracks_stiffness_and_density() {
    let base =
        solve_modal_frame_3d(&modal_3d_request(1.0, 1.0)).expect("base modal 3d frame solves");

    for case in [
        ModalScaleCase {
            stiffness_factor: 4.0,
            density_factor: 1.0,
        },
        ModalScaleCase {
            stiffness_factor: 1.0,
            density_factor: 2.25,
        },
        ModalScaleCase {
            stiffness_factor: 3.24,
            density_factor: 1.44,
        },
    ] {
        let result = solve_modal_frame_3d(&modal_3d_request(
            case.stiffness_factor,
            case.density_factor,
        ))
        .expect("scaled modal 3d frame solves");
        let frequency_scale = (case.stiffness_factor / case.density_factor).sqrt();

        assert_eq!(result.free_dofs, base.free_dofs);
        assert_close(
            result.total_mass,
            base.total_mass * case.density_factor,
            "modal 3d mass scaling",
        );
        assert_close(
            result.min_frequency_hz,
            base.min_frequency_hz * frequency_scale,
            "modal 3d min frequency scaling",
        );
        assert_close(
            result.max_frequency_hz,
            base.max_frequency_hz * frequency_scale,
            "modal 3d max frequency scaling",
        );
        assert_modal_modes_scale(&base.modes, &result.modes, frequency_scale, 12, "modal 3d");
        assert_non_decreasing(result.modes.iter().map(|mode| mode.natural_frequency_hz));
    }
}

#[derive(Clone, Copy)]
struct ModalScaleCase {
    stiffness_factor: f64,
    density_factor: f64,
}

fn modal_2d_request(stiffness_factor: f64, density_factor: f64) -> SolveModalFrame2dRequest {
    SolveModalFrame2dRequest {
        nodes: vec![node_2d("fixed", 0.0, true), node_2d("tip", 2.0, false)],
        elements: vec![ModalFrame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS * stiffness_factor,
            moment_of_inertia: MOMENT_OF_INERTIA,
            section_modulus: SECTION_MODULUS,
            density: DENSITY * density_factor,
        }],
        mode_count: Some(3),
    }
}

fn modal_3d_request(stiffness_factor: f64, density_factor: f64) -> SolveModalFrame3dRequest {
    SolveModalFrame3dRequest {
        nodes: vec![node_3d("fixed", 0.0, true), node_3d("tip", 2.0, false)],
        elements: vec![ModalFrame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS * stiffness_factor,
            shear_modulus: SHEAR_MODULUS * stiffness_factor,
            torsion_constant: TORSION_CONSTANT,
            moment_of_inertia_y: MOMENT_OF_INERTIA,
            moment_of_inertia_z: MOMENT_OF_INERTIA,
            density: DENSITY * density_factor,
        }],
        mode_count: Some(3),
    }
}

fn node_2d(id: &str, x: f64, fixed: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn node_3d(id: &str, x: f64, fixed: bool) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_modal_modes_scale<M>(
    base_modes: &[M],
    modes: &[M],
    frequency_scale: f64,
    expected_shape_len: usize,
    label: &str,
) where
    M: ModalModeView,
{
    assert_eq!(modes.len(), base_modes.len());
    for (base_mode, mode) in base_modes.iter().zip(modes.iter()) {
        assert_close(
            mode.frequency_hz(),
            base_mode.frequency_hz() * frequency_scale,
            &format!("{label} frequency scaling"),
        );
        assert_close(
            mode.frequency_rad_s(),
            base_mode.frequency_rad_s() * frequency_scale,
            &format!("{label} radian frequency scaling"),
        );
        assert_close(
            mode.eigenvalue(),
            base_mode.eigenvalue() * frequency_scale.powi(2),
            &format!("{label} eigenvalue scaling"),
        );
        assert_close(
            mode.period_s(),
            base_mode.period_s() / frequency_scale,
            &format!("{label} period scaling"),
        );
        assert_modal_shape(mode.shape(), expected_shape_len);
        assert_close(mode.participation_norm(), 1.0, "modal participation norm");
    }
}

trait ModalModeView {
    fn eigenvalue(&self) -> f64;
    fn frequency_rad_s(&self) -> f64;
    fn frequency_hz(&self) -> f64;
    fn period_s(&self) -> f64;
    fn participation_norm(&self) -> f64;
    fn shape(&self) -> &[f64];
}

impl ModalModeView for kyuubiki_protocol::ModalFrame2dModeResult {
    fn eigenvalue(&self) -> f64 {
        self.eigenvalue_rad_s_squared
    }

    fn frequency_rad_s(&self) -> f64 {
        self.natural_frequency_rad_s
    }

    fn frequency_hz(&self) -> f64 {
        self.natural_frequency_hz
    }

    fn period_s(&self) -> f64 {
        self.period_s
    }

    fn participation_norm(&self) -> f64 {
        self.participation_norm
    }

    fn shape(&self) -> &[f64] {
        &self.shape
    }
}

impl ModalModeView for kyuubiki_protocol::ModalFrame3dModeResult {
    fn eigenvalue(&self) -> f64 {
        self.eigenvalue_rad_s_squared
    }

    fn frequency_rad_s(&self) -> f64 {
        self.natural_frequency_rad_s
    }

    fn frequency_hz(&self) -> f64 {
        self.natural_frequency_hz
    }

    fn period_s(&self) -> f64 {
        self.period_s
    }

    fn participation_norm(&self) -> f64 {
        self.participation_norm
    }

    fn shape(&self) -> &[f64] {
        &self.shape
    }
}

fn assert_modal_shape(shape: &[f64], expected_len: usize) {
    assert_eq!(shape.len(), expected_len);
    assert!(shape.iter().all(|value| value.is_finite()));
    assert!(shape.iter().any(|value| value.abs() > 1.0e-6));
}

fn assert_strictly_increasing(values: impl Iterator<Item = f64>) {
    let mut previous = 0.0;
    for value in values {
        assert!(
            value > previous,
            "expected {value} to be greater than {previous}",
        );
        previous = value;
    }
}

fn assert_non_decreasing(values: impl Iterator<Item = f64>) {
    let mut previous = 0.0;
    for value in values {
        assert!(
            value >= previous - 1.0e-10,
            "expected {value} to be >= {previous}",
        );
        previous = value;
    }
}
