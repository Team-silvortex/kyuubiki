use super::*;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(3);

fn peers(count: usize) -> Vec<String> {
    (0..count).map(|index| index.to_string()).collect()
}

#[test]
fn completed_peer_is_published_before_another_probe_finishes() {
    let running = AtomicBool::new(true);
    let (started, awaiting_start) = mpsc::channel();
    let awaiting_start = Mutex::new(awaiting_start);
    let (release, awaiting_release) = mpsc::channel();
    let awaiting_release = Mutex::new(awaiting_release);
    let mut published = Vec::new();
    run_round(
        peers(2),
        &running,
        2,
        |peer| {
            if peer == "0" {
                started.send(()).unwrap();
                awaiting_release.lock().unwrap().recv_timeout(WAIT).unwrap();
            } else {
                awaiting_start.lock().unwrap().recv_timeout(WAIT).unwrap();
            }
            Ok(())
        },
        |peer, result| {
            result.unwrap();
            if peer == "1" {
                release.send(()).unwrap();
            }
            published.push(peer);
        },
    );
    assert_eq!(published, ["1", "0"]);
}

#[test]
fn round_limits_concurrency_and_probes_each_selected_peer_once() {
    let running = AtomicBool::new(true);
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let mut published = Vec::new();
    run_round(
        peers(12),
        &running,
        usize::MAX,
        |peer| {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(count, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(5));
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(peer.to_string())
        },
        |peer, result| {
            assert_eq!(result.unwrap(), peer);
            published.push(peer);
        },
    );
    published.sort();
    assert_eq!(published, peers(8));
    assert!((1..=MAX_CONCURRENT_PROBES).contains(&peak.load(Ordering::SeqCst)));
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[test]
fn stopping_skips_queued_probes_and_joins_in_flight_without_publishing() {
    let running = AtomicBool::new(true);
    let completed = AtomicUsize::new(0);
    let (started, awaiting_start) = mpsc::channel();
    let (release, awaiting_release) = mpsc::channel();
    let awaiting_release = Mutex::new(awaiting_release);
    thread::scope(|scope| {
        let round = scope.spawn(|| {
            let mut published = Vec::new();
            run_round(
                peers(8),
                &running,
                2,
                |peer| {
                    started.send(peer.to_string()).unwrap();
                    awaiting_release.lock().unwrap().recv_timeout(WAIT).unwrap();
                    completed.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
                |peer, _| published.push(peer),
            );
            published
        });
        let mut started_peers = vec![
            awaiting_start.recv_timeout(WAIT).unwrap(),
            awaiting_start.recv_timeout(WAIT).unwrap(),
        ];
        started_peers.sort();
        assert_eq!(started_peers, peers(2));
        running.store(false, Ordering::SeqCst);
        release.send(()).unwrap();
        release.send(()).unwrap();
        assert!(round.join().unwrap().is_empty());
    });
    assert_eq!(completed.load(Ordering::SeqCst), 2);
    assert!(awaiting_start.try_recv().is_err());
}

#[test]
fn failed_or_panicking_probe_does_not_drop_other_results() {
    let running = AtomicBool::new(true);
    let mut published = Vec::new();
    run_round(
        peers(8),
        &running,
        2,
        |peer| match peer {
            "0" => panic!("injected discovery probe panic"),
            "1" => Err("injected request failure".into()),
            _ => Ok(()),
        },
        |peer, result| published.push((peer, result)),
    );
    published.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(published.len(), 8);
    assert_eq!(
        published[0].1.as_ref().unwrap_err(),
        "peer discovery probe panicked"
    );
    assert_eq!(
        published[1].1.as_ref().unwrap_err(),
        "injected request failure"
    );
    assert!(published[2..].iter().all(|(_, result)| result.is_ok()));
}

#[test]
fn serial_fallback_or_single_worker_preserves_the_selected_round() {
    let running = AtomicBool::new(true);
    for workers in [0, 1] {
        let mut published = Vec::new();
        run_round(
            peers(8),
            &running,
            workers,
            |_| Ok(()),
            |peer, result| {
                result.unwrap();
                published.push(peer);
            },
        );
        assert_eq!(published, peers(8));
    }
}

#[test]
fn no_worker_fallback_also_stops_before_queued_probes() {
    let running = AtomicBool::new(true);
    let requested = Mutex::new(Vec::new());
    let mut published = Vec::new();
    run_round(
        peers(8),
        &running,
        0,
        |peer| {
            requested.lock().unwrap().push(peer.to_string());
            Ok(())
        },
        |peer, result| {
            result.unwrap();
            published.push(peer);
            if published.len() == 3 {
                running.store(false, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(published, peers(3));
    assert_eq!(*requested.lock().unwrap(), peers(3));
}

#[test]
fn stopped_or_empty_round_never_starts_a_request() {
    for (selected, running) in [(peers(8), false), (vec![], true)] {
        let running = AtomicBool::new(running);
        run_round::<()>(
            selected,
            &running,
            2,
            |_| panic!("unexpected request"),
            |_, _| panic!("unexpected publication"),
        );
    }
}
