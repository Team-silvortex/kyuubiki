# Workflow Completion Supervisor Recovery

Date: 2026-10-18

## Reproduced Failure

A workflow had already staged a verified completion checkpoint, but its runner
supervisor was unavailable when recovery tried to start the result publisher.
The scan counted that job as skipped. Restoring TaskSupervisor did not schedule
another scan, so a valid completed computation could remain visibly `solving`.

The first regression failed before the production fix: one test, one failure,
waiting for completion after TaskSupervisor returned. No solver replay or manual
second recovery call was necessary to demonstrate the missing transition.

## Fix and Invariants

- Only `workflow_runner_supervisor_unavailable` schedules this retry. It uses the
  existing configured lease retry interval, with one timer per job and a fresh
  token for each replacement; repeated scans do not accumulate timers.
- A retry retains only the job identity, token and timer reference. It re-enters
  ordinary recovery, re-reading the persisted result, checkpoint digest, execution
  claim and lease fence rather than publishing a cached result.
- Successful runner registration, accepted terminal writes, cancellation and
  deletion clear the matching retry. Losing storage authority or the lease clears
  all retries. Late or superseded timer messages are ignored.
- Rejected conflicting commits and stale failure writes leave valid recovery
  intact. The expanded tests initially exposed two failures in this boundary;
  clearing retry/progress state is now conditional on an accepted terminal write.
- A coordinator restart reconstructs pending publication from durable evidence.
  The transient timer itself is not the source of truth.
- The recovery snapshot exposes `pending_completion_retries`. Existing terminal
  reconciliation moved into WorkflowRecoveryScan to keep the coordinator below
  the 800-line source limit; its status/message semantics are unchanged.

## Regression Coverage

The new suite contains twelve cases: automatic publication after supervisor
return; coordinator restart; repeated scan and stale timer handling; cancellation;
deletion; changed result digest; changed producer claim; lease takeover; already
committed results; conflicting commits; stale failure writes; and a storage outage
during retry followed by service recovery.

Tests stop and restart real supervised children. Automatic-return cases use the
configured retry timer; deterministic interleaving cases explicitly deliver its
token and inspect timer cancellation. The publication-only fixture uses an empty
graph with `checkpoint_required` replay policy and a staged synthetic result.
Success preserves generation/attempt, publishes the expected artifacts and removes
the pending checkpoint. These tests exercise recovery, not physical accuracy.

## Verification

| Environment / suite | Result |
| --- | --- |
| macOS ARM64, full Web suite, seed 182805 | 3,434 tests, 0 failures, 77 existing skips; 167.0 seconds |
| Linux x86-64, full Web suite, seed 182805 | 3,434 tests, 0 failures, 77 existing skips; 126.2 seconds |
| macOS SQLite, five adjacent recovery suites | 56 tests, 0 failures |
| macOS JSON snapshots, same adjacent suites | 56 tests, 0 failures, 1 SQLite-only WAL assertion skipped |
| macOS SQLite, new suite, seed 182806 | 51 rounds, 612 tests, 0 failures |
| macOS JSON snapshots, new suite, seed 182807 | 51 rounds, 612 tests, 0 failures |
| Linux SQLite, new suite, seed 182808 | 51 rounds, 612 tests, 0 failures |
| Linux JSON snapshots, new suite, seed 182809 | 51 rounds, 612 tests, 0 failures |
| Linux, six CI contention/recovery suites together, seed 182810 | 3 rounds, 96 tests, 0 failures; includes real SIGKILL recovery |
| Web formatting and test-profile compilation | Passed; `--warnings-as-errors` |
| CI YAML validation | Parsed; both finalizer backends included and JSON path isolated |

The four stress configurations total 2,448 executions of the twelve new cases,
not 2,448 distinct scenarios. `--repeat-until-failure 50` means one initial round
plus fifty additional rounds. No test timeout was raised or assertion weakened.
The full-suite timings were not collected as a controlled performance benchmark.

macOS used Elixir 1.19.5 / ERTS 16.3. Linux reused the existing offline Debian 13
Elixir 1.19.5 / OTP 28.5.0.2 image, with four CPUs and 4 GiB memory limits. SQLite
used Exqlite 0.42.0. No new image, dependency download or production deployment was
needed. All 1,006 source/fixture/lock/CI manifest entries matched before the Linux
runs. Subsequent CI/documentation-only additions were synchronized without changing
the tested production code.

CI now includes the new suite alongside the existing source-built SQLite
contention and forced-process recovery tests, plus a separate three-round JSON
snapshot run under the runner's temporary directory. Local JSON tests use fresh
temporary paths; Linux JSON snapshots exist only inside disposable containers.
The six-path combined Linux run used the existing default build, not a new
source-built NIF; source-built coverage remains configured in CI. The two owned
local JSON directories were removed after their test processes exited. No shared
build cache or unrelated data was deleted.

## Limits

This is service/coordinator recovery qualification, not physical power-loss or
filesystem durability certification. PostgreSQL was not exercised in this round.
Initial task-dispatch failure policy, arbitrary supervisor start errors and retry
load at very large pending-job counts are unchanged. The Linux image is not the
exact hosted-CI toolchain; the earlier source-built OTP 28.0.4 qualification remains
documented in [CI repair](ci-repair-20261018.md). A fresh hosted CI run is still
needed after the changes are committed and pushed.

No commit, push, desktop rebuild or installed-service restart was performed.

## Follow-Up: Publication-Only Boundary

Further fault injection found a gap in the first-round retry implementation.
Removing `pending_completion` while a finalizer timer existed made recovery enter
ordinary graph dispatch. Timers supplied `:completion_retry`, which is not an
execution-claim reason: both idempotent and checkpoint-required cases raised a
FunctionClauseError in the coordinator. A manual scan could instead reclaim an
idempotent graph, or report an unrelated replay-policy error for a
checkpoint-required graph. The four new baseline cases all failed, including two
coordinator exceptions.

The fallback to graph dispatch now rejects both an explicit completion retry and
an intervening scan with a remembered per-job finalizer. Neither path passes the
finalization reason to the execution-claim state machine. Missing evidence is
recorded as `workflow_completion_checkpoint_invalid`, with the existing
publication-specific inspection action, unchanged generation/attempt and no new
artifacts. An accepted block cancels the pending timer.

A transient failure to persist the block also retains a retry. This applies to
invalid as well as missing checkpoints, persistence failures and stale snapshot
conflicts. Lease/storage authority errors still use normal ownership-loss handling;
invalid serialization payloads are not blindly retried. No cached result is used
to reconstruct damaged evidence.

Ten cases were added, bringing the suite to 22: four policy/trigger combinations,
four failed-block-write combinations, an actual automatic retry after writes
recover, and a mixed scan containing one damaged and one healthy staged job. SQL
faults use an aborting result-update trigger; JSON faults obstruct the snapshot's
temporary commit path. The failure assertion checks both job and result snapshots
remain unchanged before the fault is removed.

| Follow-up check | Result |
| --- | --- |
| Baseline checkpoint-removal cases, seed 183001 | 4 tests, 4 failures; 12 earlier cases excluded |
| macOS full Web suite, seed 183003 | 3,444 tests, 0 failures, 77 existing skips; 104.8 seconds |
| macOS SQLite, updated suite, seed 183004 | 21 rounds, 462 tests, 0 failures |
| macOS JSON snapshots, updated suite, seed 183005 | 21 rounds, 462 tests, 0 failures |
| macOS, six CI-style contention/recovery suites, seed 183006 | 3 rounds, 126 tests, 0 failures; includes real SIGKILL recovery |
| Web formatting / test-profile compilation | Passed; `--warnings-as-errors` |

The two repeated configurations total 924 test executions, not distinct scenarios.
The existing CI steps automatically include all 22 cases through the same test
path. No timeout increase, new dependency or broadened replay permission was used.
The coordinator remains 788 lines, below the 800-line source limit.
Both follow-up JSON test directories were removed after their processes exited.

Linux was not rerun for this follow-up: both the lab's discovery endpoint and SSH
to its last verified address timed out. Earlier Linux results above predate this
boundary fix and are not evidence for the follow-up. Hosted CI also remains to be
run after commit/push.

The guard uses a current coordinator's remembered finalizer intent. It does not
reconstruct deleted completion evidence after loss of that intent, nor certify
arbitrary durable-state corruption or physical power loss. Ordinary restart
recovery still follows the surviving durable envelope and its replay policy.
No commit, push, desktop rebuild or installed-service restart was performed.
