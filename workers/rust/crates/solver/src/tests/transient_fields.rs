use super::*;

#[test]
fn solves_transient_heat_bar_1d_with_implicit_steps() {
    let request = SolveTransientHeatBar1dRequest {
        nodes: vec![
            HeatBar1dNodeInput {
                id: "hot".to_string(),
                x: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatBar1dNodeInput {
                id: "mid".to_string(),
                x: 0.5,
                fix_temperature: false,
                temperature: 20.0,
                heat_load: 0.0,
            },
            HeatBar1dNodeInput {
                id: "cold".to_string(),
                x: 1.0,
                fix_temperature: true,
                temperature: 0.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![transient_element("e0", 0, 1), transient_element("e1", 1, 2)],
        time_step: 0.1,
        steps: 4,
    };

    let result = solve_transient_heat_bar_1d(&request).expect("transient heat bar should solve");

    assert_eq!(result.history.len(), 5);
    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 2);
    assert!((result.final_time - 0.4).abs() < 1.0e-12);
    assert!(result.nodes[1].temperature > 20.0);
    assert!(result.max_heat_flux > 0.0);
    assert!(result.total_thermal_energy > 0.0);
}

#[test]
fn solves_transient_spring_1d_with_newmark_steps() {
    let request = SolveTransientSpring1dRequest {
        nodes: vec![
            TransientSpring1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
                mass: 1.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
            TransientSpring1dNodeInput {
                id: "tip".to_string(),
                x: 1.0,
                fix_x: false,
                load_x: 10.0,
                mass: 2.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
            damping: 0.5,
        }],
        time_step: 0.01,
        steps: 10,
    };

    let result = solve_transient_spring_1d(&request).expect("transient spring chain should solve");

    assert_eq!(result.history.len(), 11);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.final_time - 0.1).abs() < 1.0e-12);
    assert!(result.nodes[1].ux > 0.0);
    assert!(result.max_velocity > 0.0);
    assert!(result.max_force > 0.0);
}

#[test]
fn solves_harmonic_spring_1d_frequency_response() {
    let request = SolveHarmonicSpring1dRequest {
        nodes: vec![
            TransientSpring1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
                mass: 1.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
            TransientSpring1dNodeInput {
                id: "tip".to_string(),
                x: 1.0,
                fix_x: false,
                load_x: 10.0,
                mass: 2.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
            damping: 1.0,
        }],
        frequencies_hz: vec![0.0, 0.5, 1.0],
    };

    let result = solve_harmonic_spring_1d(&request).expect("harmonic spring should solve");

    assert_eq!(result.frequencies.len(), 3);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_velocity > 0.0);
    assert!(result.max_acceleration > 0.0);
    assert!(result.max_force > 0.0);
    assert!(result.peak_frequency_hz >= 0.0);
    assert_eq!(result.frequencies[0].nodes.len(), 2);
}

fn transient_element(id: &str, node_i: usize, node_j: usize) -> TransientHeatBar1dElementInput {
    TransientHeatBar1dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        conductivity: 45.0,
        density: 7800.0,
        specific_heat: 500.0,
    }
}
