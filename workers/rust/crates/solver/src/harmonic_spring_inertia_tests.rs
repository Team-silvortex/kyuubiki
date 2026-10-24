use super::*;
use serde_json::json;

#[test]
fn harmonic_inertia_precision_assembly_keeps_the_unrounded_triple_product() {
    let d = 2.0_f64.powi(-27);
    let stiffness = 1.0 + 3.0 * d + 2.0_f64.powi(-51);
    let diagonal = 5.0 * 2.0_f64.powi(-54) - 2.0_f64.powi(-81);
    // (1+d)^3 = 1 + 3d + 3d^2 + d^3, so k - omega^2*m = diagonal.
    for power in [-500, -200, 0, 200, 500] {
        let omega = (1.0 + d) * 2.0_f64.powi(power);
        let mass = (1.0 + d) * 2.0_f64.powi(-2 * power);
        let request: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
            "nodes":[
                {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
                {"id":"tip","x":1.0,"fix_x":false,"mass":mass,"load_x":diagonal}
            ],"elements":[
                {"id":"spring","node_i":0,"node_j":1,"stiffness":stiffness}
            ],"frequencies_hz":[omega / std::f64::consts::TAU]
        }))
        .unwrap();
        assert_eq!(std::f64::consts::TAU * request.frequencies_hz[0], omega);
        let layout = reduced_layout(&request).unwrap();
        let (matrix, _) = assemble_reduced_dynamic_stiffness(
            &request,
            omega,
            &layout.free,
            &layout.reduced_index,
        )
        .unwrap();
        let path = assemble_path_dynamic_stiffness(
            &request,
            omega,
            &layout,
            layout.path_position.as_ref().unwrap(),
        )
        .unwrap();
        for value in [matrix[0][0], path.diagonal[0]] {
            assert_eq!(
                value.re.to_bits(),
                diagonal.to_bits(),
                "power={power}: {value:?}"
            );
            assert_eq!(value.im, 0.0);
        }
    }
}
