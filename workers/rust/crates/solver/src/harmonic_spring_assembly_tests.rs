use super::*;
use serde_json::json;

#[test]
fn harmonic_assembly_keeps_small_diagonal_terms_before_inertia_cancellation() {
    for damping in [0.0, 2.0] {
        for reverse_order in [false, true] {
            let mut request: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
                "nodes":[
                    {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
                    {"id":"tip","x":1.0,"fix_x":false,"mass":2.0_f64.powi(55),"load_x":1.0}
                ],"elements":[
                    {"id":"soft","node_i":0,"node_j":1,"stiffness":1.0,"damping":damping},
                    {"id":"hard","node_i":0,"node_j":1,"stiffness":2.0_f64.powi(55)}
                ],"frequencies_hz":[1.0 / std::f64::consts::TAU]
            }))
            .unwrap();
            if reverse_order {
                request.elements.reverse();
            }
            let layout = reduced_layout(&request).unwrap();
            let (dense, _) = assemble_reduced_dynamic_stiffness(
                &request,
                1.0,
                &layout.free,
                &layout.reduced_index,
            )
            .unwrap();
            let path = assemble_path_dynamic_stiffness(
                &request,
                1.0,
                &layout,
                layout.path_position.as_ref().unwrap(),
            )
            .unwrap();
            for value in [dense[0][0], path.diagonal[0]] {
                assert_eq!(value.re, 1.0, "damping={damping}, reversed={reverse_order}");
                assert_eq!(value.im, damping);
            }
        }
    }
}

#[test]
fn harmonic_assembly_parallel_members_keep_correctly_rounded_symmetric_off_diagonals() {
    let mut request: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"one","x":1.0,"fix_x":false,"mass":1.0,"load_x":1.0},
            {"id":"two","x":2.0,"fix_x":false,"mass":1.0,"load_x":0.0}
        ],"elements":[],"frequencies_hz":[1.0 / std::f64::consts::TAU]
    }))
    .unwrap();
    for stiffness in [1.0, 2.0_f64.powi(-53), f64::from_bits(1)] {
        request.elements.push(TransientSpring1dElementInput {
            id: format!("link-{stiffness:e}"),
            node_i: 1,
            node_j: 2,
            stiffness,
            damping: stiffness,
        });
    }
    for _ in 0..2 {
        let layout = reduced_layout(&request).unwrap();
        let (matrix, _) =
            assemble_reduced_dynamic_stiffness(&request, 1.0, &layout.free, &layout.reduced_index)
                .unwrap();
        let path = assemble_path_dynamic_stiffness(
            &request,
            1.0,
            &layout,
            layout.path_position.as_ref().unwrap(),
        )
        .unwrap();
        for value in [matrix[0][1], matrix[1][0], path.lower[0], path.upper[0]] {
            assert_eq!(value.re, -1.0_f64.next_up());
            assert_eq!(value.im, -1.0_f64.next_up());
        }
        for value in [
            matrix[0][0],
            matrix[1][1],
            path.diagonal[0],
            path.diagonal[1],
        ] {
            assert_eq!(value.re, 2.0_f64.powi(-53));
            assert_eq!(value.im, 1.0_f64.next_up());
        }
        request.elements.reverse();
    }
}
