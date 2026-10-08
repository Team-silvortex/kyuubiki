use kyuubiki_protocol::{
    AdvectionDiffusionBar1dScheme as Scheme, SolveAdvectionDiffusionBar1dRequest as Request,
};
use kyuubiki_solver::{
    solve_advection_diffusion_bar_1d as solve,
    solver_control::{SolverControl, SolverStage, with_solver_observer},
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn mesh(n: usize, velocity: f64) -> Request {
    serde_json::from_value(json!({
        "scheme":"upwind",
        "nodes":(0..=n).map(|i| json!({"x":i as f64/n as f64,"fix_concentration":i==0 || i==n,"concentration":if i==n {1.0} else {0.0}})).collect::<Vec<_>>(),
        "elements":(0..n).map(|i| json!({"node_i":i,"node_j":i+1,"area":1.0,"diffusivity":1.0,"velocity":velocity})).collect::<Vec<_>>()
    })).unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

fn profile(i: usize, n: usize, velocity: f64) -> f64 {
    if velocity == 0.0 {
        return i as f64 / n as f64;
    }
    if velocity < 0.0 {
        return 1.0 - profile(n - i, n, -velocity);
    }
    let log_r = (velocity / n as f64).ln_1p();
    (((i as f64 - n as f64) * log_r).exp() * (-(-(i as f64) * log_r).exp_m1()))
        / (-(-(n as f64) * log_r).exp_m1())
}

#[test]
fn high_peclet_upwind_is_monotone_and_matches_independent_discrete_solution() {
    for n in [2, 8, 64] {
        for velocity in [-1000.0, -20.0, -1.0, 0.0, 1.0, 20.0, 1000.0] {
            let result = solve(&mesh(n, velocity)).unwrap();
            for (i, node) in result.nodes.iter().enumerate() {
                close(node.concentration, profile(i, n, velocity));
                assert!((-1e-12..=1.0 + 1e-12).contains(&node.concentration));
            }
            assert!(
                result
                    .nodes
                    .windows(2)
                    .all(|w| w[0].concentration <= w[1].concentration + 1e-12)
            );
            for e in &result.elements {
                let s = e
                    .stabilization
                    .as_ref()
                    .expect("explicit stabilization receipt");
                close(s.artificial_diffusivity, velocity.abs() * e.length * 0.5);
                close(
                    s.stabilization_flux,
                    -s.artificial_diffusivity * e.concentration_gradient,
                );
                close(s.numerical_flux, e.total_flux + s.stabilization_flux);
            }
        }
    }
}

#[test]
fn central_default_is_preserved_and_does_not_silently_enable_upwind() {
    let mut request = mesh(2, 20.0);
    request.scheme = Scheme::Galerkin;
    let result = solve(&request).unwrap();
    close(result.nodes[1].concentration, -2.0);
    let json = serde_json::to_value(result).unwrap();
    assert!(json["input"].get("scheme").is_none());
    assert!(json.get("max_numerical_flux").is_none());
    assert!(json["elements"][0].get("stabilization").is_none());
}

#[test]
fn upwind_source_balance_separates_physical_and_numerical_flux() {
    let mut request = mesh(2, 20.0);
    request.nodes[1].source = 4.0;
    let result = solve(&request).unwrap();
    close(result.nodes[1].concentration, 0.25);
    close(result.elements[0].total_flux, 2.0);
    close(result.elements[1].total_flux, 11.0);
    close(
        result.elements[0]
            .stabilization
            .as_ref()
            .unwrap()
            .numerical_flux,
        -0.5,
    );
    close(
        result.elements[1]
            .stabilization
            .as_ref()
            .unwrap()
            .numerical_flux,
        3.5,
    );
    close(result.max_numerical_flux.unwrap(), 3.5);
    assert_eq!(
        serde_json::to_value(result).unwrap()["input"]["scheme"],
        "upwind"
    );
}

#[test]
fn layered_coefficients_preserve_conservative_nodal_balance_and_orientation() {
    for dense in [false, true] {
        let mut request = mesh(2, 3.0);
        request.nodes[1].source = 4.0;
        request.elements[0].area = 2.0;
        request.elements[1].area = 4.0;
        request.elements[1].diffusivity = 0.5;
        request.elements[1].velocity = 5.0;
        if dense {
            let mut edge = request.elements[0].clone();
            edge.id = "bypass".into();
            edge.node_j = 2;
            request.elements.push(edge);
        }
        for reversed in [false, true] {
            if reversed {
                for e in &mut request.elements {
                    std::mem::swap(&mut e.node_i, &mut e.node_j);
                }
            }
            let result = solve(&request).unwrap();
            close(result.nodes[1].concentration, 2.0 / 7.0);
            let flow = |i: usize| {
                result.elements[i]
                    .stabilization
                    .as_ref()
                    .unwrap()
                    .numerical_flux
                    * request.elements[i].area
            };
            close(flow(1) - flow(0), 4.0);
        }
    }
}

#[test]
fn upwind_refinement_converges_to_continuous_solution_with_first_order_error() {
    let mut previous: Option<f64> = None;
    for n in [16, 32, 64, 128] {
        let result = solve(&mesh(n, 2.0)).unwrap();
        let error = result
            .nodes
            .iter()
            .map(|node| {
                let exact = (2.0 * node.x).exp_m1() / 2.0_f64.exp_m1();
                (node.concentration - exact).abs()
            })
            .fold(0.0_f64, f64::max);
        if let Some(old) = previous {
            assert!((1.8..2.2).contains(&(old / error)), "{old}/{error}");
        }
        previous = Some(error);
        close(
            result.elements[0]
                .stabilization
                .as_ref()
                .unwrap()
                .artificial_diffusivity,
            1.0 / n as f64,
        );
    }
}

#[test]
fn zero_velocity_upwind_matches_diffusion_without_artificial_flux() {
    let upwind = solve(&mesh(8, 0.0)).unwrap();
    let mut request = mesh(8, 0.0);
    request.scheme = Scheme::Galerkin;
    let central = solve(&request).unwrap();
    assert_eq!(upwind.nodes, central.nodes);
    for e in &upwind.elements {
        let s = e.stabilization.as_ref().unwrap();
        close(s.artificial_diffusivity, 0.0);
        close(s.stabilization_flux, 0.0);
        close(s.numerical_flux, e.total_flux);
    }
}

#[test]
fn typed_scheme_rejects_unknown_wrong_typed_and_null_values() {
    let original = serde_json::to_value(mesh(2, 20.0)).unwrap();
    for value in [
        json!("auto"),
        json!("UPWIND"),
        json!(null),
        json!(true),
        json!({}),
    ] {
        let mut model = original.clone();
        model["scheme"] = value;
        assert!(serde_json::from_value::<Request>(model).is_err());
    }
    let mut old = original;
    old.as_object_mut().unwrap().remove("scheme");
    assert_eq!(
        serde_json::from_value::<Request>(old).unwrap().scheme,
        Scheme::Galerkin
    );
}

#[test]
fn upwind_result_cancellation_including_numerical_flux_summary_allows_recovery() {
    let mut request = mesh(100, 0.5);
    for node in &mut request.nodes {
        node.fix_concentration = true;
        node.concentration = 2.0;
    }
    for (stage, steps, occurrence) in [
        (SolverStage::ResultNodes, 64, 1),
        (SolverStage::ResultElements, 64, 1),
        (SolverStage::ResultNodeSummary, 101, 1),
        (SolverStage::ResultElementSummary, 100, 3),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let hits = Arc::new(AtomicUsize::new(0));
        let observed = hits.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage
                    && point.completed_steps == steps
                    && observed.fetch_add(1, Ordering::Relaxed) + 1 == occurrence
                {
                    cancel.request_cancel();
                }
            },
            || solve(&request),
        );
        assert!(result.unwrap_err().starts_with("solver cancelled"));
        assert_eq!(hits.load(Ordering::Relaxed), occurrence);
        close(solve(&request).unwrap().max_numerical_flux.unwrap(), 1.0);
    }
}
