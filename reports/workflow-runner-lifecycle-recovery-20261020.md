# Workflow Runner Lifecycle Recovery

Date: 2026-10-20

## Reproduced Startup Gap

The registry outage follow-up found that `Task.Supervisor.start_child/2` returning
a PID does not prove that the child has registered or entered its intended phase.
The coordinator tracked the process but discarded its publication-only retry
intent. If registration then failed, its DOWN event entered ordinary runner-loss
recovery instead of result-publication recovery.

The baseline test suspended TaskSupervisor after the coordinator's registry lookup,
removed the registry, removed the staged completion checkpoint, and then released
startup. With an idempotent envelope, recovery attempted graph execution rather
than blocking the missing completion evidence. The minimal fixture then failed
graph-schema validation; this demonstrated entry into the wrong execution path,
not a completed numerical calculation. The initial 14-test run had one failure
(seed 183201), with the expected registration exception captured in its log.

## Fix

`WorkflowRecoveryRunners` now owns runner monitors and their lifecycle purpose:
executing, finalizing or retired. The existing coordinator delegates tracking,
DOWN handling and terminal cleanup to this helper.

- Starting a finalizer records publication-only intent before its registration
  can finish. A later DOWN schedules a publication retry, never a generic replay.
- Successful checkpoint staging changes an already tracked execution into the
  finalizing phase. Rejected staging leaves its recovery purpose unchanged.
- A healthy registry with no entry does not authorize duplicate startup while the
  coordinator still tracks a live process. An unavailable registry still returns
  unavailability; tracked state does not override that boundary.
- Replacing a tracked PID replaces its monitor and removes the old reference.
  Late DOWN messages from the previous process cannot remove the new runner.
- A manual scan also respects a tracked finalizer whose DOWN is not yet handled.
  Missing evidence remains a publication error, without a new execution claim.
- Accepted completion, failure, cancellation, deletion and watchdog/deadline
  retirement clear retries and mark the tracked runner retired. Its subsequent
  exit must not create a new recovery timer. Rejected writes do not retire it.

The initial implementation exposed six existing finalizer-test failures because
completed publishers still scheduled a retry on exit. Terminal retirement is now
explicit; those assertions were preserved. No replay policy, durable envelope
schema, solver interface or lease ownership rule changed.

The coordinator is 740 lines, down from 785. The monitor helper is 113 lines and
the expanded registry suite is 444 lines, all below the 800-line source limit.

## Added Coverage

The registry outage suite grew from twelve to 23 cases. The eleven additions are:

1. Four real startup-gap cases: valid/missing completion evidence crossed with
   idempotent/checkpoint-required policies.
2. A tracked live process before registry registration, including repeated scans.
3. Monitor replacement, a late old DOWN and subsequent publication-only recovery.
4. Accepted and rejected staging transitions.
5. Accepted cancellation, deletion and watchdog retirement of a tracked finalizer.

The four startup-gap cases pause the actual test TaskSupervisor and remove the
actual test Registry. The remaining additions use controlled idle processes and
install monitor state through the coordinator's system interface to make the
interleavings deterministic. They are lifecycle/state tests, not end-to-end
numerical solver runs. Existing automatic timer and real runner-outage tests
remain in the same suite.

## Lease Test Jitter

The first full macOS run reported 3,467 tests, one failure and 77 skips. The
failure was the existing memory-lease test's one-second wait for takeover;
the new recovery cases passed. An isolated 21-round run of the nine lease tests
passed, so the failure was not treated as a deterministic new recovery defect.

Inspection of the installed OTP `global` implementation showed randomly doubling
lock-retry sleeps. A task-local `exsplus` seed `{16585, 16586, 16587}` produces
initial sleeps of 6, 4, 22 and 1,898 milliseconds and reproduced the same timeout
in the single lease test (seed 183205, one failure, eight excluded).

The test now fixes only the takeover task's random sequence to `{17, 18, 19}`.
Its initial retry delay is 214 milliseconds on the tested runtime. This keeps the
test about exclusion while the guard is held, rather than uncontrolled backoff
latency. The original 50-millisecond blocked assertion, one-second completion
timeout, fencing-token increment and release assertions remain unchanged.
Production locking, jitter and timeout behavior were not modified.

CI repeats this lease suite alongside the existing native contention and recovery
paths. This deterministic case is not a stochastic fairness or lock-throughput
benchmark.

## Verification

| Check | Result |
| --- | --- |
| Initial startup-gap baseline, seed 183201 | 14 tests, 1 pre-fix failure |
| Intermediate adjacent recovery suites, seed 183204 | 77 tests, 0 failures |
| Unmodified lease suite repeated alone, seed 183205 | 21 rounds, 189 executions, 0 failures |
| Adverse-jitter lease reproducer | 1 test, 1 expected timeout; 8 excluded |
| Final macOS ARM64 full Web suite, seed 183205 | 3,467 tests, 0 failures, 77 skips; 150.9 seconds |
| Final Linux x86-64 full Web suite, seed 183206 | 3,467 tests, 0 failures, 77 skips; 124.6 seconds |
| macOS SQLite, registry and finalizer suites, seed 183207 | 21 rounds, 945 executions, 0 failures |
| Linux SQLite, same suites, seed 183208 | 21 rounds, 945 executions, 0 failures |
| macOS JSON snapshots, same suites, seed 183209 | 21 rounds, 945 executions, 0 failures |
| Linux JSON snapshots, same suites, seed 183210 | 21 rounds, 945 executions, 0 failures |
| macOS, eight CI-style suites, seed 183211 | 3 rounds, 222 executions, 0 failures |
| Linux, same eight CI-style suites, seed 183212 | 3 rounds, 222 executions, 0 failures |
| Formatting, test-profile compilation, diff checks | Passed; compilation uses `--warnings-as-errors` |
| CI YAML and path checks | Passed; registry suites retained in both backend steps, lease suite added to native repeats |

Both final full runs include the lease test's fixed task-local random sequence.
Timings are not controlled performance measurements. The source manifest contained
1,008 matching entries before the final remote runs. macOS used Elixir 1.19.5 /
ERTS 16.3; Linux reused the offline Debian 13 Elixir 1.19.5 / OTP 28.5.0.2 image,
with four CPU and 4 GiB limits. No new dependency or image download was needed.

The four repeated platform/backend configurations total 3,780 executions of 45
tests, not 3,780 distinct scenarios. `--repeat-until-failure 20` means one initial
round plus twenty additional rounds. The 45 tests consist of the expanded
23-case registry suite and the existing 22-case finalizer suite.
The eight-path combined runs also include contention, disposable database cleanup,
real child-process SIGKILL recovery and the lease fencing suite. They reuse the
default native build rather than rebuilding the SQLite NIF from source.

The owned local JSON test directory was removed after its test process exited.
Remote JSON data exists only in automatically removed containers. No shared cache,
unrelated file or installed runtime data was deleted.

## Scope and Limits

Publication purpose is coordinator-local intent, not new durable evidence.
Coordinator loss still reconstructs authority from surviving persisted envelopes
and checkpoints. This work cannot reconstruct deleted checkpoint data after loss
of that local intent, and it does not certify arbitrary corruption or physical
power-loss safety.

Registration can still fail after task spawn. The fix preserves recovery semantics
when it does; it does not make startup atomic. Initial admission API semantics,
large pending-job retry load, PostgreSQL and Windows remain outside this round.
The local and remote runs reuse existing default builds; a fresh source-built
SQLite NIF and hosted GitHub CI still require their respective CI runs.

See [registry outage recovery](workflow-registry-outage-recovery-20261020.md) and
[completion recovery](workflow-completion-supervisor-recovery-20261018.md) for the
preceding fault boundaries.

No commit, push, desktop rebuild or installed-service restart was performed.
