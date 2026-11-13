#[path = "support/operator_benchmark_metrics.rs"]
mod metrics;

#[cfg(target_os = "linux")]
#[path = "support/operator_benchmark_fixture.rs"]
mod fixture;
#[cfg(target_os = "linux")]
#[path = "support/operator_benchmark_resources.rs"]
mod resources;
#[cfg(target_os = "linux")]
#[path = "support/operator_isolation_soak.rs"]
mod soak;
#[cfg(target_os = "linux")]
#[path = "support/operator_cross_host_transport.rs"]
mod transport;

#[cfg(target_os = "linux")]
mod linux {
    use super::{
        fixture::{CAPACITY, Fixture},
        metrics::latency_summary,
        resources::{Monitor, snapshot},
    };
    use serde_json::{Value, json};
    use std::{
        fs,
        io::Write,
        path::PathBuf,
        sync::Barrier,
        thread,
        time::{Duration, Instant},
    };

    const SAMPLES: usize = 24;
    const ROUNDS: usize = 3;

    pub(super) fn checked_run(f: &Fixture, id: &str, task: Value, count: usize) -> f64 {
        let started = Instant::now();
        let result = f
            .execute(id, task)
            .expect("benchmark task failed; never omit failures");
        assert_eq!(
            result["execution_runtime_status"],
            "external_operator_package_executed"
        );
        assert_eq!(
            result["operator_package_execution"]["entrypoint_sha256"],
            f.pin.entrypoint_sha256
        );
        assert_eq!(
            result["operator_package_execution"]["isolation"]["mode"],
            "per_call_process"
        );
        assert_eq!(
            result["operator_package_execution"]["isolation"]["automatic_retry"],
            false
        );
        assert_eq!(result["result"]["operator_id"], "extract.template_summary");
        assert_eq!(result["result"]["summary"]["count"], count);
        assert_eq!(result["result"]["summary"]["sum"], count as f64);
        started.elapsed().as_secs_f64() * 1000.0
    }

    fn idle_resources(f: &Fixture) -> super::resources::Resources {
        settled_resources(f, usize::MAX)
    }

    pub(super) fn settled_resources(f: &Fixture, fd_ceiling: usize) -> super::resources::Resources {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let sample = snapshot(f.pid()).unwrap();
            if sample.processes == 1 && sample.agent_file_descriptors <= fd_ceiling {
                return sample;
            }
            assert!(
                Instant::now() < deadline,
                "owned children or RPC descriptors did not settle after completion"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn measure(f: &Fixture, count: usize, concurrency: usize, round: usize) -> Value {
        let input = json!({"values":vec![1.0; count]});
        let tasks: Vec<_> = (0..SAMPLES)
            .map(|n| {
                let id = format!("bench-{count}-{concurrency}-{round}-{n}");
                let task = f.task(&id, json!({}), input.clone());
                (id, task)
            })
            .collect();
        let task_bytes = serde_json::to_vec(&tasks[0].1).unwrap().len();
        let mut tasks = tasks.into_iter();
        let batches: Vec<Vec<_>> = (0..concurrency)
            .map(|_| tasks.by_ref().take(SAMPLES / concurrency).collect())
            .collect();
        let before = idle_resources(f);
        let monitor = Monitor::start(f.pid());
        let barrier = Barrier::new(concurrency + 1);
        let (samples, elapsed) = thread::scope(|scope| {
            let workers: Vec<_> = batches
                .into_iter()
                .map(|chunk| {
                    let barrier = &barrier;
                    scope.spawn(move || {
                        barrier.wait();
                        chunk
                            .into_iter()
                            .map(|(id, task)| checked_run(f, &id, task, count))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            let started = Instant::now();
            barrier.wait();
            let samples: Vec<_> = workers
                .into_iter()
                .flat_map(|w| w.join().unwrap())
                .collect();
            (samples, started.elapsed().as_secs_f64())
        });
        let (peak, observations) = monitor.finish();
        let after = settled_resources(f, before.agent_file_descriptors + 4);
        assert_eq!(samples.len(), SAMPLES);
        assert!(observations > 0);
        assert!(
            after.agent_file_descriptors <= before.agent_file_descriptors + 4,
            "file descriptors did not settle"
        );
        let mut summary = latency_summary(&samples, elapsed).unwrap();
        let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        assert!(ticks_per_second > 0);
        summary["agent_and_reaped_child_cpu_ms"] = json!(
            (after
                .cpu_ticks_agent_and_reaped_children
                .checked_sub(before.cpu_ticks_agent_and_reaped_children)
                .unwrap()) as f64
                * 1000.0
                / ticks_per_second as f64
        );
        summary["cpu_tick_ms"] = json!(1000.0 / ticks_per_second as f64);
        summary["round"] = json!(round);
        summary["input_values"] = json!(count);
        summary["task_json_bytes"] = json!(task_bytes);
        summary["concurrency"] = json!(concurrency);
        summary["sample_ms"] = json!(samples);
        summary["resources_before"] = json!(before);
        summary["resources_sampled_peak"] = json!(peak);
        summary["resources_after"] = json!(after);
        summary["resource_samples"] = json!(observations);
        println!(
            "isolation bench: values={count} concurrent={concurrency} round={round} p50={}ms p95={}ms throughput={}/s",
            summary["p50_ms"], summary["p95_ms"], summary["throughput_per_second"]
        );
        summary
    }

    fn saturation_recovery() -> Value {
        let f = Fixture::new(None);
        let baseline = idle_resources(&f);
        let mut rounds = Vec::new();
        for round in 0..ROUNDS {
            let ids: Vec<_> = (0..CAPACITY).map(|n| format!("hold-{round}-{n}")).collect();
            let markers: Vec<_> = ids
                .iter()
                .map(|id| f.root.join(format!("{id}.pid")))
                .collect();
            let monitor = Monitor::start(f.pid());
            let recovery_ms = thread::scope(|scope| {
                let jobs: Vec<_> = ids
                    .iter()
                    .zip(&markers)
                    .map(|(id, marker)| {
                        let f = &f;
                        scope.spawn(move || {
                            f.execute(
                                id,
                                f.task(id, json!({"fault":"hang", "marker":marker}), json!({})),
                            )
                        })
                    })
                    .collect();
                let deadline = Instant::now() + Duration::from_secs(5);
                while !markers.iter().all(|p| {
                    fs::read_to_string(p)
                        .ok()
                        .and_then(|s| s.parse::<u32>().ok())
                        .is_some()
                }) {
                    assert!(
                        Instant::now() < deadline,
                        "not all four native slots became active"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
                assert_eq!(snapshot(f.pid()).unwrap().processes, CAPACITY + 1);
                let rejected_marker = f.root.join(format!("rejected-{round}.pid"));
                for n in 0..4 {
                    let id = format!("excess-{round}-{n}");
                    let error = f
                        .execute(
                            &id,
                            f.task(
                                &id,
                                json!({"fault":"healthy", "marker":rejected_marker}),
                                json!({}),
                            ),
                        )
                        .unwrap_err();
                    assert!(error.contains("agent_at_capacity"), "{error}");
                }
                assert!(!rejected_marker.exists(), "rejected work executed anyway");
                let started = Instant::now();
                for id in &ids {
                    f.client
                        .request_named(&format!("cancel-{id}"), "cancel_job", json!({"job_id":id}))
                        .unwrap();
                }
                for job in jobs {
                    assert!(job.join().unwrap().unwrap_err().contains("cancelled"));
                }
                let id = format!("healthy-after-{round}");
                let result = f
                    .execute(&id, f.task(&id, json!({"fault":"healthy"}), json!({})))
                    .unwrap();
                assert_eq!(result["result"]["summary"]["sum"], 14);
                let after = settled_resources(&f, baseline.agent_file_descriptors + 4);
                assert!(after.agent_file_descriptors <= baseline.agent_file_descriptors + 4);
                started.elapsed().as_secs_f64() * 1000.0
            });
            let (peak, observations) = monitor.finish();
            rounds.push(json!({"round":round, "held_slots":CAPACITY, "rejected_requests":4,
                "cancelled_requests":CAPACITY,"healthy_after":true,"cancel_to_healthy_ms":recovery_ms,
                "resources_sampled_peak":peak,"resource_samples":observations}));
        }
        json!({"fixture":"test-only-fault-library", "rounds":rounds})
    }

    #[test]
    #[ignore = "Linux-only owned Installer-managed Agent benchmark; needs trusted template library and new report path"]
    fn installed_native_isolation_whole_call_and_saturation_benchmark() {
        let output = PathBuf::from(
            std::env::var("KYUUBIKI_ISOLATION_BENCH_REPORT").expect("new report path"),
        );
        // Reserve before starting processes: existing evidence must not be silently overwritten.
        let mut report_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .unwrap();
        let library = PathBuf::from(
            std::env::var("KYUUBIKI_ISOLATION_BENCH_LIBRARY")
                .expect("trusted native template library"),
        );
        let f = Fixture::new(Some(&library));
        let descriptor = f.client.request("describe_agent", json!({})).unwrap();
        let mut rows = Vec::new();
        for count in [3, 8192] {
            for n in 0..4 {
                let id = format!("warmup-{count}-{n}");
                checked_run(
                    &f,
                    &id,
                    f.task(&id, json!({}), json!({"values":vec![1.0;count]})),
                    count,
                );
            }
            for round in 0..ROUNDS {
                // Alternate ordering to reduce one-sided cache/order bias.
                for concurrency in if round % 2 == 0 {
                    [1, CAPACITY]
                } else {
                    [CAPACITY, 1]
                } {
                    rows.push(measure(&f, count, concurrency, round));
                }
            }
        }
        let report = json!({
            "schema_version":"kyuubiki.operator-isolation-benchmark/v1", "complete":true,
            "package_metadata_version":env!("CARGO_PKG_VERSION"),
            "source_revision":std::env::var("KYUUBIKI_BENCH_SOURCE_REVISION").ok(),
            "profile":if cfg!(debug_assertions) {"debug"} else {"optimized"},
            "platform":format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH),
            "kernel":fs::read_to_string("/proc/sys/kernel/osrelease").unwrap().trim(),
            "cpu_model":fs::read_to_string("/proc/cpuinfo").unwrap().lines().find_map(|l| l.strip_prefix("model name\t: ")).unwrap_or("unknown"),
            "available_parallelism":thread::available_parallelism().unwrap().get(),
            "agent_sha256":f.agent_sha256,"operator_pin":f.pin,
            "operator_library_bytes":fs::metadata(library).unwrap().len(),
            "agent_process_policy":descriptor["operator_package_runtime"],
            "clock":"monotonic Instant", "percentiles":"nearest-rank, per round; no discarded failures",
            "measurement":"fresh loopback RPC through native child load/IPC/validate/cleanup plus client result assertions; excludes task preparation and initial Agent install/start",
            "resource_scope":"sampled Agent + observable descendants via all /proc thread child lists, 5ms sleep; non-atomic observations, not hard ceilings; protected child fd/list permission denials counted explicitly, not treated as zero usage; RSS sums double-count shared pages; observer/client excluded",
            "transport":"loopback TCP, no TLS; cross-host TLS qualification is separate",
            "rows":rows,"saturation_recovery":saturation_recovery()
        });
        report_file
            .write_all(&serde_json::to_vec_pretty(&report).unwrap())
            .unwrap();
        report_file.sync_all().unwrap();
        println!("owned native isolation benchmark completed; no automatic task retry");
    }
}
