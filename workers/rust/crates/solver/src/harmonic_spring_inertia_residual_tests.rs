use super::*;
use crate::harmonic_spring_linear::arithmetic::CoefficientSum;
use serde_json::json;

#[test]
fn harmonic_inertia_precision_original_equation_visits_every_inertia_part() {
    let d = 2.0_f64.powi(-27);
    let omega = 1.0 + d;
    let expected = 5.0 * 2.0_f64.powi(-54) - 2.0_f64.powi(-81);
    let request: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":omega,"load_x":expected}
        ],"elements":[
            {"id":"spring","node_i":0,"node_j":1,"stiffness":1.0+3.0*d+2.0_f64.powi(-51)}
        ],"frequencies_hz":[omega/std::f64::consts::TAU]
    }))
    .unwrap();
    let displacement = [Complex::default(), Complex::real(1.0)];
    let reduced = [None, Some(0)];
    let mut sum = CoefficientSum::default();
    visit_terms(&request, omega, &displacement, &reduced, |row, products| {
        assert_eq!(row, 0);
        for (index, product) in products.into_iter().enumerate().take(2) {
            let sign = if index == 0 { 1.0 } else { -1.0 };
            sum.add(sign * restore(product.high, product.exponent))?;
            sum.add(sign * restore(product.low, product.exponent))?;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(sum.finish().unwrap(), expected);
    validate_harmonic_equilibrium(&request, omega, &displacement, &reduced).unwrap();
}
