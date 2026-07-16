const REL_TOL: f64 = 1.0e-11;
const ABS_TOL: f64 = 1.0e-12;

pub(crate) fn assert_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= ABS_TOL.max(REL_TOL * scale),
        "{label}: expected {actual} to be close to {expected}",
    );
}
