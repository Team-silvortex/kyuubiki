use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, SolveBeam1dRequest, SolveThermalBeam1dRequest,
    SolveTorsion1dRequest, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
    Torsion1dElementInput, Torsion1dNodeInput,
};
use kyuubiki_solver::{solve_beam_1d, solve_thermal_beam_1d, solve_torsion_1d};

#[test]
fn beam_1d_rejects_non_finite_node_and_distributed_load_inputs() {
    let mut request = beam_request();
    request.nodes[1].x = f64::NAN;
    assert!(solve_beam_1d(&request).is_err());

    let mut request = beam_request();
    request.nodes[1].load_y = f64::INFINITY;
    assert!(solve_beam_1d(&request).is_err());

    let mut request = beam_request();
    request.nodes[1].moment_z = f64::NEG_INFINITY;
    assert!(solve_beam_1d(&request).is_err());

    let mut request = beam_request();
    request.elements[0].distributed_load_y = f64::NAN;
    assert!(solve_beam_1d(&request).is_err());
}

#[test]
fn thermal_beam_1d_rejects_non_finite_section_properties() {
    let mut request = thermal_beam_request();
    request.elements[0].youngs_modulus = f64::NAN;
    assert!(solve_thermal_beam_1d(&request).is_err());

    let mut request = thermal_beam_request();
    request.elements[0].moment_of_inertia = f64::INFINITY;
    assert!(solve_thermal_beam_1d(&request).is_err());

    let mut request = thermal_beam_request();
    request.elements[0].section_depth = f64::NAN;
    assert!(solve_thermal_beam_1d(&request).is_err());
}

#[test]
fn torsion_1d_rejects_non_finite_node_inputs() {
    let mut request = torsion_request();
    request.nodes[1].x = f64::NAN;
    assert!(solve_torsion_1d(&request).is_err());

    let mut request = torsion_request();
    request.nodes[1].torque_z = f64::INFINITY;
    assert!(solve_torsion_1d(&request).is_err());
}

fn beam_request() -> SolveBeam1dRequest {
    SolveBeam1dRequest {
        nodes: vec![
            Beam1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_y: true,
                fix_rz: true,
                load_y: 0.0,
                moment_z: 0.0,
            },
            Beam1dNodeInput {
                id: "tip".to_string(),
                x: 2.0,
                fix_y: false,
                fix_rz: false,
                load_y: -1000.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![Beam1dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            distributed_load_y: 0.0,
        }],
    }
}

fn thermal_beam_request() -> SolveThermalBeam1dRequest {
    SolveThermalBeam1dRequest {
        nodes: vec![
            ThermalBeam1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_y: true,
                fix_rz: true,
                load_y: 0.0,
                moment_z: 0.0,
            },
            ThermalBeam1dNodeInput {
                id: "tip".to_string(),
                x: 2.0,
                fix_y: false,
                fix_rz: false,
                load_y: -1000.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![ThermalBeam1dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion: 1.2e-5,
            section_depth: 0.2,
            distributed_load_y: 0.0,
            temperature_gradient_y: 5.0,
        }],
    }
}

fn torsion_request() -> SolveTorsion1dRequest {
    SolveTorsion1dRequest {
        nodes: vec![
            Torsion1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_rz: true,
                torque_z: 0.0,
            },
            Torsion1dNodeInput {
                id: "tip".to_string(),
                x: 1.5,
                fix_rz: false,
                torque_z: 2500.0,
            },
        ],
        elements: vec![Torsion1dElementInput {
            id: "shaft".to_string(),
            node_i: 0,
            node_j: 1,
            shear_modulus: 79.0e9,
            polar_moment: 1.8e-6,
            section_modulus: 1.2e-4,
        }],
    }
}
