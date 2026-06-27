use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_triangle_model,
    resolve_electrostatic_to_heat_bridge_contract,
};
use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneNodeResult,
    ElectrostaticPlaneTriangleElementInput, ElectrostaticPlaneTriangleElementResult,
    HeatPlaneNodeInput, HeatPlaneTriangleElementInput, SolveElectrostaticPlaneTriangle2dRequest,
    SolveElectrostaticPlaneTriangle2dResult, SolveHeatPlaneTriangle2dRequest,
};

#[test]
fn bridges_triangle_fields_with_max_reduction() {
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": {
                "field": "electric_field_magnitude",
                "distribution": "element_to_nodes",
                "node_index_fields": ["node_i", "node_j", "node_k"]
            },
            "transform": { "scale": 1.0, "reduction": "max", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("max reduction contract should resolve");

    let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &triangle_result(),
        &triangle_heat_seed_model(),
        &contract,
    )
    .expect("triangle max bridge should build");

    assert_eq!(bridged.nodes[0].heat_load, 2.0);
    assert_eq!(bridged.nodes[1].heat_load, 8.0);
    assert_eq!(bridged.nodes[2].heat_load, 8.0);
    assert_eq!(bridged.nodes[3].heat_load, 8.0);
    assert_eq!(diagnostics.reduction.as_deref(), Some("max"));
}

#[test]
fn bridges_triangle_fields_with_min_reduction() {
    let contract = resolve_electrostatic_to_heat_bridge_contract(&serde_json::json!({
        "contract": {
            "source": {
                "field": "electric_field_magnitude",
                "distribution": "element_to_nodes",
                "node_index_fields": ["node_i", "node_j", "node_k"]
            },
            "transform": { "scale": 1.0, "reduction": "min", "default_value": 0.0 },
            "target": { "field": "heat_load" }
        }
    }))
    .expect("min reduction contract should resolve");

    let (bridged, diagnostics) = bridge_electrostatic_result_to_heat_plane_triangle_model(
        &triangle_result(),
        &triangle_heat_seed_model(),
        &contract,
    )
    .expect("triangle min bridge should build");

    assert_eq!(bridged.nodes[0].heat_load, 2.0);
    assert_eq!(bridged.nodes[1].heat_load, 2.0);
    assert_eq!(bridged.nodes[2].heat_load, 2.0);
    assert_eq!(bridged.nodes[3].heat_load, 8.0);
    assert_eq!(diagnostics.reduction.as_deref(), Some("min"));
}

fn triangle_result() -> SolveElectrostaticPlaneTriangle2dResult {
    SolveElectrostaticPlaneTriangle2dResult {
        input: SolveElectrostaticPlaneTriangle2dRequest {
            nodes: vec![
                source_node("e0", 0.0, 0.0),
                source_node("e1", 1.0, 0.0),
                source_node("e2", 0.0, 1.0),
                source_node("e3", 1.0, 1.0),
            ],
            elements: vec![
                ElectrostaticPlaneTriangleElementInput {
                    id: "et0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.05,
                    permittivity: 2.5,
                },
                ElectrostaticPlaneTriangleElementInput {
                    id: "et1".to_string(),
                    node_i: 1,
                    node_j: 3,
                    node_k: 2,
                    thickness: 0.05,
                    permittivity: 2.5,
                },
            ],
        },
        nodes: vec![
            result_node("e0", 0.0, 0.0),
            result_node("e1", 1.0, 0.0),
            result_node("e2", 0.0, 1.0),
            result_node("e3", 1.0, 1.0),
        ],
        elements: vec![
            ElectrostaticPlaneTriangleElementResult {
                index: 0,
                id: "et0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                area: 1.0,
                average_potential: 4.0,
                potential_gradient_x: 0.0,
                potential_gradient_y: 0.0,
                electric_field_x: 2.0,
                electric_field_y: 0.0,
                electric_field_magnitude: 2.0,
                electric_flux_density_x: 0.0,
                electric_flux_density_y: 0.0,
                electric_flux_density_magnitude: 0.0,
                stored_energy: 0.0,
            },
            ElectrostaticPlaneTriangleElementResult {
                index: 1,
                id: "et1".to_string(),
                node_i: 1,
                node_j: 3,
                node_k: 2,
                area: 2.0,
                average_potential: 6.0,
                potential_gradient_x: 0.0,
                potential_gradient_y: 0.0,
                electric_field_x: 8.0,
                electric_field_y: 0.0,
                electric_field_magnitude: 8.0,
                electric_flux_density_x: 0.0,
                electric_flux_density_y: 0.0,
                electric_flux_density_magnitude: 0.0,
                stored_energy: 0.0,
            },
        ],
        max_potential: 6.0,
        max_electric_field: 8.0,
        max_flux_density: 0.0,
        total_stored_energy: 0.0,
    }
}

fn triangle_heat_seed_model() -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            heat_node("h0", 0.0, 0.0),
            heat_node("h1", 1.0, 0.0),
            heat_node("h2", 0.0, 1.0),
            heat_node("h3", 1.0, 1.0),
        ],
        elements: vec![
            HeatPlaneTriangleElementInput {
                id: "ht0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            },
            HeatPlaneTriangleElementInput {
                id: "ht1".to_string(),
                node_i: 1,
                node_j: 3,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            },
        ],
    }
}

fn source_node(id: &str, x: f64, y: f64) -> ElectrostaticPlaneNodeInput {
    ElectrostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_potential: false,
        potential: 0.0,
        charge_density: 0.0,
    }
}

fn result_node(id: &str, x: f64, y: f64) -> ElectrostaticPlaneNodeResult {
    ElectrostaticPlaneNodeResult {
        index: 0,
        id: id.to_string(),
        x,
        y,
        potential: 0.0,
        charge_density: 0.0,
    }
}

fn heat_node(id: &str, x: f64, y: f64) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature: false,
        temperature: 0.0,
        heat_load: 0.0,
    }
}
