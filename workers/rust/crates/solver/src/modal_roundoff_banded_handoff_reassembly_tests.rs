use super::super::super::reassembly_tests::renumber;
use super::super::{
    Fixture, Order, Selection, reference as inertia, search_canonical_direction_ranked,
};
use super::{Prepared, Profile, bits, reference::Factor, rounded};
use crate::modal_frame_validation::validate_modal_frame_2d_request;
use crate::solver_control::{
    SolverControl, SolverStage, checkpoint, checkpoint_chunk, with_solver_observer,
};
use kyuubiki_protocol::{SolveModalFrame2dRequest, SolveModalFrame2dResult};
use std::{cell::Cell, rc::Rc};

#[path = "modal_roundoff_material_anchor_tests.rs"]
mod material_anchor;

// Test-only request reconstruction, not a new production topology/ordering path.
struct Rebuilt {
    original: SolveModalFrame2dRequest,
    canonical: SolveModalFrame2dRequest,
    old_nodes: Vec<usize>,
}

impl Rebuilt {
    fn prepare(input: &SolveModalFrame2dRequest) -> Result<Self, String> {
        validate_modal_frame_2d_request(input)?;
        if input.mode_count != Some(1) || input.nodes.len() != 129 {
            return Err(
                "candidate reassembly requires the retained 128-element single-mode size".into(),
            );
        }
        Self::rebuild_validated(input)
    }

    fn prepare_bounded(input: &SolveModalFrame2dRequest) -> Result<Self, String> {
        validate_modal_frame_2d_request(input)?;
        if input.mode_count != Some(1) || !(2..=129).contains(&input.nodes.len()) {
            return Err("candidate reassembly requires 1..=128 single-mode chain members".into());
        }
        Self::rebuild_validated(input)
    }

    fn rebuild_validated(input: &SolveModalFrame2dRequest) -> Result<Self, String> {
        let y = input.nodes[0].y;
        if input
            .nodes
            .iter()
            .any(|n| !n.fix_x || n.y != y || n.fix_y != n.fix_rz)
        {
            return Err(
                "candidate reassembly requires a restrained horizontal bending chain".into(),
            );
        }
        let mut old_nodes: Vec<_> = (0..input.nodes.len()).collect();
        old_nodes.sort_unstable_by(|&a, &b| input.nodes[a].x.total_cmp(&input.nodes[b].x));
        if old_nodes
            .windows(2)
            .any(|p| input.nodes[p[0]].x == input.nodes[p[1]].x)
            || !input.nodes[old_nodes[0]].fix_y
            || old_nodes[1..].iter().any(|&i| input.nodes[i].fix_y)
        {
            return Err(
                "candidate reassembly requires distinct coordinates and the left end restrained"
                    .into(),
            );
        }
        let mut inverse = vec![0; old_nodes.len()];
        for (new, &old) in old_nodes.iter().enumerate() {
            inverse[old] = new;
        }
        let mut canonical = input.clone();
        canonical.nodes = old_nodes
            .iter()
            .map(|&old| input.nodes[old].clone())
            .collect();
        let mut seen = vec![false; canonical.nodes.len() - 1];
        for element in &mut canonical.elements {
            element.node_i = inverse[element.node_i];
            element.node_j = inverse[element.node_j];
            if element.node_j != element.node_i + 1 || seen[element.node_i] {
                return Err(
                    "candidate reassembly requires every forward adjacent member exactly once"
                        .into(),
                );
            }
            seen[element.node_i] = true;
        }
        if seen.iter().any(|&v| !v) {
            return Err("candidate reassembly requires a complete adjacent chain".into());
        }
        // Rebuild from wire data; never permute a retained matrix or eigenvector.
        canonical = serde_json::from_slice(&serde_json::to_vec(&canonical).unwrap()).unwrap();
        validate_modal_frame_2d_request(&canonical)?;
        Ok(Self {
            original: input.clone(),
            canonical,
            old_nodes,
        })
    }

    fn restore(
        &self,
        mut result: SolveModalFrame2dResult,
    ) -> Result<SolveModalFrame2dResult, String> {
        assert_eq!(result.input, self.canonical);
        assert_eq!(result.modes.len(), 1);
        let mut shape = vec![0.0; result.modes[0].shape.len()];
        for (new, &old) in self.old_nodes.iter().enumerate() {
            shape[3 * old..3 * old + 3]
                .copy_from_slice(&result.modes[0].shape[3 * new..3 * new + 3]);
            checkpoint_chunk(SolverStage::ResultNodes, new + 1, self.old_nodes.len())?;
        }
        result.modes[0].shape = shape;
        result.modes[0].participation_norm =
            crate::modal_math::checked_shape_norm(&result.modes[0].shape)?;
        if (result.modes[0].participation_norm - 1.0).abs() >= 1e-10 {
            return Err("candidate reassembly lost its original-coordinate unit norm".into());
        }
        for dof in &mut result.free_dofs {
            *dof = 3 * self.old_nodes[*dof / 3] + *dof % 3;
        }
        result.free_dofs.sort_unstable();
        result.input = self.original.clone();
        checkpoint(SolverStage::ResultTotals, 1)?;
        Ok(result)
    }
}

#[derive(Debug, PartialEq)]
struct Receipt {
    root: u64,
    seed: Vec<u64>,
    orders: [Order; 2],
    factors: [usize; 2],
    certificates: [usize; 2],
    inertia: [f64; 2],
    output: SolveModalFrame2dResult,
}

#[derive(Clone, Copy, Debug)]
enum PhysicalPolicy {
    Handoff,
    IndependentRanked,
    IndependentFull,
}

fn candidate(layout: &Rebuilt) -> Receipt {
    propose(layout, PhysicalPolicy::Handoff).unwrap()
}

fn propose(layout: &Rebuilt, policy: PhysicalPolicy) -> Result<Receipt, String> {
    let fixture = Prepared::from_request(layout.canonical.clone());
    let original = bits(&fixture.seed);
    let root = fixture.value.to_bits();
    let seed = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)?,
    );
    let internal = search_canonical_direction_ranked(&fixture.directions, &seed, 1e-8, |v| {
        fixture.checked(v)
    })?;
    let selection = match policy {
        PhysicalPolicy::Handoff => Selection::Single(internal.order),
        PhysicalPolicy::IndependentRanked => Selection::RankedThenReverse,
        PhysicalPolicy::IndependentFull => Selection::Full,
    };
    let mut best = f64::INFINITY;
    let mut calls = 0;
    let physical = fixture.physical_candidate_observed(
        &internal.shape,
        selection,
        || calls += 1,
        |r| best = best.min(r),
    );
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), original);
    let physical = physical
        .map_err(|error| {
            assert!(best.is_finite() && best > 1e-8);
            format!(
                "physical proposal policy={policy:?} internal_order={:?} best_physical_residual={best:e} actual_checks={calls}: {error}",
                internal.order
            )
        })?;
    assert!(internal.usage.attempts <= 2 && internal.usage.certificates <= 13);
    let (max_factors, max_certificates) = match policy {
        PhysicalPolicy::Handoff => (1, 7),
        PhysicalPolicy::IndependentRanked => (2, 13),
        PhysicalPolicy::IndependentFull => (3, 19),
    };
    assert!(physical.usage.attempts <= max_factors);
    assert!(physical.usage.certificates <= max_certificates);
    let (lower, upper) = inertia::bracket(&fixture.directions, fixture.value).unwrap();
    assert!((upper - lower) / fixture.value < 1e-14);
    assert!((fixture.value / (lower + (upper - lower) * 0.5) - 1.0).abs() < 1e-12);
    Ok(Receipt {
        root,
        seed: bits(&seed),
        orders: [internal.order, physical.order],
        factors: [internal.usage.attempts, physical.usage.attempts],
        certificates: [internal.usage.certificates, physical.usage.certificates],
        inertia: [lower, upper],
        output: Fixture::canonical_recovered_result(
            &fixture.input,
            fixture.total_mass,
            fixture.value,
            &physical.shape,
        ),
    })
}

fn run(input: &SolveModalFrame2dRequest) -> Receipt {
    try_run(input, PhysicalPolicy::Handoff).unwrap()
}

fn try_run(input: &SolveModalFrame2dRequest, policy: PhysicalPolicy) -> Result<Receipt, String> {
    let layout = Rebuilt::prepare(input)?;
    let mut receipt = propose(&layout, policy)?;
    receipt.output = layout.restore(receipt.output)?;
    Fixture::check_public_readback(&receipt.output);
    Ok(receipt)
}

fn compare(baseline: &Receipt, actual: &Receipt) {
    assert_eq!(actual.root, baseline.root);
    assert_eq!(actual.seed, baseline.seed);
    assert_eq!(actual.orders, baseline.orders);
    assert_eq!(actual.factors, baseline.factors);
    assert_eq!(actual.certificates, baseline.certificates);
    assert_eq!(actual.inertia, baseline.inertia);
    assert!((actual.output.total_mass / baseline.output.total_mass - 1.0).abs() < 1e-14);
    compare_shape(baseline, actual);
}

fn compare_shape(baseline: &Receipt, actual: &Receipt) {
    let expected: Vec<_> = actual
        .output
        .input
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(i, n)| {
            [n.fix_x, n.fix_y, n.fix_rz]
                .into_iter()
                .enumerate()
                .filter(|(_, fixed)| !fixed)
                .map(move |(axis, _)| 3 * i + axis)
        })
        .collect();
    assert_eq!(actual.output.free_dofs, expected);
    for (new, node) in actual.output.input.nodes.iter().enumerate() {
        let old = baseline
            .output
            .input
            .nodes
            .iter()
            .position(|n| n.id == node.id)
            .unwrap();
        assert_eq!(
            bits(&actual.output.modes[0].shape[3 * new..3 * new + 3]),
            bits(&baseline.output.modes[0].shape[3 * old..3 * old + 3])
        );
        for (axis, fixed) in [node.fix_x, node.fix_y, node.fix_rz]
            .into_iter()
            .enumerate()
        {
            if fixed {
                assert_eq!(
                    actual.output.modes[0].shape[3 * new + axis].to_bits(),
                    0.0_f64.to_bits()
                );
            }
        }
    }
}

// A reference-only existence witness, never an initializer or search fallback.
fn check_material_direction_witness(baseline: &Receipt, input: &SolveModalFrame2dRequest) {
    let layout = Rebuilt::prepare(input).unwrap();
    let fixture = Prepared::from_request(layout.canonical.clone());
    let direction: Vec<_> = layout
        .canonical
        .nodes
        .iter()
        .skip(1)
        .flat_map(|node| {
            let old = baseline
                .output
                .input
                .nodes
                .iter()
                .position(|n| n.id == node.id)
                .unwrap();
            [
                baseline.output.modes[0].shape[3 * old + 1],
                baseline.output.modes[0].shape[3 * old + 2],
            ]
        })
        .collect();
    let applied = fixture
        .system
        .operator
        .apply_physical_compensated(&direction)
        .unwrap();
    let residual = fixture
        .system
        .operator
        .physical_residual(fixture.value, &direction, &fixture.system.mass, &applied)
        .unwrap()
        .0;
    println!(
        "material reference_only_witness residual={residual:e} frozen_root={:e}",
        fixture.value
    );
    assert!(
        residual <= 1e-8,
        "reference-only physical witness failed: {residual:e}"
    );
    let output = Fixture::canonical_recovered_result(
        &fixture.input,
        fixture.total_mass,
        fixture.value,
        &direction,
    );
    let output = layout.restore(output).unwrap();
    Fixture::check_public_readback(&output);
    assert_eq!(&output.input, input);
}

#[test]
fn modal_banded_handoff_fresh_request_reassembly_and_original_numbered_readback() {
    let mut layouts = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let input = Fixture::request(profile, 128, scale);
            let baseline = run(&input);
            for key in [0, 1, 7, 113] {
                let input = renumber(&input, key);
                let unchanged = serde_json::to_vec(&input).unwrap();
                let input: SolveModalFrame2dRequest = serde_json::from_slice(&unchanged).unwrap();
                let actual = run(&input);
                compare(&baseline, &actual);
                assert_eq!(serde_json::to_vec(&actual.output.input).unwrap(), unchanged);
                assert_eq!(serde_json::to_vec(&input).unwrap(), unchanged);
                println!(
                    "handoff request profile={} scale={scale:e} key={key} factors={:?} certificates={:?}",
                    profile.name(),
                    actual.factors,
                    actual.certificates
                );
                layouts += 1;
            }
        }
    }
    assert_eq!(layouts, 24);
    println!(
        "banded handoff fresh_request_layouts=24 original_numbered_readbacks=24 exact_roots_seeds_orders_counts=24"
    );
}

#[test]
fn modal_banded_handoff_request_boundaries_and_last_restore_cancel_preserve_fresh_replay() {
    let input = renumber(&Fixture::request(Profile::Graded, 128, 1e14), 113);
    let unchanged = serde_json::to_vec(&input).unwrap();
    let baseline = run(&input);
    let declined: [fn(&mut SolveModalFrame2dRequest); 9] = [
        |r: &mut SolveModalFrame2dRequest| r.mode_count = Some(2),
        |r: &mut SolveModalFrame2dRequest| r.elements[0].node_j = r.nodes.len(),
        |r: &mut SolveModalFrame2dRequest| r.nodes[0].fix_x = false,
        |r: &mut SolveModalFrame2dRequest| r.nodes[0].y += 1.0,
        |r: &mut SolveModalFrame2dRequest| {
            r.elements.pop();
        },
        |r: &mut SolveModalFrame2dRequest| {
            let e = &mut r.elements[0];
            std::mem::swap(&mut e.node_i, &mut e.node_j);
        },
        |r: &mut SolveModalFrame2dRequest| {
            r.elements[0].node_i = r.elements[1].node_i;
            r.elements[0].node_j = r.elements[1].node_j;
        },
        |r: &mut SolveModalFrame2dRequest| r.nodes[0].x = r.nodes[1].x,
        |r: &mut SolveModalFrame2dRequest| {
            let n = r.nodes.iter_mut().find(|n| !n.fix_y).unwrap();
            n.fix_y = true;
            n.fix_rz = true;
        },
    ];
    for mutate in declined {
        let mut request = input.clone();
        mutate(&mut request);
        assert!(Rebuilt::prepare(&request).is_err());
    }
    let mut swapped = input.clone();
    swapped.elements.swap(0, 1);
    compare(&baseline, &run(&swapped));
    let layout = Rebuilt::prepare(&input).unwrap();
    let retained = candidate(&layout);
    let control = SolverControl::default();
    let cancel = control.clone();
    let saw = Rc::new(Cell::new(false));
    let observed = saw.clone();
    let last = input.nodes.len() as u64;
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ResultNodes && p.completed_steps == last {
                observed.set(true);
                cancel.request_cancel();
            }
        },
        || layout.restore(retained.output),
    )
    .unwrap_err();
    assert!(saw.get() && error.contains("cancel"));
    assert_eq!(run(&input), baseline);
    assert_eq!(serde_json::to_vec(&input).unwrap(), unchanged);
    println!(
        "banded handoff request_boundary_rejections=9 last_restore_cancellations=1 fresh_request_replays=1"
    );
}

#[test]
fn modal_banded_handoff_reassembled_material_ratios_preserve_spectral_and_shape_scaling() {
    let mut controls = 0;
    let mut handoff_rejections = 0;
    let mut ranked_rejections = 0;
    let mut full_readbacks = 0;
    let mut unresolved = 0;
    let mut witnesses = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let input = Fixture::request(profile, 128, scale);
            let baseline = run(&input);
            for (young, density) in [(4.0, 1.0), (1.0, 4.0), (16.0, 4.0)] {
                let mut changed = renumber(&input, 7);
                for e in &mut changed.elements {
                    e.youngs_modulus *= young;
                    e.density *= density;
                }
                let wire = serde_json::to_vec(&changed).unwrap();
                let changed: SolveModalFrame2dRequest = serde_json::from_slice(&wire).unwrap();
                let handoff = try_run(&changed, PhysicalPolicy::Handoff);
                // Separate cold routes: never start this one after a partial handoff in production.
                if let Err(error) = &handoff {
                    assert!(
                        matches!(profile, Profile::Layered) && scale == 1e-10 && density == 4.0,
                        "{error}"
                    );
                    assert!(
                        error.contains("policy=Handoff")
                            && error.contains("exhausted its bounded policies")
                            && error.contains("attempts=1"),
                        "{error}"
                    );
                    println!(
                        "handoff material rejected profile={} scale={scale:e} young={young} density={density} error={error}",
                        profile.name()
                    );
                    handoff_rejections += 1;
                }
                let actual = match try_run(&changed, PhysicalPolicy::IndependentRanked) {
                    Ok(actual) => actual,
                    Err(error) => {
                        assert!(handoff.is_err(), "{error}");
                        assert!(
                            error.contains("policy=IndependentRanked")
                                && error.contains("attempts=2")
                                && error.contains("exhausted its bounded policies"),
                            "{error}"
                        );
                        ranked_rejections += 1;
                        println!(
                            "ranked material rejected profile={} scale={scale:e} young={young} density={density} error={error}",
                            profile.name()
                        );
                        // A new request rebuild for diagnosis, not a live-route retry chain.
                        match try_run(&changed, PhysicalPolicy::IndependentFull) {
                            Ok(actual) => {
                                full_readbacks += 1;
                                actual
                            }
                            Err(error) => {
                                assert!(
                                    error.contains("policy=IndependentFull")
                                        && error.contains("attempts=3")
                                        && error.contains("exhausted its bounded policies"),
                                    "{error}"
                                );
                                println!(
                                    "full material rejected profile={} scale={scale:e} young={young} density={density} error={error}",
                                    profile.name()
                                );
                                assert_eq!(serde_json::to_vec(&changed).unwrap(), wire);
                                check_material_direction_witness(&baseline, &changed);
                                witnesses += 1;
                                compare(&baseline, &run(&input));
                                assert_eq!(serde_json::to_vec(&changed).unwrap(), wire);
                                unresolved += 1;
                                controls += 1;
                                continue;
                            }
                        }
                    }
                };
                if let Ok(handoff) = handoff {
                    assert_eq!(handoff.root, actual.root);
                    assert_eq!(handoff.seed, actual.seed);
                    assert_eq!(handoff.output, actual.output);
                }
                let expected_root = f64::from_bits(baseline.root) * (young / density);
                let root_error = (f64::from_bits(actual.root) / expected_root - 1.0).abs();
                assert!(
                    root_error < 1e-12,
                    "material root ratio error={root_error:e}"
                );
                let midpoint = |v: [f64; 2]| v[0] + (v[1] - v[0]) * 0.5;
                let inertia_error = (midpoint(actual.inertia)
                    / (midpoint(baseline.inertia) * (young / density))
                    - 1.0)
                    .abs();
                assert!(
                    inertia_error < 1e-13,
                    "independent inertia ratio error={inertia_error:e}"
                );
                assert!(
                    (actual.output.total_mass / (density * baseline.output.total_mass) - 1.0).abs()
                        < 1e-14
                );
                let expected_frequency =
                    baseline.output.modes[0].natural_frequency_hz * (young / density).sqrt();
                assert!(
                    (actual.output.modes[0].natural_frequency_hz / expected_frequency - 1.0).abs()
                        < 1e-12
                );
                let mut overlap = 0.0_f64;
                for (new, node) in actual.output.input.nodes.iter().enumerate() {
                    let old = baseline
                        .output
                        .input
                        .nodes
                        .iter()
                        .position(|n| n.id == node.id)
                        .unwrap();
                    for axis in 0..3 {
                        overlap += actual.output.modes[0].shape[3 * new + axis]
                            * baseline.output.modes[0].shape[3 * old + axis];
                    }
                }
                // Compare the eigendirection modulo global sign, without altering published shapes.
                let sign = 1.0_f64.copysign(overlap);
                let actual_norm =
                    crate::modal_math::checked_shape_norm(&actual.output.modes[0].shape).unwrap();
                let baseline_norm =
                    crate::modal_math::checked_shape_norm(&baseline.output.modes[0].shape).unwrap();
                let mut shape_difference = 0.0_f64;
                for (new, node) in actual.output.input.nodes.iter().enumerate() {
                    let old = baseline
                        .output
                        .input
                        .nodes
                        .iter()
                        .position(|n| n.id == node.id)
                        .unwrap();
                    for axis in 0..3 {
                        shape_difference = shape_difference.hypot(
                            sign * actual.output.modes[0].shape[3 * new + axis] / actual_norm
                                - baseline.output.modes[0].shape[3 * old + axis] / baseline_norm,
                        );
                    }
                }
                assert!(
                    shape_difference < 1e-10,
                    "material mode difference={shape_difference:e}"
                );
                assert_eq!(serde_json::to_vec(&actual.output.input).unwrap(), wire);
                assert_eq!(serde_json::to_vec(&changed).unwrap(), wire);
                println!(
                    "handoff material profile={} scale={scale:e} young={young} density={density} root_ratio={} root_error={root_error:e} inertia_error={inertia_error:e} direction_difference={shape_difference:e} sign={sign} factors={:?} certificates={:?}",
                    profile.name(),
                    young / density,
                    actual.factors,
                    actual.certificates,
                );
                controls += 1;
            }
        }
    }
    assert_eq!(controls, 18);
    assert_eq!(handoff_rejections, 2);
    assert_eq!(ranked_rejections, 2);
    assert_eq!(full_readbacks, 0);
    assert_eq!(unresolved, 2);
    assert_eq!(witnesses, 2);
    println!(
        "banded handoff fresh_material_controls=18 handoff_successes=16 handoff_rejections=2 independent_ranked_readbacks=16 independent_ranked_rejections=2 independent_full_readbacks={full_readbacks} unresolved={unresolved} checked_spectral_ratios={} independent_inertia_ratios={} reference_only_direction_witnesses={witnesses}",
        18 - unresolved,
        18 - unresolved
    );
}
