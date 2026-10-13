# Durable workflow final-publication checkpoint

Date: 2026-10-13. Source line: daji 3.6.0. Base: `c2f97f47` plus the existing
uncommitted recovery overlay and this follow-up. No package rebuild, deployment,
version increment, commit or push is included.

## Gap and Repair

The earlier completion retry retained the shaped result in a live runner. It
handled transient write failures and a coordinator-only restart, but **RAM is
not a power-loss-safe record**. Losing the whole BEAM also lost that finalizer.
Existing bounded node checkpoints did not constitute a final-result handoff.

The new sequence is computation, shaped-result staging commit, then atomic final
job/result publication. Staging is private inside the existing recovery record:

- One `pending_completion` record binds the job id and creation time, execution
  envelope digest, exact producer generation/attempt/session and lossless result
  JSON. SHA-256 detects changed data; it is not an authentication signature.
- Staging uses the same current-lease, active-job and snapshot-CAS transaction as
  other durable writes. Selected persistence failures retain the live waiter and
  retry only staging, not the solver; no success is published before persistence.
- Startup/runner-loss recovery reads the record before considering graph replay.
  A valid record starts a publication-only runner with the original execution
  claim. It does not increment execution attempts or call the graph/solver, even
  for a graph whose original policy forbids replay without a checkpoint.
- Publication revalidates the stored handoff, result identity, claim and lease.
  Corrupt or claim-mismatched records block recovery rather than falling back to
  execution. Cancellation, deletion, competing terminal decisions and lease loss
  retain their existing priority. Initial dispatch keeps its original error contract.
- Ordinary completed/failed/cancelled transitions remove the private record from
  the current runtime. Recovery-blocked records remain for inspection. There is
  no additional snapshot directory or backup daemon; JSON's existing previous
  generation can retain the prior record until its next normal rotation.
- Encoded result is capped at 64 MiB and 2,000,000 decoded JSON nodes, with the
  existing depth-64 and 1,000,000-byte string limits. Over-budget/invalid staging
  rejects explicitly, never silently claims durable success. Large externalized
  result handoff is not introduced by this change.

The final-publication transaction moved from the coordinator to a cohesive
completion store; the coordinator remains below the 800-line source limit.
No numerical solver or Rust Agent implementation changed.

## Storage Boundary

The SQLite driver previously defaulted to `synchronous=NORMAL`. Runtime config
now explicitly uses `journal_mode=WAL` and `synchronous=FULL`; the regression
queries the actual repository connection and requires `wal` and `2` respectively.
This adds synchronization cost and an extra staging transaction per successful
workflow. No throughput improvement or scale qualification is claimed.

The JSON-backed memory store already writes a synchronized `.next` file and
rotates verified whole-state generations. It does **not** synchronize directory
metadata after rename. It therefore has process-crash recovery evidence here,
not a complete physical power-loss guarantee. PostgreSQL still depends on its
actual server durability configuration and was not run in this follow-up.

## Forced-Termination Test

`workflow_completion_crash_test.exs` launches a genuinely separate BEAM with
isolated temporary storage and a controlled solve adapter. It does not merely
stop/restart an OTP child in the parent test VM.

The parent checks that the child-reported OS pid matches its owned port, sends
`SIGKILL` to that child only, requires a nonzero exit, and starts a fresh BEAM
against the same data. Each case then starts another fresh BEAM to verify stable
recovery. Test-created child processes and data are cleaned up on success/failure.

| Backend | Kill Boundary | Fresh-Process Outcome | Total Solver Calls |
| --- | --- | --- | --- |
| SQLite | Before staging begins | Original checkpoint-required graph is recovery-blocked | 1 |
| SQLite | After staging commits, before publication | Original result published without execution replay | 1 |
| JSON/memory | Before staging begins | Original checkpoint-required graph is recovery-blocked | 1 |
| JSON/memory | After staging commits, before publication | Original result published without execution replay | 1 |

The adapter records each invocation through a synchronized append and returns
both `1.0` and `1.234567890123456e-12`. Fresh-process checks require exact shaped
result equality, unchanged execution generation/attempt and terminal checkpoint
pruning. This tests orchestration durability using controlled computation, not
physical solver accuracy, a live Rust Agent or exactly-once external side effects.

The portable source harness is POSIX-only and explicitly skips Windows. No
machine reboot, hardware power interruption or unrelated service was performed.

## Verification

- Initial focused store/runner/commit/restart selection: **43 tests, 0 failures**,
  seed `68210`, warnings denied.
- Actual child-process SIGKILL plus staging/publication retry selection:
  **15 tests, 0 failures**, seed `68212`, warnings denied.
- Expanded SQLite selection initially exposed nine error-contract regressions
  in two categories: initial re-dispatch and initial storage-outage responses.
  The recovery-only branch was narrowed instead of weakening those assertions.
- Contract repair, storage outage, handoff and real process-crash selection:
  **70 tests, 0 failures**, seed `68214`, warnings denied.
- JSON/memory workflow, storage, jobs and result selection: **880 total,
  782 passed, 98 conditional skips, 0 failures**, seed `68215`, warnings denied.
  This includes the independent child-process cases for both storage backends.
- Final expanded SQLite workflow/API/dispatch/playground/library/storage/jobs/results
  selection: **3,322 total, 3,245 passed, 77 conditional skips, 0 failures**,
  seed `68216`, warnings denied, 123.1 seconds. The initial-dispatch contract
  regressions are fixed without changing their assertions.
- Formatting, diff whitespace checks, the 26-page HTML book check, documentation
  inventory, tensor self-test/full check and project organization audit pass.
  Tensor maturity is unchanged: four maturity, 20 evidence-grade and 15 P0 gaps;
  release qualification remains blocked. The coordinator is 782 lines.
- The owned memory test directory was removed. Child crash-test directories clean
  themselves up; no large fixture or generated test snapshot was added to Git.

## Limits

Process SIGKILL is not physical power loss: the operating system and disk caches
remain alive. FULL config alone does not qualify hardware cache flush behavior,
filesystem/device faults or every power-cut timing window. Actual power-cut/VM
fault injection, PostgreSQL, installed runtime, large-result performance and
broader distributed recovery remain open.

Before the staging transaction succeeds, the new final output is still volatile.
After it succeeds, the tested restart path publishes the saved result only; it
does not authorize replay of unknown Agent dispatches or repair failed earlier
checkpoints. Watchdog expiry can still retire a long-waiting finalizer. Pending
ordinary failure reasons remain a separate RAM-only boundary until their terminal
write succeeds. No whole-system power-loss-safe or exactly-once claim is made.
