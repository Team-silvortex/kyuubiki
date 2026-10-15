use super::*;

const SEED: &str = "127.0.0.1:5001";

fn hint(address: &str) -> ClusterPeerDescriptor {
    ClusterPeerDescriptor {
        address: address.into(),
        status: "healthy".into(),
        failure_count: 0,
        last_seen_unix_s: Some(u64::MAX),
    }
}

fn pool() -> CandidatePool {
    CandidatePool::new(vec![SEED.into()], vec!["127.0.0.1:5000".into()])
}

fn fail(pool: &mut CandidatePool, peer: &str) {
    for _ in 0..FAILURE_THRESHOLD {
        pool.record_failure(peer);
    }
}

#[test]
fn full_candidate_pool_reclaims_failed_hints_but_not_configured_seeds() {
    let now = Instant::now();
    let mut pool = pool();
    pool.merge_hints(
        (6000..6063)
            .map(|p| hint(&format!("127.0.0.1:{p}")))
            .collect(),
        now,
    );
    assert_eq!(pool.known.len(), MAX_PEERS);
    let fresh = "127.0.0.1:7000";
    pool.merge_hints(vec![hint(fresh)], now);
    assert!(!pool.known.iter().any(|p| p == fresh));
    let healthy = "127.0.0.1:6000";
    pool.record_success(healthy, vec![], now, 42);
    for peer in pool.known.clone() {
        if peer != healthy {
            fail(&mut pool, &peer);
        }
    }
    let retained = pool.finish_round(now);
    assert_eq!(retained.len(), 2);
    assert!(pool.known.iter().any(|p| p == SEED));
    assert_eq!(
        retained
            .iter()
            .find(|p| p.address == healthy)
            .unwrap()
            .status,
        "healthy"
    );
    assert_eq!(pool.retired.len(), MAX_PEERS - 2);
    pool.merge_hints(vec![hint(fresh)], now);
    let rows = pool.finish_round(now);
    let added = rows.iter().find(|p| p.address == fresh).unwrap();
    assert_eq!(added.status, "unreachable");
    assert_eq!(added.failure_count, 0);
    assert_eq!(added.last_seen_unix_s, None);
}

#[test]
fn only_direct_success_resets_consecutive_failure_retirement() {
    let now = Instant::now();
    let peer = "127.0.0.1:6000";
    let mut pool = pool();
    pool.merge_hints(vec![hint(peer)], now);
    for _ in 0..2 {
        pool.record_failure(peer);
    }
    pool.record_success(peer, vec![], now, 10);
    for _ in 0..2 {
        pool.record_failure(peer);
    }
    pool.record_success(SEED, vec![hint(peer)], now, 20);
    let rows = pool.finish_round(now);
    let failed = rows.iter().find(|p| p.address == peer).unwrap();
    assert_eq!(failed.failure_count, 2);
    assert_eq!(failed.last_seen_unix_s, Some(10));
    assert_eq!(failed.status, "degraded");
    pool.record_failure(peer);
    assert_eq!(pool.finish_round(now).len(), 1);
    assert!(!pool.failures.contains_key(peer));
    assert!(!pool.last_seen.contains_key(peer));
}

#[test]
fn retired_gossip_cannot_bypass_or_extend_monotonic_cooldown() {
    let now = Instant::now();
    let until = now + Duration::from_millis(COOLDOWN_MS);
    let peer = "lab.local:6000";
    let mut pool = pool();
    pool.merge_hints(vec![hint(peer)], now);
    pool.record_success(peer, vec![], now, u64::MAX);
    fail(&mut pool, peer);
    pool.finish_round(now);
    let before = until - Duration::from_millis(1);
    // Neither canonical spelling variations nor a future advertised timestamp are proof.
    pool.record_success(SEED, vec![hint(" LAB.LOCAL:06000 ")], before, 0);
    assert_eq!(pool.retired[peer], until);
    assert_eq!(pool.finish_round(before).len(), 1);
    assert_eq!(
        pool.finish_round(until).len(),
        1,
        "expiry alone must not resurrect a peer"
    );
    pool.merge_hints(vec![hint(peer)], until);
    let rows = pool.finish_round(until);
    let candidate = rows.iter().find(|p| p.address == peer).unwrap();
    assert_eq!(candidate.status, "unreachable");
    assert_eq!(candidate.failure_count, 0);
    assert_eq!(candidate.last_seen_unix_s, None);
    pool.record_success(peer, vec![], until, 3);
    assert_eq!(
        pool.finish_round(until)
            .iter()
            .find(|p| p.address == peer)
            .unwrap()
            .status,
        "healthy"
    );
}

#[test]
fn cooldown_saturation_is_bounded_without_shortening_existing_retirements() {
    let now = Instant::now();
    let mut pool = pool();
    for batch in 0..3 {
        pool.merge_hints(
            (0..63)
                .map(|i| hint(&format!("127.0.0.1:{}", 10000 + batch * 64 + i)))
                .collect(),
            now,
        );
        for peer in pool.known.clone() {
            if peer != SEED {
                fail(&mut pool, &peer);
            }
        }
        pool.finish_round(now);
        assert!(pool.known.len() <= MAX_PEERS);
        assert!(pool.retired.len() <= MAX_RETIRED);
        assert!(pool.failures.len() <= MAX_PEERS);
        assert!(pool.last_seen.len() <= MAX_PEERS);
    }
    assert_eq!(pool.retired.len(), MAX_RETIRED);
    assert_eq!(pool.known.len(), 62);
    let until = now + Duration::from_millis(COOLDOWN_MS);
    pool.merge_hints(
        vec![hint("127.0.0.1:10000")],
        until - Duration::from_millis(1),
    );
    assert_eq!(pool.retired["127.0.0.1:10000"], until);
    assert!(!pool.known.iter().any(|p| p == "127.0.0.1:10000"));
    assert_eq!(pool.finish_round(until).len(), 1);
    assert_eq!(pool.retired.len(), 61);
    assert!(pool.failures.is_empty());
    pool.merge_hints(vec![hint("127.0.0.1:10000")], until);
    assert_eq!(pool.finish_round(until).len(), 2);
}

#[test]
fn retired_candidates_do_not_break_round_robin_or_seed_reconnection() {
    let now = Instant::now();
    let mut pool = pool();
    pool.merge_hints(
        (6000..6063)
            .map(|p| hint(&format!("127.0.0.1:{p}")))
            .collect(),
        now,
    );
    pool.finish_round(now);
    for _ in 0..7 {
        pool.next_round();
    }
    for peer in pool.known.clone().into_iter().step_by(2) {
        fail(&mut pool, &peer);
    }
    pool.finish_round(now);
    let mut visited = HashSet::new();
    for _ in 0..pool.known.len().div_ceil(super::super::PROBES_PER_ROUND) {
        visited.extend(pool.next_round());
    }
    assert_eq!(visited, pool.known.iter().cloned().collect());
    pool.record_success(SEED, vec![], now, 100);
    let rows = pool.finish_round(now);
    let seed = rows.iter().find(|p| p.address == SEED).unwrap();
    assert_eq!(seed.status, "healthy");
    assert_eq!(seed.failure_count, 0);
    assert!(!pool.retired.contains_key(SEED));
}

#[test]
fn unprobed_or_unknown_candidates_cannot_gain_health_or_failure_state() {
    let now = Instant::now();
    let mut pool = pool();
    let peer = "127.0.0.1:6000";
    pool.record_success(peer, vec![hint("127.0.0.1:7000")], now, 5);
    pool.record_failure(peer);
    assert_eq!(pool.finish_round(now).len(), 1);
    assert!(pool.failures.is_empty());
    pool.merge_hints(vec![hint(peer)], now);
    assert_eq!(pool.finish_round(now + Duration::from_secs(86400)).len(), 2);
    assert!(
        pool.retired.is_empty(),
        "unprobed candidates are not failed probes"
    );
    pool.failures.insert(SEED.into(), u32::MAX);
    pool.record_failure(SEED);
    assert_eq!(
        pool.finish_round(now + Duration::from_secs(86400))[0].failure_count,
        u32::MAX
    );
}

#[test]
fn retained_peers_keep_the_eight_round_probe_bound_during_retirement() {
    let now = Instant::now();
    let address = |i| format!("peer-{i:02}:5001");
    let seeds: Vec<_> = (8..16).map(address).collect();
    let mut pool = CandidatePool::new(seeds.clone(), vec![]);
    pool.merge_hints((0..64).map(|i| hint(&address(i))).collect(), now);
    pool.finish_round(now);
    let mut last_probe = HashMap::new();
    for round in 0..32 {
        let tick = now + Duration::from_secs(round);
        for peer in pool.next_round() {
            if seeds.contains(&peer) {
                if let Some(previous) = last_probe.insert(peer.clone(), round) {
                    assert!(
                        round - previous <= 8,
                        "{peer} waited {} rounds",
                        round - previous
                    );
                }
                pool.record_success(&peer, vec![], tick, round);
            } else {
                pool.record_failure(&peer);
            }
        }
        pool.finish_round(tick);
    }
    assert_eq!(last_probe.len(), seeds.len());
    assert_eq!(pool.known, seeds);
}

#[test]
fn new_hints_wait_behind_existing_unprobed_candidates() {
    let now = Instant::now();
    let seeds: Vec<_> = (0..16).map(|i| format!("seed-{i:02}:5001")).collect();
    let mut pool = CandidatePool::new(seeds.clone(), vec![]);
    assert_eq!(pool.next_round(), seeds[..8]);
    pool.record_success(&seeds[0], vec![hint("a-new-peer:5001")], now, 1);
    pool.finish_round(now);
    assert_eq!(pool.next_round(), seeds[8..]);
}

#[test]
fn bounded_probe_queue_preserves_membership_and_fairness_across_a_thousand_churn_rounds() {
    let now = Instant::now();
    let seeds: Vec<_> = (0..3).map(|i| format!("seed-{i}:5001")).collect();
    let mut pool = CandidatePool::new(seeds.clone(), vec![]);
    let mut last_probe = HashMap::new();
    for round in 0..1000 {
        let tick = now + Duration::from_secs(round);
        let scheduled = pool.next_round();
        assert!(scheduled.len() <= PROBES_PER_ROUND);
        assert_eq!(
            scheduled.iter().collect::<HashSet<_>>().len(),
            scheduled.len()
        );
        for peer in scheduled {
            assert!(pool.known.contains(&peer));
            if seeds.contains(&peer) {
                last_probe.insert(peer.clone(), round);
                let hints = (0..16)
                    .map(|i| hint(&format!("peer-{round}-{i}:5001")))
                    .collect();
                pool.record_success(&peer, hints, tick, round);
            } else {
                pool.record_failure(&peer);
            }
        }
        let rows = pool.finish_round(tick);
        assert!(rows.len() <= MAX_PEERS);
        assert!(pool.retired.len() <= MAX_RETIRED);
        assert_eq!(pool.probe_queue.len(), rows.len());
        assert_eq!(
            pool.probe_queue.iter().collect::<HashSet<_>>(),
            pool.known.iter().collect()
        );
        assert!(
            pool.probe_queue
                .iter()
                .all(|peer| !pool.retired.contains_key(peer))
        );
        assert!(rows.windows(2).all(|p| p[0].address < p[1].address));
        for seed in &seeds {
            assert!(
                round - last_probe[seed] < 8,
                "seed starvation in round {round}"
            );
        }
    }
}
