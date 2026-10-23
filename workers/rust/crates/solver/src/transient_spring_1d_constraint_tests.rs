use super::*;
use serde_json::json;

fn request(scale: f64, sign: f64) -> SolveTransientSpring1dRequest {
    serde_json::from_value(json!({
        "nodes": [
            {"id":"root", "x":0.0, "fix_x":true, "mass":scale,
             "load_x":17.0 * sign * scale, "initial_displacement":0.0, "initial_velocity":0.0},
            {"id":"a", "x":1.0, "fix_x":false, "mass":scale,
             "load_x":2.0 * sign * scale, "initial_displacement":0.0, "initial_velocity":0.0},
            {"id":"b", "x":2.0, "fix_x":false, "mass":scale,
             "load_x":9.0 * sign * scale, "initial_displacement":0.0, "initial_velocity":0.0}
        ],
        "elements": [
            {"id":"ka", "node_i":0, "node_j":1, "stiffness":2.0 * scale, "damping":0.0},
            {"id":"kb", "node_i":0, "node_j":2, "stiffness":5.0 * scale, "damping":0.0}
        ],
        "time_step":1.0, "steps":1
    }))
    .unwrap()
}

fn first_step(
    request: &SolveTransientSpring1dRequest,
    scatter: &[usize],
) -> Result<NewmarkStateVectors, String> {
    let coefficients = newmark_coefficients(request.time_step)?;
    let effective = assemble_effective_system(request, coefficients)?;
    let (reduced, _, _) = reduce_sparse_system(
        &effective,
        &vec![0.0; request.nodes.len()],
        &constrained_dofs(request),
    )?;
    let solver = PreparedSpdSolver::factor(reduced)?;
    let mass = request
        .nodes
        .iter()
        .map(|node| node.mass)
        .collect::<Vec<_>>();
    let zero = vec![0.0; request.nodes.len()];
    let acceleration = state_acceleration(request, &zero, &zero, "initial")?;
    newmark_step(
        &NewmarkSystem {
            time_step: request.time_step,
            coefficients,
            mass: &mass,
            elements: &request.elements,
            free: scatter,
            solver: &solver,
            validator: PrescribedSystemValidator::new_homogeneous(
                &effective,
                &constrained_dofs(request),
            )?,
            request,
        },
        NewmarkState {
            displacement: &zero,
            velocity: &zero,
            acceleration: &acceleration,
        },
    )
}

#[test]
fn newmark_original_constraints_reject_permuted_scatter_after_reduced_success() {
    let request = request(1.0, 1.0);
    first_step(&request, &[1, 2]).unwrap();
    let error = first_step(&request, &[2, 1]).unwrap_err();
    assert!(error.contains("original constrained-system row"), "{error}");
    first_step(&request, &[1, 2]).unwrap();
}

#[test]
fn newmark_original_constraints_reject_scatter_into_a_fixed_component() {
    let mut request = request(1.0, 1.0);
    request.elements[1].node_i = 1;
    let error = first_step(&request, &[0, 2]).unwrap_err();
    assert!(
        error.contains("prescribed value changed at DOF 0"),
        "{error}"
    );
    first_step(&request, &[1, 2]).unwrap();
}

#[test]
fn newmark_original_constraints_preserve_signed_scaled_closed_form_increments() {
    for exponent in [-300, 0, 300] {
        for sign in [-1.0, 1.0] {
            let request = request(2.0_f64.powi(exponent), sign);
            let (u, v, a) = first_step(&request, &[1, 2]).unwrap();
            for (actual, expected) in [
                (u[1], sign * 2.0 / 3.0),
                (u[2], sign * 2.0),
                (v[1], sign * 4.0 / 3.0),
                (v[2], sign * 4.0),
                (a[1], sign * 2.0 / 3.0),
                (a[2], -sign),
            ] {
                assert!(
                    (actual - expected).abs() < 2.0e-14,
                    "{actual} != {expected}"
                );
            }
            assert_eq!([u[0], v[0], a[0]], [0.0; 3]);
            let borrowed = solve_transient_spring_1d(&request).unwrap();
            let owned = solve_transient_spring_1d_owned(request).unwrap();
            assert_eq!(
                serde_json::to_value(&borrowed).unwrap(),
                serde_json::to_value(owned).unwrap()
            );
            for (index, node) in borrowed.nodes.iter().enumerate() {
                assert_eq!([node.ux, node.vx, node.ax], [u[index], v[index], a[index]]);
            }
        }
    }
}
