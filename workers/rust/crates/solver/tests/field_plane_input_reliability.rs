use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneTriangleElementInput, HeatPlaneNodeInput, HeatPlaneQuadElementInput,
    HeatPlaneTriangleElementInput, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneTriangle2dRequest,
};
use kyuubiki_solver::{
    solve_electrostatic_plane_quad_2d, solve_electrostatic_plane_triangle_2d,
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d,
};

#[test]
fn heat_plane_rejects_non_finite_node_inputs_and_duplicate_nodes() {
    let mut request = heat_triangle_request();
    request.nodes[2].heat_load = f64::INFINITY;
    assert!(solve_heat_plane_triangle_2d(&request).is_err());

    let mut request = heat_triangle_request();
    request.elements[0].node_k = request.elements[0].node_i;
    assert!(solve_heat_plane_triangle_2d(&request).is_err());

    let mut request = heat_quad_request();
    request.elements[0].conductivity = f64::NAN;
    assert!(solve_heat_plane_quad_2d(&request).is_err());
}

#[test]
fn electrostatic_plane_rejects_non_finite_node_inputs_and_duplicate_nodes() {
    let mut request = electrostatic_triangle_request();
    request.nodes[2].charge_density = f64::NEG_INFINITY;
    assert!(solve_electrostatic_plane_triangle_2d(&request).is_err());

    let mut request = electrostatic_triangle_request();
    request.elements[0].node_k = request.elements[0].node_i;
    assert!(solve_electrostatic_plane_triangle_2d(&request).is_err());

    let mut request = electrostatic_quad_request();
    request.elements[0].node_l = request.elements[0].node_i;
    assert!(solve_electrostatic_plane_quad_2d(&request).is_err());
}

fn heat_triangle_request() -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            heat_node("n0", 0.0, 0.0, true, 100.0),
            heat_node("n1", 1.0, 0.0, false, 0.0),
            heat_node("n2", 1.0, 1.0, true, 20.0),
        ],
        elements: vec![HeatPlaneTriangleElementInput {
            id: "tri".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            conductivity: 45.0,
        }],
    }
}

fn heat_quad_request() -> SolveHeatPlaneQuad2dRequest {
    SolveHeatPlaneQuad2dRequest {
        nodes: vec![
            heat_node("n0", 0.0, 0.0, true, 100.0),
            heat_node("n1", 1.0, 0.0, false, 0.0),
            heat_node("n2", 1.0, 1.0, true, 20.0),
            heat_node("n3", 0.0, 1.0, true, 20.0),
        ],
        elements: vec![HeatPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            conductivity: 45.0,
        }],
    }
}

fn electrostatic_triangle_request() -> SolveElectrostaticPlaneTriangle2dRequest {
    SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            electrostatic_node("n0", 0.0, 0.0, 12.0),
            electrostatic_node("n1", 1.0, 0.0, 4.0),
            electrostatic_node("n2", 0.0, 1.0, 12.0),
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "tri".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    }
}

fn electrostatic_quad_request() -> SolveElectrostaticPlaneQuad2dRequest {
    SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            electrostatic_node("n0", 0.0, 0.0, 12.0),
            electrostatic_node("n1", 1.0, 0.0, 4.0),
            electrostatic_node("n2", 1.0, 1.0, 4.0),
            electrostatic_node("n3", 0.0, 1.0, 12.0),
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    }
}

fn heat_node(
    id: &str,
    x: f64,
    y: f64,
    fix_temperature: bool,
    temperature: f64,
) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature,
        temperature,
        heat_load: 0.0,
    }
}

fn electrostatic_node(id: &str, x: f64, y: f64, potential: f64) -> ElectrostaticPlaneNodeInput {
    ElectrostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_potential: true,
        potential,
        charge_density: 0.0,
    }
}
