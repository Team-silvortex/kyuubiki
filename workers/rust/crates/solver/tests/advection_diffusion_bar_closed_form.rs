use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementInput, AdvectionDiffusionBar1dNodeInput,
    SolveAdvectionDiffusionBar1dRequest,
};
use kyuubiki_solver::solve_advection_diffusion_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn advection_diffusion_bar_1d_matches_closed_form_flux_across_peclet_regimes() {
    for case in [
        TransportCase {
            length: 1.0,
            left_concentration: 1.0,
            right_concentration: 0.2,
            diffusivity: 0.05,
            velocity: 0.02,
        },
        TransportCase {
            length: 2.0,
            left_concentration: 0.1,
            right_concentration: 0.9,
            diffusivity: 0.04,
            velocity: 0.24,
        },
    ] {
        let result = solve_advection_diffusion_bar_1d(&case.request())
            .expect("closed-form transport case should solve");
        let expected = case.expected();
        let element = &result.elements[0];

        assert_close(result.nodes[0].concentration, case.left_concentration);
        assert_close(result.nodes[1].concentration, case.right_concentration);
        assert_close(result.max_concentration, expected.max_concentration);
        assert_close(result.max_total_flux, expected.total_flux.abs());
        assert_close(result.max_peclet_number, expected.peclet_number);
        assert_close(
            element.average_concentration,
            expected.average_concentration,
        );
        assert_close(
            element.concentration_gradient,
            expected.concentration_gradient,
        );
        assert_close(element.diffusive_flux, expected.diffusive_flux);
        assert_close(element.advective_flux, expected.advective_flux);
        assert_close(element.total_flux, expected.total_flux);
        assert_close(element.peclet_number, expected.peclet_number);
    }
}

#[test]
fn advection_diffusion_bar_1d_reports_zero_advective_flux_for_zero_velocity() {
    let case = TransportCase {
        length: 1.5,
        left_concentration: 0.8,
        right_concentration: 0.2,
        diffusivity: 0.03,
        velocity: 0.0,
    };
    let result =
        solve_advection_diffusion_bar_1d(&case.request()).expect("zero-velocity case should solve");
    let element = &result.elements[0];

    assert_close(element.advective_flux, 0.0);
    assert_close(element.peclet_number, 0.0);
    assert_close(element.total_flux, element.diffusive_flux);
}

#[test]
fn advection_diffusion_bar_1d_tracks_diffusivity_and_velocity_scaling() {
    let baseline_case = TransportCase {
        length: 1.8,
        left_concentration: 0.9,
        right_concentration: 0.15,
        diffusivity: 0.06,
        velocity: 0.08,
    };
    let baseline = solve_advection_diffusion_bar_1d(&baseline_case.request())
        .expect("baseline transport scaling case should solve");
    let baseline_element = &baseline.elements[0];

    let diffusivity_scale = 1.5;
    let diffusive_case = TransportCase {
        diffusivity: baseline_case.diffusivity * diffusivity_scale,
        ..baseline_case
    };
    let diffusive = solve_advection_diffusion_bar_1d(&diffusive_case.request())
        .expect("diffusivity-scaled transport case should solve");
    let diffusive_element = &diffusive.elements[0];

    assert_close(
        diffusive.nodes[0].concentration,
        baseline.nodes[0].concentration,
    );
    assert_close(
        diffusive.nodes[1].concentration,
        baseline.nodes[1].concentration,
    );
    assert_close(
        diffusive_element.concentration_gradient,
        baseline_element.concentration_gradient,
    );
    assert_close(
        diffusive_element.diffusive_flux / baseline_element.diffusive_flux,
        diffusivity_scale,
    );
    assert_close(
        diffusive_element.advective_flux,
        baseline_element.advective_flux,
    );
    assert_close(
        diffusive_element.peclet_number / baseline_element.peclet_number,
        1.0 / diffusivity_scale,
    );

    let velocity_scale = 1.75;
    let advective_case = TransportCase {
        velocity: baseline_case.velocity * velocity_scale,
        ..baseline_case
    };
    let advective = solve_advection_diffusion_bar_1d(&advective_case.request())
        .expect("velocity-scaled transport case should solve");
    let advective_element = &advective.elements[0];

    assert_close(
        advective.nodes[0].concentration,
        baseline.nodes[0].concentration,
    );
    assert_close(
        advective.nodes[1].concentration,
        baseline.nodes[1].concentration,
    );
    assert_close(
        advective_element.concentration_gradient,
        baseline_element.concentration_gradient,
    );
    assert_close(
        advective_element.diffusive_flux,
        baseline_element.diffusive_flux,
    );
    assert_close(
        advective_element.advective_flux / baseline_element.advective_flux,
        velocity_scale,
    );
    assert_close(
        advective_element.peclet_number / baseline_element.peclet_number,
        velocity_scale,
    );

    let length_scale = 1.4;
    let longer_case = TransportCase {
        length: baseline_case.length * length_scale,
        ..baseline_case
    };
    let longer = solve_advection_diffusion_bar_1d(&longer_case.request())
        .expect("length-scaled transport case should solve");
    let longer_element = &longer.elements[0];

    assert_close(
        longer.nodes[0].concentration,
        baseline.nodes[0].concentration,
    );
    assert_close(
        longer.nodes[1].concentration,
        baseline.nodes[1].concentration,
    );
    assert_close(
        longer_element.average_concentration,
        baseline_element.average_concentration,
    );
    assert_close(
        longer_element.concentration_gradient / baseline_element.concentration_gradient,
        1.0 / length_scale,
    );
    assert_close(
        longer_element.diffusive_flux / baseline_element.diffusive_flux,
        1.0 / length_scale,
    );
    assert_close(
        longer_element.advective_flux,
        baseline_element.advective_flux,
    );
    assert_close(
        longer_element.peclet_number / baseline_element.peclet_number,
        length_scale,
    );
}

#[test]
fn advection_diffusion_bar_1d_tracks_internal_source_and_area_scaling() {
    let baseline_case = TransportSourceCase {
        half_length: 0.9,
        area: 0.04,
        left_concentration: 0.25,
        right_concentration: 0.75,
        diffusivity: 0.08,
        velocity: 0.03,
        source: 0.012,
    };
    let baseline = solve_advection_diffusion_bar_1d(&baseline_case.request())
        .expect("baseline transport source case should solve");
    assert_source_response(&baseline, baseline_case);

    let source_scale = 2.0;
    let sourced_case = TransportSourceCase {
        source: baseline_case.source * source_scale,
        ..baseline_case
    };
    let sourced = solve_advection_diffusion_bar_1d(&sourced_case.request())
        .expect("source-scaled transport case should solve");
    assert_source_response(&sourced, sourced_case);
    assert_close(
        sourced.nodes[1].concentration - baseline_case.no_source_mid_concentration(),
        (baseline.nodes[1].concentration - baseline_case.no_source_mid_concentration())
            * source_scale,
    );

    let area_scale = 2.5;
    let wider_case = TransportSourceCase {
        area: baseline_case.area * area_scale,
        ..baseline_case
    };
    let wider = solve_advection_diffusion_bar_1d(&wider_case.request())
        .expect("area-scaled transport case should solve");
    assert_source_response(&wider, wider_case);
    assert_close(
        wider.nodes[1].concentration - baseline_case.no_source_mid_concentration(),
        (baseline.nodes[1].concentration - baseline_case.no_source_mid_concentration())
            / area_scale,
    );
}

#[derive(Clone, Copy)]
struct TransportCase {
    length: f64,
    left_concentration: f64,
    right_concentration: f64,
    diffusivity: f64,
    velocity: f64,
}

#[derive(Clone, Copy)]
struct TransportSourceCase {
    half_length: f64,
    area: f64,
    left_concentration: f64,
    right_concentration: f64,
    diffusivity: f64,
    velocity: f64,
    source: f64,
}

impl TransportSourceCase {
    fn request(self) -> SolveAdvectionDiffusionBar1dRequest {
        SolveAdvectionDiffusionBar1dRequest {
            nodes: vec![
                transport_node("inlet", 0.0, true, self.left_concentration, 0.0),
                transport_node("source", self.half_length, false, 0.0, self.source),
                transport_node(
                    "outlet",
                    2.0 * self.half_length,
                    true,
                    self.right_concentration,
                    0.0,
                ),
            ],
            elements: vec![
                self.element("transport-left", 0, 1),
                self.element("transport-right", 1, 2),
            ],
        }
    }

    fn element(
        self,
        id: &str,
        node_i: usize,
        node_j: usize,
    ) -> AdvectionDiffusionBar1dElementInput {
        AdvectionDiffusionBar1dElementInput {
            id: id.to_string(),
            node_i,
            node_j,
            area: self.area,
            diffusivity: self.diffusivity,
            velocity: self.velocity,
        }
    }

    fn diffusion(self) -> f64 {
        self.diffusivity * self.area / self.half_length
    }

    fn advection(self) -> f64 {
        self.velocity * self.area * 0.5
    }

    fn no_source_mid_concentration(self) -> f64 {
        let diffusion = self.diffusion();
        let advection = self.advection();
        ((diffusion + advection) * self.left_concentration
            + (diffusion - advection) * self.right_concentration)
            / (2.0 * diffusion)
    }

    fn mid_concentration(self) -> f64 {
        self.no_source_mid_concentration() + self.source / (2.0 * self.diffusion())
    }

    fn expected_element(self, left: f64, right: f64) -> ExpectedTransport {
        let average_concentration = 0.5 * (left + right);
        let concentration_gradient = (right - left) / self.half_length;
        let diffusive_flux = -self.diffusivity * concentration_gradient;
        let advective_flux = self.velocity * average_concentration;
        let total_flux = diffusive_flux + advective_flux;
        let peclet_number = self.velocity.abs() * self.half_length / (2.0 * self.diffusivity);
        ExpectedTransport {
            average_concentration,
            concentration_gradient,
            diffusive_flux,
            advective_flux,
            total_flux,
            peclet_number,
            max_concentration: left.abs().max(right.abs()),
        }
    }
}

impl TransportCase {
    fn request(self) -> SolveAdvectionDiffusionBar1dRequest {
        SolveAdvectionDiffusionBar1dRequest {
            nodes: vec![
                node("inlet", 0.0, self.left_concentration),
                node("outlet", self.length, self.right_concentration),
            ],
            elements: vec![AdvectionDiffusionBar1dElementInput {
                id: "transport".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                diffusivity: self.diffusivity,
                velocity: self.velocity,
            }],
        }
    }

    fn expected(self) -> ExpectedTransport {
        let average_concentration = 0.5 * (self.left_concentration + self.right_concentration);
        let concentration_gradient =
            (self.right_concentration - self.left_concentration) / self.length;
        let diffusive_flux = -self.diffusivity * concentration_gradient;
        let advective_flux = self.velocity * average_concentration;
        let total_flux = diffusive_flux + advective_flux;
        let peclet_number = self.velocity.abs() * self.length / (2.0 * self.diffusivity);
        ExpectedTransport {
            average_concentration,
            concentration_gradient,
            diffusive_flux,
            advective_flux,
            total_flux,
            peclet_number,
            max_concentration: self
                .left_concentration
                .abs()
                .max(self.right_concentration.abs()),
        }
    }
}

struct ExpectedTransport {
    average_concentration: f64,
    concentration_gradient: f64,
    diffusive_flux: f64,
    advective_flux: f64,
    total_flux: f64,
    peclet_number: f64,
    max_concentration: f64,
}

fn node(id: &str, x: f64, concentration: f64) -> AdvectionDiffusionBar1dNodeInput {
    transport_node(id, x, true, concentration, 0.0)
}

fn transport_node(
    id: &str,
    x: f64,
    fix_concentration: bool,
    concentration: f64,
    source: f64,
) -> AdvectionDiffusionBar1dNodeInput {
    AdvectionDiffusionBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_concentration,
        concentration,
        source,
    }
}

fn assert_source_response(
    result: &kyuubiki_protocol::SolveAdvectionDiffusionBar1dResult,
    case: TransportSourceCase,
) {
    let mid_concentration = case.mid_concentration();
    let left_expected = case.expected_element(case.left_concentration, mid_concentration);
    let right_expected = case.expected_element(mid_concentration, case.right_concentration);

    assert_close(result.nodes[0].concentration, case.left_concentration);
    assert_close(result.nodes[1].concentration, mid_concentration);
    assert_close(result.nodes[2].concentration, case.right_concentration);
    assert_close(
        result.max_concentration,
        case.left_concentration
            .abs()
            .max(mid_concentration.abs())
            .max(case.right_concentration.abs()),
    );
    assert_close(
        result.elements[0].diffusive_flux,
        left_expected.diffusive_flux,
    );
    assert_close(
        result.elements[0].advective_flux,
        left_expected.advective_flux,
    );
    assert_close(result.elements[0].total_flux, left_expected.total_flux);
    assert_close(
        result.elements[1].diffusive_flux,
        right_expected.diffusive_flux,
    );
    assert_close(
        result.elements[1].advective_flux,
        right_expected.advective_flux,
    );
    assert_close(result.elements[1].total_flux, right_expected.total_flux);
    assert_close(
        result.elements[1].total_flux - result.elements[0].total_flux,
        case.source / case.area,
    );
    assert_close(
        result.max_total_flux,
        left_expected
            .total_flux
            .abs()
            .max(right_expected.total_flux.abs()),
    );
    assert_close(result.max_peclet_number, left_expected.peclet_number);
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
