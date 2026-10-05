use super::super::super::super::super::{
    refine_with_budget, relative_residual, residual_vector, vector_norm,
};
use super::super::super::super::{
    BlockFit,
    triangular_grid::{GridFit, Policy},
    wide_projection::{Projection, physical_directions},
};
use super::super::super::{Accepted, Order, Plan, Selection, tests::shuffled};
use super::super::{search_canonical_direction, search_canonical_unit_shape};
use super::super::{search_canonical_direction_ranked, search_canonical_unit_shape_ranked};
use super::super::{search_canonical_direction_single, search_canonical_unit_shape_single};
use super::{Fixture, Profile};
use crate::modal_frame_spectrum::{checked_mode_shape, checked_published_mode_shape};
use crate::modal_math::jacobi_eigenpairs;
use crate::modal_sparse::ReducedSparseModalSystem;
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_protocol::SolveModalFrame2dRequest;

#[path = "modal_roundoff_grid_normalized_reference.rs"]
mod reference;

#[path = "modal_roundoff_grid_normalized_order_tests.rs"]
mod order_tests;

#[path = "modal_roundoff_grid_normalized_benchmark.rs"]
mod benchmark;

#[path = "modal_roundoff_grid_wide_comparison_tests.rs"]
mod wide_comparison_tests;

#[path = "modal_roundoff_banded_inverse_tests.rs"]
mod banded_inverse_tests;

struct Prepared {
    input: SolveModalFrame2dRequest,
    total_mass: f64,
    system: ReducedSparseModalSystem,
    value: f64,
    seed: Vec<f64>,
    directions: Vec<Vec<Wide>>,
    physical: Vec<Vec<f64>>,
}

impl Prepared {
    fn new(profile: Profile, scale: f64) -> Self {
        Self::new_measured(profile, scale).0
    }

    fn new_measured(profile: Profile, scale: f64) -> (Self, [f64; 4]) {
        let phase = std::time::Instant::now();
        let input = Fixture::request(profile, 128, scale);
        Self::from_request_measured(input, phase)
    }

    fn from_request(input: SolveModalFrame2dRequest) -> Self {
        Self::from_request_measured(input, std::time::Instant::now()).0
    }

    fn from_request_measured(
        input: SolveModalFrame2dRequest,
        phase: std::time::Instant,
    ) -> (Self, [f64; 4]) {
        crate::modal_frame_validation::validate_modal_frame_2d_request(&input).unwrap();
        assert_eq!(input.mode_count, Some(1));
        let (system, physical, total_mass) = Fixture::assemble(&input).unwrap();
        let assembly_ms = phase.elapsed().as_secs_f64() * 1000.0;
        let phase = std::time::Instant::now();
        let mut pairs =
            jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
        pairs.truncate(1);
        let jacobi_ms = phase.elapsed().as_secs_f64() * 1000.0;
        let phase = std::time::Instant::now();
        let original = pairs[0].0;
        let applied = system.operator.apply_compensated(&pairs[0].1).unwrap();
        let initial = relative_residual(original, &pairs[0].1, &applied).unwrap();
        // Capture the real best pair before production roundoff, not a new oracle seed.
        let error = refine_with_budget(&system, &mut pairs, 1e-8, 4).unwrap_err();
        assert!(error.contains("within 4 steps"), "{error}");
        let (value, seed) = pairs.remove(0);
        assert!((value / original - 1.0).abs() <= 4.0 * initial.max(1e-8));
        let refinement_ms = phase.elapsed().as_secs_f64() * 1000.0;
        let phase = std::time::Instant::now();
        let directions = physical
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, &k)| {
                        let mut entry = Wide::from(k)
                            .mul(Wide::from(system.mass[i].sqrt().recip()))
                            .mul(Wide::from(system.mass[j].sqrt().recip()));
                        if i == j {
                            entry = entry.sub(Wide::from(value));
                        }
                        entry
                    })
                    .collect()
            })
            .collect();
        let prepared = Self {
            input,
            total_mass,
            system,
            value,
            seed,
            directions,
            physical,
        };
        let direction_ms = phase.elapsed().as_secs_f64() * 1000.0;
        (
            prepared,
            [assembly_ms, jacobi_ms, refinement_ms, direction_ms],
        )
    }

    fn checked(&self, v: &[f64]) -> Result<(f64, Vec<f64>), String> {
        let applied = self.system.operator.apply_compensated(v)?;
        Ok((
            relative_residual(self.value, v, &applied)?,
            residual_vector(self.value, v, &applied)?,
        ))
    }

    fn publish(&self, v: &[f64]) -> Result<f64, String> {
        let mut full_mass = vec![0.0; 2 * self.input.nodes.len()];
        for (&dof, &mass) in self.system.free_dofs.iter().zip(&self.system.mass) {
            full_mass[dof] = mass;
        }
        let (shape, _) = checked_published_mode_shape(
            &self.system,
            v,
            &full_mass,
            2 * self.input.nodes.len(),
            self.value,
            1e-8,
            true,
        )?;
        Ok(Fixture::check_recovered_readback(
            &self.input,
            self.total_mass,
            self.value,
            &shape[2..],
        ))
    }

    fn physical_candidate(&self, v: &[f64]) -> Result<Accepted, String> {
        self.physical_candidate_with_checks(v, || {})
    }

    fn physical_candidate_with_checks(
        &self,
        v: &[f64],
        on_check: impl FnMut(),
    ) -> Result<Accepted, String> {
        self.physical_candidate_selected(v, None, on_check)
    }

    fn physical_candidate_selected(
        &self,
        v: &[f64],
        order: Option<Order>,
        on_check: impl FnMut(),
    ) -> Result<Accepted, String> {
        self.physical_candidate_ordered(
            v,
            order.map_or(Selection::Full, Selection::Single),
            on_check,
        )
    }

    fn physical_candidate_ordered(
        &self,
        v: &[f64],
        selection: Selection,
        on_check: impl FnMut(),
    ) -> Result<Accepted, String> {
        self.physical_candidate_observed(v, selection, on_check, |_| {})
    }

    fn physical_candidate_observed(
        &self,
        v: &[f64],
        selection: Selection,
        mut on_check: impl FnMut(),
        mut on_residual: impl FnMut(f64),
    ) -> Result<Accepted, String> {
        let size = v.len();
        let (shape, _) =
            checked_mode_shape(v, &self.system.mass, &(0..size).collect::<Vec<_>>(), size)?;
        let seed =
            self.system
                .operator
                .roundoff_comparison_seed(self.value, &shape, &self.system.mass)?;
        let matrix = physical_directions(&self.physical, &self.system.mass, self.value)?;
        let checked = |v: &[f64]| {
            on_check();
            let applied = self.system.operator.apply_physical_compensated(v)?;
            let certificate = self.system.operator.physical_residual(
                self.value,
                v,
                &self.system.mass,
                &applied,
            )?;
            on_residual(certificate.0);
            Ok(certificate)
        };
        match selection {
            Selection::Single(order) => search_canonical_unit_shape_single(
                &matrix,
                &seed,
                &self.system.mass,
                1e-8,
                order,
                checked,
            ),
            Selection::Full => {
                search_canonical_unit_shape(&matrix, &seed, &self.system.mass, 1e-8, checked)
            }
            Selection::RankedThenReverse => {
                search_canonical_unit_shape_ranked(&matrix, &seed, &self.system.mass, 1e-8, checked)
            }
        }
    }

    fn publish_grid(&self, v: &[f64]) -> Result<(Accepted, f64), String> {
        let accepted = self.physical_candidate(v)?;
        let independent = Fixture::check_recovered_readback(
            &self.input,
            self.total_mass,
            self.value,
            &accepted.shape,
        );
        Ok((accepted, independent))
    }
}

fn expect_rejection(outcome: Result<Vec<f64>, String>) {
    let error = outcome.unwrap_err();
    assert!(error.contains("unchanged residual gate"), "{error}");
}

#[test]
fn triangular_grid_normalized_heterogeneous_fixed_root_and_rejected_alternatives() {
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let seed = &fixture.seed;
            let initial = fixture.checked(seed).unwrap().0;
            assert!(initial > 1e-8);
            let norm = vector_norm(seed.iter().copied()).unwrap();
            assert!((0.49..1.01).contains(&norm) && (norm - 1.0).abs() > 1e-10);
            let mut matrix = fixture.system.operator.dense_fallback_matrix().unwrap();
            for (i, row) in matrix.iter_mut().enumerate() {
                row[i] -= fixture.value;
            }
            let checked = |v: &[f64]| fixture.checked(v);
            let mut retained = seed.clone();
            let mut best = initial;
            let mut calls = 0;
            let gram = BlockFit::prepare_automatic(matrix.clone(), seed)
                .unwrap()
                .correct(seed, 1e-8, |v| {
                    calls += 1;
                    let certificate = checked(v)?;
                    if certificate.0 < best {
                        best = certificate.0;
                        retained.copy_from_slice(v);
                    }
                    Ok(certificate)
                });
            expect_rejection(gram);
            assert!(calls <= 80 && best > 1e-8 && best < initial);
            let qr = BlockFit::prepare_qr_automatic(matrix.clone(), seed)
                .unwrap()
                .correct(seed, 1e-8, checked);
            expect_rejection(qr);
            let anchor = seed
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
                .unwrap()
                .0;
            let fine: Vec<_> = (0..seed.len()).filter(|&i| i != anchor).collect();
            expect_rejection(
                BlockFit::prepare_qr_partition(matrix, fine.clone())
                    .unwrap()
                    .correct(seed, 1e-8, checked),
            );
            let order: Vec<_> = fine.iter().copied().rev().collect();
            let mut grid_residuals = Vec::new();
            for start in [seed, &retained] {
                let mut lowest = fixture.checked(start).unwrap().0;
                let outcome = GridFit::prepare(&fixture.directions, start, anchor, &order)
                    .unwrap()
                    .correct(1 << 22, Policy::InBacksolve, 1e-8, |v| {
                        let certificate = checked(v)?;
                        lowest = lowest.min(certificate.0);
                        Ok(certificate)
                    });
                expect_rejection(outcome);
                grid_residuals.push(lowest);
            }
            expect_rejection(
                Projection::prepare(fixture.directions.clone(), fine)
                    .unwrap()
                    .correct_rounding(seed, 1e-8, checked),
            );
            let mut amplitude = best;
            let (mut up, mut down) = (1.0_f64, 1.0_f64);
            for _ in 0..16 {
                up = up.next_up();
                down = down.next_down();
                for factor in [up, down] {
                    let candidate: Vec<_> = retained.iter().map(|v| v * factor).collect();
                    amplitude = amplitude.min(checked(&candidate).unwrap().0);
                }
            }
            assert!(amplitude > 1e-8);
            let (lower, upper) = reference::bracket(&fixture.directions, fixture.value).unwrap();
            assert!((upper - lower) / fixture.value < 1e-14);
            let root_error = fixture.value / ((lower + upper) * 0.5) - 1.0;
            assert!(root_error.abs() < 1e-12);
            println!(
                "normalized diagnosis profile={} scale={scale:e} initial={initial:e} norm={norm:e} gram={best:e} gram_checks={calls} raw_grid={grid_residuals:?} amplitude={amplitude:e} root_error={root_error:e}",
                profile.name()
            );
        }
    }
}

#[test]
fn triangular_grid_normalized_canonical_numbering_sign_and_binary_scale_boundaries() {
    let mut counts = [0; 2];
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let size = fixture.seed.len();
            let expected = !(matches!(profile, Profile::Graded) && scale == 1e14);
            let mut baseline: Option<Vec<f64>> = None;
            let mut published_baseline: Option<Vec<f64>> = None;
            for layout in [0, 1, 7, 113] {
                let permutation: Vec<_> = match layout {
                    0 => (0..size).collect(),
                    1 => (0..size).rev().collect(),
                    seed => shuffled(size, seed),
                };
                let matrix: Vec<Vec<_>> = permutation
                    .iter()
                    .map(|&i| {
                        permutation
                            .iter()
                            .map(|&j| fixture.directions[i][j])
                            .collect()
                    })
                    .collect();
                for sign in [1.0, -1.0] {
                    let seed: Vec<_> = permutation
                        .iter()
                        .map(|&i| sign * fixture.seed[i])
                        .collect();
                    let restore = |v: &[f64]| {
                        let mut original = vec![0.0; size];
                        for (&i, &a) in permutation.iter().zip(v) {
                            original[i] = a;
                        }
                        original
                    };
                    let mut calls = 0;
                    let outcome = search_canonical_direction(&matrix, &seed, 1e-8, |v| {
                        calls += 1;
                        let (relative, residual) = fixture.checked(&restore(v))?;
                        Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                    });
                    assert_eq!(outcome.is_ok(), expected);
                    counts[usize::from(!expected)] += 1;
                    assert!(calls <= 19);
                    let accepted = match outcome {
                        Ok(result) => result,
                        Err(error) => {
                            assert!(
                                error.contains("attempts=3") && error.contains("norm_rejections=0"),
                                "{error}"
                            );
                            continue;
                        }
                    };
                    assert_eq!(calls, accepted.usage.certificates);
                    let reverse = matches!(profile, Profile::Layered) && scale != 1.0;
                    assert_eq!(
                        accepted.order,
                        if reverse {
                            Order::Reverse
                        } else {
                            Order::GridNorm
                        }
                    );
                    assert_eq!(accepted.usage.attempts, if reverse { 1 } else { 3 });
                    assert_eq!(calls, if reverse { 4 } else { 11 });
                    let plan = Plan::canonical(size).unwrap();
                    assert_eq!(
                        accepted.usage.component_visits,
                        plan.preflight_visits + accepted.usage.attempts * plan.fit_visits
                    );
                    assert_eq!(
                        accepted.shape[accepted.anchor].to_bits(),
                        seed[accepted.anchor].to_bits()
                    );
                    let original = restore(&accepted.shape);
                    let positive: Vec<_> = original.iter().map(|v| sign * v).collect();
                    let residual = fixture.checked(&positive).unwrap().0;
                    assert!(residual <= 1e-8);
                    let (published, independent) = fixture.publish_grid(&original).unwrap();
                    assert_eq!(published.usage.attempts, 1);
                    assert_eq!(published.usage.certificates, 4);
                    assert_eq!(published.order, Order::Reverse);
                    assert_eq!(
                        published.usage.component_visits,
                        plan.preflight_visits + plan.fit_visits
                    );
                    let published_positive: Vec<_> =
                        published.shape.iter().map(|v| sign * v).collect();
                    if let Some(reference) = &published_baseline {
                        assert!(
                            published_positive
                                .iter()
                                .zip(reference)
                                .all(|(a, b)| a.to_bits() == b.to_bits())
                        );
                    } else {
                        published_baseline = Some(published_positive);
                    }
                    if let Some(reference) = &baseline {
                        assert!(
                            positive
                                .iter()
                                .zip(reference)
                                .all(|(a, b)| a.to_bits() == b.to_bits())
                        );
                    } else {
                        let publication = fixture.publish(&positive);
                        assert!(
                            publication
                                .as_ref()
                                .unwrap_err()
                                .contains("physical modal roundoff recovery failed")
                        );
                        println!(
                            "normalized canonical profile={} scale={scale:e} residual={residual:e} order={:?} calls={calls} physical={publication:?} independent_pipeline={independent:e} pipeline_grid_checks={}",
                            profile.name(),
                            accepted.order,
                            calls + published.usage.certificates
                        );
                        baseline = Some(positive);
                    }
                }
            }
            for factor in [0.5, 2.0] {
                let seed: Vec<_> = fixture.seed.iter().map(|v| factor * v).collect();
                let outcome = search_canonical_direction(&fixture.directions, &seed, 1e-8, |v| {
                    fixture.checked(v)
                });
                assert_eq!(outcome.is_ok(), expected);
                if let Ok(accepted) = outcome {
                    assert!(
                        accepted
                            .shape
                            .iter()
                            .zip(baseline.as_ref().unwrap())
                            .all(|(a, b)| (a / factor).to_bits() == b.to_bits())
                    );
                    let (published, _) = fixture.publish_grid(&accepted.shape).unwrap();
                    assert!(
                        published
                            .shape
                            .iter()
                            .zip(published_baseline.as_ref().unwrap())
                            .all(|(a, b)| a.to_bits() == b.to_bits())
                    );
                }
            }
        }
    }
    assert_eq!(counts, [40, 8]);
    println!(
        "normalized canonical executions=48 accepted=40 rejected=8; physical readbacks=40; binary scale controls=12 accepted=10 rejected=2"
    );
}

#[test]
fn triangular_grid_normalized_real_candidate_late_cancellation_and_fresh_replay() {
    let fixture = Prepared::new(Profile::Graded, 1.0);
    let baseline = search_canonical_direction(&fixture.directions, &fixture.seed, 1e-8, |v| {
        fixture.checked(v)
    })
    .unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let mut calls = 0;
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 5 {
                cancel.request_cancel();
            }
        },
        || {
            search_canonical_direction(&fixture.directions, &fixture.seed, 1e-8, |v| {
                calls += 1;
                fixture.checked(v)
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(calls, 11);
    let replay = search_canonical_direction(&fixture.directions, &fixture.seed, 1e-8, |v| {
        fixture.checked(v)
    })
    .unwrap();
    assert_eq!(replay.usage.certificates, 11);
    assert!(
        replay
            .shape
            .iter()
            .zip(&baseline.shape)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    fixture.publish_grid(&replay.shape).unwrap();
}

#[test]
fn triangular_grid_normalized_contract_faults_and_late_cancellation_replay() {
    let matrix: Vec<Vec<_>> = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| Wide::from(if i == j { (i + 1) as f64 } else { 0.0 }))
                .collect()
        })
        .collect();
    let seed = [0.1, 0.2, 0.5];
    assert!(search_canonical_direction(&[], &seed, 1e-8, |_| unreachable!()).is_err());
    assert!(search_canonical_direction(&matrix, &seed[..2], 1e-8, |_| unreachable!()).is_err());
    for bad in [vec![0.0; 3], vec![1e-3; 3], vec![2.0; 3], vec![f64::NAN; 3]] {
        assert!(search_canonical_direction(&matrix, &bad, 1e-8, |_| unreachable!()).is_err());
    }
    for malformed in [
        (f64::NAN, vec![0.0; 3]),
        (0.0, vec![0.0; 2]),
        (0.0, vec![f64::INFINITY; 3]),
    ] {
        let mut calls = 0;
        let error = search_canonical_direction(&matrix, &seed, 1e-8, |_| {
            calls += 1;
            Ok(malformed.clone())
        })
        .unwrap_err();
        assert!(
            error.contains("certificate") || error.contains("matching original residual"),
            "{error}"
        );
        assert_eq!(calls, 1);
    }
    let mut calls = 0;
    let error = search_canonical_direction(&matrix, &seed, 1e-8, |_| {
        calls += 1;
        Err("normalized operator fault".into())
    })
    .unwrap_err();
    assert_eq!(error, "normalized operator fault");
    assert_eq!(calls, 1);
    calls = 0;
    let error = search_canonical_direction(&matrix, &seed, 1e-8, |_| {
        calls += 1;
        Ok((if calls == 3 { 1.0 } else { 0.0 }, vec![0.0; 3]))
    })
    .unwrap_err();
    assert!(error.contains("final normalized certificate"), "{error}");
    assert_eq!(calls, 3);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 5 {
                cancel.request_cancel();
            }
        },
        || search_canonical_direction(&matrix, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 3]))),
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    let replay =
        search_canonical_direction(&matrix, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 3]))).unwrap();
    assert_eq!(replay.shape, seed);
    assert_eq!(replay.usage.certificates, 3);
}
