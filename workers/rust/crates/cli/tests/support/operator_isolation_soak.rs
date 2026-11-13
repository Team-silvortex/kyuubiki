use super::{
    fixture::{CAPACITY, Fixture},
    linux::{checked_run, settled_resources},
    metrics::latency_summary,
    resources::Monitor,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::Barrier,
    thread,
    time::{Duration, Instant},
};

const WINDOWS: usize = 6;
const WINDOW_SECONDS: u64 = 10;
const BATCH: usize = 32;
const MAX_WINDOW_CALLS: usize = 16_384;
const FAULT_ROUNDS: usize = 8;

fn verify_accounting(value: &Value, completed: u64, failed: u64) -> Result<(), String> {
    let watchdog = &value["watchdog"];
    for (key, expected) in [
        ("active_execution_count", 0),
        ("total_started_execution_count", completed + failed),
        ("total_completed_execution_count", completed),
        ("total_failed_execution_count", failed),
        ("recent_failure_count", failed.min(16)),
    ] {
        if watchdog[key].as_u64() != Some(expected) {
            return Err(format!(
                "unexpected watchdog {key}: {} != {expected}",
                watchdog[key]
            ));
        }
    }
    if watchdog["active_executions"].as_array().map(Vec::len) != Some(0)
        || watchdog["recent_failures"].as_array().map(Vec::len) != Some(failed.min(16) as usize)
        || value["execution_admission"]["active_execution_count"] != 0
        || value["execution_admission"]["max_active_executions"] != CAPACITY
    {
        return Err("execution admission or bounded failure history did not settle".into());
    }
    Ok(())
}

fn checkpoint(f: &Fixture, completed: u64, failed: u64, fd_baseline: usize) -> Value {
    // Delivery leases can outlive client receipt briefly; never infer leakage from that race.
    let deadline = Instant::now() + Duration::from_secs(3);
    let descriptor = loop {
        let descriptor = f.client.request("describe_agent", json!({})).unwrap();
        match verify_accounting(&descriptor, completed, failed) {
            Ok(()) => break descriptor,
            Err(error) => {
                assert!(Instant::now() < deadline, "{error}");
                thread::sleep(Duration::from_millis(5));
            }
        }
    };
    let resources = settled_resources(f, fd_baseline);
    assert_eq!(resources.agent_file_descriptors, fd_baseline);
    json!({"resources":resources, "watchdog":descriptor["watchdog"],
        "execution_admission":descriptor["execution_admission"]})
}

fn continuous_template(library: &Path) -> Value {
    let f = Fixture::new(Some(library));
    for n in 0..8 {
        let id = format!("soak-warm-{n}");
        checked_run(
            &f,
            &id,
            f.task(&id, json!({}), json!({"values":[1,1,1]})),
            3,
        );
    }
    let baseline = settled_resources(&f, 4);
    let fd_baseline = baseline.agent_file_descriptors;
    let mut completed = 8;
    let mut windows = Vec::new();
    for window in 0..WINDOWS {
        let count = if window % 2 == 0 { 3 } else { 8192 };
        let input = json!({"values":vec![1.0; count]});
        let started = Instant::now();
        let monitor = Monitor::start(f.pid());
        let mut samples = Vec::new();
        let mut measured_seconds = 0.0;
        while started.elapsed() < Duration::from_secs(WINDOW_SECONDS) {
            assert!(
                samples.len() + BATCH <= MAX_WINDOW_CALLS,
                "bounded sample budget exhausted"
            );
            let chunks: Vec<Vec<_>> = (0..CAPACITY)
                .map(|worker| {
                    (0..BATCH / CAPACITY)
                        .map(|n| {
                            let id = format!("soak-{window}-{}-{worker}-{n}", samples.len());
                            let task = f.task(&id, json!({}), input.clone());
                            (id, task)
                        })
                        .collect()
                })
                .collect();
            let barrier = Barrier::new(CAPACITY + 1);
            let (batch, elapsed) = thread::scope(|scope| {
                let workers: Vec<_> = chunks
                    .into_iter()
                    .map(|chunk| {
                        let (f, barrier) = (&f, &barrier);
                        scope.spawn(move || {
                            barrier.wait();
                            chunk
                                .into_iter()
                                .map(|(id, task)| checked_run(f, &id, task, count))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                let clock = Instant::now();
                barrier.wait();
                let samples: Vec<_> = workers
                    .into_iter()
                    .flat_map(|w| w.join().unwrap())
                    .collect();
                (samples, clock.elapsed().as_secs_f64())
            });
            samples.extend(batch);
            measured_seconds += elapsed;
        }
        let elapsed = started.elapsed().as_secs_f64();
        let (peak, observations) = monitor.finish();
        completed += samples.len() as u64;
        let after = checkpoint(&f, completed, 0, fd_baseline);
        // A bounded regression guard, not a proof of zero allocator retention or a hard RSS cap.
        assert!(
            after["resources"]["rss_kib"].as_u64().unwrap() <= baseline.rss_kib + 32 * 1024,
            "settled Agent RSS grew more than 32 MiB above warm baseline"
        );
        let latency = latency_summary(&samples, measured_seconds).unwrap();
        println!(
            "soak window {window}: {} calls, {:.2}s, p95={}ms, settled RSS={}KiB",
            samples.len(),
            elapsed,
            latency["p95_ms"],
            after["resources"]["rss_kib"]
        );
        windows.push(
            json!({"window":window,"input_values":count,"window_wall_seconds":elapsed,
            "window_throughput_per_second":samples.len() as f64 / elapsed,
            "rpc_batches_seconds":measured_seconds,"latency":latency,
            "resources_sampled_peak":peak,"resource_samples":observations,"after":after}),
        );
    }
    json!({"fixture":"trusted-template", "agent_sha256":f.agent_sha256,"operator_pin":f.pin,
        "warmup_calls":8,"timed_calls":completed-8,"concurrency":CAPACITY,
        "warm_baseline":baseline,"settled_rss_growth_guard_kib":32*1024,"windows":windows})
}

fn healthy(f: &Fixture, id: &str) {
    let value = f
        .execute(id, f.task(id, json!({"fault":"healthy"}), json!({})))
        .unwrap();
    assert_eq!(value["result"]["operator_id"], "extract.template_summary");
    assert_eq!(value["result"]["summary"]["sum"], 14);
    assert_eq!(
        value["operator_package_execution"]["entrypoint_sha256"],
        f.pin.entrypoint_sha256
    );
    assert_eq!(
        value["operator_package_execution"]["isolation"]["automatic_retry"],
        false
    );
}

fn repeated_faults() -> Value {
    let f = Fixture::with_run_timeout(None, 1000);
    for n in 0..4 {
        healthy(&f, &format!("fault-warm-{n}"));
    }
    let baseline = settled_resources(&f, 4);
    let mut windows = Vec::new();
    for round in 0..FAULT_ROUNDS {
        let monitor = Monitor::start(f.pid());
        let start = Instant::now();
        for faults in [
            [
                ("abort", "operator_process_crashed"),
                ("invalid_pointer", "operator_process_crashed"),
                ("hang", "operator_process_timeout"),
                ("free_hang", "operator_process_timeout"),
            ],
            [
                ("wrong_result_id", "operator_package_dispatch_failed"),
                ("empty_result_id", "operator_package_dispatch_failed"),
                ("ambiguous_success", "operator_package_dispatch_failed"),
                ("healthy", ""),
            ],
        ] {
            thread::scope(|scope| {
                let workers: Vec<_> = faults
                    .into_iter()
                    .map(|(fault, code)| {
                        let f = &f;
                        scope.spawn(move || {
                            let id = format!("fault-{round}-{fault}");
                            if code.is_empty() {
                                healthy(f, &id);
                            } else {
                                let error = f
                                    .execute(&id, f.task(&id, json!({"fault":fault}), json!({})))
                                    .unwrap_err();
                                assert_eq!(error, format!("RPC rejected: \"{code}\""), "{id}");
                            }
                        })
                    })
                    .collect();
                for w in workers {
                    w.join().unwrap();
                }
            });
        }
        let ids: Vec<_> = (0..CAPACITY)
            .map(|n| format!("soak-cancel-{round}-{n}"))
            .collect();
        let markers: Vec<_> = ids
            .iter()
            .map(|id| f.root.join(format!("{id}.pid")))
            .collect();
        thread::scope(|scope| {
            let jobs: Vec<_> = ids
                .iter()
                .zip(&markers)
                .map(|(id, marker)| {
                    let f = &f;
                    scope.spawn(move || {
                        f.execute(
                            id,
                            f.task(id, json!({"fault":"hang","marker":marker}), json!({})),
                        )
                    })
                })
                .collect();
            let deadline = Instant::now() + Duration::from_millis(800);
            while !markers.iter().all(|p| {
                fs::read_to_string(p)
                    .ok()
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some()
            }) {
                assert!(
                    Instant::now() < deadline,
                    "held calls did not start inside their 1s budget"
                );
                thread::sleep(Duration::from_millis(2));
            }
            let rejected = f.root.join(format!("not-executed-{round}.pid"));
            for n in 0..CAPACITY {
                let id = format!("soak-excess-{round}-{n}");
                let error = f
                    .execute(
                        &id,
                        f.task(&id, json!({"fault":"healthy","marker":rejected}), json!({})),
                    )
                    .unwrap_err();
                assert_eq!(error, "RPC rejected: \"agent_at_capacity\"");
            }
            assert!(!rejected.exists());
            for id in &ids {
                f.client
                    .request_named(&format!("cancel-{id}"), "cancel_job", json!({"job_id":id}))
                    .unwrap();
            }
            for job in jobs {
                assert_eq!(
                    job.join().unwrap().unwrap_err(),
                    "RPC rejected: \"cancelled\""
                );
            }
        });
        thread::scope(|scope| {
            let jobs: Vec<_> = (0..CAPACITY)
                .map(|n| {
                    let f = &f;
                    scope.spawn(move || healthy(f, &format!("recovered-{round}-{n}")))
                })
                .collect();
            for job in jobs {
                job.join().unwrap();
            }
        });
        let (peak, observations) = monitor.finish();
        let after = checkpoint(
            &f,
            4 + (round as u64 + 1) * 5,
            (round as u64 + 1) * 11,
            baseline.agent_file_descriptors,
        );
        assert!(after["resources"]["rss_kib"].as_u64().unwrap() <= baseline.rss_kib + 32 * 1024);
        windows.push(
            json!({"round":round,"wall_ms":start.elapsed().as_secs_f64()*1000.0,
            "expected_failures":11,"healthy_calls":5,"not_admitted":4,"after":after,
            "resources_sampled_peak":peak,"resource_samples":observations}),
        );
        println!(
            "mixed-fault round {round}: 11 expected failures, 5 healthy, 4 capacity rejections; settled"
        );
    }
    json!({"fixture":"test-only-fault-library","agent_sha256":f.agent_sha256,
        "operator_pin":f.pin,"run_timeout_ms":1000,"warmup_calls":4,"warm_baseline":baseline,"rounds":windows})
}

#[test]
#[ignore = "Linux-only 60s owned Installer-managed continuous load plus repeated native faults; explicit report/library required"]
fn installed_native_isolation_continuous_load_and_mixed_fault_recovery() {
    let output = std::env::var("KYUUBIKI_ISOLATION_SOAK_REPORT").expect("new report path");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    let library =
        std::env::var("KYUUBIKI_ISOLATION_BENCH_LIBRARY").expect("trusted template library");
    let report = json!({"schema_version":"kyuubiki.operator-isolation-soak/v1","complete":true,
        "source_revision":std::env::var("KYUUBIKI_BENCH_SOURCE_REVISION").ok(),
        "package_metadata_version":env!("CARGO_PKG_VERSION"),
        "profile":if cfg!(debug_assertions) {"debug"} else {"optimized"},
        "platform":format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH),
        "scope":"same owned Agent per phase; loopback/no TLS; short soak not production longevity or a security sandbox; 5ms non-atomic resource sampling with unreadable children counted; 32MiB settled RSS growth guard; no automatic retry",
        "continuous":continuous_template(Path::new(&library)),"mixed_faults":repeated_faults()});
    file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
    file.sync_all().unwrap();
}

#[test]
fn soak_accounting_rejects_leaked_slots_missing_failures_and_unbounded_history() {
    let valid = json!({"execution_admission":{"active_execution_count":0,"max_active_executions":4},
        "watchdog":{"active_execution_count":0,"active_executions":[],
            "total_started_execution_count":25,"total_completed_execution_count":5,"total_failed_execution_count":20,
            "recent_failure_count":16,"recent_failures":vec![json!({});16]}});
    assert!(verify_accounting(&valid, 5, 20).is_ok());
    for (key, value) in [
        ("active_execution_count", json!(1)),
        ("total_failed_execution_count", json!(19)),
        ("recent_failure_count", json!(20)),
        ("recent_failures", json!(vec![json!({}); 20])),
        ("active_executions", json!([{}])),
    ] {
        let mut bad = valid.clone();
        bad["watchdog"][key] = value;
        assert!(verify_accounting(&bad, 5, 20).is_err(), "{key}");
    }
    let mut bad = valid;
    bad["execution_admission"]["active_execution_count"] = json!(1);
    assert!(verify_accounting(&bad, 5, 20).is_err());
}
