use kyuubiki_protocol::{
    HeatBar1dNodeInput, SolveTransientHeatBar1dRequest, TransientHeatBar1dElementInput,
};
use kyuubiki_solver::solve_transient_heat_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn transient_heat_bar_1d_tracks_lumped_capacity_and_heat_load_response() {
    let baseline_case = HeatTransientCase {
        length: 2.0,
        area: 0.2,
        density: 5.0,
        specific_heat: 4.0,
        conductivity: 3.0,
        initial_temperature: 10.0,
        heat_load: 12.0,
        time_step: 0.25,
        steps: 4,
    };
    let baseline =
        solve_transient_heat_bar_1d(&baseline_case.request()).expect("baseline transient heat bar");
    assert_response(&baseline, baseline_case);

    let load_scale = 2.5;
    let loaded_case = HeatTransientCase {
        heat_load: baseline_case.heat_load * load_scale,
        ..baseline_case
    };
    let loaded =
        solve_transient_heat_bar_1d(&loaded_case.request()).expect("load-scaled transient heat");
    assert_response(&loaded, loaded_case);
    assert_close(
        loaded.nodes[1].temperature - baseline_case.initial_temperature,
        (baseline.nodes[1].temperature - baseline_case.initial_temperature) * load_scale,
    );
    assert_close(
        loaded.total_thermal_energy - baseline_case.initial_energy(),
        (baseline.total_thermal_energy - baseline_case.initial_energy()) * load_scale,
    );

    let capacity_scale = 3.0;
    let massive_case = HeatTransientCase {
        density: baseline_case.density * capacity_scale,
        ..baseline_case
    };
    let massive = solve_transient_heat_bar_1d(&massive_case.request())
        .expect("capacity-scaled transient heat");
    assert_response(&massive, massive_case);
    assert!(
        massive.nodes[1].temperature < baseline.nodes[1].temperature,
        "larger lumped capacity should slow the same transient heat load"
    );

    let length_scale = 1.5;
    let longer_case = HeatTransientCase {
        length: baseline_case.length * length_scale,
        ..baseline_case
    };
    let longer =
        solve_transient_heat_bar_1d(&longer_case.request()).expect("length-scaled transient heat");
    assert_response(&longer, longer_case);
    assert_close(
        longer_case.node_capacity() / baseline_case.node_capacity(),
        length_scale,
    );
    assert_close(
        longer_case.conductance() / baseline_case.conductance(),
        1.0 / length_scale,
    );
    assert!(
        longer.nodes[1].temperature < baseline.nodes[1].temperature,
        "longer bars should heat more slowly over the same finite time window"
    );
}

#[derive(Clone, Copy)]
struct HeatTransientCase {
    length: f64,
    area: f64,
    density: f64,
    specific_heat: f64,
    conductivity: f64,
    initial_temperature: f64,
    heat_load: f64,
    time_step: f64,
    steps: usize,
}

impl HeatTransientCase {
    fn request(self) -> SolveTransientHeatBar1dRequest {
        SolveTransientHeatBar1dRequest {
            nodes: vec![
                node("insulated-root", 0.0, true, self.initial_temperature, 0.0),
                node(
                    "loaded-tip",
                    self.length,
                    false,
                    self.initial_temperature,
                    self.heat_load,
                ),
            ],
            elements: vec![TransientHeatBar1dElementInput {
                id: "lumped-heat-capacity".to_string(),
                node_i: 0,
                node_j: 1,
                area: self.area,
                conductivity: self.conductivity,
                density: self.density,
                specific_heat: self.specific_heat,
            }],
            time_step: self.time_step,
            steps: self.steps,
        }
    }

    fn node_capacity(self) -> f64 {
        0.5 * self.density * self.specific_heat * self.area * self.length
    }

    fn conductance(self) -> f64 {
        self.conductivity * self.area / self.length
    }

    fn tip_temperature_at(self, step: usize) -> f64 {
        let capacity_rate = self.node_capacity() / self.time_step;
        let conductance = self.conductance();
        let decay = capacity_rate / (capacity_rate + conductance);
        let steady_temperature = self.initial_temperature + self.heat_load / conductance;
        steady_temperature
            + (self.initial_temperature - steady_temperature) * decay.powi(step as i32)
    }

    fn final_temperature(self) -> f64 {
        self.tip_temperature_at(self.steps)
    }

    fn initial_energy(self) -> f64 {
        self.node_capacity() * self.initial_temperature * 2.0
    }

    fn final_energy(self) -> f64 {
        self.node_capacity() * (self.initial_temperature + self.final_temperature())
    }

    fn final_gradient(self) -> f64 {
        (self.final_temperature() - self.initial_temperature) / self.length
    }

    fn final_heat_flux(self) -> f64 {
        -self.conductivity * self.final_gradient()
    }
}

fn assert_response(
    result: &kyuubiki_protocol::SolveTransientHeatBar1dResult,
    case: HeatTransientCase,
) {
    assert_eq!(result.history.len(), case.steps + 1);
    assert_close(result.final_time, case.steps as f64 * case.time_step);
    assert_close(result.nodes[0].temperature, case.initial_temperature);
    assert_close(result.nodes[1].temperature, case.final_temperature());
    assert_close(result.max_temperature, case.final_temperature().abs());
    assert_close(result.total_thermal_energy, case.final_energy());
    assert_close(result.max_heat_flux, case.final_heat_flux().abs());
    assert_close(
        result.elements[0].temperature_gradient,
        case.final_gradient(),
    );
    assert_close(result.elements[0].heat_flux, case.final_heat_flux());

    for step in &result.history {
        let expected_temperature = case.tip_temperature_at(step.step);
        let expected_energy =
            case.node_capacity() * (case.initial_temperature + expected_temperature);
        assert_close(step.time, step.step as f64 * case.time_step);
        assert_close(step.nodal_temperatures[0], case.initial_temperature);
        assert_close(step.nodal_temperatures[1], expected_temperature);
        assert_close(step.max_temperature, expected_temperature.abs());
        assert_close(step.total_thermal_energy, expected_energy);
    }
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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
