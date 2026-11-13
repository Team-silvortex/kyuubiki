# Native Operator Isolation: Bounded Linux Whole-Call Benchmark

Date: 2026-11-13. Source: `43020db6` plus the working-tree benchmark and adaptive
IPC change. The commit label is 3.8.0; compiled package metadata is still 3.7.8.
No version bump, daily installation, Store login or paid model API was performed.

## Method

- Physical Linux x86_64, AMD Ryzen 7 7735H, 16 available logical CPUs,
  kernel `7.0.0-42-generic`. Both Agent and trusted template were built with
  `--release --locked --offline`; compilation finished before timed execution.
- Installer installed and verified each Agent into a fresh owned test directory.
  The Agent listened only on an ephemeral loopback endpoint, with four execution
  slots and a 15-second native-call deadline. No daily service was restarted.
- Real `operator.template.summary` 0.1.0 library: 716168 bytes, digest
  `a5a23f4c4a94efd77ffdfc08ff368ee73da7b628b99148302bd6192457696cd5`.
- Inputs contain 3 or 8192 ones. Complete TaskIR sizes were 1759 and 34518 bytes.
  The latter is about 32 KiB of input, within the project's 64 KiB inline contract.
  This is a cheap summary operator measuring execution overhead, not a FEM solver.
- Three rounds per input/concurrency pair, 24 timed calls per round, 288 timed
  calls per build. Four warmup calls per input are excluded. Single/four-way
  ordering alternates; payloads are prepared before the timing barrier.
- A fresh RPC connection includes request serialization, Agent dispatch,
  child spawn/load, IPC, exact result identity/digest validation, child cleanup,
  response decode and client assertions. Installation/startup/task preparation
  are excluded. This is whole RPC latency, not plugin-body-only execution.
- Every call must produce the expected count/sum and report per-call process
  isolation with no automatic retry. Any failed call aborts the campaign rather
  than disappearing from the denominator. Throughput uses elapsed parallel wall
  time; percentiles use nearest rank. Results below are medians of three round
  statistics, not pooled percentiles or a confidence interval.

## Controlled Change And Results

Before: fixed 5 ms polling at accept/read/write waits. After: 1 ms waits during
the first 20 ms of an idle interval, then the original 5 ms backoff. A successful
transfer restarts only this idle interval, never the total deadline. Each sleep
is capped by remaining time. Cancellation, process ownership and kill/reap paths
are unchanged; there is no busy-spin or resident plugin reuse.

| Input values | Concurrency | p50 before / after, ms | p95 before / after, ms | Calls/s before / after |
| ---: | ---: | ---: | ---: | ---: |
| 3 | 1 | 11.058 / 6.206 | 11.154 / 7.263 | 90.3 / 155.2 |
| 3 | 4 | 11.035 / 6.052 | 16.185 / 6.950 | 336.7 / 640.2 |
| 8192 | 1 | 13.257 / 9.901 | 18.450 / 10.601 | 66.9 / 101.3 |
| 8192 | 4 | 13.041 / 8.992 | 18.103 / 9.835 | 267.8 / 434.1 |

The small-call p50 reduction is about 44-45%; the 8192-value reduction is about
25-31%. These are same-host short-run comparisons, not promised production rates.
CPU accounting includes Agent and reaped-child ticks: 2140 ms before and 2010 ms
after over the 288 calls. Tick resolution is 10 ms, so this shows no observed
increase, not a statistically established CPU saving.

Before Agent SHA-256:
`104cd5c1e4e7b41a6ef96076f5c807d0799d4c2f65df56d1ecb72d07d93d8b9f`.
After Agent SHA-256:
`ff17981ae58562ecf3541f497073d922de3f77e83f71f378200d41156968c90e`.

## Recovery And Resource Limits

A separate test-only fault library holds all four native slots. Each of three
rounds rejects four extra requests with `agent_at_capacity` before their marker
can be written, explicitly cancels all four held jobs, then executes a distinct
healthy task. All rounds pass before and after. Cancel-to-healthy times were
21.24-21.34 ms before and 9.79-10.01 ms after. This is short saturation recovery,
not a sustained overload or long-hang CPU benchmark.

The observer samples the owned Agent and visible descendants through every
thread's `/proc` child list, sleeping 5 ms between samples. Post-round checks
wait for child cleanup and RPC descriptor settlement; all measured rows ended
with one Agent and four Agent descriptors. Maximum observed process count was
five. Sampled aggregate RSS peaks were 45944 KiB before and 46912 KiB after.
RSS sums double-count shared pages and exclude the driver/observer.

Some short-lived native child descriptor directories returned permission errors;
the report counts these explicitly instead of treating them as zero descriptors.
The controlled held jobs were readable in these runs. No claim is made about
the exact cause of those short-lived permission windows, and no security policy
was disabled. Iterator disappearance is tolerated; other observation errors
remain failures. Samples are non-atomic observations, not hard memory/process/FD
ceilings or proof that an unobservable descendant cannot exist.

## Scope And Reproduction

The native test is `workers/rust/crates/cli/tests/operator_isolation_benchmark.rs`.
It is opt-in and Linux-only; statistics tests also run on macOS. Build the trusted
template first, set `KYUUBIKI_ISOLATION_BENCH_LIBRARY` to that release library,
`KYUUBIKI_ISOLATION_BENCH_REPORT` to a new report path and optionally
`KYUUBIKI_BENCH_SOURCE_REVISION` to the tested revision/overlay description, then:

```text
cargo test --release --locked --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test operator_isolation_benchmark installed_native_isolation_whole_call_and_saturation_benchmark -- --ignored --nocapture
```

Existing report files are not overwritten. Only a completed campaign writes a
complete JSON report; a failed campaign can leave an empty reserved output.
Owned test Agents, installations and package copies are removed on exit/unwind;
external SIGKILL or host power loss is outside that cleanup guarantee. Keep this
compact record and generators, not binary/store snapshots. Raw before/after JSON
is disposable ignored `tmp/` output, not Git content.

The tensor records only `verified` Linux evidence for
`agent-isolation-whole-call-performance`; its `qualified` target remains open.
Still missing: sustained/high-concurrency budgets, larger artifact-backed input
and output workloads, many-package load cost, cold starts, hard resource limits,
Mac/Windows performance and encrypted/network end-to-end timing. Prior native
TLS verification is separate. This does not certify malicious-code containment,
all installed fault cases, physics correctness or million-node solver throughput.

## Verification And Observer Fix

The percentile/throughput regression and adaptive-wait policy regression first
failed, then passed. Linux ran all eight benchmark tests including the opt-in
campaign. Mac and Linux each passed 338 Agent main-unit tests and four API
security tests. The final isolation suites on both platforms passed ten tests
(nine real fault/lifecycle cases and the new accepted-socket mode regression).

Mac's initial parallel isolation run had one auxiliary-connection EOF timeout
in `descendants_stop_on_success_crash_timeout_and_cancellation`. The isolated
case and two subsequent eight-thread runs passed, but those green repeats alone
were not accepted as a fix. A new regression then deterministically showed that
the accepted socket retained `O_NONBLOCK` on Mac (flag 4, expected 0). A read
timeout does not undo that flag, so the helper EOF check could fail immediately
before asynchronous descendant socket closure became visible. The fixture now
explicitly restores blocking mode before reading PID/EOF, without increasing
the 1500 ms deadline. Failure diagnostics retain the scenario, helper PID,
process group and liveness. The new regression went red-to-green and the full
ten-test suites passed on both platforms. This fixes a concrete observer race,
not proof that the product had leaked a descendant or that all installed
recovery is qualified.

Linux's full-dependency strict Clippy invocation stopped on six pre-existing
protocol style warnings in `operator_task_ir.rs` and
`types/nonlinear_structural.rs`. The three affected integration targets passed
strict `--no-deps` Clippy, including the Linux-only observer code. This is not a
claim that whole-workspace strict lint is clean. No whole-repository/Windows
test or fresh cross-host TLS deployment campaign was run in this performance step.

## Same-Day Continuous Load And Capacity Handoff

The later opt-in `installed_native_isolation_continuous_load_and_mixed_fault_recovery`
campaign keeps one Installer-managed Linux Agent for six 10-second windows, with
four client workers per batch and alternating 3 / 8192-value template inputs.
Each window includes task preparation and batch setup; separate RPC-batch timing
excludes those, and latency still includes result assertions. Short batch and
checkpoint gaps exist: this is not a fixed-arrival-rate or unbroken saturation
test. At most 16384 samples per window are retained in memory; raw samples are
not written to disk. Any unexpected failure aborts without retries or omission.

The first unmodified-capacity attempt failed within 0.41 seconds with
`agent_at_capacity` despite only four synchronous workers. A peer can receive the
final frame before `ReplyWriter::finish` completes accounting and releases its
execution lease. Retaining that lease is intentional protection against slow
response receivers, so releasing it before sending the frame is not the fix.

Full admission now waits on a condition variable with one 5 ms monotonic budget.
The last execution/delivery lease notifies a waiter; shutdown and drain notify
all. Every wake rechecks the control boundary and capacity under the same mutex.
Spurious wakeups do not renew the budget. Still-full admission is rejected before
execution. This is not automatic retry, extra capacity, fairness guarantees or a
promise against scheduling delays exceeding 5 ms. The descriptor exposes the
read-only `capacity_handoff_wait_ms` and `capacity_handoff_policy` fields.

Five deterministic unit regressions cover final-owner release, held-slot
rejection, drain/shutdown fencing, continuous spurious notifications and three
competing waiters for one slot. A test hook signals while holding the admission
mutex immediately before waiting, rather than claiming that a sleeping test
thread must have reached that boundary. Existing reply-delivery failure and
abandonment semantics are retained.

Two repaired release-build campaigns passed: 29184 and 27680 timed normal calls,
56864 total, plus eight warmups each. Both have six windows of at least ten
seconds. They are repeated functional/resource runs, not a controlled speedup
comparison. Final Agent digest:
`f9623c0ecee3dbda6a3f3e6354776d6e93f9c14cdfcc999b811fcebb7a7c2101`.
Library identity and physical host are unchanged from the earlier method.

| Final window | Input values | Calls | RPC p95, ms | Settled Agent RSS, KiB |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 3 | 5824 | 8.336 | 9488 |
| 1 | 8192 | 3232 | 11.560 | 11352 |
| 2 | 3 | 5952 | 8.157 | 11352 |
| 3 | 8192 | 3360 | 10.715 | 11512 |
| 4 | 3 | 6080 | 8.103 | 11512 |
| 5 | 8192 | 3232 | 12.666 | 11512 |

The final warm baseline was 8548 KiB. Every checkpoint returned to one observed
Agent, exactly four Agent descriptors and zero active watchdog/admission entries.
The settled RSS growth guard is 32 MiB above warm baseline, a bounded regression
check rather than proof of zero leaks or a security resource ceiling. The same
non-atomic `/proc` observability limitations described above still apply.

A separate Installer-managed fault-library Agent has a 1000 ms per-call budget.
Each of eight rounds runs abort, invalid pointer, entry hang, free hang, wrong
identity, empty identity, contradictory success, four held calls cancelled after
entry markers, four excess requests which must never execute, then four distinct
healthy calls. Per campaign this is 88 expected execution failures, 40 healthy
results, 32 pre-admission capacity rejections and four healthy warmups. Exact
watchdog totals are 132 admitted / 44 completed / 88 failed; recent failures stay
at 16, not an unbounded history. Final fault-Agent RSS was 10024 KiB versus an
8480 KiB baseline, with one process and four descriptors. Tests validate sum=14,
operator identity, pinned library digest and no automatic retry for healthy work.

Set `KYUUBIKI_ISOLATION_SOAK_REPORT` to a new output and the same trusted
`KYUUBIKI_ISOLATION_BENCH_LIBRARY`, optionally record the source overlay, then:

```text
cargo test --release --locked --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test operator_isolation_benchmark installed_native_isolation_continuous_load_and_mixed_fault_recovery -- --ignored --nocapture
```

Do not run the short benchmark and soak concurrently when comparing timing.
Failed attempts leave an empty reserved report, never a success document. Reports
are small ignored `tmp/` outputs. Owned installations are cleaned on normal exit
or panic; external kill/power loss remains outside this fixture cleanup guarantee.
Daily services were not restarted. No test-named `/var/crash` reports were found;
the Linux host routes crashes through Apport even with the shell core limit zero.

The tensor adds only `linux-installed-native-soak-recovery` at `verified` grade.
The broader Unix-installed recovery target stays `qualified` and unmet: matching
GUI/SDK journeys, Mac installed faults, parent-loss in this installed campaign,
hours/days of load, large artifacts, TLS/network faults and hostile code remain
outside this run. Separately, inspection found that unmatched legacy `cancel_job`
requests still accumulate in an unbounded pending-ID set; these campaigns cancel
active jobs only and do not establish that the pending-cancel path is bounded.

Final follow-up checks: Mac and Linux each passed 343 Agent unit tests, four API
security tests and ten real/default process-isolation tests. The five handoff
tests include cleanup on their own timeout-failure path, so a renewed-deadline
regression cannot strand the test's held slot during thread joining. Linux's
eight default benchmark/observer guards passed, with the two opt-in campaigns
listed as ignored in that default invocation; the new soak was run explicitly
twice above. The native tool suite passed all 419 tests. Documentation inventory,
book, Store contracts, formatting and source/doc line limits passed.

Mac strict targeted Clippy and Linux's dedicated benchmark/soak target with
`--no-deps -D warnings` passed. Linux's expanded strict invocation including
the Agent binary and the older process-isolation test encountered nine existing
binary style findings plus one nested-if finding in that older test under
Clippy 1.95. These are not this run's execution failures and are not silently
suppressed or treated as a clean whole-target lint result. No remote GitHub CI,
Windows qualification, installed App update or commit/push was performed.

## Pending-Cancellation Follow-Up

A later, separate correctness campaign closed the unbounded pending-ID memory
gap identified above, without reclassifying this soak as its evidence. Pending
intents are now capped at 1024 identities of at most 256 bytes; excess requests
fail explicitly without eviction or package cleanup. Active/exact cancellation
remains available at that capacity. Mac and Linux source Agents each retained
and consumed all 1024 intents over local TLS, rejected 32 excess requests and
preserved one-shot behavior across capacity/drain rejection and healthy next
tasks. Restart is explicitly nondurable, not a way to safely clear cancellations.
The [separate record](tensor-recalibration-20261113.md#same-day-bounded-pending-cancellation-follow-up)
documents test counts, package-retention evidence limits and a new Linux short
TLS-request wall-time discrepancy requiring independent performance diagnosis.

## Cold TLS Control-Frame Latency

The pending-cancellation follow-up exposed 350.00 seconds on Linux versus 4.53
seconds on Mac for the same correctness integration binary. This discrepancy
was investigated separately, not attributed to solver work or registry growth.

`agent-transport` now sets `TCP_NODELAY` on both endpoints before TLS handshake.
The frame format, split length/body writes, TLS 1.3, full authentication,
disabled client session resumption, deadlines and synchronous TLS-drain-before-
write-success contract are unchanged. Socket-option failure propagates as an
error; there is no plaintext fallback. Infallible plain `Connection::from`
keeps its previous socket behavior.

The new both-endpoint socket-policy regression failed before the change and
passed afterwards. The controlled experiment uses the production security and
connection implementations, one fresh TCP/TLS/auth connection per short frame
exchange, three sequential rounds of 32 calls, exact echoed payload checks and
nearest-rank percentiles. It does not execute an Agent solver. The listener
polls with a bounded 1 ms readiness interval, included in authentication timing;
the same fixture is used before and after. Certificates and test-only token stay
in memory. Test threads have bounded accept/I/O waits on failure.

Same physical Linux x86_64 host and optimized toolchain, no compilation or other
owned tests concurrent with the final filtered measurements:

| Phase / milliseconds | Before R1 / R2 / R3 | After R1 / R2 / R3 |
| --- | --- | --- |
| TCP connect p50 | 0.063 / 0.062 / 0.061 | 0.035 / 0.033 / 0.035 |
| TLS handshake + token authentication p50 | 82.867 / 82.822 / 82.894 | 1.394 / 1.382 / 1.379 |
| Split-frame request/response p50 | 41.075 / 41.107 / 41.078 | 0.038 / 0.038 / 0.040 |
| Whole cold call p50 | 123.993 / 124.535 / 124.858 | 1.475 / 1.460 / 1.454 |
| Whole cold call p95 | 125.916 / 125.955 / 125.950 | 1.640 / 1.678 / 1.597 |

The only production change for this comparison is the socket option. These
results support the small-write/Nagle interaction as the dominant delay in this
workload; no packet capture was taken, so a precise delayed-ACK packet sequence
is not claimed. Authentication timing combines handshake and token exchange,
not a separate cryptographic-cost measurement. Background host services were
not stopped. An earlier exploratory after-run occurred alongside transport unit
tests; the table uses the isolated filtered rerun instead.

The full Linux pending-cancellation integration then passed in 1.39 seconds
instead of 350.00, with the same 1024 inserts/consumptions and rejection/recovery
assertions. Mac's after-run was 4.61 seconds; no Mac speedup is claimed. Both
hosts again passed 351 Agent main-unit tests, 11 integration entries (including
shared readiness guards), and all eight transport tests including the explicitly
run latency experiment. Existing wrong-token/trust/name/unauthenticated-input
rejection remains tested. The other seven transport tests remain in the default
suite; the timing experiment is opt-in and has no host-dependent timing gate.

Reproduce alone, without mixing compilation into the measured stages:

```sh
cargo test --release --locked --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-agent-transport cold_tls_auth_and_split_frame_latency -- --ignored --nocapture
```

This is short, loopback, cold-connection evidence, not WAN/LAN throughput,
large-artifact bandwidth, persistent-session scaling, native-library execution,
or numerical speed. More small packets can trade latency for packet overhead;
those traffic shapes require separate measurements. No authentication weakening,
batching, retry or higher concurrency is used. Daily services are not restarted.
