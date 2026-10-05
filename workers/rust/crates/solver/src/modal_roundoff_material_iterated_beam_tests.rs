use super::*;

#[path = "modal_roundoff_material_iterated_beam_readback_tests.rs"]
mod readback;

#[path = "modal_roundoff_material_pair_lattice_tests.rs"]
mod pair_lattice;

#[test]
fn modal_material_iterated_beam_difficult_internal_probe() {
    let mut accepted = [0; 4];
    let mut published = [0; 4];
    for (index, (members, scale)) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)]
        .into_iter()
        .enumerate()
    {
        let layout =
            Rebuilt::prepare_bounded(&renumber(&Recipe::ThreeLayers.request(members, scale), 7))
                .unwrap();
        let (fixture, refined) = prepare(layout.canonical).unwrap();
        assert!(!refined);
        let root = fixture.value.to_bits();
        let seed = bits(&fixture.seed);
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        );
        let case = Case::new(&fixture, &direction);
        for order in [Order::GridNorm, Order::Reverse, Order::Natural] {
            for (width, passes) in [(64, 1), (32, 2), (16, 4)] {
                for hybrid in [false, true] {
                    let mut calls = 0;
                    let mut best = f64::INFINITY;
                    let (result, factors) = measured(|| {
                        let fit = GridFit::prepare(
                            &case.matrix,
                            &case.seed,
                            case.anchor,
                            &case.order(order),
                        )?;
                        let checked = |v: &[f64]| {
                            calls += 1;
                            let (relative, residual) = fixture.checked(&case.restore(v))?;
                            best = best.min(relative);
                            Ok((
                                relative,
                                case.permutation.iter().map(|&i| residual[i]).collect(),
                            ))
                        };
                        if hybrid {
                            fit.attempt_direction_iterated_hybrid(
                                1 << 22,
                                width,
                                passes,
                                1e-8,
                                checked,
                            )
                        } else {
                            fit.attempt_iterated_beam(1 << 22, width, passes, 1e-8, checked)
                        }
                    });
                    assert_eq!(factors, 1);
                    assert!(calls <= width * passes + 2 + if hybrid { 6 } else { 0 });
                    let result = result.unwrap();
                    assert_eq!(
                        matches!(result, Attempt::Accepted(_)),
                        index == 0 && matches!(order, Order::Reverse),
                        "input={index} order={order:?} width={width} passes={passes} hybrid={hybrid}"
                    );
                    if let Attempt::Accepted(shape) = &result {
                        assert_eq!(
                            shape[case.anchor].to_bits(),
                            case.seed[case.anchor].to_bits()
                        );
                        assert!(fixture.checked(&case.restore(shape)).unwrap().0 <= 1e-8);
                        accepted[index] += 1;
                        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
                        let layout = Rebuilt::prepare_bounded(&input).unwrap();
                        let (fresh, refined) = prepare(layout.canonical.clone()).unwrap();
                        assert!(!refined);
                        assert_eq!(fresh.value.to_bits(), root);
                        assert_eq!(bits(&fresh.seed), seed);
                        let mut record = Record::new();
                        record.root = Some(root);
                        record.seed = seed.clone();
                        record.order = Some(order);
                        let tail =
                            finish_hybrid(&mut record, fresh, layout, &case.restore(shape), order);
                        assert_eq!(tail.is_ok(), width == 64 && passes == 1);
                        assert_eq!(record.output.is_some(), tail.is_ok());
                        assert_eq!(record.readback.is_some(), tail.is_ok());
                        published[index] += usize::from(tail.is_ok());
                        println!(
                            "material iterated physical members={members} scale={scale:e} order={order:?} width={width} passes={passes} hybrid={hybrid} stage={:?} readback={:?} factors={:?} certificates={:?} best={:e} error={:?}",
                            record.stage,
                            record.readback,
                            record.factors,
                            record.certificates,
                            record.best,
                            tail.err()
                        );
                    }
                    println!(
                        "material iterated probe members={members} scale={scale:e} order={order:?} width={width} passes={passes} hybrid={hybrid} accepted={} best={best:e} certificates={calls}",
                        matches!(result, Attempt::Accepted(_))
                    );
                }
            }
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), seed);
    }
    println!(
        "material iterated probe difficult_inputs=4 cold_routes=72 accepted_by_input={accepted:?} published_by_input={published:?}"
    );
    assert_eq!(accepted, [6, 0, 0, 0]);
    assert_eq!(published, [2, 0, 0, 0]);
}
