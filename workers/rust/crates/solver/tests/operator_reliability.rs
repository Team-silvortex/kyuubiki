use kyuubiki_protocol::{
    ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput, ElectrostaticPlaneNodeInput,
    ElectrostaticPlaneQuadElementInput, HeatBar1dElementInput, HeatBar1dNodeInput,
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, PlaneNodeInput, PlaneQuadElementInput,
    SolveBarRequest, SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolvePlaneQuad2dRequest,
    SolveThermalPlaneQuad2dRequest, SolveThermalTruss2dRequest, SolveTruss2dRequest,
    ThermalPlaneNodeInput, ThermalPlaneQuadElementInput, ThermalTruss2dElementInput,
    ThermalTruss2dNodeInput, TrussElementInput, TrussNodeInput,
};
use kyuubiki_solver::{
    solve_bar_1d, solve_electrostatic_bar_1d, solve_electrostatic_plane_quad_2d, solve_heat_bar_1d,
    solve_heat_plane_quad_2d, solve_plane_quad_2d, solve_thermal_plane_quad_2d,
    solve_thermal_truss_2d, solve_truss_2d,
};

fn assert_err_contains<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.expect_err("operator should reject invalid input");
    assert!(
        error.contains(expected),
        "expected error to contain {expected:?}, got {error:?}",
    );
}

#[test]
fn bar_1d_rejects_non_positive_element_count_before_solving() {
    assert_err_contains(
        solve_bar_1d(&SolveBarRequest {
            length: 1.0,
            area: 0.01,
            youngs_modulus: 210.0e9,
            elements: 0,
            tip_force: 1000.0,
        }),
        "elements must be a positive integer",
    );
}

#[test]
fn heat_bar_rejects_missing_temperature_support() {
    let mut request = heat_bar_request();
    for node in &mut request.nodes {
        node.fix_temperature = false;
    }

    assert_err_contains(
        solve_heat_bar_1d(&request),
        "must include at least one temperature support",
    );
}

#[test]
fn heat_bar_rejects_non_positive_conductivity() {
    let mut request = heat_bar_request();
    request.elements[0].conductivity = 0.0;

    assert_err_contains(solve_heat_bar_1d(&request), "conductivity must be positive");
}

#[test]
fn electrostatic_quad_rejects_degenerate_geometry() {
    let mut request = electrostatic_quad_request();
    request.nodes[2].x = 1.0;
    request.nodes[2].y = 0.0;

    assert_err_contains(
        solve_electrostatic_plane_quad_2d(&request),
        "triangles must have positive area",
    );
}

#[test]
fn plane_quad_rejects_invalid_poisson_ratio() {
    let mut request = plane_quad_request();
    request.elements[0].poisson_ratio = 0.5;

    assert_err_contains(
        solve_plane_quad_2d(&request),
        "poisson_ratio must be between -1.0 and 0.5",
    );
}

#[test]
fn thermal_plane_quad_rejects_negative_expansion() {
    let mut request = thermal_plane_quad_request();
    request.elements[0].thermal_expansion = -1.0e-6;

    assert_err_contains(
        solve_thermal_plane_quad_2d(&request),
        "thermal_expansion must be non-negative",
    );
}

#[test]
fn valid_core_operator_results_stay_finite() {
    let bar = solve_bar_1d(&SolveBarRequest {
        length: 1.0,
        area: 0.01,
        youngs_modulus: 210.0e9,
        elements: 2,
        tip_force: 1000.0,
    })
    .expect("valid bar should solve");
    assert_finite("bar tip_displacement", bar.tip_displacement);
    assert_finite("bar max_stress", bar.max_stress);
    for element in &bar.elements {
        assert_finite("bar element stress", element.stress);
        assert_finite("bar element axial_force", element.axial_force);
    }

    let heat = solve_heat_bar_1d(&heat_bar_request()).expect("valid heat bar should solve");
    assert_finite("heat max_temperature", heat.max_temperature);
    assert_finite("heat max_heat_flux", heat.max_heat_flux);
    for element in &heat.elements {
        assert_finite("heat element gradient", element.temperature_gradient);
        assert_finite("heat element flux", element.heat_flux);
    }
}

#[test]
fn valid_2d_field_operator_results_stay_finite() {
    let electrostatic = solve_electrostatic_plane_quad_2d(&electrostatic_quad_request())
        .expect("valid electrostatic quad should solve");
    assert_finite("electro max_potential", electrostatic.max_potential);
    assert_finite(
        "electro max_electric_field",
        electrostatic.max_electric_field,
    );
    assert_finite("electro max_flux_density", electrostatic.max_flux_density);
    for element in &electrostatic.elements {
        assert_finite("electro element field", element.electric_field_magnitude);
        assert_finite(
            "electro element flux",
            element.electric_flux_density_magnitude,
        );
    }

    let thermal = solve_thermal_plane_quad_2d(&thermal_plane_quad_request())
        .expect("valid thermal plane quad should solve");
    assert_finite("thermal max_displacement", thermal.max_displacement);
    assert_finite("thermal max_stress", thermal.max_stress);
    assert_finite(
        "thermal max_temperature_delta",
        thermal.max_temperature_delta,
    );
    for element in &thermal.elements {
        assert_finite("thermal element stress", element.von_mises);
        assert_finite("thermal element thermal strain", element.thermal_strain);
    }
}

#[test]
fn valid_structural_quad_result_stays_finite() {
    let result =
        solve_plane_quad_2d(&loaded_plane_quad_request()).expect("valid plane quad should solve");

    assert_finite("plane max_displacement", result.max_displacement);
    assert_finite("plane max_stress", result.max_stress);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_stress > 0.0);
    for element in &result.elements {
        assert_finite("plane element strain_x", element.strain_x);
        assert_finite("plane element stress_x", element.stress_x);
        assert_finite("plane element von_mises", element.von_mises);
    }
}

#[test]
fn valid_extended_operator_results_stay_finite() {
    let electro_bar = solve_electrostatic_bar_1d(&electrostatic_bar_request())
        .expect("valid electrostatic bar should solve");
    assert_finite("electro bar max_potential", electro_bar.max_potential);
    assert_finite(
        "electro bar max_electric_field",
        electro_bar.max_electric_field,
    );
    assert_finite("electro bar max_flux_density", electro_bar.max_flux_density);
    for element in &electro_bar.elements {
        assert_finite("electro bar element field", element.electric_field);
        assert_finite("electro bar element flux", element.electric_flux_density);
    }

    let heat_quad =
        solve_heat_plane_quad_2d(&heat_plane_quad_request()).expect("valid heat quad should solve");
    assert_finite("heat quad max_temperature", heat_quad.max_temperature);
    assert_finite("heat quad max_heat_flux", heat_quad.max_heat_flux);
    for element in &heat_quad.elements {
        assert_finite(
            "heat quad element gradient_x",
            element.temperature_gradient_x,
        );
        assert_finite(
            "heat quad element gradient_y",
            element.temperature_gradient_y,
        );
        assert_finite("heat quad element flux", element.heat_flux_magnitude);
    }

    let truss = solve_truss_2d(&truss_2d_request()).expect("valid truss should solve");
    assert_finite("truss max_displacement", truss.max_displacement);
    assert_finite("truss max_stress", truss.max_stress);
    assert!(truss.max_displacement > 0.0);
    assert!(truss.max_stress > 0.0);
    for element in &truss.elements {
        assert_finite("truss element strain", element.strain);
        assert_finite("truss element stress", element.stress);
        assert_finite("truss element axial_force", element.axial_force);
    }

    let thermal_truss = solve_thermal_truss_2d(&thermal_truss_2d_request())
        .expect("valid thermal truss should solve");
    assert_finite(
        "thermal truss max_displacement",
        thermal_truss.max_displacement,
    );
    assert_finite("thermal truss max_stress", thermal_truss.max_stress);
    assert_finite(
        "thermal truss max_temperature_delta",
        thermal_truss.max_temperature_delta,
    );
    for element in &thermal_truss.elements {
        assert_finite("thermal truss thermal_strain", element.thermal_strain);
        assert_finite("thermal truss stress", element.stress);
        assert_finite("thermal truss axial_force", element.axial_force);
    }
}

fn heat_bar_request() -> SolveHeatBar1dRequest {
    SolveHeatBar1dRequest {
        nodes: vec![
            HeatBar1dNodeInput {
                id: "hot".to_string(),
                x: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatBar1dNodeInput {
                id: "cold".to_string(),
                x: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![HeatBar1dElementInput {
            id: "bar".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            conductivity: 45.0,
        }],
    }
}

fn electrostatic_bar_request() -> SolveElectrostaticBar1dRequest {
    SolveElectrostaticBar1dRequest {
        nodes: vec![
            electro_bar_node("left", 0.0, true, 10.0),
            electro_bar_node("right", 1.0, true, 0.0),
        ],
        elements: vec![ElectrostaticBar1dElementInput {
            id: "bar".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            permittivity: 2.0,
        }],
    }
}

fn electrostatic_quad_request() -> SolveElectrostaticPlaneQuad2dRequest {
    SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            electro_node("n0", 0.0, 0.0, true, 10.0),
            electro_node("n1", 1.0, 0.0, true, 0.0),
            electro_node("n2", 1.0, 1.0, true, 0.0),
            electro_node("n3", 0.0, 1.0, true, 10.0),
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.05,
            permittivity: 2.0,
        }],
    }
}

fn heat_plane_quad_request() -> SolveHeatPlaneQuad2dRequest {
    SolveHeatPlaneQuad2dRequest {
        nodes: vec![
            heat_plane_node("n0", 0.0, 0.0, true, 100.0),
            heat_plane_node("n1", 1.0, 0.0, true, 20.0),
            heat_plane_node("n2", 1.0, 1.0, true, 20.0),
            heat_plane_node("n3", 0.0, 1.0, true, 100.0),
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

fn plane_quad_request() -> SolvePlaneQuad2dRequest {
    SolvePlaneQuad2dRequest {
        nodes: vec![
            plane_node("n0", 0.0, 0.0, true, true),
            plane_node("n1", 1.0, 0.0, false, true),
            plane_node("n2", 1.0, 1.0, false, false),
            plane_node("n3", 0.0, 1.0, true, false),
        ],
        elements: vec![PlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    }
}

fn loaded_plane_quad_request() -> SolvePlaneQuad2dRequest {
    let mut request = plane_quad_request();
    request.nodes[2].load_y = -1000.0;
    request
}

fn thermal_plane_quad_request() -> SolveThermalPlaneQuad2dRequest {
    SolveThermalPlaneQuad2dRequest {
        nodes: vec![
            thermal_plane_node("n0", 0.0, 0.0, true, true),
            thermal_plane_node("n1", 1.0, 0.0, true, true),
            thermal_plane_node("n2", 1.0, 1.0, true, true),
            thermal_plane_node("n3", 0.0, 1.0, true, true),
        ],
        elements: vec![ThermalPlaneQuadElementInput {
            id: "quad".to_string(),
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

fn truss_2d_request() -> SolveTruss2dRequest {
    SolveTruss2dRequest {
        nodes: vec![
            truss_node("n0", 0.0, 0.0, true, true, 0.0, 0.0),
            truss_node("n1", 1.0, 0.0, false, true, 0.0, 0.0),
            truss_node("n2", 0.5, 0.75, false, false, 0.0, -1000.0),
        ],
        elements: vec![
            truss_element("left", 0, 2),
            truss_element("right", 1, 2),
            truss_element("base", 0, 1),
        ],
    }
}

fn thermal_truss_2d_request() -> SolveThermalTruss2dRequest {
    SolveThermalTruss2dRequest {
        nodes: vec![
            thermal_truss_node("n0", 0.0, 0.0, true, true),
            thermal_truss_node("n1", 1.0, 0.0, false, true),
            thermal_truss_node("n2", 0.5, 0.75, false, false),
        ],
        elements: vec![
            thermal_truss_element("left", 0, 2),
            thermal_truss_element("right", 1, 2),
            thermal_truss_element("base", 0, 1),
        ],
    }
}

fn electro_bar_node(
    id: &str,
    x: f64,
    fix_potential: bool,
    potential: f64,
) -> ElectrostaticBar1dNodeInput {
    ElectrostaticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_potential,
        potential,
        charge_density: 0.0,
    }
}

fn electro_node(
    id: &str,
    x: f64,
    y: f64,
    fix_potential: bool,
    potential: f64,
) -> ElectrostaticPlaneNodeInput {
    ElectrostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_potential,
        potential,
        charge_density: 0.0,
    }
}

fn heat_plane_node(
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

fn plane_node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool) -> PlaneNodeInput {
    PlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y: 0.0,
    }
}

fn truss_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> TrussNodeInput {
    TrussNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

fn truss_element(id: &str, node_i: usize, node_j: usize) -> TrussElementInput {
    TrussElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
    }
}

fn thermal_plane_node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool) -> ThermalPlaneNodeInput {
    ThermalPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y: 0.0,
        temperature_delta: 30.0,
    }
}

fn thermal_truss_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
) -> ThermalTruss2dNodeInput {
    ThermalTruss2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y: -250.0,
        temperature_delta: 35.0,
    }
}

fn thermal_truss_element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss2dElementInput {
    ThermalTruss2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
        thermal_expansion: 12.0e-6,
    }
}

fn assert_finite(label: &str, value: f64) {
    assert!(value.is_finite(), "{label} must be finite, got {value}");
}
