# Workflow Terminal Diagnostic Isolation

Date: 2026-10-28. Source line: daji 3.7.1. Local development evidence.

## Reproduced Failure

Terminal recovery reconciliation read the final history entry with `List.last/1`
without checking the stored field's shape. Null, object, string, number and
boolean histories raised `FunctionClauseError` inside the recovery scan. The
storage outage boundary deliberately does not swallow programming exceptions,
so a malformed diagnostic could terminate the coordinator and recur at startup.

A second path passed oversized messages directly to the job progress contract.
The 4096-byte gate rejected the update, leaving the active job unreconciled.
Reason text could fit by itself but exceed the limit after adding its prefix.
Empty or whitespace-only messages also obscured the terminal failure reason.

The initial direct regression ran 35 cases before the production fix: 19 failed.
These are failing inputs across related boundaries, not 19 independent defects.
Four coordinator-level cases were excluded from that red run to avoid repeatedly
restarting the application's supervisor against intentionally malformed data.

## Changed Boundary

`WorkflowRecoveryScan` now reads the last entry only when history is a list.
For failed or recovery-blocked records it selects a nonblank, valid UTF-8 message,
then a usable reason from that same final entry, then the existing state-specific
fallback. It does not search older entries for a potentially stale explanation.

The final prefixed text must fit `Jobs.ProgressEvent.message_max_bytes/0`, sharing
the existing 4096-byte progress limit rather than introducing a second limit.
Exact-limit text is preserved; oversized text is not truncated or written back.
The raw runtime, recovery envelope, history and cancellation evidence remain
unchanged. Completed and cancelled reconciliation retain their existing behavior.
No schema migration, additional backup, broad exception rescue, solver replay or
new cancellation authority is introduced.

## Regression Coverage

`apps/web/test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs`
contains 46 cases:

- Missing, null, wrong-type, empty and malformed-last-entry histories in both
  failed and recovery-blocked records.
- Empty, blank and oversized messages/reasons, including UTF-8 byte limits and
  the reason prefix crossing the final message limit.
- Message precedence, same-entry reason fallback and exact-limit preservation.
- Completed/cancelled controls independent of diagnostic history.
- Manual scans and coordinator restarts with a malformed or oversized record
  ordered before a healthy sibling. Both jobs reconcile, no runner is tracked,
  raw runtime records stay unchanged and the next scan sees no active jobs.

The initial focused SQLite pass, before the seven extra text-boundary cases,
passed 68 tests including storage outages, restart recovery, envelope validation
and the progress-event contract (seed 281902).

Final verification:

| Run | Result |
| --- | --- |
| Complete Web suite, disposable SQLite, seed 281903 | 4023 tests, 0 failures, 77 existing skips; 211.9 seconds |
| Focused selection, isolated memory/JSON, seed 281904 | 75 tests, 0 failures; 2.1 seconds |
| Elixir formatting and diff whitespace | Passed |
| Documentation book and inventory | Passed |
| Project organization and tensor checks, including self-tests | Passed |

The 46 added cases ran in both final selections. The memory/JSON directory was
removed after its test process exited; no test store or backup is retained.
The tensor claim is limited to local contract/recovery evidence. Overall tensor
status remains blocked: 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

To run the focused selection from `apps/web`:

```text
mix test --warnings-as-errors --seed 281902 \
  test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs \
  test/kyuubiki_web/orchestra/workflow_restart_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_recovery_envelope_test.exs \
  test/kyuubiki_web/jobs/progress_event_test.exs
```

Tests reset their stores. Use the test harness's disposable SQLite database or a
fresh disposable `KYUUBIKI_DATA_DIR` with `KYUUBIKI_STORAGE_BACKEND=memory`, never
installed runtime or research data. Remove owned temporary stores after exit.

## Diagnostic Scope

This is local terminal-diagnostic isolation evidence, not PostgreSQL, distributed
failover, power-loss, concurrent terminal-write atomicity or numerical qualification.
The malformed record is deliberately retained, not repaired or authenticated.
This does not change which terminal records the existing coordinator trusts, or
guarantee recovery completion for unrelated storage/write failures.

## Atomic Terminal Reconciliation Follow Up

The next pass checked the commit boundary after a terminal runtime was read.
Its starting checkout was `f28de0dd` (commit label `daji 3.7.7`); the Web package
still declares 3.7.1. This change does not alter either release metadata or tags.

The old reconciliation path updated only the job. A runtime replaced, annotated
or deleted after the initial read could still finalize an active job using the
old terminal decision. Live SQLite tests paused the coordinator after its result
query, committed a competing mutation, and observed a completed scan using that
stale evidence. A rejected SQL job write also surfaced as lease unavailability,
preventing an unrelated healthy sibling from progressing in that pass.
The initial selection had 22 cases: 19 failed and one memory-only case skipped.
This is a set of regression inputs and controls, not 19 independent defects.

Reconciliation now derives the terminal decision from the complete runtime
snapshot, reads the current job under the existing lease guard, and calls
`Store.apply_progress_with_result` with the exact job/runtime pair. Replacing the
runtime with its own payload retains all raw evidence while requiring it to
match at commit. Existing SQL transactions and shared JSON snapshots provide
the atomic boundary; no new store, lock service or schema is introduced. A
concurrent terminal job is left untouched, and a deleted job is not recreated.
SQL result storage timestamps may advance even though the payload is unchanged.

A rejected terminal reconciliation contributes to the existing skipped count
but marks that scan `interrupted` with
`workflow_terminal_reconciliation_incomplete`; `recovery_runs` does not advance.
Registry or ownership loss keeps its more specific existing reason. Ordinary
record conflicts and bounded commit failures do not block healthy siblings while
ownership remains valid. A later explicit/startup recovery reads current data
again rather than retaining the rejected decision or entering an automatic retry
loop. No new solver replay or cancellation authority is granted.

`workflow_terminal_commit_test.exs` adds 29 cases:

- Four terminal states reject changed terminal/runtime annotations, restored
  active runtime snapshots and deleted results without changing either record.
- Existing completed, failed and cancelled jobs retain their independent terminal
  decision; deleted jobs stay absent.
- Five live SQLite query-pause races cover result replacement, annotation and
  deletion, plus job cancellation and deletion; healthy siblings still progress.
- SQLite job/result UPDATE aborts roll back the pair and leave a fresh scan usable.
- A writer process killed after its job UPDATE but before the enclosing lease
  transaction commits leaves both records unchanged and permits a fresh write.
- A memory/JSON pending-file collision leaves disk bytes and in-memory job/result
  state unchanged, reports an interrupted scan, then permits an explicit retry.

The first expanded SQLite selection passed 133 tests with 10 backend-specific
skips (seed 282002). It includes the previous 46 diagnostic cases, storage-outage
coverage and shared workflow-commit regressions. The process-kill case terminates
one BEAM writer process, not the whole VM or a physical host.

Final follow-up verification:

| Run | Result |
| --- | --- |
| Complete Web suite, disposable SQLite, seed 282003 | 4052 tests, 0 failures, 78 skips; 213.9 seconds |
| Same 133-case focused selection, isolated memory/JSON, seed 282004 | 0 failures, 29 SQLite-only skips; 1.8 seconds |
| Formatting, whitespace, documentation, organization and tensor checks | Passed |

The new module has 20 shared cases, eight SQLite-specific cases and one JSON
persistence-fault case; all 29 ran across the two backend selections. The full
suite's additional skip is the new JSON-only case, separately exercised above.
The owned JSON test directory was removed after process exit. Existing tensor
gaps remain unchanged; this does not promote local evidence to remote acceptance.

Reproduce the focused follow-up from `apps/web`, using disposable storage:

```text
mix test --warnings-as-errors --seed 282002 \
  test/kyuubiki_web/orchestra/workflow_terminal_commit_test.exs \
  test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit_sql_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit_memory_test.exs
```

This is local paired-snapshot terminal-reconciliation evidence, not PostgreSQL,
distributed failover, physical power-loss or numerical qualification. Snapshot
equality is not authenticated provenance or protection against a complete ABA
replacement with identical data. Other recovery paths and missing-runtime scan
policy are unchanged; this is not a guarantee that every recovery write failure
is classified. No generated database or backup belongs in source control.

## Legacy Recovery Commit Follow Up

The next audit found that recognized legacy workflows still used a job-only
failure update. A runtime changed after the coordinator read could still fail
the active job using stale legacy evidence. An independently failed job could
also be counted as newly blocked. Rejected or missing-job writes could leave the
scan incorrectly completed, or turn a bounded write fault into lease loss.
The pre-fix regression ran 25 cases: nine failed and one JSON-only case skipped.
These are related failure inputs, not nine independent defects.

Legacy blocking now reuses the terminal reconciliation paired-commit helper
under the existing ownership guard. The exact observed runtime and current
active job must match when the pair commits. Raw legacy data remains unchanged;
no recovery envelope is invented. Only a successful active-job write counts as
blocked. Independently completed, failed or cancelled jobs are skipped unchanged;
deleted jobs and results are not recreated.

A rejected legacy commit contributes to skipped but interrupts the scan with
`workflow_legacy_reconciliation_incomplete`, without advancing `recovery_runs`.
Mixed scans retain their first reconciliation failure reason; registry or
ownership failures keep precedence. Healthy siblings can proceed while ownership
remains valid. A fresh explicit/startup scan reads current data rather than
retrying a retained stale decision.

The legacy discriminator is unchanged: a binary `workflow_id` without a map
`_workflow_recovery` envelope. Ordinary jobs with no runtime or no binary workflow
identity are still skipped, not guessed to be failed workflows. A completed scan
therefore does not assert that every skipped job has recovered. This change does
not add new runtime classification, migration, reconstruction or replay authority.

`workflow_legacy_recovery_test.exs` contains 31 cases:

- Manual scans and coordinator restarts with absent, null, list or string recovery
  metadata block the recognized legacy job without fabricating an envelope.
- Four ordinary-job controls retain missing or non-workflow runtime unchanged.
- Five paired-commit checks reject replacement with modern active/terminal data,
  annotations, plain result data or a deleted result on both backends.
- Five live SQLite query-pause races verify stale legacy evidence is rejected,
  healthy siblings progress and a fresh scan consumes current data where applicable.
- Four concurrent job outcomes preserve independent terminal decisions or deletion
  without inflating the blocked count or resurrecting a job.
- SQLite job/result UPDATE aborts roll back and permit a later explicit scan.
- Two JSON pending-file collisions retain disk bytes and in-memory state, preserve
  the first failure reason in both legacy/terminal orders, and allow explicit retry.
- Lease replacement after the legacy read prevents both active jobs being changed.

The local SQLite focused selection passed 120 tests with three JSON-only skips
(seed 282102). It includes the legacy tests, the previous terminal commit and
diagnostic suites, and storage-outage coverage.

Final legacy follow-up verification:

| Run | Result |
| --- | --- |
| Complete Web suite, disposable SQLite, seed 282103 | 4083 tests, 0 failures, 80 skips; 216.2 seconds |
| Same 120-case focused selection, isolated memory/JSON, seed 282104 | 0 failures, 20 SQLite-only skips; 3.4 seconds |
| Formatting, whitespace, documentation, organization and tensor checks | Passed |

The 31 new cases comprise 17 shared, 12 SQLite-specific and two JSON-specific
tests. All ran on their applicable backends; a skipped case is not a passing
execution. The two new skips in the full SQLite suite are the JSON collision
cases exercised separately above. The owned JSON files and temporary directory
were removed after the test process exited. Tensor totals remain unchanged:
2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; daji remains blocked.

Reproduce from `apps/web`, using disposable storage:

```text
mix test --warnings-as-errors --seed 282102 \
  test/kyuubiki_web/orchestra/workflow_legacy_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_terminal_commit_test.exs \
  test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs
```

This is local legacy reconciliation evidence, not PostgreSQL, physical power-loss,
distributed failover or an exactly-once guarantee. Runtime equality does not
authenticate provenance or detect identical-data ABA replacement. Product version
metadata, installed applications and the existing tensor maturity grades are not
changed by this follow-up. No generated database belongs in source control.

## Dispatch Refusal Snapshot Follow Up

The next pass checked nonterminal claim/replay admission. That path derived a
refusal from one runtime, then `block_recovery` fetched another runtime before
writing the blocked state. Its paired commit protected only that second read;
it did not bind the reason to the original checked snapshot. A concurrent repair,
annotation or completed runtime could therefore receive an obsolete refusal.
Deleted/plain replacements and rejected blocking writes could also leave a scan
reported completed even though it had not handled the active workflow.

The pre-fix 20-case regression had 13 failures and one JSON-only skip. This is a
set of inputs exposing related snapshot/accounting gaps, not 13 independent bugs.
SQLite telemetry paused the exact claim read, outside its later write transaction,
then a competing writer changed the stored evidence. One failing control changed
the runtime to completed, yet the old digest refusal replaced it with blocked.

Claim/replay refusal now carries its checked runtime through to the existing
lease-guarded paired commit without a second decision-source read. The current
active job and original runtime must match at commit. No new store, schema,
service or broad retry policy is added. Independent terminal jobs stay untouched;
deleted jobs/results are not recreated. The initial-dispatch entry uses the same
snapshot-bound path. Pending-completion invalidation/finalizer retry is a separate
decision path and is not qualified by this follow-up.

Unresolved dispatch errors contribute to skipped but interrupt the scan with
`workflow_recovery_dispatch_incomplete`; `recovery_runs` does not advance. Existing
independent terminal jobs remain skipped without making the scan incomplete.
Registry/lease loss keeps its more specific existing reason. Healthy siblings
can still progress while ownership remains valid. A later explicit/startup scan
reads current records; neither an old refusal nor an interrupted scan grants
automatic replay or causes a new retry loop.

`workflow_dispatch_reconciliation_test.exs` contains 28 cases:

- Three unchanged refusal controls cover exhausted attempts, digest mismatch
  and a graph requiring a verified checkpoint before replay.
- Fifteen live SQLite races replace each refusal source with repaired active,
  terminal, annotated, plain or deleted runtime. The pair is preserved and the
  healthy sibling completes; fresh scans consume terminal/annotated data.
- Two initial-dispatch races preserve a newer active or terminal snapshot.
- Four concurrent job outcomes retain completion, failure, cancellation or
  deletion without counting an independent decision as a new block.
- SQLite job/result UPDATE aborts interrupt the scan and permit explicit retry.
- One JSON pending-file collision retains disk bytes and in-memory state, does
  not count a completed scan, and permits a fresh scan after removing the fault.
- Lease replacement after the claim read prevents both active jobs being changed.

The 169-case SQLite focused selection passed with four JSON-only skips (seed
282202), including legacy/terminal reconciliation, shared commit and storage
outage coverage.

Final dispatch follow-up verification:

| Run | Result |
| --- | --- |
| Complete Web suite, disposable SQLite, seed 282203 | 4111 tests, 0 failures, 81 skips; 214.8 seconds |
| Same 169-case selection, isolated memory/JSON, seed 282204 | 0 failures, 65 SQLite-only skips; 2.1 seconds |
| Formatting, whitespace, documentation, organization and tensor checks | Passed |

The 28 new cases comprise three shared cases, 24 SQLite-specific cases and one
JSON-specific case; all ran on their applicable backend. The full SQLite suite's
additional skip is that new JSON persistence-fault case, executed separately.
The test process exited before its owned JSON files and temporary directory were
removed. The coordinator remains below the 800-line source limit. The tensor
still has two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; readiness
remains blocked, and local tests are not promoted to remote acceptance.

Reproduce from `apps/web` with disposable storage:

```text
mix test --warnings-as-errors --seed 282202 \
  test/kyuubiki_web/orchestra/workflow_dispatch_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit_sql_test.exs \
  test/kyuubiki_web/orchestra/workflow_legacy_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_terminal_commit_test.exs \
  test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs
```

This is local admission-refusal reconciliation evidence, not PostgreSQL,
distributed failover, physical power-loss or an exactly-once guarantee. Snapshot
equality is not authenticated provenance and does not detect identical-data ABA
replacement. No installed package, product version or tensor maturity grade is
changed. Raw test stores are disposable and are not retained as source artifacts.

## Pending Completion Refusal Follow Up

The earlier dispatch follow-up deliberately excluded pending-completion decisions.
This pass found a separate read-to-block race: checkpoint validation ran under a
lease, but the later blocking path re-read runtime and could attach its old
invalid/missing decision to repaired or terminal evidence. Job identity/progress
could also change between validation and the blocking commit. A finalizer timer
could lose its publication-only marker after the result disappeared, leaving a
later metadata restoration exposed to ordinary replay policy.

The first deterministic SQLite regression reproduced an invalid checkpoint
overwriting a repaired pending result. Expanding that selection to 28 cases
produced 20 failures before the implementation changed (seed 282301). Tests pause
the coordinator after the pending-read transaction commits, not while its SQLite
writer reservation is held. This enables a real competing writer without sleeps
or a production-only test hook. Existing immediate-read probes remain available.

`WorkflowCompletionStore.pending` now retains the exact active job and runtime
behind a blocking decision. `block_pending` commits against both snapshots under
the current lease. No extra storage bucket, backup or service is introduced.
The coordinator no longer fetches a new runtime to apply that older decision.
Snapshot conflicts and missing result rows retain the existing publication-only
retry; a missing-runtime timer checks that the job is still active before keeping
that retry. Deleted/terminal jobs do not receive new missing-runtime retries.
Manual dispatch of a known finalizer revalidates and publishes its checkpoint,
rather than unconditionally blocking it or replaying the graph.

The new `workflow_completion_reconciliation_test.exs` has 52 cases: 18 shared
storage controls and 34 SQLite interleaving cases. Coverage includes invalid and
missing checkpoints, timer/manual-scan entry, repair, terminal result, annotation,
plain replacement, result deletion, job progress, same-ID job recreation,
independent terminal/deleted jobs, lease takeover, first-scan retry retention and
known-finalizer manual dispatch. A removed runtime stays absent; restoring only
execution metadata is blocked without advancing generation/attempt. Restoring a
valid pending checkpoint permits publication without graph replay.

Final pending-completion verification:

| Run | Result |
| --- | --- |
| Completion/checkpoint/supervisor/retry/crash plus dispatch regressions, SQLite, seed 282302 | 130 tests, 0 failures, 1 JSON-only skip; 17.2 seconds |
| Same selection, isolated memory/JSON, seed 282304 | 130 tests, 0 failures, 59 SQLite-only skips; 17.3 seconds |
| Complete Web suite, disposable SQLite, seed 282303 | 4163 tests, 0 failures, 81 skips; 216.6 seconds |
| Formatting, whitespace, documentation, organization and tensor checks | Passed |

The tensor remains at two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps;
readiness remains blocked. Tests are not promoted to remote acceptance. The JSON
test process exited before its 8 KiB of owned disposable files and directory were
removed; no new test-store backup is retained.

Reproduce the focused selection from `apps/web` with disposable storage:

```text
mix test --warnings-as-errors --seed 282302 \
  test/kyuubiki_web/orchestra/workflow_completion_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_checkpoint_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_supervisor_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_retry_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_crash_test.exs \
  test/kyuubiki_web/orchestra/workflow_dispatch_reconciliation_test.exs
```

This is local SQLite/JSON evidence, not PostgreSQL, distributed failover,
physical power-loss or an exactly-once guarantee. Retry markers remain in process;
they are not durable tombstones across checkpoint deletion plus coordinator
restart. Snapshot equality does not authenticate provenance or detect an identical
ABA replacement. The production coordinator shrinks from 775 to 765 lines, below
the 800-line source limit. No installed package, release version or maturity grade
is changed, and the owned temporary JSON store is removed after its process exits.

## Durable Finalization Phase Follow Up

The preceding in-process retry fix did not persist publication-only intent outside
the completion payload itself. A staged workflow still said `running`. Removing
`pending_completion` and restarting the coordinator therefore made an idempotent
workflow look eligible for replay. For checkpoint-required work it selected an
ordinary replay-policy refusal rather than diagnosing lost completion evidence.

The new 12-case phase selection initially failed eight cases, with one JSON-only
skip, on SQLite (seed 283001). Two actual SIGKILL/restart cases for missing staged
payloads also failed before the implementation changed, one per storage backend.
These are application-process termination tests, not physical host power loss.

Changes stay within the existing completion/recovery owners:

- Staging atomically persists state `finalizing` and the completion checkpoint via
  the existing paired job/result commit. No extra database, table, backup or SQL
  migration is introduced, and generation/attempt/owner do not advance.
- Recovery cannot claim or replay that phase. Missing payload returns the same
  invalid-checkpoint refusal as corrupt payload, even without an in-memory retry.
- Publication has its own original-claim fence. Ordinary execution progress,
  checkpoint and failure paths still require `running`; a late execution writer
  cannot consume finalization authority.
- Duplicate staging is a no-op only for matching retained evidence. Restaging or
  committing a missing finalizing checkpoint cannot reconstruct it from cached
  output. Explicit cancellation remains separate and existing tests still apply.
- Valid old `running` checkpoints remain readable and publish without replay;
  restaging upgrades them atomically to `finalizing`. Old readers are not promised
  forward compatibility with that new phase. The recovery schema lists the phase
  and continues requiring its retained execution envelope.

The 12 new phase tests comprise nine shared-backend cases, two SQLite write-abort
cases and one JSON pending-file failure. The crash fixture adds four cases to the
existing four: missing/corrupt payload after staging, on both SQLite and JSON.
Each kills the actual child VM, opens the same on-disk store in a fresh VM, then
opens it a second time. Original generation/attempt and a single durable solver
invocation are checked, not just eventual job status. Before-stage and healthy
after-stage crash controls are retained.

Final durable-phase verification:

| Run | Result |
| --- | --- |
| Completion phase/checkpoint/retry/supervisor/reconciliation/crash, SQLite, seed 283002 | 118 tests, 0 failures, 1 JSON-only skip; 24.3 seconds |
| Same focused selection, isolated memory/JSON, seed 283004 | 118 tests, 0 failures, 37 SQLite-only skips; 24.9 seconds |
| Complete Web suite, disposable SQLite, seed 283003 | 4179 tests, 0 failures, 82 skips; 223.4 seconds |
| Formatting, whitespace, documentation, organization and tensor checks | Passed |

The new JSON persistence-fault case accounts for the additional full-suite skip
and ran in the isolated JSON selection. Its process exited before removing its
8 KiB temporary store; child crash stores are also removed by the fixture after
the child exits. Source files remain below 800 lines and documents below 2000.
The tensor still reports two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps;
readiness remains blocked. No release version, installation or maturity grade is
changed, and no commit or push is part of this pass.

Reproduce from `apps/web` using disposable storage:

```text
mix test --warnings-as-errors --seed 283002 \
  test/kyuubiki_web/orchestra/workflow_completion_phase_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_checkpoint_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_retry_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_supervisor_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_completion_crash_test.exs
```

Boundary: the durable marker is part of the runtime record. This closes removal
of the checkpoint payload while retaining that phase, not deletion or rollback
of the entire runtime, malicious rewrites of both fields, or legacy records whose
payload was already gone before staging under this version. No independent job
tombstone is claimed. SQLite/JSON process-crash evidence does not qualify
PostgreSQL, distributed failover, hardware power loss or exactly-once execution.
All synthetic child stores are disposable and removed after child processes exit.
