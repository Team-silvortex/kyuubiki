use super::*;

fn wait_for_resolvers(active: &AtomicUsize) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while active.load(Ordering::Acquire) != 0 {
        assert!(Instant::now() < deadline, "resolver slot was leaked");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn delayed_dns_returns_on_deadline_and_releases_its_slot_after_resolution() {
    let active = Arc::new(AtomicUsize::new(0));
    let (release, released) = mpsc::channel();
    let started = Instant::now();
    let result = resolve_with(
        started + Duration::from_millis(50),
        Arc::clone(&active),
        move || {
            released.recv().unwrap();
            Ok(vec!["127.0.0.1:80".parse().unwrap()])
        },
    );
    let elapsed = started.elapsed();
    let error = result.unwrap_err();
    assert!(error.message.contains("deadline exhausted"), "{error:?}");
    assert!(elapsed < Duration::from_millis(300), "elapsed={elapsed:?}");
    assert_eq!(active.load(Ordering::Acquire), 1);
    release.send(()).unwrap();
    wait_for_resolvers(&active);
}

#[test]
fn repeated_dns_timeouts_cannot_create_unbounded_resolver_threads() {
    let active = Arc::new(AtomicUsize::new(0));
    let mut releases = Vec::new();
    for _ in 0..MAX_RESOLVERS {
        let (release, released) = mpsc::channel();
        releases.push(release);
        let error = resolve_with(
            Instant::now() + Duration::from_millis(50),
            Arc::clone(&active),
            move || {
                released.recv().unwrap();
                Ok(Vec::new())
            },
        )
        .unwrap_err();
        assert!(error.message.contains("deadline exhausted"), "{error:?}");
    }
    let error = resolve_with(
        Instant::now() + Duration::from_secs(1),
        Arc::clone(&active),
        || panic!("saturated resolver pool must not start another lookup"),
    )
    .unwrap_err();
    assert!(error.message.contains("resolver capacity exhausted"));
    assert_eq!(active.load(Ordering::Acquire), MAX_RESOLVERS);
    for release in releases {
        release.send(()).unwrap();
    }
    wait_for_resolvers(&active);
}

#[test]
fn completed_or_failed_dns_lookups_release_capacity() {
    for result in [
        Ok(vec!["127.0.0.1:80".parse().unwrap()]),
        Err(io::Error::other("no address")),
    ] {
        let expected_success = result.is_ok();
        let active = Arc::new(AtomicUsize::new(0));
        let actual = resolve_with(
            Instant::now() + Duration::from_secs(1),
            Arc::clone(&active),
            || result,
        );
        assert_eq!(actual.is_ok(), expected_success);
        wait_for_resolvers(&active);
    }
}

#[test]
fn expired_deadlines_reject_resolution_without_starting_work() {
    let active = Arc::new(AtomicUsize::new(0));
    let result = resolve_with(Instant::now(), Arc::clone(&active), || {
        panic!("expired lookup")
    });
    assert!(result.is_err());
    assert_eq!(active.load(Ordering::Acquire), 0);
    assert!(resolve_before_deadline("127.0.0.1", 80, Instant::now()).is_err());
}

#[test]
fn literal_ip_resolution_does_not_need_a_background_lookup() {
    let result =
        resolve_before_deadline("127.0.0.1", 80, Instant::now() + Duration::from_secs(1)).unwrap();
    assert_eq!(result, vec!["127.0.0.1:80".parse::<SocketAddr>().unwrap()]);
}
