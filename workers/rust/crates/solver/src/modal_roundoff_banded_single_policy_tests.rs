use super::super::{Order, search_canonical_direction_single};
use super::{Prepared, Profile, bits, reference::Factor, rounded};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[path = "modal_roundoff_banded_ranked_control_tests.rs"]
mod controls;

pub(super) fn one_factor<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let factors = Rc::new(Cell::new(0));
    let count = factors.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                count.set(count.get() + 1);
            }
        },
        operation,
    );
    assert_eq!(factors.get(), 1);
    result
}

#[test]
fn modal_banded_single_policy_isolated_order_comparison() {
    let mut internal_successes = [0; 3];
    let mut physical_successes = [0; 3];
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let original = bits(&fixture.seed);
            let root = fixture.value.to_bits();
            let seed = rounded(
                &Factor::prepare_shifted(&fixture.directions, fixture.value)
                    .unwrap()
                    .final_direction(&fixture.seed, 4)
                    .unwrap(),
            );
            for (index, order) in [Order::Reverse, Order::Natural, Order::GridNorm]
                .into_iter()
                .enumerate()
            {
                let mut calls = 0;
                let internal = one_factor(|| {
                    search_canonical_direction_single(
                        &fixture.directions,
                        &seed,
                        1e-8,
                        order,
                        |v| {
                            calls += 1;
                            fixture.checked(v)
                        },
                    )
                });
                assert!(calls <= 7);
                println!(
                    "banded single internal profile={} scale={scale:e} order={order:?} accepted={} calls={calls}",
                    profile.name(),
                    internal.is_ok()
                );
                let internal = match internal {
                    Ok(accepted) => accepted,
                    Err(error) => {
                        assert!(error.contains("attempts=1"), "{error}");
                        continue;
                    }
                };
                assert_eq!(internal.order, order);
                assert_eq!(internal.usage.attempts, 1);
                assert_eq!(internal.usage.certificates, calls);
                assert!(fixture.checked(&internal.shape).unwrap().0 <= 1e-8);
                internal_successes[index] += 1;
                for (physical_index, physical_order) in
                    [Order::Reverse, Order::Natural, Order::GridNorm]
                        .into_iter()
                        .enumerate()
                {
                    let mut calls = 0;
                    let physical = one_factor(|| {
                        fixture.physical_candidate_selected(
                            &internal.shape,
                            Some(physical_order),
                            || calls += 1,
                        )
                    });
                    assert!(calls <= 7);
                    println!(
                        "banded single physical profile={} scale={scale:e} internal={order:?} order={physical_order:?} accepted={} calls={calls}",
                        profile.name(),
                        physical.is_ok()
                    );
                    match physical {
                        Ok(accepted) => {
                            assert_eq!(accepted.usage.attempts, 1);
                            assert_eq!(accepted.usage.certificates, calls);
                            let residual = super::super::Fixture::check_recovered_readback(
                                &fixture.input,
                                fixture.total_mass,
                                fixture.value,
                                &accepted.shape,
                            );
                            assert!(residual <= 1e-8);
                            physical_successes[physical_index] += 1;
                        }
                        Err(error) => assert!(error.contains("attempts=1"), "{error}"),
                    }
                }
            }
            assert_eq!(fixture.value.to_bits(), root);
            assert_eq!(bits(&fixture.seed), original);
        }
    }
    println!(
        "banded single isolated_internal_successes={internal_successes:?} physical_proposals={physical_successes:?}"
    );
    assert_eq!(internal_successes, [2, 0, 5]);
    assert_eq!(physical_successes, [6, 0, 6]);
}
