use super::{Fixture, Profile};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_protocol::{SolveModalFrame2dRequest, SolveModalFrame2dResult};
use std::{cell::Cell, rc::Rc};

fn renumber(input: &SolveModalFrame2dRequest, key: u64) -> SolveModalFrame2dRequest {
    let size = input.nodes.len();
    let order = match key {
        0 => (0..size).collect::<Vec<_>>(),
        1 => (0..size).rev().collect(),
        key => super::super::super::tests::shuffled(size, key),
    };
    let mut inverse = vec![0; size];
    for (new, &old) in order.iter().enumerate() {
        inverse[old] = new;
    }
    let mut mapped = input.clone();
    mapped.nodes = order.iter().map(|&old| input.nodes[old].clone()).collect();
    for element in &mut mapped.elements {
        element.node_i = inverse[element.node_i];
        element.node_j = inverse[element.node_j];
    }
    if key == 1 {
        mapped.elements.reverse();
    } else if key > 1 {
        let order = super::super::super::tests::shuffled(mapped.elements.len(), key);
        mapped.elements = order.iter().map(|&i| mapped.elements[i].clone()).collect();
    }
    mapped
}

fn compare(original: &SolveModalFrame2dResult, mapped: &SolveModalFrame2dResult) {
    Fixture::check_public_readback(mapped);
    assert_eq!(mapped.modes.len(), original.modes.len());
    let root = original.modes[0].eigenvalue_rad_s_squared;
    assert_eq!(
        mapped.modes[0].eigenvalue_rad_s_squared.to_bits(),
        root.to_bits()
    );
    assert!((mapped.total_mass / original.total_mass - 1.0).abs() < 1e-14);
    let expected: Vec<_> = mapped
        .input
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(i, node)| {
            [node.fix_x, node.fix_y, node.fix_rz]
                .into_iter()
                .enumerate()
                .filter(|(_, fixed)| !fixed)
                .map(move |(axis, _)| 3 * i + axis)
        })
        .collect();
    assert_eq!(mapped.free_dofs, expected);
    let mut restored = vec![0.0; original.modes[0].shape.len()];
    for (new, node) in mapped.input.nodes.iter().enumerate() {
        let old = original
            .input
            .nodes
            .iter()
            .position(|n| n.id == node.id)
            .unwrap();
        restored[3 * old..3 * old + 3]
            .copy_from_slice(&mapped.modes[0].shape[3 * new..3 * new + 3]);
    }
    let overlap: f64 = restored
        .iter()
        .zip(&original.modes[0].shape)
        .map(|(a, b)| a * b)
        .sum();
    let sign = 1.0_f64.copysign(overlap);
    let difference = restored
        .iter()
        .zip(&original.modes[0].shape)
        .fold(0.0_f64, |norm, (a, b)| norm.hypot(sign * a - b));
    assert_eq!(
        difference, 0.0,
        "reassembled shape difference={difference:e}"
    );
    assert!(
        restored
            .iter()
            .zip(&original.modes[0].shape)
            .all(|(a, b)| (sign * a).to_bits() == b.to_bits())
    );
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Accepted,
    NormalizedRejection,
    PhysicalRejection,
}

fn expected(profile: Profile, segments: usize, scale: f64) -> Outcome {
    match (profile, segments) {
        (Profile::Graded | Profile::Layered, 128) => Outcome::NormalizedRejection,
        (Profile::Layered, 96) if scale == 1e-10 => Outcome::PhysicalRejection,
        (Profile::UnequalLengths, 128) if scale != 1e14 => Outcome::PhysicalRejection,
        _ => Outcome::Accepted,
    }
}

fn check_outcome(result: &Result<SolveModalFrame2dResult, String>, expected: Outcome) {
    match expected {
        Outcome::Accepted => {
            Fixture::check_public_readback(result.as_ref().unwrap());
        }
        Outcome::NormalizedRejection | Outcome::PhysicalRejection => {
            let prefix = if expected == Outcome::NormalizedRejection {
                "normalized modal roundoff recovery failed:"
            } else {
                "physical modal roundoff recovery failed:"
            };
            let error = result.as_ref().unwrap_err();
            assert!(
                error.starts_with(prefix) && error.contains("unchanged residual gate"),
                "{error}"
            );
        }
    }
}

#[test]
fn modal_public_reassembly_heterogeneous_owned_borrowed_outcomes_and_physical_readback() {
    let mut counts = [0; 3];
    for profile in [Profile::Graded, Profile::Layered, Profile::UnequalLengths] {
        for segments in [64, 96, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let input = Fixture::request(profile, segments, scale);
                let baseline = crate::solve_modal_frame_2d(&input);
                let expected = expected(profile, segments, scale);
                check_outcome(&baseline, expected);
                for key in [0, 1, 7, 113] {
                    let request = renumber(&input, key);
                    crate::modal_frame_validation::validate_modal_frame_2d_request(&request)
                        .unwrap();
                    let original_json = serde_json::to_vec(&request).unwrap();
                    let borrowed = if key == 0 {
                        baseline.clone()
                    } else {
                        crate::solve_modal_frame_2d(&request)
                    };
                    let owned = crate::solve_modal_frame_2d_owned(request.clone());
                    assert_eq!(borrowed, owned);
                    for result in [borrowed, owned] {
                        counts[expected as usize] += 1;
                        check_outcome(&result, expected);
                        match result {
                            Ok(result) => {
                                assert_eq!(
                                    serde_json::to_vec(&result.input).unwrap(),
                                    original_json
                                );
                                compare(baseline.as_ref().unwrap(), &result);
                            }
                            Err(error) => assert_eq!(error, *baseline.as_ref().unwrap_err()),
                        }
                    }
                    assert_eq!(serde_json::to_vec(&request).unwrap(), original_json);
                    println!(
                        "public reassembly profile={} segments={segments} scale={scale:e} key={key} outcome={expected:?}",
                        profile.name()
                    );
                }
            }
        }
    }
    assert_eq!(counts, [144, 48, 24]);
    println!(
        "public reassembly fixtures=27 layouts=108 entrypoint_runs=216 accepted=144 normalized_rejections=48 physical_rejections=24"
    );
}

#[test]
fn modal_public_reassembly_late_restore_cancellation_and_failures_preserve_fresh_replay() {
    let request = renumber(&Fixture::request(Profile::Graded, 96, 1e-10), 113);
    let unchanged = serde_json::to_vec(&request).unwrap();
    let baseline = crate::solve_modal_frame_2d(&request).unwrap();
    Fixture::check_public_readback(&baseline);
    let steps = request.nodes.len() as u64;
    let control = SolverControl::default();
    let cancel = control.clone();
    let observed = Rc::new(Cell::new(false));
    let saw_restore = observed.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResultNodes && point.completed_steps == steps {
                saw_restore.set(true);
                cancel.request_cancel();
            }
        },
        || crate::solve_modal_frame_2d(&request),
    );
    assert!(observed.get());
    assert!(result.unwrap_err().contains("cancel"));
    for (profile, segments, scale) in [
        (Profile::Graded, 128, 1e14),
        (Profile::UnequalLengths, 128, 1e-10),
    ] {
        let failing = renumber(&Fixture::request(profile, segments, scale), 7);
        check_outcome(
            &crate::solve_modal_frame_2d(&failing),
            expected(profile, segments, scale),
        );
        let replay = crate::solve_modal_frame_2d(&request).unwrap();
        Fixture::check_public_readback(&replay);
        assert_eq!(replay, baseline);
        assert_eq!(serde_json::to_vec(&request).unwrap(), unchanged);
    }
    let mut invalid = request.clone();
    invalid.elements[0].node_j = invalid.nodes.len();
    assert!(
        crate::solve_modal_frame_2d(&invalid)
            .unwrap_err()
            .contains("node")
    );
    assert_eq!(
        crate::solve_modal_frame_2d_owned(request).unwrap(),
        baseline
    );
}
