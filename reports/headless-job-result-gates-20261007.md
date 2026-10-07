# Headless Job Result Gates

Date: 2026-10-07. Base checkout: `76aaf10d`, preserving the preceding native
binding, confirmation and original-receipt repairs. Native job observations
must match the requested identity, and result reads require successful job
completion. A retained runtime object is not sufficient to advance a batch.

## Repair

Test-first regression reproduced five failing test groups: retained results
from incomplete jobs, null/scalar results, invalid job identities/statuses,
invalid result envelopes, and the metadata-only fetch path. A later regression
also reproduced acceptance of a result envelope carrying an explicit failed
status. All six final result-gate tests now pass, including exact status-set
agreement with the public `schemas/job.schema.json` enum.

`job_wait` validates the nested job object, exact requested ID, recognized
public status, and any repeated top-level identity/status before normalization
or another poll. Missing, foreign and contradictory observations fail closed.
Rejected foreign values are not echoed in the failure message. Existing
deadline tests now use the contract's `solving` state rather than the nonexistent
`running` state, with the missing fixture ID restored; timeout budgets, poll
counts and recovery assertions remain unchanged.

`result_fetch` requires a matching completed job and an explicit object result.
Queued, preprocessing, partitioning, solving, postprocessing, failed and
cancelled records cannot authorize later actions, even with a stored object.
Null, scalar and array results fail instead of becoming executed steps.

The default path checks `/api/v1/jobs/:job_id` and consumes its object result.
A completed response without a result may fall back to `/api/v1/results/:job_id`,
which must explicitly return the same ID and an object. Any supplied job or
status must also agree with completion. `prefer_job_result: false` first reads
the metadata-only `/status` route, then the result route; it no longer bypasses
completion, and does not download a large result twice. No duplicate `raw`
result mirror is retained.

Invalid identity/status receipts report `kyuubiki.headless.job_receipt_invalid`
at `job_observation`; unavailable results report
`kyuubiki.headless.job_result_unavailable` at `result_fetch`. Both gate failures
are non-retryable with strategy `none`. The batch stops without publishing
that step's bindings or dispatching subsequent writes. Callers inspect the
same job, explicitly wait if still active, and author an intended continuation
after completion; these failures never grant replacement submission.

The unit HTTP fixtures cover both fetch preferences, all seven non-completed
states, malformed values/identity/status, fallback envelopes, empty objects,
and opaque result references. A 256-item array is compacted in the run preview
without a `raw` mirror; the real positive test separately checks complete
payload forwarding before report compaction. The shared receipt validator and
result transport live in separate small modules, keeping `service_executor.rs`
below the 800-line source limit without changing engine or operator semantics.

## Real Service Chains

`real_retained_incomplete_results_fail_sdk_and_cli_before_downstream_writes`
owns an Orchestra, SQLite database, two real Agents and temporary storage.
The opt-in test harness creates queued, solving, failed and cancelled runtime
records through the real transactional job/result store and progress updater.
These are seeded incomplete records, not simulations of scientific failures
or fabricated HTTP responses. Ordinary harness startup explicitly disables
the fixture; production startup has no new fault or fixture switch.

The real job API reports `has_result: true`, and the real result API exposes
each retained object. For all four states and both fetch preferences, native
SDK execution stops at the first step with zero successful steps and no
project creation. The actual CLI repeats the eight cases, exits nonzero and
emits the same failed report to stdout and `--report-out`; JSON stderr is
`headless_execution_failed` with `retryable: false`. Job/result/project listings
and each job observation remain unchanged. Both Agent execution counters stay
zero, and no TaskIR dispatch journal is created.

`real_completed_bar_result_can_be_fetched_and_continued_without_resubmission`
uses a separate unseeded real service chain. Its four native steps submit a
bar solve, wait on its bound ID, fetch through the non-preferred path, and
create an owned project using the completed result. Axial tip displacement
matches the independently calculated `FL/EA` within relative `1e-12`. The
captured downstream payload equals the full solver result, the project belongs
to the same job, and the two Agents together execute exactly one calculation.
Subsequent explicit result reads with both preferences return the same object
and do not increase that count. This verifies result reuse, not replay deduplication.

## Verification

Final macOS ARM64 source checks:

- Complete `headless_live`: 28/28 passed in 47.22 seconds.
- Complete Headless SDK: 327 unit tests in 31.37 seconds and 25 integrations passed.
- Complete Protocol: 114 unit tests in 0.55 seconds and seven integrations passed.
- Native CLI binary: 182 units in 2.36 seconds and `headless_task_completion`
  five integrations in 0.14 seconds passed.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied. Rust and
  Elixir support-file formatting and whitespace checks passed.

Remote Ubuntu x86_64 source checks used a disposable unprivileged container
with Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory and 512 PIDs, four
Rust test threads and two Erlang schedulers. No installed service was restarted.

- Complete `headless_live`: 28/28 passed in 41.99 seconds.
- Complete Headless SDK: 327 unit tests in 31.34 seconds and 25 integrations passed.
- Complete Protocol: 114 unit tests in 0.39 seconds and seven integrations passed.
- SHA-256 matched 16 selected final source/support/schema/manifest/lock files.
  Cargo and Mix lockfiles remained unchanged.

The test container was removed; its 453 MiB temporary directory was deleted
and directory absence verified. Installed deployments, their data and the
existing public dependency cache were not removed.

All complete targets above had zero ignored or filtered tests. The full live
suite includes its existing fixture-based and real-Agent cases; the two new
chains have the concrete storage and computation scope described above.
Durations describe correctness runs, not throughput benchmarks.

Runtime API contracts, module topology/matrix, tensor self-test and actual tensor
validation, documentation book/inventory and project organization audit passed.
Source and documentation limits remain 800 and 2000 lines, with zero tracked
debt. Evidence is registered as `verified`; the tensor still has four maturity
gaps, 19 evidence-grade gaps and 14 P0 gaps, and release status remains blocked.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo test --quiet --locked --offline -p kyuubiki-cli --test headless_live -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --test headless_task_completion
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Scope

The gates qualify native `job_wait`/`result_fetch` source behavior, not the
entire job schema, standalone SDK language parity, installed Apps, service
authentication or scientific accuracy beyond the simple axial-bar check.
Raw metadata/result APIs remain diagnostic read routes; callers cannot infer
completion from HTTP 200, object presence or `has_result` alone. Empty objects
and opaque references pass this availability gate without proving model
semantics or artifact integrity. Separate status/result reads are not an atomic
snapshot and do not solve concurrent deletion/replacement or durable recovery.
Existing transport/HTTP-error classification is unchanged. No release metadata,
commit, App bundle, engine implementation or solver algorithm was changed.
