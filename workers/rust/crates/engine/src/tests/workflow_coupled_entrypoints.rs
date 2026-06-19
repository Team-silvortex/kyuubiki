use crate::{
    run_electrostatic_to_heat_to_thermo_plane_quad_2d_workflow,
    run_electrostatic_to_heat_to_thermo_plane_triangle_2d_workflow,
};
use kyuubiki_protocol::{
    ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest,
    ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest, ElectrostaticPlaneNodeInput,
    ElectrostaticPlaneQuadElementInput, ElectrostaticPlaneTriangleElementInput, HeatPlaneNodeInput,
    HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput, ThermalPlaneNodeInput,
    ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
};

#[test]
fn runs_electrostatic_to_heat_to_thermo_plane_quad_workflow() {
    let result = run_electrostatic_to_heat_to_thermo_plane_quad_2d_workflow(
        ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest {
            electrostatic_model: quad_electrostatic_model(),
            heat_seed_model: quad_heat_seed_model(),
            thermo_seed_model: quad_thermo_seed_model(),
        },
    )
    .expect("electrostatic -> heat -> thermo quad workflow should run");

    assert_eq!(
        result.workflow_id,
        "workflow.electrostatic-heat-to-thermo-quad-2d"
    );
    assert!(result.electrostatic_result.max_electric_field > 0.0);
    assert!(result
        .bridged_heat_model
        .nodes
        .iter()
        .all(|node| node.heat_load > 0.0));
    assert!(result.heat_result.max_heat_flux > 0.0);
    assert!(result
        .bridged_thermo_model
        .nodes
        .iter()
        .any(|node| node.temperature_delta > 30.0));
    assert!(result.thermo_result.max_stress > 0.0);
}

#[test]
fn runs_electrostatic_to_heat_to_thermo_plane_triangle_workflow() {
    let result = run_electrostatic_to_heat_to_thermo_plane_triangle_2d_workflow(
        ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest {
            electrostatic_model: triangle_electrostatic_model(),
            heat_seed_model: triangle_heat_seed_model(),
            thermo_seed_model: triangle_thermo_seed_model(),
        },
    )
    .expect("electrostatic -> heat -> thermo triangle workflow should run");

    assert_eq!(
        result.workflow_id,
        "workflow.electrostatic-heat-to-thermo-triangle-2d"
    );
    assert!(result.electrostatic_result.max_electric_field > 0.0);
    assert!(result
        .bridged_heat_model
        .nodes
        .iter()
        .all(|node| node.heat_load > 0.0));
    assert!(result.heat_result.max_heat_flux > 0.0);
    assert!(result
        .bridged_thermo_model
        .nodes
        .iter()
        .any(|node| node.temperature_delta > 30.0));
    assert!(result.thermo_result.max_stress > 0.0);
}

fn quad_electrostatic_model() -> kyuubiki_protocol::SolveElectrostaticPlaneQuad2dRequest {
    kyuubiki_protocol::SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            ElectrostaticPlaneNodeInput {
                id: "e0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_potential: false,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "eq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.05,
            permittivity: 2.5,
        }],
    }
}

fn quad_heat_seed_model() -> kyuubiki_protocol::SolveHeatPlaneQuad2dRequest {
    kyuubiki_protocol::SolveHeatPlaneQuad2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![HeatPlaneQuadElementInput {
            id: "hq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            conductivity: 45.0,
        }],
    }
}

fn quad_thermo_seed_model() -> kyuubiki_protocol::SolveThermalPlaneQuad2dRequest {
    kyuubiki_protocol::SolveThermalPlaneQuad2dRequest {
        nodes: vec![
            ThermalPlaneNodeInput {
                id: "t0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "t1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "t2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "t3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
        ],
        elements: vec![ThermalPlaneQuadElementInput {
            id: "tq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion: 11.0e-6,
        }],
    }
}

fn triangle_electrostatic_model() -> kyuubiki_protocol::SolveElectrostaticPlaneTriangle2dRequest {
    kyuubiki_protocol::SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            ElectrostaticPlaneNodeInput {
                id: "e0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e2".to_string(),
                x: 0.0,
                y: 1.0,
                fix_potential: false,
                potential: 0.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "et0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.05,
            permittivity: 2.5,
        }],
    }
}

fn triangle_heat_seed_model() -> kyuubiki_protocol::SolveHeatPlaneTriangle2dRequest {
    kyuubiki_protocol::SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h2".to_string(),
                x: 0.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![HeatPlaneTriangleElementInput {
            id: "ht0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            conductivity: 45.0,
        }],
    }
}

fn triangle_thermo_seed_model() -> kyuubiki_protocol::SolveThermalPlaneTriangle2dRequest {
    kyuubiki_protocol::SolveThermalPlaneTriangle2dRequest {
        nodes: vec![
            ThermalPlaneNodeInput {
                id: "t0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "t1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "t2".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
        ],
        elements: vec![ThermalPlaneTriangleElementInput {
            id: "tt0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion: 11.0e-6,
        }],
    }
}
