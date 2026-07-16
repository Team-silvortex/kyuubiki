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

#[derive(Clone, Copy)]
struct TransportCase {
    length: f64,
    left_concentration: f64,
    right_concentration: f64,
    diffusivity: f64,
    velocity: f64,
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
    AdvectionDiffusionBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_concentration: true,
        concentration,
        source: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
