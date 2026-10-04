use super::super::super::super::super::{
    triangular_grid::{Attempt, Rejection},
    wide_factor::dot,
};
use super::super::super::canonical_permutation;
use super::{GridFit, Prepared, Profile, Wide, shuffled, vector_norm};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

struct Case {
    seed: Vec<f64>,
    matrix: Vec<Vec<Wide>>,
    permutation: Vec<usize>,
    anchor: usize,
}

impl Case {
    fn new(fixture: &Prepared, layout: u64, amplitude: f64) -> Self {
        let size = fixture.seed.len();
        let layout: Vec<_> = match layout {
            0 => (0..size).collect(),
            1 => (0..size).rev().collect(),
            key => shuffled(size, key),
        };
        let mapped_seed: Vec<_> = layout
            .iter()
            .map(|&i| amplitude * fixture.seed[i])
            .collect();
        let mapped: Vec<Vec<_>> = layout
            .iter()
            .map(|&i| layout.iter().map(|&j| fixture.directions[i][j]).collect())
            .collect();
        let canonical = canonical_permutation(&mapped, &mapped_seed, &vec![1.0; size]).unwrap();
        let permutation: Vec<_> = canonical.iter().map(|&i| layout[i]).collect();
        let seed: Vec<_> = permutation
            .iter()
            .map(|&i| amplitude * fixture.seed[i])
            .collect();
        let matrix = permutation
            .iter()
            .map(|&i| {
                permutation
                    .iter()
                    .map(|&j| fixture.directions[i][j])
                    .collect()
            })
            .collect();
        let anchor = seed
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
            .unwrap()
            .0;
        Self {
            seed,
            matrix,
            permutation,
            anchor,
        }
    }

    fn order(&self, name: &str) -> Vec<usize> {
        let natural: Vec<_> = (0..self.seed.len()).filter(|&i| i != self.anchor).collect();
        if name == "reverse" {
            return natural.into_iter().rev().collect();
        }
        assert_eq!(name, "ascending");
        let mut ranked: Vec<_> = natural
            .iter()
            .map(|&j| {
                let grid = self.seed[j].next_up() - self.seed[j];
                (
                    j,
                    vector_norm(
                        self.matrix
                            .iter()
                            .map(|row| row[j].mul(Wide::from(grid)).rounded()),
                    )
                    .unwrap(),
                )
            })
            .collect();
        ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        ranked.into_iter().map(|(j, _)| j).collect()
    }

    fn restore(&self, v: &[f64]) -> Vec<f64> {
        assert_eq!(v.len(), self.seed.len());
        let mut original = vec![0.0; v.len()];
        for (&i, &a) in self.permutation.iter().zip(v) {
            original[i] = a;
        }
        original
    }
}

#[derive(Debug, PartialEq)]
struct Run {
    best: f64,
    calls: usize,
    shape: Option<Vec<f64>>,
    physical: Option<Vec<f64>>,
    readback: Option<f64>,
}

fn run(fixture: &Prepared, case: &Case, order: &str, precision: &str) -> Run {
    let order = case.order(order);
    let fit = match precision {
        "rounded" => GridFit::prepare(&case.matrix, &case.seed, case.anchor, &order),
        _ => GridFit::prepare_wide(
            &case.matrix,
            &case.seed,
            case.anchor,
            &order,
            precision == "wide-rounded-input",
        ),
    }
    .unwrap();
    let mut calls = 0;
    let mut best = f64::INFINITY;
    let mut receipt = |v: &[f64]| {
        calls += 1;
        let (relative, residual) = fixture.checked(&case.restore(v))?;
        best = best.min(relative);
        Ok((
            relative,
            case.permutation.iter().map(|&i| residual[i]).collect(),
        ))
    };
    let result = if precision == "wide-beam" {
        fit.attempt_wide_beam(1 << 22, 1e-8, &mut receipt)
    } else {
        fit.attempt_direction(1 << 22, 1e-8, &mut receipt)
    }
    .unwrap();
    assert!(calls <= 6);
    let (shape, physical, readback) = match result {
        Attempt::Accepted(v) => {
            assert_eq!(v[case.anchor].to_bits(), case.seed[case.anchor].to_bits());
            let original = case.restore(&v);
            assert!((0.25..=2.0).contains(&vector_norm(original.iter().copied()).unwrap()));
            assert!(fixture.checked(&original).unwrap().0 <= 1e-8);
            let (published, residual) = fixture.publish_grid(&original).unwrap();
            assert!(residual <= 1e-8);
            assert!(
                (crate::modal_math::checked_shape_norm(&published.shape).unwrap() - 1.0).abs()
                    < 1e-10
            );
            (Some(original), Some(published.shape), Some(residual))
        }
        Attempt::Rejected(Rejection::Residual(relative)) => {
            assert!(relative > 1e-8);
            assert_eq!(relative.to_bits(), best.to_bits());
            (None, None, None)
        }
        Attempt::Rejected(Rejection::UnitNorm(_)) => unreachable!(),
    };
    Run {
        best,
        calls,
        shape,
        physical,
        readback,
    }
}

fn expected(profile: Profile, scale: f64, order: &str, beam: bool) -> bool {
    match profile {
        Profile::Graded => scale != 1e14 && (order == "ascending" || (beam && scale == 1e-10)),
        Profile::Layered => {
            order == "reverse" && (beam || scale != 1.0) || order == "ascending" && scale != 1e-10
        }
        Profile::UnequalLengths => panic!("unequal-length fixtures are outside this comparison"),
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|v| v.to_bits()).collect()
}

#[test]
fn triangular_grid_wide_normalized_original_operator_and_precision_comparison() {
    let mut counts = [0; 2];
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let original_seed = &fixture.seed;
            let size = original_seed.len();
            let (_, residual) = fixture.checked(original_seed).unwrap();
            let wide_seed: Vec<_> = original_seed.iter().copied().map(Wide::from).collect();
            let predicted: Vec<_> = fixture
                .directions
                .iter()
                .map(|row| dot(row, &wide_seed).unwrap().rounded())
                .collect();
            let denominator = fixture.value * vector_norm(original_seed.iter().copied()).unwrap();
            let mismatch = vector_norm(predicted.iter().zip(&residual).map(|(a, b)| a - b))
                .unwrap()
                / denominator;
            let prediction = vector_norm(predicted.iter().copied()).unwrap() / denominator;
            assert!(mismatch.is_finite() && prediction.is_finite());
            assert!(mismatch < 1e-15 && prediction > 1e-8);
            println!(
                "wide operator profile={} scale={scale:e} prediction={prediction:e} mismatch={mismatch:e}",
                profile.name()
            );
            assert_eq!(size, 256);
            let case = Case::new(&fixture, 0, 1.0);
            for order_name in ["reverse", "ascending"] {
                let mut baseline = None;
                for precision in ["rounded", "wide-rounded-input", "wide", "wide-beam"] {
                    let result = run(&fixture, &case, order_name, precision);
                    counts[usize::from(result.shape.is_none())] += 1;
                    assert_eq!(
                        result.shape.is_some(),
                        expected(profile, scale, order_name, precision == "wide-beam")
                    );
                    println!(
                        "wide grid profile={} scale={scale:e} order={order_name} precision={precision} best={:e} calls={} physical={:?}",
                        profile.name(),
                        result.best,
                        result.calls,
                        result.readback
                    );
                    if precision != "wide-beam" {
                        if let Some(reference) = &baseline {
                            assert_eq!(&result, reference);
                            assert_eq!(result.best.to_bits(), reference.best.to_bits());
                            if let Some(shape) = &result.shape {
                                assert_eq!(bits(shape), bits(reference.shape.as_ref().unwrap()));
                                assert_eq!(
                                    bits(result.physical.as_ref().unwrap()),
                                    bits(reference.physical.as_ref().unwrap())
                                );
                            }
                        } else {
                            baseline = Some(result);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(counts, [26, 22]);
    println!("wide precision/order fits=48 accepted=26 rejected=22; greedy backends match bits");
}

#[test]
fn triangular_grid_wide_beam_numbering_sign_and_binary_scale_reproduce_physical_readbacks() {
    let mut numbered = [0; 2];
    let mut binary = [0; 2];
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            for order in ["reverse", "ascending"] {
                let mut baseline = None;
                let mut physical_baseline = None;
                for (layout, amplitude) in [
                    (0, 1.0),
                    (0, -1.0),
                    (1, 1.0),
                    (1, -1.0),
                    (113, 1.0),
                    (113, -1.0),
                    (0, 0.5),
                    (0, 2.0),
                ] {
                    let case = Case::new(&fixture, layout, amplitude);
                    let result = run(&fixture, &case, order, "wide-beam");
                    let accepted = result.shape.is_some();
                    assert_eq!(accepted, expected(profile, scale, order, true));
                    assert_eq!(result.calls, 6);
                    if amplitude.abs() == 1.0 {
                        numbered[usize::from(!accepted)] += 1;
                    } else {
                        binary[usize::from(!accepted)] += 1;
                    }
                    let positive: Vec<_> = result
                        .shape
                        .as_ref()
                        .map_or_else(Vec::new, |v| v.iter().map(|v| v / amplitude).collect());
                    let published: Vec<_> = result.physical.as_ref().map_or_else(Vec::new, |v| {
                        v.iter().map(|v| v * amplitude.signum()).collect()
                    });
                    let normalized_best = result.best.to_bits();
                    if let Some((reference, relative)) = &baseline {
                        assert_eq!(
                            &bits(&positive),
                            reference,
                            "{} {scale:e} {order} {layout} {amplitude}",
                            profile.name()
                        );
                        assert_eq!(normalized_best, *relative);
                        assert_eq!(bits(&published), *physical_baseline.as_ref().unwrap());
                    } else {
                        baseline = Some((bits(&positive), normalized_best));
                        physical_baseline = Some(bits(&published));
                    }
                }
            }
        }
    }
    assert_eq!(numbered, [48, 24]);
    assert_eq!(binary, [16, 8]);
    println!(
        "wide beam numbered/sign runs=72 accepted=48 rejected=24; binary runs=24 accepted=16 rejected=8"
    );
}

#[test]
fn triangular_grid_wide_beam_real_candidate_late_cancellation_replays_exactly() {
    let fixture = Prepared::new(Profile::Graded, 1e-10);
    let case = Case::new(&fixture, 0, 1.0);
    let baseline = run(&fixture, &case, "reverse", "wide-beam");
    let seed_bits = bits(&fixture.seed);
    let root_bits = fixture.value.to_bits();
    let fit = GridFit::prepare_wide(
        &case.matrix,
        &case.seed,
        case.anchor,
        &case.order("reverse"),
        false,
    )
    .unwrap();
    let receipt = |v: &[f64]| {
        let (relative, residual) = fixture.checked(&case.restore(v))?;
        Ok((
            relative,
            case.permutation.iter().map(|&i| residual[i]).collect(),
        ))
    };
    let control = SolverControl::default();
    let cancel = control.clone();
    let mut calls = 0;
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            fit.attempt_wide_beam(1 << 22, 1e-8, |v| {
                calls += 1;
                receipt(v)
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(calls, 6);
    let mut replay_calls = 0;
    let replay = fit
        .attempt_wide_beam(1 << 22, 1e-8, |v| {
            replay_calls += 1;
            receipt(v)
        })
        .unwrap();
    let Attempt::Accepted(replay) = replay else {
        panic!("fresh actual beam replay must recover its candidate");
    };
    assert_eq!(replay_calls, 6);
    let original = case.restore(&replay);
    assert_eq!(bits(&original), bits(baseline.shape.as_ref().unwrap()));
    let (published, readback) = fixture.publish_grid(&original).unwrap();
    assert_eq!(
        bits(&published.shape),
        bits(baseline.physical.as_ref().unwrap())
    );
    assert_eq!(readback.to_bits(), baseline.readback.unwrap().to_bits());
    assert_eq!(bits(&fixture.seed), seed_bits);
    assert_eq!(fixture.value.to_bits(), root_bits);
}
