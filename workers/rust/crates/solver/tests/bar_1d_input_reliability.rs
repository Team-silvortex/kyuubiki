use kyuubiki_protocol::{
    ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput, HeatBar1dElementInput,
    HeatBar1dNodeInput, SolveBarRequest, SolveElectrostaticBar1dRequest, SolveHeatBar1dRequest,
    SolveThermalBar1dRequest, ThermalBar1dElementInput, ThermalBar1dNodeInput,
};
use kyuubiki_solver::{
    solve_bar_1d, solve_electrostatic_bar_1d, solve_heat_bar_1d, solve_thermal_bar_1d,
};

#[test]
fn simple_bar_rejects_non_finite_tip_force() {
    let request = SolveBarRequest {
        length: 1.0,
        area: 0.01,
        youngs_modulus: 70.0e9,
        elements: 2,
        tip_force: f64::NAN,
    };

    let error = solve_bar_1d(&request).expect_err("non-finite tip force should be rejected");
    assert!(
        error.contains("tip_force must be a finite number"),
        "unexpected tip force error: {error}"
    );
}

#[test]
fn thermal_bar_rejects_non_finite_node_load() {
    let mut request = valid_thermal_bar_request();
    request.nodes[1].load_x = f64::INFINITY;

    let error =
        solve_thermal_bar_1d(&request).expect_err("non-finite thermal bar load should be rejected");
    assert!(
        error.contains("load_x must be finite"),
        "unexpected thermal bar load error: {error}"
    );
}

#[test]
fn thermal_bar_rejects_duplicate_element_nodes() {
    let mut request = valid_thermal_bar_request();
    request.elements[0].node_j = request.elements[0].node_i;

    let error =
        solve_thermal_bar_1d(&request).expect_err("duplicate thermal bar nodes should be rejected");
    assert!(
        error.contains("two distinct nodes"),
        "unexpected thermal bar duplicate-node error: {error}"
    );
}

#[test]
fn heat_bar_rejects_non_finite_heat_load() {
    let mut request = valid_heat_bar_request();
    request.nodes[1].heat_load = f64::NAN;

    let error = solve_heat_bar_1d(&request).expect_err("non-finite heat load should be rejected");
    assert!(
        error.contains("heat_load must be finite"),
        "unexpected heat load error: {error}"
    );
}

#[test]
fn electrostatic_bar_rejects_non_finite_charge_density() {
    let mut request = valid_electrostatic_bar_request();
    request.nodes[1].charge_density = f64::INFINITY;

    let error = solve_electrostatic_bar_1d(&request)
        .expect_err("non-finite charge density should be rejected");
    assert!(
        error.contains("charge_density must be finite"),
        "unexpected charge density error: {error}"
    );
}

fn valid_thermal_bar_request() -> SolveThermalBar1dRequest {
    SolveThermalBar1dRequest {
        nodes: vec![
            ThermalBar1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
                temperature_delta: 20.0,
            },
            ThermalBar1dNodeInput {
                id: "free".to_string(),
                x: 1.0,
                fix_x: false,
                load_x: 10.0,
                temperature_delta: 20.0,
            },
        ],
        elements: vec![ThermalBar1dElementInput {
            id: "bar".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 70.0e9,
            thermal_expansion: 12.0e-6,
        }],
    }
}

fn valid_heat_bar_request() -> SolveHeatBar1dRequest {
    SolveHeatBar1dRequest {
        nodes: vec![
            heat_node("hot", 0.0, true, 100.0),
            heat_node("mid", 0.5, false, 0.0),
            heat_node("cold", 1.0, true, 20.0),
        ],
        elements: vec![HeatBar1dElementInput {
            id: "bar".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            conductivity: 50.0,
        }],
    }
}

fn valid_electrostatic_bar_request() -> SolveElectrostaticBar1dRequest {
    SolveElectrostaticBar1dRequest {
        nodes: vec![
            electrostatic_node("left", 0.0, true, 12.0),
            electrostatic_node("mid", 1.0, false, 0.0),
            electrostatic_node("right", 2.0, true, 4.0),
        ],
        elements: vec![ElectrostaticBar1dElementInput {
            id: "bar".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            permittivity: 3.0,
        }],
    }
}

fn heat_node(id: &str, x: f64, fix_temperature: bool, temperature: f64) -> HeatBar1dNodeInput {
    HeatBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_temperature,
        temperature,
        heat_load: 0.0,
    }
}

fn electrostatic_node(
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
