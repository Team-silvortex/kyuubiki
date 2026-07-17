use kyuubiki_protocol::{
    HeatBar1dElementInput, HeatBar1dNodeInput, SolveHeatBar1dRequest, SolveHeatBar1dResult,
};
use kyuubiki_solver::solve_heat_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn heat_bar_1d_tracks_heat_load_and_conductivity_scaling() {
    let baseline = HeatCase {
        length: 2.5,
        area: 0.25,
        conductivity: 30.0,
        heat_load: 12.0,
    };
    let baseline_result = solve_heat_bar_1d(&baseline.request()).expect("baseline heat bar");
    assert_response(&baseline_result, baseline.expected());

    let load_factor = 2.5;
    let loaded = HeatCase {
        heat_load: baseline.heat_load * load_factor,
        ..baseline
    };
    let loaded_result = solve_heat_bar_1d(&loaded.request()).expect("load-scaled heat bar");
    assert_response(&loaded_result, loaded.expected());
    assert_close(
        loaded_result.nodes[1].temperature,
        baseline_result.nodes[1].temperature * load_factor,
    );
    assert_close(
        loaded_result.elements[0].temperature_gradient,
        baseline_result.elements[0].temperature_gradient * load_factor,
    );
    assert_close(
        loaded_result.elements[0].heat_flux,
        baseline_result.elements[0].heat_flux * load_factor,
    );

    let conductivity_factor = 4.0;
    let conductive = HeatCase {
        conductivity: baseline.conductivity * conductivity_factor,
        ..baseline
    };
    let conductive_result =
        solve_heat_bar_1d(&conductive.request()).expect("conductivity-scaled heat bar");
    assert_response(&conductive_result, conductive.expected());
    assert_close(
        conductive_result.nodes[1].temperature,
        baseline_result.nodes[1].temperature / conductivity_factor,
    );
    assert_close(
        conductive_result.elements[0].temperature_gradient,
        baseline_result.elements[0].temperature_gradient / conductivity_factor,
    );
    assert_close(
        conductive_result.elements[0].heat_flux,
        baseline_result.elements[0].heat_flux,
    );

    let area_factor = 2.0;
    let wider = HeatCase {
        area: baseline.area * area_factor,
        ..baseline
    };
    let wider_result = solve_heat_bar_1d(&wider.request()).expect("area-scaled heat bar");
    assert_response(&wider_result, wider.expected());
    assert_close(
        wider_result.nodes[1].temperature,
        baseline_result.nodes[1].temperature / area_factor,
    );
    assert_close(
        wider_result.elements[0].temperature_gradient,
        baseline_result.elements[0].temperature_gradient / area_factor,
    );
    assert_close(
        wider_result.elements[0].heat_flux,
        baseline_result.elements[0].heat_flux / area_factor,
    );

    let length_factor = 1.8;
    let longer = HeatCase {
        length: baseline.length * length_factor,
        ..baseline
    };
    let longer_result = solve_heat_bar_1d(&longer.request()).expect("length-scaled heat bar");
    assert_response(&longer_result, longer.expected());
    assert_close(
        longer_result.nodes[1].temperature,
        baseline_result.nodes[1].temperature * length_factor,
    );
    assert_close(
        longer_result.elements[0].temperature_gradient,
        baseline_result.elements[0].temperature_gradient,
    );
    assert_close(
        longer_result.elements[0].heat_flux,
        baseline_result.elements[0].heat_flux,
    );
}

#[derive(Clone, Copy)]
struct HeatCase {
    length: f64,
    area: f64,
    conductivity: f64,
    heat_load: f64,
}

impl HeatCase {
    fn request(self) -> SolveHeatBar1dRequest {
        SolveHeatBar1dRequest {
            nodes: vec![
                node("fixed", 0.0, true, 0.0, 0.0),
                node("loaded", self.length, false, 0.0, self.heat_load),
            ],
            elements: vec![HeatBar1dElementInput {
                id: "conductor".to_string(),
                node_i: 0,
                node_j: 1,
                area: self.area,
                conductivity: self.conductivity,
            }],
        }
    }

    fn expected(self) -> ExpectedHeatResponse {
        let conductance = self.conductivity * self.area / self.length;
        let tip_temperature = self.heat_load / conductance;
        let gradient = tip_temperature / self.length;
        let heat_flux = -self.conductivity * gradient;
        ExpectedHeatResponse {
            tip_temperature,
            gradient,
            heat_flux,
        }
    }
}

struct ExpectedHeatResponse {
    tip_temperature: f64,
    gradient: f64,
    heat_flux: f64,
}

fn node(
    id: &str,
    x: f64,
    fix_temperature: bool,
    temperature: f64,
    heat_load: f64,
) -> HeatBar1dNodeInput {
    HeatBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_temperature,
        temperature,
        heat_load,
    }
}

fn assert_response(result: &SolveHeatBar1dResult, expected: ExpectedHeatResponse) {
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].temperature, 0.0);
    assert_close(result.nodes[1].temperature, expected.tip_temperature);
    assert_close(result.max_temperature, expected.tip_temperature.abs());
    assert_close(result.max_heat_flux, expected.heat_flux.abs());
    assert_heat_bar_summary(result);
    assert_close(
        result.elements[0].average_temperature,
        expected.tip_temperature / 2.0,
    );
    assert_close(result.elements[0].temperature_gradient, expected.gradient);
    assert_close(result.elements[0].heat_flux, expected.heat_flux);
}

fn assert_heat_bar_summary(result: &SolveHeatBar1dResult) {
    for node in &result.nodes {
        let input = &result.input.nodes[node.index];
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.heat_load, input.heat_load);
    }

    let max_temperature = result
        .nodes
        .iter()
        .map(|node| node.temperature.abs())
        .fold(0.0_f64, f64::max);
    assert_close(result.max_temperature, max_temperature);

    let max_heat_flux = result
        .elements
        .iter()
        .map(|element| element.heat_flux.abs())
        .fold(0.0_f64, f64::max);
    assert_close(result.max_heat_flux, max_heat_flux);

    for element in &result.elements {
        let input = &result.input.elements[element.index];
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let expected_length = (node_j.x - node_i.x).abs();
        let expected_average_temperature = 0.5 * (node_i.temperature + node_j.temperature);
        let expected_gradient = (node_j.temperature - node_i.temperature) / expected_length;
        assert_close(element.length, expected_length);
        assert_close(element.average_temperature, expected_average_temperature);
        assert_close(element.temperature_gradient, expected_gradient);
        assert_close(
            element.heat_flux,
            -input.conductivity * element.temperature_gradient,
        );
        let nodal_load = node_i.heat_load + node_j.heat_load;
        assert_close(element.heat_flux * input.area + nodal_load, 0.0);
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
