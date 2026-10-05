use super::*;
use crate::modal_frame_spectrum::refinement::roundoff::triangular_grid::pair_lattice::PairFit;

#[path = "modal_roundoff_material_lattice_control_tests.rs"]
mod controls;

#[test]
fn modal_material_pair_lattice_difficult_probe() {
    let mut accepted = [0; 4];
    let mut published = [0; 4];
    for (index, (members, scale)) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)]
        .into_iter()
        .enumerate()
    {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical.clone()).unwrap();
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
            for sweeps in [1, 4] {
                let mut calls = 0;
                let mut best = f64::INFINITY;
                let (result, factors) = measured(|| {
                    let fit = PairFit::prepare(
                        &case.matrix,
                        &case.seed,
                        case.anchor,
                        &case.order(order),
                        64,
                        sweeps,
                    )?;
                    println!(
                        "material pair basis members={members} scale={scale:e} order={order:?} sweeps={sweeps} steps={} shears={} swaps={}",
                        fit.pair_steps, fit.shears, fit.swaps
                    );
                    fit.attempt(1 << 22, 1e-8, |v| {
                        calls += 1;
                        let (relative, residual) = fixture.checked(&case.restore(v))?;
                        best = best.min(relative);
                        Ok((
                            relative,
                            case.permutation.iter().map(|&i| residual[i]).collect(),
                        ))
                    })
                });
                assert!(factors <= 1 && calls <= 66);
                if let Ok(Attempt::Accepted(shape)) = &result {
                    assert_eq!(
                        shape[case.anchor].to_bits(),
                        case.seed[case.anchor].to_bits()
                    );
                    assert!(fixture.checked(&case.restore(shape)).unwrap().0 <= 1e-8);
                    accepted[index] += 1;
                    let fresh = prepare(layout.canonical.clone()).unwrap().0;
                    assert_eq!(fresh.value.to_bits(), root);
                    assert_eq!(bits(&fresh.seed), seed);
                    let mut record = Record::new();
                    record.root = Some(root);
                    record.seed = seed.clone();
                    let tail = finish_hybrid(
                        &mut record,
                        fresh,
                        Rebuilt::prepare_bounded(&input).unwrap(),
                        &case.restore(shape),
                        order,
                    );
                    assert_eq!(record.output.is_some(), tail.is_ok());
                    assert_eq!(record.readback.is_some(), tail.is_ok());
                    published[index] += usize::from(tail.is_ok());
                    println!(
                        "material pair physical input={index} order={order:?} sweeps={sweeps} stage={:?} readback={:?} error={:?}",
                        record.stage,
                        record.readback,
                        tail.err()
                    );
                }
                println!(
                    "material pair probe input={index} order={order:?} sweeps={sweeps} best={best:e} factors={factors} certificates={calls} accepted={} error={:?}",
                    matches!(result, Ok(Attempt::Accepted(_))),
                    result.err()
                );
            }
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), seed);
    }
    println!(
        "material pair probe difficult_inputs=4 isolated_routes=24 accepted={accepted:?} published={published:?} production_admission=unchanged"
    );
    assert_eq!(accepted, [0; 4]);
    assert_eq!(published, [0; 4]);
}

#[test]
fn modal_material_triangular_lattice_difficult_probe() {
    let mut accepted = [0; 4];
    let mut published = [0; 4];
    let mut stops = [0; 3];
    for (index, (members, scale)) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)]
        .into_iter()
        .enumerate()
    {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical.clone()).unwrap();
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
            let mut calls = 0;
            let mut best = f64::INFINITY;
            let (result, factors) = measured(|| {
                let fit = PairFit::prepare_reduced(
                    &case.matrix,
                    &case.seed,
                    case.anchor,
                    &case.order(order),
                    64,
                )?;
                println!(
                    "material triangular basis input={index} order={order:?} steps={} shears={} swaps={} stop={:?}",
                    fit.pair_steps, fit.shears, fit.swaps, fit.stop
                );
                stops[match fit.stop.unwrap() {
                    crate::modal_frame_spectrum::refinement::roundoff::qr::lattice::Stop::Complete => 0,
                    crate::modal_frame_spectrum::refinement::roundoff::qr::lattice::Stop::StepBudget => 1,
                    crate::modal_frame_spectrum::refinement::roundoff::qr::lattice::Stop::IntegerBudget => 2,
                }] += 1;
                fit.attempt(1 << 22, 1e-8, |v| {
                    calls += 1;
                    let (relative, residual) = fixture.checked(&case.restore(v))?;
                    best = best.min(relative);
                    Ok((
                        relative,
                        case.permutation.iter().map(|&i| residual[i]).collect(),
                    ))
                })
            });
            assert!(factors <= 2 && calls <= 66);
            assert_eq!(factors, 2);
            assert_eq!(calls, 66);
            assert_eq!(
                matches!(result, Ok(Attempt::Accepted(_))),
                index == 0 && !matches!(order, Order::Natural)
            );
            if let Ok(Attempt::Accepted(shape)) = &result {
                assert_eq!(
                    shape[case.anchor].to_bits(),
                    case.seed[case.anchor].to_bits()
                );
                assert!(fixture.checked(&case.restore(shape)).unwrap().0 <= 1e-8);
                accepted[index] += 1;
                let fresh = prepare(layout.canonical.clone()).unwrap().0;
                assert_eq!(fresh.value.to_bits(), root);
                assert_eq!(bits(&fresh.seed), seed);
                let mut record = Record::new();
                record.root = Some(root);
                record.seed = seed.clone();
                let tail = finish_hybrid(
                    &mut record,
                    fresh,
                    Rebuilt::prepare_bounded(&input).unwrap(),
                    &case.restore(shape),
                    order,
                );
                assert!(tail.is_ok());
                assert_eq!(record.output.is_some(), tail.is_ok());
                assert_eq!(record.readback.is_some(), tail.is_ok());
                published[index] += usize::from(tail.is_ok());
                println!(
                    "material triangular physical input={index} order={order:?} stage={:?} readback={:?} error={:?}",
                    record.stage,
                    record.readback,
                    tail.err()
                );
            }
            println!(
                "material triangular probe input={index} order={order:?} best={best:e} factors={factors} certificates={calls} accepted={} error={:?}",
                matches!(result, Ok(Attempt::Accepted(_))),
                result.err()
            );
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), seed);
    }
    println!(
        "material triangular probe difficult_inputs=4 isolated_routes=12 accepted={accepted:?} published={published:?} production_admission=unchanged"
    );
    assert_eq!(accepted, [2, 0, 0, 0]);
    assert_eq!(published, [2, 0, 0, 0]);
    assert_eq!(stops, [0, 8, 4]);
    println!(
        "material triangular bounded_stops complete_step_integer={stops:?} strict_failures_not_numeric_rejections=12"
    );
}
