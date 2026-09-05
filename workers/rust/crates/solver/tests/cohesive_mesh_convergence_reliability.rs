#[path = "cohesive_mesh_convergence/plane.rs"]
mod plane;
#[path = "cohesive_mesh_convergence/surface.rs"]
mod surface;

fn assert_close(actual: f64, expected: f64) {
    assert!(actual.is_finite(), "non-finite result: {actual}");
    assert!(
        (actual - expected).abs() <= 1.0e-10 * expected.abs().max(1.0),
        "expected {expected:.12e}, got {actual:.12e}"
    );
}
