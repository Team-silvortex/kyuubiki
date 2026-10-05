use super::super::super::super::super::{
    search_canonical_direction_inward, search_canonical_unit_shape_inward,
};
use super::super::super::{physical_directions, search_canonical_unit_shape_single};
use super::{
    Factor, Fixture, Order, Prepared, Profile, Rebuilt, bits, renumber, rounded,
    search_canonical_direction_ranked,
};
use crate::modal_frame_spectrum::checked_mode_shape;
use kyuubiki_protocol::SolveModalFrame2dRequest;

#[path = "modal_roundoff_material_inward_control_tests.rs"]
mod controls;

#[path = "modal_roundoff_material_holdout_tests.rs"]
mod holdouts;

fn internal(fixture: &Prepared) -> (Vec<f64>, super::super::super::Accepted) {
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    );
    let accepted = search_canonical_direction_ranked(&fixture.directions, &direction, 1e-8, |v| {
        fixture.checked(v)
    })
    .unwrap();
    (direction, accepted)
}

fn physical_seed(fixture: &Prepared, internal: &[f64]) -> Vec<f64> {
    let size = internal.len();
    let (shape, _) = checked_mode_shape(
        internal,
        &fixture.system.mass,
        &(0..size).collect::<Vec<_>>(),
        size,
    )
    .unwrap();
    fixture
        .system
        .operator
        .roundoff_comparison_seed(fixture.value, &shape, &fixture.system.mass)
        .unwrap()
}

fn adjacent(mut value: f64, steps: i32) -> f64 {
    for _ in 0..steps.unsigned_abs() {
        value = if steps > 0 {
            value.next_up()
        } else {
            value.next_down()
        };
    }
    value
}

#[test]
fn modal_material_frozen_anchor_isolated_chart_diagnosis() {
    let base = Prepared::new(Profile::Layered, 1e-10);
    let baseline = super::run(&base.input);
    let original_shape: Vec<_> = baseline.output.modes[0]
        .shape
        .chunks_exact(3)
        .skip(1)
        .flat_map(|v| [v[1], v[2]])
        .collect();
    for young in [1.0, 16.0] {
        let mut input = renumber(&base.input, 7);
        for e in &mut input.elements {
            e.youngs_modulus *= young;
            e.density *= 4.0;
        }
        let layout = Rebuilt::prepare(&input).unwrap();
        let fixture = Prepared::from_request(layout.canonical.clone());
        let original = bits(&fixture.seed);
        let root = fixture.value.to_bits();
        let (_, internal) = internal(&fixture);
        let seed = physical_seed(&fixture, &internal.shape);
        let matrix =
            physical_directions(&fixture.physical, &fixture.system.mass, fixture.value).unwrap();
        let anchor = (0..seed.len())
            .max_by(|&a, &b| {
                (seed[a].abs() * fixture.system.mass[a].sqrt())
                    .total_cmp(&(seed[b].abs() * fixture.system.mass[b].sqrt()))
            })
            .unwrap();
        println!(
            "material chart young={young} anchor={anchor} seed_anchor={:e} witness_anchor={:e} bits_distance={}",
            seed[anchor],
            original_shape[anchor],
            seed[anchor]
                .to_bits()
                .abs_diff(original_shape[anchor].to_bits())
        );
        let mut successes = 0;
        for steps in [0, -1, 1, -2, 2, -4, 4, -8, 8] {
            // Separate alternative chart; never mutate a running fit's frozen anchor.
            let mut chart = seed.clone();
            chart[anchor] = adjacent(chart[anchor], steps);
            let frozen = bits(&chart);
            let mut best = f64::INFINITY;
            let outcome = search_canonical_unit_shape_single(
                &matrix,
                &chart,
                &fixture.system.mass,
                1e-8,
                internal.order,
                |v| {
                    let applied = fixture.system.operator.apply_physical_compensated(v)?;
                    let certificate = fixture.system.operator.physical_residual(
                        fixture.value,
                        v,
                        &fixture.system.mass,
                        &applied,
                    )?;
                    best = best.min(certificate.0);
                    Ok(certificate)
                },
            );
            let expected = [-1, 1, -2, 4, -8].contains(&steps);
            assert_eq!(outcome.is_ok(), expected, "young={young} steps={steps}");
            match outcome {
                Ok(actual) => {
                    assert_eq!(actual.order, Order::Reverse);
                    assert_eq!(actual.usage.attempts, 1);
                    assert!(actual.usage.certificates <= 7);
                    assert_eq!(actual.shape[anchor].to_bits(), chart[anchor].to_bits());
                    let output = Fixture::canonical_recovered_result(
                        &fixture.input,
                        fixture.total_mass,
                        fixture.value,
                        &actual.shape,
                    );
                    Fixture::check_public_readback(&layout.restore(output).unwrap());
                    println!(
                        "material chart young={young} steps={steps} accepted=true residual={best:e} certificates={}",
                        actual.usage.certificates
                    );
                    successes += 1;
                }
                Err(error) => {
                    assert!(error.contains("exhausted its bounded policies"), "{error}");
                    println!(
                        "material chart young={young} steps={steps} accepted=false residual={best:e} error={error}"
                    );
                }
            }
            assert_eq!(bits(&chart), frozen);
            assert_eq!(fixture.value.to_bits(), root);
            assert_eq!(bits(&fixture.seed), original);
        }
        println!("material chart young={young} isolated_charts=9 accepted_charts={successes}");
        assert_eq!(successes, 5);
    }
}

struct InwardReceipt {
    request: super::Receipt,
    anchor: usize,
    physical_seed: Vec<u64>,
    visits: usize,
    relative: f64,
}

fn run_inward(input: &SolveModalFrame2dRequest) -> Result<InwardReceipt, String> {
    let layout = Rebuilt::prepare(input)?;
    let fixture = Prepared::from_request(layout.canonical.clone());
    let original = bits(&fixture.seed);
    let root = fixture.value.to_bits();
    let (direction, internal) = internal(&fixture);
    let seed = physical_seed(&fixture, &internal.shape);
    let retained = bits(&seed);
    let matrix = physical_directions(&fixture.physical, &fixture.system.mass, fixture.value)?;
    let mut calls = 0;
    let physical = super::super::single_policy_tests::one_factor(|| {
        search_canonical_unit_shape_inward(
            &matrix,
            &seed,
            &fixture.system.mass,
            1e-8,
            internal.order,
            |v| {
                calls += 1;
                let applied = fixture.system.operator.apply_physical_compensated(v)?;
                fixture.system.operator.physical_residual(
                    fixture.value,
                    v,
                    &fixture.system.mass,
                    &applied,
                )
            },
        )
    })?;
    assert_eq!(physical.usage.attempts, 1);
    assert_eq!(physical.usage.certificates, calls);
    assert!(calls <= 7);
    assert!(internal.usage.attempts <= 2 && internal.usage.certificates <= 13);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), original);
    assert_eq!(bits(&seed), retained);
    let (lower, upper) = super::inertia::bracket(&fixture.directions, fixture.value)?;
    assert!((upper - lower) / fixture.value < 1e-14);
    assert!((fixture.value / (lower + (upper - lower) * 0.5) - 1.0).abs() < 1e-12);
    let output = Fixture::canonical_recovered_result(
        &fixture.input,
        fixture.total_mass,
        fixture.value,
        &physical.shape,
    );
    let output = layout.restore(output)?;
    let relative = Fixture::check_public_readback(&output);
    Ok(InwardReceipt {
        request: super::Receipt {
            root,
            seed: bits(&direction),
            orders: [internal.order, physical.order],
            factors: [internal.usage.attempts, physical.usage.attempts],
            certificates: [internal.usage.certificates, physical.usage.certificates],
            inertia: [lower, upper],
            output,
        },
        anchor: physical.anchor,
        physical_seed: retained,
        visits: physical.usage.component_visits,
        relative,
    })
}

fn compare_inward(baseline: &InwardReceipt, actual: &InwardReceipt) {
    super::compare(&baseline.request, &actual.request);
    assert_eq!(actual.anchor, baseline.anchor);
    assert_eq!(actual.physical_seed, baseline.physical_seed);
    assert_eq!(actual.visits, baseline.visits);
    // The independent JSON assembly sums members in original request order.
    assert!(
        (actual.relative - baseline.relative).abs()
            <= 1e-12 * actual.relative.abs().max(baseline.relative.abs())
    );
}

fn check_material_relation(
    base: &super::Receipt,
    actual: &super::Receipt,
    young: f64,
    density: f64,
) {
    let ratio = young / density;
    assert!(
        (f64::from_bits(actual.root) / (f64::from_bits(base.root) * ratio) - 1.0).abs() < 1e-12
    );
    let midpoint = |v: [f64; 2]| v[0] + (v[1] - v[0]) * 0.5;
    assert!((midpoint(actual.inertia) / (midpoint(base.inertia) * ratio) - 1.0).abs() < 1e-13);
    assert!((actual.output.total_mass / (base.output.total_mass * density) - 1.0).abs() < 1e-14);
    assert!(
        (actual.output.modes[0].natural_frequency_hz
            / (base.output.modes[0].natural_frequency_hz * ratio.sqrt())
            - 1.0)
            .abs()
            < 1e-12
    );
    assert_eq!(actual.output.input.nodes, base.output.input.nodes);
    let a = &actual.output.modes[0].shape;
    let b = &base.output.modes[0].shape;
    let overlap: f64 = a.iter().zip(b).map(|(a, b)| a * b).sum();
    let sign = 1.0_f64.copysign(overlap);
    let an = crate::modal_math::checked_shape_norm(a).unwrap();
    let bn = crate::modal_math::checked_shape_norm(b).unwrap();
    // A sign-invariant comparison view, never a modified published shape or search seed.
    let distance = a
        .iter()
        .zip(b)
        .fold(0.0_f64, |d, (a, b)| d.hypot(sign * a / an - b / bn));
    assert!(distance < 1e-10, "material direction distance={distance:e}");
}

#[test]
fn modal_material_inward_chart_cold_material_and_numbering_comparison() {
    let mut successes = 0;
    let mut max_relative = 0.0_f64;
    let mut max_certificates = 0;
    let mut material_relations = 0;
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let base = Fixture::request(profile, 128, scale);
            let mut reference = None;
            for (young, density) in [(1.0, 1.0), (4.0, 1.0), (1.0, 4.0), (16.0, 4.0)] {
                let mut input = base.clone();
                for e in &mut input.elements {
                    e.youngs_modulus *= young;
                    e.density *= density;
                }
                let frozen = input.clone();
                let baseline = run_inward(&input).unwrap();
                if let Some(reference) = &reference {
                    check_material_relation(reference, &baseline.request, young, density);
                    material_relations += 1;
                }
                successes += 1;
                let keys: &[u64] =
                    if matches!(profile, Profile::Layered) && scale == 1e-10 && density == 4.0 {
                        &[7, 1, 113]
                    } else {
                        &[7]
                    };
                for &key in keys {
                    let actual = run_inward(&renumber(&input, key)).unwrap();
                    compare_inward(&baseline, &actual);
                    successes += 1;
                }
                assert_eq!(input, frozen);
                max_relative = max_relative.max(baseline.relative);
                max_certificates = max_certificates.max(baseline.request.certificates[1]);
                println!(
                    "material inward profile={profile:?} scale={scale:e} young={young} density={density} relative={:e} certificates={} visits={}",
                    baseline.relative, baseline.request.certificates[1], baseline.visits
                );
                if reference.is_none() {
                    reference = Some(baseline.request);
                }
            }
        }
    }
    assert_eq!(successes, 52);
    assert_eq!(material_relations, 18);
    assert!(max_relative <= 1e-8);
    assert!(max_certificates <= 7);
    println!(
        "material inward fresh_materials=24 fresh_request_readbacks=52 exact_numbering_controls=28 rejections=0 checked_material_relations=18 max_original_numbered_residual={max_relative:e} max_physical_certificates={max_certificates} physical_factors=1"
    );
}
