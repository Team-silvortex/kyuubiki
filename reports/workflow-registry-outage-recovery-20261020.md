# Workflow Runner Registry Outage Recovery

Date: 2026-10-20

## Reproduced Failure

`WorkflowJobRunner.running/1` called `Registry.lookup/2` without distinguishing
an unavailable registry from a missing runner. Terminating the real registry
made that lookup raise `ArgumentError: unknown registry`. Recovery scans, runner
loss handling and watchdog retirement depended on this lookup.

The baseline regression reproduced the exception with one test and one failure
(seed 183101). It did not intentionally drive the whole application into a
supervision restart cascade. The post-fix integration tests separately terminate
the registry during a real runner's controlled solver request: the linked runner
and solver terminate, while the coordinator, application supervisor and task
supervisor retain their original process identities.

## Recovery Contract

- Registry lookup has a narrow exception boundary and returns
  `workflow_runner_registry_unavailable`, not the absent-runner `:error`.
  An unavailable lookup alone must not consume an execution claim.
- A recovery scan checks registry availability before listing stored jobs. During
  an outage it reports `unavailable` with an unknown active-job count, rather than
  claiming that there are no jobs or that recovery completed.
- One coordinator-wide retry, using the existing lease retry interval, waits for
  the registry to return. Repeated scans share that timer; replacement tokens
  invalidate late messages. Lease loss cancels the timer and prevents stale-owner
  recovery. The snapshot exposes `runner_registry_retry_pending`.
- After restoration, recovery re-reads durable jobs and execution evidence under
  the normal ownership fence. Idempotent work may replay; checkpoint-required
  work without sufficient evidence is blocked. No replay policy is relaxed.
- A pending publication-only retry retains that intent if its timer encounters
  a registry outage. Losing the staged checkpoint during a combined registry and
  TaskSupervisor outage must block publication, not grant a fresh graph execution.
- Accepted cancellation, deletion and watchdog failure remain authoritative.
  Registry absence must not crash the coordinator after durable retirement.
  Cancellation delivery reports registry unavailability before spawning its
  background sweep; existing durable cancellation intent is retained.

Lease renewal moved from the coordinator to its existing ownership helper without
changing renewal token/expiry matching. The coordinator is 785 lines; all changed
source and test files remain below the 800-line limit.

## Coverage

The new suite is
`apps/web/test/kyuubiki_web/orchestra/workflow_registry_outage_test.exs`.
Its twelve cases cover:

1. Missing registry versus absent runner.
2. Dispatch and cancellation delivery without consuming a claim.
3. Automatic idempotent replay after an in-flight runner loses the registry.
4. Safe blocking of checkpoint-required work after the same failure.
5. Automatic publication of an already staged completion.
6. Coordinator restart while the registry remains unavailable.
7. Retry deduplication, replacement and stale token rejection.
8. Publication-only intent through combined registry/TaskSupervisor failure.
9. Cancellation surviving registry restoration.
10. Deletion surviving registry restoration.
11. Lease takeover cancelling deferred recovery.
12. Watchdog retirement while the registry is unavailable.

Automatic restoration cases exercise actual timers. Deterministic ordering cases
deliver recorded timer tokens explicitly and verify cancellation/replacement.
Solver responses and completion artifacts are controlled fixtures, not numerical
validation of physical operators.

## Verification

| Environment / suite | Result |
| --- | --- |
| Baseline lookup regression, seed 183101 | 1 test, 1 expected pre-fix failure |
| macOS, five adjacent recovery suites, seed 183104 | 61 tests, 0 failures |
| macOS ARM64, full Web suite, seed 183105 | 3,456 tests, 0 failures, 77 skips; 106.5 seconds |
| Linux x86-64, full Web suite, seed 183106 | 3,456 tests, 0 failures, 77 skips; 124.7 seconds |
| macOS SQLite, registry and finalizer suites, seed 183107 | 21 rounds, 714 executions, 0 failures |
| macOS JSON snapshots, same suites, seed 183108 | 21 rounds, 714 executions, 0 failures |
| Linux SQLite, same suites, seed 183109 | 21 rounds, 714 executions, 0 failures |
| Linux JSON snapshots, same suites, seed 183111 | 21 rounds, 714 executions, 0 failures |
| macOS, seven CI-style contention/recovery suites, seed 183110 | 3 rounds, 162 executions, 0 failures; includes real SIGKILL recovery |
| Linux, same seven CI-style suites, seed 183112 | 3 rounds, 162 executions, 0 failures; includes real SIGKILL recovery |
| Web formatting and test-profile compilation | Passed, including `--warnings-as-errors` |
| CI YAML validation | Both recovery steps include the new suite; JSON data directory remains isolated |

Each focused repeated round contains 34 tests: twelve registry cases plus 22
existing finalizer cases. The four platform/backend combinations total 2,856
executions. Repetition counts are executions, not distinct scenarios.
`--repeat-until-failure 20` requests an initial round and twenty further rounds.
No timeout increase or weakened assertion was used. Full-suite timings are not
a controlled performance benchmark.

macOS used Elixir 1.19.5 / ERTS 16.3. Linux reused the existing offline Debian 13
Elixir 1.19.5 / OTP 28.5.0.2 container image, restricted to four CPUs and 4 GiB.
All 1,007 source/fixture/lock/CI manifest entries matched before execution.
Only a test README report link was added after that source verification; the
production code, test files and CI configuration did not change during the runs.
No new image, dependency download, full backup or production-service restart
was needed. The local JSON test directory was removed after its process exited;
remote JSON data is confined to the disposable container.

CI now includes registry recovery in both the source-built SQLite
contention/forced-process suite and the isolated JSON snapshot recovery step.
The local/remote runs here reuse cached default builds, not a newly source-built
SQLite NIF or the exact hosted-CI image. A hosted CI run still requires commit/push.

## Boundaries

This qualifies service failure isolation and recovery ordering, not physical
power-loss durability, arbitrary state-corruption recovery or solver accuracy.
PostgreSQL and Windows were not exercised. The prior finalizer boundary report
describes which publication-only intent survives coordinator state loss:
[completion recovery](workflow-completion-supervisor-recovery-20261018.md).

Availability checks are not atomic with later runner registration. An outage
between lookup and startup still uses the existing startup-failure and runner-loss
handling. Initial dispatch rejection does not itself schedule automatic graph
replay; admission/dispatch API semantics and large pending-job retry load are
unchanged. This round does not claim that every registry race is eliminated.

No commit, push, desktop rebuild or installed-service restart was performed.
