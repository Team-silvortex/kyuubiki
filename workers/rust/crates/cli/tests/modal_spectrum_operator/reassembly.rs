use super::*;
use kyuubiki_protocol::SolveModalFrame2dRequest;
use std::{cell::Cell, rc::Rc};

fn graded(segments: usize, scale: f64) -> Value {
    let mut input = model_with_segments(segments, scale);
    for (i, element) in input["elements"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let fraction = (i as f64 + 0.5) / segments as f64;
        element["youngs_modulus"] = json!(scale.powi(3) * (0.7 + 0.6 * fraction));
        element["area"] = json!(1.0 + 0.3 * fraction);
        element["moment_of_inertia"] = json!(0.75 + 0.5 * fraction);
        element["density"] = json!(scale.recip() * (1.25 - 0.5 * fraction));
    }
    input
}

fn renumber(input: Value) -> Value {
    let mut input: SolveModalFrame2dRequest = serde_json::from_value(input).unwrap();
    let size = input.nodes.len();
    let order: Vec<_> = (0..size).map(|i| 37 * i % size).collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..size).collect::<Vec<_>>());
    let mut inverse = vec![0; size];
    for (new, &old) in order.iter().enumerate() {
        inverse[old] = new;
    }
    input.nodes = order.iter().map(|&old| input.nodes[old].clone()).collect();
    for element in &mut input.elements {
        element.node_i = inverse[element.node_i];
        element.node_j = inverse[element.node_j];
    }
    input.elements.reverse();
    serde_json::to_value(input).unwrap()
}

fn checked(value: &Value) -> SolveModalFrame2dResult {
    let restored: SolveModalFrame2dResult =
        serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap();
    assert_eq!(restored.modes.len(), 1);
    assert_eq!(restored.free_dofs.len(), 192);
    let residual = reference::reassembled_residual(&restored);
    assert!(residual.is_finite() && residual <= 1e-8, "{residual:e}");
    let norm = restored.modes[0]
        .shape
        .iter()
        .fold(0.0_f64, |n, v| n.hypot(*v));
    assert!((norm - 1.0).abs() < 1e-10);
    assert!((restored.modes[0].participation_norm - norm).abs() < 1e-14);
    restored
}

#[test]
fn headless_planar_reassembly_preserves_payload_mapping_and_checked_physical_shapes() {
    for scale in [1.0, 1e-10] {
        let baseline = checked(&planned_solve(false, graded(96, scale)).unwrap());
        let input = renumber(graded(96, scale));
        let result = planned_solve(false, input.clone()).unwrap();
        assert_eq!(result["input"], input);
        let result = checked(&result);
        assert_eq!(
            result.modes[0].eigenvalue_rad_s_squared.to_bits(),
            baseline.modes[0].eigenvalue_rad_s_squared.to_bits()
        );
        for (new, node) in result.input.nodes.iter().enumerate() {
            let old = baseline
                .input
                .nodes
                .iter()
                .position(|n| n.id == node.id)
                .unwrap();
            for axis in 0..3 {
                assert_eq!(
                    result.modes[0].shape[3 * new + axis].to_bits(),
                    baseline.modes[0].shape[3 * old + axis].to_bits()
                );
            }
        }
    }
}

#[test]
fn headless_planar_reassembly_restore_cancellation_and_numerical_failure_replay() {
    let input = renumber(graded(96, 1e-10));
    let baseline = planned_solve(false, input.clone()).unwrap();
    let observed = Rc::new(Cell::new(false));
    let saw_restore = observed.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResultNodes && point.completed_steps == 97 {
                saw_restore.set(true);
                cancel.request_cancel();
            }
        },
        || planned_solve(false, input.clone()),
    )
    .unwrap_err();
    assert!(observed.get() && error.contains("cancel"), "{error}");
    assert_eq!(planned_solve(false, input.clone()).unwrap(), baseline);
    let error = planned_solve(false, renumber(graded(128, 1e14))).unwrap_err();
    assert!(
        error.contains("normalized modal roundoff recovery failed"),
        "{error}"
    );
    let replay = planned_solve(false, input).unwrap();
    assert_eq!(replay, baseline);
    checked(&replay);
}
