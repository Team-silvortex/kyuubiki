use super::*;

#[path = "modal_roundoff_material_hybrid_tests.rs"]
mod hybrid_tests;

#[path = "modal_roundoff_material_iterated_beam_tests.rs"]
mod iterated_beam_tests;

#[derive(Clone, Copy, Debug)]
enum Tail {
    Greedy,
    Beam,
    Hybrid,
}

fn run_ablation(input: &SolveModalFrame2dRequest, coupled: bool, tail: Tail) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    let result = attempt_ablation(&mut record, input, coupled, tail);
    if let Err(error) = result {
        assert!(record.output.is_none());
        record.readback = None;
        record.error = Some(error);
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

fn attempt_ablation(
    record: &mut Record,
    input: &SolveModalFrame2dRequest,
    coupled: bool,
    tail: Tail,
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
    record.stage = Stage::Internal;
    let (selected, factors) = measured(|| {
        if coupled {
            let case = Case::new(&fixture, &direction);
            let fit = GridFit::prepare(
                &case.matrix,
                &case.seed,
                case.anchor,
                &case.order(Order::GridNorm),
            )?;
            match fit.attempt_rounded_coupled(1 << 22, 16, 1e-8, |v| {
                record.certificates[0] += 1;
                checked(&fixture, &case, v)
            })? {
                Attempt::Accepted(shape) => Ok((case.restore(&shape), Order::GridNorm)),
                rejected => Err(format!("ablation coupled internal: {rejected:?}")),
            }
        } else {
            let accepted =
                search_canonical_direction_ranked(&fixture.directions, &direction, 1e-8, |v| {
                    record.certificates[0] += 1;
                    fixture.checked(v)
                })?;
            Ok((accepted.shape, accepted.order))
        }
    });
    record.factors[0] = factors;
    assert!(factors <= if coupled { 1 } else { 2 });
    assert!(record.certificates[0] <= if coupled { 18 } else { 13 });
    let (internal, order) = selected?;
    record.order = Some(order);
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    match tail {
        Tail::Beam => finish_beam(record, fixture, layout, &internal, order, Chart::Original),
        Tail::Greedy => record.finish(fixture, layout, &internal, order, Chart::Original),
        Tail::Hybrid => finish_hybrid(record, fixture, layout, &internal, order),
    }
}

fn finish_hybrid(
    record: &mut Record,
    fixture: Prepared,
    layout: Rebuilt,
    internal: &[f64],
    order: Order,
) -> Result<(), String> {
    record.stage = Stage::Physical;
    let case = physical_case(&fixture, internal, Chart::Original)?;
    let (actual, factors) = measured(|| {
        let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
        fit.attempt_unit_hybrid(1 << 22, 16, 1e-8, |v| {
            record.certificates[1] += 1;
            let receipt = physical_checked(&fixture, &case, v)?;
            record.best = record.best.min(receipt.0);
            Ok(receipt)
        })
    });
    record.factors[1] = factors;
    assert!(factors <= 1 && record.certificates[1] <= 25);
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    let Attempt::Accepted(shape) = actual? else {
        return Err("hybrid physical proposal numerically rejected".into());
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    record.publish(fixture, layout, &case.restore(&shape))
}

#[test]
fn modal_material_coupled_loss_stage_ablation() {
    let inputs = [
        Recipe::UnequalLengths.request(128, 1e-10),
        Recipe::UnequalLengths.request(128, 1e14),
        Fixture::request(Profile::Layered, 128, 1e-10),
    ];
    let mut stages = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let input = renumber(input, 7);
        let old = run(&input, Chart::Original);
        for coupled in [false, true] {
            for tail in [Tail::Greedy, Tail::Beam] {
                let record = run_ablation(&input, coupled, tail);
                assert_eq!(record.root, old.root);
                assert_eq!(record.seed, old.seed);
                assert_eq!(record.output.is_some(), record.stage == Stage::Accepted);
                assert_eq!(record.readback.is_some(), record.output.is_some());
                stages.push(record.stage);
                println!(
                    "material coupled ablation input={index} coupled_internal={coupled} tail={tail:?} stage={:?} order={:?} readback={:?} factors={:?} certificates={:?} error={:?}",
                    record.stage,
                    record.order,
                    record.readback,
                    record.factors,
                    record.certificates,
                    record.error
                );
            }
        }
    }
    assert_eq!(stages.len(), 12);
    use Stage::{Accepted as A, Internal as I, Physical as P};
    assert_eq!(stages, [P, A, P, A, A, A, P, P, A, A, I, I]);
    println!("material coupled ablation cold_routes=12 stages={stages:?}");
}
