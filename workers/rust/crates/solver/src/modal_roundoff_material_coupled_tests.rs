use super::*;
use crate::modal_frame_spectrum::refinement::roundoff::triangular_grid::coupled::Receipt;

#[path = "modal_roundoff_material_coupled_control_tests.rs"]
mod controls;

#[path = "modal_roundoff_material_coupled_ablation_tests.rs"]
mod ablation;

fn checked(fixture: &Prepared, case: &Case, candidate: &[f64]) -> Result<Receipt, String> {
    let direction = case.restore(candidate);
    let (internal, residual) = fixture.checked(&direction)?;
    let (shape, norm) = checked_mode_shape(
        &direction,
        &fixture.system.mass,
        &(0..direction.len()).collect::<Vec<_>>(),
        direction.len(),
    )?;
    if (norm - 1.0).abs() >= 1e-10 {
        return Err("coupled mapping lost its unchanged unit norm".into());
    }
    let applied = fixture.system.operator.apply_physical_compensated(&shape)?;
    let mapped = fixture
        .system
        .operator
        .physical_residual(fixture.value, &shape, &fixture.system.mass, &applied)?
        .0;
    Ok(Receipt {
        internal,
        mapped,
        residual: case.permutation.iter().map(|&i| residual[i]).collect(),
    })
}

fn run_coupled(input: &SolveModalFrame2dRequest, order: Order, chart: Chart) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    if let Err(error) = attempt(&mut record, input, order, chart) {
        assert!(record.output.is_none());
        record.readback = None;
        record.error = Some(error);
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

fn attempt(
    record: &mut Record,
    input: &SolveModalFrame2dRequest,
    order: Order,
    chart: Chart,
) -> Result<(), String> {
    let layout = Rebuilt::prepare_bounded(input)?;
    let (fixture, refined) = prepare(layout.canonical.clone())?;
    record.refined = refined;
    record.root = Some(fixture.value.to_bits());
    record.seed = bits(&fixture.seed);
    record.stage = Stage::Initializer;
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)?
            .final_direction(&fixture.seed, 4)?,
    );
    let case = Case::new(&fixture, &direction);
    record.stage = Stage::Internal;
    let (outcome, factors) = measured(|| {
        let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
        fit.attempt_rounded_coupled(1 << 22, 16, 1e-8, |v| {
            record.certificates[0] += 1;
            let receipt = checked(&fixture, &case, v)?;
            record.best = record.best.min(receipt.internal);
            Ok(receipt)
        })
    });
    record.factors[0] = factors;
    assert!(factors <= 1 && record.certificates[0] <= 18);
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    let internal = match outcome? {
        Attempt::Accepted(shape) => shape,
        rejected => return Err(format!("coupled internal proposal rejected: {rejected:?}")),
    };
    assert_eq!(
        internal[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    record.order = Some(order);
    let internal = case.restore(&internal);
    drop(case);
    finish_beam(record, fixture, layout, &internal, order, chart)
}

fn physical_case(fixture: &Prepared, direction: &[f64], chart: Chart) -> Result<Case, String> {
    let size = direction.len();
    let (shape, _) = checked_mode_shape(
        direction,
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
    let permutation = canonical_permutation(&matrix, &seed, &fixture.system.mass)?;
    let mut seed: Vec<_> = permutation.iter().map(|&i| seed[i]).collect();
    let anchor = (0..size)
        .max_by(|&a, &b| {
            (seed[a].abs() * fixture.system.mass[permutation[a]].sqrt())
                .total_cmp(&(seed[b].abs() * fixture.system.mass[permutation[b]].sqrt()))
        })
        .unwrap();
    if matches!(chart, Chart::Inward) {
        seed[anchor] = if seed[anchor] > 0.0 {
            seed[anchor].next_down()
        } else {
            seed[anchor].next_up()
        };
        let amplitude = seed[anchor].abs() * fixture.system.mass[permutation[anchor]].sqrt();
        if seed.iter().enumerate().any(|(i, v)| {
            i != anchor && v.abs() * fixture.system.mass[permutation[i]].sqrt() >= amplitude
        }) {
            return Err("coupled physical chart lost its uniquely dominant anchor".into());
        }
    }
    Ok(Case {
        seed,
        anchor,
        matrix: permutation
            .iter()
            .map(|&i| permutation.iter().map(|&j| matrix[i][j]).collect())
            .collect(),
        permutation,
    })
}

fn physical_checked(fixture: &Prepared, case: &Case, v: &[f64]) -> Result<(f64, Vec<f64>), String> {
    let shape = case.restore(v);
    let applied = fixture.system.operator.apply_physical_compensated(&shape)?;
    let (relative, residual) = fixture.system.operator.physical_residual(
        fixture.value,
        &shape,
        &fixture.system.mass,
        &applied,
    )?;
    Ok((
        relative,
        case.permutation.iter().map(|&i| residual[i]).collect(),
    ))
}

fn finish_beam(
    record: &mut Record,
    fixture: Prepared,
    layout: Rebuilt,
    internal: &[f64],
    order: Order,
    chart: Chart,
) -> Result<(), String> {
    record.stage = Stage::Physical;
    let case = physical_case(&fixture, internal, chart)?;
    let (actual, factors) = measured(|| {
        let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
        fit.attempt_rounded_unit_beam(1 << 22, 16, 1e-8, |v| {
            record.certificates[1] += 1;
            let receipt = physical_checked(&fixture, &case, v)?;
            record.best = record.best.min(receipt.0);
            Ok(receipt)
        })
    });
    record.factors[1] = factors;
    assert!(factors <= 1 && record.certificates[1] <= 18);
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    let shape = match actual? {
        Attempt::Accepted(shape) => shape,
        rejected => return Err(format!("coupled physical proposal rejected: {rejected:?}")),
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    record.publish(fixture, layout, &case.restore(&shape))
}

#[test]
fn modal_material_coupled_difficult_candidate_publication_diagnosis() {
    let mut diagnostic_routes = 0;
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, _) = prepare(layout.canonical).unwrap();
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        );
        let case = Case::new(&fixture, &direction);
        let mut eligible: Vec<(usize, Vec<f64>, f64, f64)> = Vec::new();
        let fit = GridFit::prepare(
            &case.matrix,
            &case.seed,
            case.anchor,
            &case.order(Order::Reverse),
        )
        .unwrap();
        let mut calls = 0;
        fit.attempt_rounded_beam(1 << 22, 16, 1e-8, |v| {
            calls += 1;
            let receipt = checked(&fixture, &case, v)?;
            if receipt.internal <= 1e-8
                && !eligible
                    .iter()
                    .any(|(_, saved, _, _)| bits(saved) == bits(v))
            {
                eligible.push((calls, v.to_vec(), receipt.internal, receipt.mapped));
            }
            Ok((receipt.internal, receipt.residual))
        })
        .unwrap();
        assert!(calls <= 18);
        assert_eq!(eligible.len(), if members == 100 { 16 } else { 0 });
        for (callback, candidate, internal, mapped) in eligible {
            for chart in [Chart::Original, Chart::Inward] {
                let layout = Rebuilt::prepare_bounded(&input).unwrap();
                let (fresh, _) = prepare(layout.canonical.clone()).unwrap();
                assert_eq!(fresh.value.to_bits(), fixture.value.to_bits());
                assert_eq!(bits(&fresh.seed), bits(&fixture.seed));
                let natural = case.restore(&candidate);
                assert!(fresh.checked(&natural).unwrap().0 <= 1e-8);
                let mut record = Record::new();
                record.root = Some(fresh.value.to_bits());
                record.seed = bits(&fresh.seed);
                if let Err(error) = record.finish(fresh, layout, &natural, Order::Reverse, chart) {
                    assert!(record.output.is_none());
                    record.error = Some(error);
                }
                assert_eq!(record.stage, Stage::Physical);
                assert!(record.output.is_none() && record.readback.is_none());
                assert_eq!(record.factors[1], 1);
                assert_eq!(record.certificates[1], 4);
                diagnostic_routes += 1;
                println!(
                    "material coupled diagnostic members={members} scale={scale:e} callback={callback} chart={chart:?} internal={internal:e} mapped={mapped:e} stage={:?} readback={:?} final={:e}",
                    record.stage, record.readback, record.best
                );
            }
        }
        for order in [Order::GridNorm, Order::Reverse] {
            for chart in [Chart::Original, Chart::Inward] {
                let actual = run_coupled(&input, order, chart);
                let expected = if members == 100 && matches!(order, Order::Reverse) {
                    Stage::Physical
                } else {
                    Stage::Internal
                };
                assert_eq!(actual.stage, expected);
                assert_eq!(
                    actual.certificates,
                    if expected == Stage::Physical {
                        [18, 18]
                    } else {
                        [18, 0]
                    }
                );
                assert!(actual.output.is_none() && actual.readback.is_none());
                println!(
                    "material coupled selected members={members} scale={scale:e} order={order:?} chart={chart:?} stage={:?} readback={:?} factors={:?} certificates={:?} error={:?}",
                    actual.stage,
                    actual.readback,
                    actual.factors,
                    actual.certificates,
                    actual.error
                );
            }
        }
    }
    assert_eq!(diagnostic_routes, 32);
    println!(
        "material coupled eligible_diagnostics=32 old_physical_publications=0 fresh_roots_seeds=32"
    );
}

#[test]
fn modal_material_coupled_full_readback_recipe_mesh_comparison() {
    let mut boundaries = Vec::new();
    let mut compare = |input: &SolveModalFrame2dRequest| {
        let old = run(input, Chart::Original);
        let mut boundary = [old.stage; 3];
        for (index, order) in [Order::GridNorm, Order::Reverse].into_iter().enumerate() {
            let record = run_coupled(input, order, Chart::Original);
            assert_eq!(record.root, old.root);
            assert_eq!(record.seed, old.seed);
            assert_eq!(record.output.is_some(), record.stage == Stage::Accepted);
            assert_eq!(record.readback.is_some(), record.output.is_some());
            boundary[index + 1] = record.stage;
            println!(
                "material coupled full order={order:?} stage={:?} readback={:?} factors={:?} certificates={:?} error={:?}",
                record.stage, record.readback, record.factors, record.certificates, record.error
            );
        }
        boundaries.push(boundary);
        boundary
    };
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let boundary = compare(&renumber(&recipe.request(members, scale), 7));
                println!(
                    "material coupled full boundary recipe={recipe:?} members={members} scale={scale:e} stages={boundary:?}"
                );
            }
        }
    }
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let boundary = compare(&renumber(&Fixture::request(profile, 128, scale), 7));
            println!(
                "material coupled baseline profile={profile:?} scale={scale:e} stages={boundary:?}"
            );
        }
    }
    let mut stages = [[0; 7]; 3];
    for boundary in &boundaries {
        for (index, stage) in boundary.iter().enumerate() {
            stages[index][*stage as usize] += 1;
        }
    }
    assert_eq!(boundaries.len(), 42);
    assert_eq!(
        stages,
        [
            [0, 0, 4, 1, 0, 0, 37],
            [0, 0, 5, 1, 0, 0, 36],
            [0, 0, 11, 3, 0, 0, 28]
        ]
    );
    use Stage::{Accepted as A, Internal as I, Physical as P};
    assert_eq!(
        boundaries,
        [
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, I],
            [A, A, I],
            [A, A, I],
            [A, A, I],
            [A, P, I],
            [P, A, I],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [I, I, P],
            [I, I, I],
            [I, I, I],
            [I, I, I],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, A, P],
            [A, A, I],
            [A, A, P],
            [A, A, A],
            [A, A, I],
            [A, A, A],
            [A, A, A],
            [A, A, A],
            [A, I, A]
        ]
    );
    println!(
        "material coupled full_inputs=42 cold_routes=126 stages_old_gridnorm_reverse={stages:?} boundaries={boundaries:?} production_admission=unchanged"
    );
}
