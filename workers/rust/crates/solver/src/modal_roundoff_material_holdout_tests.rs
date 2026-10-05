use super::*;
use crate::modal_math::jacobi_eigenpairs;
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverControl, SolverStage, checkpoint, with_solver_observer};
use std::{cell::Cell, rc::Rc};

use super::super::super::super::{refine_with_budget, relative_residual};

#[path = "modal_roundoff_material_construction_tests.rs"]
mod construction;

#[derive(Clone, Copy, Debug)]
enum Recipe {
    UnequalLengths,
    NonbinaryGradient,
    ThreeLayers,
    Jittered,
}

impl Recipe {
    const ALL: [Self; 4] = [
        Self::UnequalLengths,
        Self::NonbinaryGradient,
        Self::ThreeLayers,
        Self::Jittered,
    ];

    fn request(self, members: usize, scale: f64) -> SolveModalFrame2dRequest {
        let mut input = Fixture::request(Profile::Graded, members, scale);
        if matches!(self, Self::UnequalLengths) {
            return Fixture::request(Profile::UnequalLengths, members, scale);
        }
        let mut position = 0.0;
        let mut state = 0x59d2_c391_728b_04a7_u64;
        for (i, e) in input.elements.iter_mut().enumerate() {
            let t = (i as f64 + 0.5) / members as f64;
            let [length, young, area, inertia, density] = match self {
                Self::NonbinaryGradient => [
                    1.0,
                    1.7 * (0.6 + 0.8 * t),
                    0.85 + 0.22 * t,
                    0.9 + 0.15 * t,
                    2.3 * (1.9 - 0.7 * t),
                ],
                Self::ThreeLayers => [
                    [1.0, 1.0, 1.1, 0.8, 1.4],
                    [1.0, 3.7, 0.7, 1.2, 0.9],
                    [1.0, 11.0, 1.3, 0.55, 2.2],
                ][(3 * i / members).min(2)],
                Self::Jittered => {
                    let values = std::array::from_fn::<_, 5, _>(|_| {
                        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                        (state >> 32) as f64 / u32::MAX as f64
                    });
                    [
                        0.88 + 0.24 * values[0],
                        0.8 + 0.7 * values[1],
                        0.8 + 0.4 * values[2],
                        0.75 + 0.5 * values[3],
                        0.7 + 0.6 * values[4],
                    ]
                }
                Self::UnequalLengths => unreachable!(),
            };
            position += length;
            input.nodes[i + 1].x = position * scale;
            e.youngs_modulus = scale.powi(3) * young;
            e.area = area;
            e.moment_of_inertia = inertia;
            e.density = scale.recip() * density;
        }
        input
    }
}

// Fallible preparation keeps holdout-stage failures distinct from completed searches.
fn prepare(input: SolveModalFrame2dRequest) -> Result<(Prepared, bool), String> {
    crate::modal_frame_validation::validate_modal_frame_2d_request(&input)?;
    let (system, physical, total_mass) = Fixture::assemble(&input)?;
    let mut pairs = jacobi_eigenpairs(system.operator.dense_fallback_matrix()?)?;
    pairs.truncate(1);
    let original = pairs[0].0;
    let applied = system.operator.apply_compensated(&pairs[0].1)?;
    let initial = relative_residual(original, &pairs[0].1, &applied)?;
    let refined = match refine_with_budget(&system, &mut pairs, 1e-8, 4) {
        Ok(()) => true,
        Err(error) if error.contains("within 4 steps") => false,
        Err(error) => return Err(error),
    };
    let (value, seed) = pairs.remove(0);
    assert!((value / original - 1.0).abs() <= 4.0 * initial.max(1e-8));
    let directions = physical
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, &k)| {
                    let a = Wide::from(k)
                        .mul(Wide::from(system.mass[i].sqrt().recip()))
                        .mul(Wide::from(system.mass[j].sqrt().recip()));
                    if i == j { a.sub(Wide::from(value)) } else { a }
                })
                .collect()
        })
        .collect();
    Ok((
        Prepared {
            input,
            total_mass,
            system,
            physical,
            value,
            seed,
            directions,
        },
        refined,
    ))
}

#[derive(Clone, Copy, Debug)]
enum Chart {
    Original,
    Inward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Preparation,
    Initializer,
    Internal,
    Physical,
    Spectral,
    Readback,
    Accepted,
}

fn measured<T>(operation: impl FnOnce() -> Result<T, String>) -> (Result<T, String>, usize) {
    let factors = Rc::new(Cell::new(0));
    let observed = factors.clone();
    let outcome = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        operation,
    );
    (outcome, factors.get())
}

#[derive(Debug)]
struct Record {
    stage: Stage,
    refined: bool,
    root: Option<u64>,
    seed: Vec<u64>,
    order: Option<Order>,
    factors: [usize; 2],
    certificates: [usize; 2],
    best: f64,
    readback: Option<f64>,
    error: Option<String>,
    output: Option<kyuubiki_protocol::SolveModalFrame2dResult>,
}

impl Record {
    fn new() -> Self {
        Self {
            stage: Stage::Preparation,
            refined: false,
            root: None,
            seed: vec![],
            order: None,
            factors: [0; 2],
            certificates: [0; 2],
            best: f64::INFINITY,
            readback: None,
            error: None,
            output: None,
        }
    }

    fn attempt(&mut self, input: &SolveModalFrame2dRequest, chart: Chart) -> Result<(), String> {
        self.attempt_charts(input, Chart::Original, chart)
    }

    fn attempt_charts(
        &mut self,
        input: &SolveModalFrame2dRequest,
        internal_chart: Chart,
        chart: Chart,
    ) -> Result<(), String> {
        let layout = Rebuilt::prepare_bounded(input)?;
        let (fixture, refined) = prepare(layout.canonical.clone())?;
        self.refined = refined;
        self.root = Some(fixture.value.to_bits());
        self.seed = bits(&fixture.seed);
        self.stage = Stage::Initializer;
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)?
                .final_direction(&fixture.seed, 4)?,
        );
        self.stage = Stage::Internal;
        let (internal, factors) = measured(|| {
            let checked = |v: &[f64]| {
                self.certificates[0] += 1;
                fixture.checked(v)
            };
            match internal_chart {
                Chart::Original => search_canonical_direction_ranked(
                    &fixture.directions,
                    &direction,
                    1e-8,
                    checked,
                ),
                Chart::Inward => search_canonical_direction_inward(
                    &fixture.directions,
                    &direction,
                    1e-8,
                    checked,
                ),
            }
        });
        self.factors[0] = factors;
        assert!(factors <= 2 && self.certificates[0] <= 13);
        assert_eq!(fixture.value.to_bits(), self.root.unwrap());
        assert_eq!(bits(&fixture.seed), self.seed);
        let internal = internal?;
        assert_eq!(self.factors[0], internal.usage.attempts);
        assert_eq!(self.certificates[0], internal.usage.certificates);
        self.order = Some(internal.order);
        self.finish(fixture, layout, &internal.shape, internal.order, chart)
    }

    // Shared unchanged publication tail for independently selected test-only proposals.
    fn finish(
        &mut self,
        fixture: Prepared,
        layout: Rebuilt,
        internal: &[f64],
        order: Order,
        chart: Chart,
    ) -> Result<(), String> {
        self.stage = Stage::Physical;
        let size = internal.len();
        let (shape, _) = checked_mode_shape(
            internal,
            &fixture.system.mass,
            &(0..size).collect::<Vec<_>>(),
            size,
        )?;
        let seed = fixture.system.operator.roundoff_comparison_seed(
            fixture.value,
            &shape,
            &fixture.system.mass,
        )?;
        let matrix = physical_directions(&fixture.physical, &fixture.system.mass, fixture.value)?;
        let checked = |v: &[f64]| {
            self.certificates[1] += 1;
            let applied = fixture.system.operator.apply_physical_compensated(v)?;
            let certificate = fixture.system.operator.physical_residual(
                fixture.value,
                v,
                &fixture.system.mass,
                &applied,
            )?;
            self.best = self.best.min(certificate.0);
            Ok(certificate)
        };
        let (actual, factors) = measured(|| match chart {
            Chart::Original => search_canonical_unit_shape_single(
                &matrix,
                &seed,
                &fixture.system.mass,
                1e-8,
                order,
                checked,
            ),
            Chart::Inward => search_canonical_unit_shape_inward(
                &matrix,
                &seed,
                &fixture.system.mass,
                1e-8,
                order,
                checked,
            ),
        });
        self.factors[1] = factors;
        assert!(factors <= 1 && self.certificates[1] <= 7);
        assert_eq!(fixture.value.to_bits(), self.root.unwrap());
        assert_eq!(bits(&fixture.seed), self.seed);
        let actual = actual?;
        assert_eq!(actual.usage.attempts, 1);
        assert_eq!(actual.usage.certificates, self.certificates[1]);
        assert!(self.certificates[1] <= 7);
        self.publish(fixture, layout, &actual.shape)
    }

    // The final spectral, numbering and independent readback gates are policy-neutral.
    fn publish(&mut self, fixture: Prepared, layout: Rebuilt, shape: &[f64]) -> Result<(), String> {
        self.stage = Stage::Spectral;
        let (lower, upper) = super::super::inertia::bracket(&fixture.directions, fixture.value)?;
        let width = (upper - lower) / fixture.value;
        let root_error = (fixture.value / (lower + (upper - lower) * 0.5) - 1.0).abs();
        if width >= 1e-14 || root_error >= 1e-12 {
            return Err(format!(
                "holdout inertia cross-check rejected width={width:e} root_error={root_error:e}"
            ));
        }
        self.stage = Stage::Readback;
        let output = Fixture::canonical_recovered_result(
            &fixture.input,
            fixture.total_mass,
            fixture.value,
            shape,
        );
        let output = layout.restore(output)?;
        let expected: Vec<_> = output
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
        assert_eq!(output.free_dofs, expected);
        for (i, node) in output.input.nodes.iter().enumerate() {
            for (axis, fixed) in [node.fix_x, node.fix_y, node.fix_rz]
                .into_iter()
                .enumerate()
            {
                if fixed {
                    assert_eq!(
                        output.modes[0].shape[3 * i + axis].to_bits(),
                        0.0_f64.to_bits()
                    );
                }
            }
        }
        self.readback = Some(Fixture::check_public_readback(&output));
        checkpoint(SolverStage::ResultTotals, 2)?;
        self.output = Some(output);
        self.stage = Stage::Accepted;
        Ok(())
    }
}

fn run(input: &SolveModalFrame2dRequest, chart: Chart) -> Record {
    run_charts(input, Chart::Original, chart)
}

fn run_charts(input: &SolveModalFrame2dRequest, internal: Chart, chart: Chart) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    if let Err(error) = record.attempt_charts(input, internal, chart) {
        assert!(record.output.is_none());
        record.readback = None;
        record.error = Some(error);
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

#[test]
fn modal_material_inward_chart_unseen_recipes_and_meshes_cold_comparison() {
    let mut pairs = [0; 4];
    let mut stages = [[0; 7]; 2];
    let mut margins = [0; 3];
    let mut refined = 0;
    let mut max_regression = 1.0_f64;
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let input = renumber(&recipe.request(members, scale), 7);
                let original = run(&input, Chart::Original);
                let inward = run(&input, Chart::Inward);
                assert_eq!(original.root, inward.root);
                assert_eq!(original.seed, inward.seed);
                assert_eq!(original.refined, inward.refined);
                assert_eq!(original.order, inward.order);
                refined += usize::from(original.refined);
                // Pin each observed boundary, not only totals that could hide shifted failures.
                let expected = if matches!(recipe, Recipe::ThreeLayers)
                    && (members == 128 || (members == 100 && scale == 1e-10))
                {
                    Stage::Internal
                } else if matches!(recipe, Recipe::UnequalLengths)
                    && members == 128
                    && scale == 1e-10
                {
                    Stage::Physical
                } else {
                    Stage::Accepted
                };
                for (policy, record) in [(0, &original), (1, &inward)] {
                    assert_eq!(
                        record.stage, expected,
                        "{recipe:?} members={members} scale={scale:e} chart={policy}"
                    );
                    stages[policy][record.stage as usize] += 1;
                    assert_eq!(record.stage == Stage::Accepted, record.output.is_some());
                    println!(
                        "material holdout recipe={recipe:?} members={members} scale={scale:e} chart={policy} stage={:?} refined={} best={:e} readback={:?} factors={:?} certificates={:?} error={:?}",
                        record.stage,
                        record.refined,
                        record.best,
                        record.readback,
                        record.factors,
                        record.certificates,
                        record.error
                    );
                }
                let accepted = (
                    original.stage == Stage::Accepted,
                    inward.stage == Stage::Accepted,
                );
                pairs[match accepted {
                    (true, true) => 0,
                    (false, true) => 1,
                    (true, false) => 2,
                    (false, false) => 3,
                }] += 1;
                if let (Some(a), Some(b)) = (original.readback, inward.readback) {
                    let ratio = b / a;
                    margins[if (ratio - 1.0).abs() <= 1e-12 {
                        2
                    } else if ratio < 1.0 {
                        0
                    } else {
                        1
                    }] += 1;
                    max_regression = max_regression.max(ratio);
                }
            }
        }
    }
    assert_eq!(pairs.iter().sum::<usize>(), 36);
    assert!(stages.iter().all(|v| v.iter().sum::<usize>() == 36));
    assert_eq!(pairs, [31, 0, 0, 5]);
    assert_eq!(stages, [[0, 0, 4, 1, 0, 0, 31]; 2]);
    assert_eq!(margins, [16, 15, 0]);
    assert_eq!(refined, 11);
    assert!(max_regression > 1.57 && max_regression < 1.59);
    println!(
        "material holdout inputs=36 cold_routes=72 paired_both_inward_only_original_only_neither={pairs:?} original_stages={:?} inward_stages={:?}",
        stages[0], stages[1]
    );
    println!(
        "material holdout compared_margins=31 inward_better_worse_equal={margins:?} max_regression_ratio={max_regression:e} ordinary_refinement_converged={refined}"
    );
}

fn same_case(base: &Record, actual: &Record) {
    assert_eq!(actual.stage, base.stage);
    assert_eq!(actual.refined, base.refined);
    assert_eq!(actual.root, base.root);
    assert_eq!(actual.seed, base.seed);
    assert_eq!(actual.order, base.order);
    assert_eq!(actual.factors, base.factors);
    assert_eq!(actual.certificates, base.certificates);
    assert_eq!(actual.best.to_bits(), base.best.to_bits());
    assert_eq!(actual.error, base.error);
    match (&base.output, &actual.output) {
        (None, None) => assert!(actual.readback.is_none()),
        (Some(base), Some(actual)) => {
            assert_eq!(actual.free_dofs.len(), base.free_dofs.len());
            assert!((actual.total_mass / base.total_mass - 1.0).abs() < 1e-14);
            for (new, node) in actual.input.nodes.iter().enumerate() {
                let old = base
                    .input
                    .nodes
                    .iter()
                    .position(|n| n.id == node.id)
                    .unwrap();
                assert_eq!(
                    bits(&actual.modes[0].shape[3 * new..3 * new + 3]),
                    bits(&base.modes[0].shape[3 * old..3 * old + 3])
                );
            }
        }
        _ => panic!("fresh holdout reconstruction changed publication boundary"),
    }
}

#[test]
fn modal_material_inward_chart_holdout_rejections_final_cancel_and_fresh_replay() {
    let mut rejections = 0;
    let mut cancellations = 0;
    for chart in [Chart::Original, Chart::Inward] {
        let healthy = Recipe::NonbinaryGradient.request(80, 1e-10);
        let baseline = run(&healthy, chart);
        assert_eq!(baseline.stage, Stage::Accepted);
        for (recipe, expected) in [
            (Recipe::ThreeLayers, Stage::Internal),
            (Recipe::UnequalLengths, Stage::Physical),
        ] {
            let input = recipe.request(128, 1e-10);
            let rejected = run(&input, chart);
            assert_eq!(rejected.stage, expected);
            assert!(rejected.output.is_none() && rejected.readback.is_none());
            assert_eq!(
                rejected.factors,
                if expected == Stage::Internal {
                    [2, 0]
                } else {
                    [1, 1]
                }
            );
            assert_eq!(
                rejected.certificates,
                if expected == Stage::Internal {
                    [8, 0]
                } else {
                    [4, 4]
                }
            );
            same_case(&rejected, &run(&renumber(&input, 113), chart));
            same_case(&baseline, &run(&renumber(&healthy, 113), chart));
            rejections += 1;
        }
        let input = renumber(&healthy, 7);
        for (stage, step) in [
            (SolverStage::ResultNodes, input.nodes.len() as u64),
            (SolverStage::ResultTotals, 2),
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let saw = Rc::new(Cell::new(false));
            let observed = saw.clone();
            let mut stopped = Record::new();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && p.completed_steps == step {
                        observed.set(true);
                        cancel.request_cancel();
                    }
                },
                || stopped.attempt(&input, chart),
            )
            .unwrap_err();
            assert!(saw.get() && error.contains("cancel"), "{error}");
            assert_eq!(stopped.stage, Stage::Readback);
            assert!(stopped.output.is_none());
            same_case(&baseline, &run(&input, chart));
            cancellations += 1;
        }
    }
    assert_eq!(rejections, 4);
    assert_eq!(cancellations, 4);
    println!(
        "material holdout stage_rejections=4 failed_numbering_replays=4 healthy_numbering_replays=4 late_cancellations=4 fresh_replays=4 no_partial_output=1"
    );
}

#[test]
fn modal_material_inward_chart_holdout_preparation_boundaries_and_replay() {
    let healthy = Recipe::NonbinaryGradient.request(80, 1e-10);
    let mut declined = vec![Recipe::NonbinaryGradient.request(129, 1e-10)];
    let mut multimode = healthy.clone();
    multimode.mode_count = Some(2);
    declined.push(multimode);
    let mut invalid = healthy.clone();
    invalid.elements[0].youngs_modulus = 0.0;
    declined.push(invalid);
    let mut reversed = healthy.clone();
    let e = &mut reversed.elements[0];
    std::mem::swap(&mut e.node_i, &mut e.node_j);
    declined.push(reversed);
    for chart in [Chart::Original, Chart::Inward] {
        let baseline = run(&healthy, chart);
        assert_eq!(baseline.stage, Stage::Accepted);
        for input in &declined {
            let record = run(input, chart);
            assert_eq!(record.stage, Stage::Preparation);
            assert!(record.error.is_some());
            assert!(record.root.is_none() && record.seed.is_empty());
            assert_eq!(record.factors, [0, 0]);
            assert_eq!(record.certificates, [0, 0]);
            assert!(record.output.is_none() && record.readback.is_none());
            same_case(&baseline, &run(&renumber(&healthy, 7), chart));
        }
    }
    println!(
        "material holdout declined_preparations=8 search_factors=0 certificates=0 fresh_healthy_replays=8"
    );
}
