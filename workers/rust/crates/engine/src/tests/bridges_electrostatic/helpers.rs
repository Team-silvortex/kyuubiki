use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneNodeResult, HeatPlaneNodeInput,
    HeatPlaneTriangleElementInput, SolveHeatPlaneTriangle2dRequest,
};

pub(super) fn triangle_source_nodes() -> Vec<ElectrostaticPlaneNodeInput> {
    vec![
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
            x: 0.0,
            y: 1.0,
            fix_potential: false,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeInput {
            id: "e3".to_string(),
            x: 1.0,
            y: 1.0,
            fix_potential: true,
            potential: 0.0,
            charge_density: 0.0,
        },
    ]
}

pub(super) fn triangle_result_nodes() -> Vec<ElectrostaticPlaneNodeResult> {
    vec![
        ElectrostaticPlaneNodeResult {
            index: 0,
            id: "e0".to_string(),
            x: 0.0,
            y: 0.0,
            potential: 10.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeResult {
            index: 1,
            id: "e1".to_string(),
            x: 1.0,
            y: 0.0,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeResult {
            index: 2,
            id: "e2".to_string(),
            x: 0.0,
            y: 1.0,
            potential: 0.0,
            charge_density: 0.0,
        },
        ElectrostaticPlaneNodeResult {
            index: 3,
            id: "e3".to_string(),
            x: 1.0,
            y: 1.0,
            potential: 0.0,
            charge_density: 0.0,
        },
    ]
}

pub(super) fn triangle_heat_seed_model() -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: false,
                temperature: 0.0,
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
                x: 0.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h3".to_string(),
                x: 1.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
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
