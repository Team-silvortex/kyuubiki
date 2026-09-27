# Atomic Workflow State Commit Reliability

Date: 2026-09-27. Source line: daji 3.4.0. Local development evidence only.

## Defect And Contract

Workflow progress, completion, failure, and cancellation previously saved their
runtime/recovery record separately from the job transition. A rejected progress
event or a failed second write could leave an advanced node receipt, a terminal
recovery record, or a discarded execution envelope alongside an unchanged job.
Some paths also ignored the job-write error or counted an unsuccessful recovery
block as durable. The public cancellation facade could continue with a separate
job cancellation after the coordinator rejected its write.

Before production changes, the initial nine regression cases produced eight
failures on SQLite and eight on memory. These are failing cases, not eight
independently classified defects.

`Store.apply_progress_with_result/4` now validates one progress event and commits
the new job together with a replacement of its existing result/runtime record.
Both observed snapshots must still match; this operation never rebases a stale
decision onto newer data. A missing or changed snapshot returns a typed error.

The coordinator uses that boundary for node progress, final publication,
failure, cancellation, recovery blocking, and execution-claim publication.
Activity-only heartbeats use conditional job updates without changing results.
The execution engine and physical operators remain independent of this storage
protocol; no numerical algorithm or operator interface changes here.

## Storage And Ownership

- SQLite uses an immediate transaction. The shared SQL backend joins an existing
  Orchestra lease transaction rather than opening a nested transaction and
  hiding its rollback behind an error tuple.
- A rejected result CAS or SQL write rolls back the job update. Conditional job
  updates match all persisted fields. A result-only change still executes a
  conditional job UPDATE, so PostgreSQL can retain the row lock until commit;
  a read-only existence check would not provide that protection.
- Memory uses the shared `AnalysisMemoryState` transaction introduced for solver
  completion. It verifies both live snapshots, persists one combined generation,
  and installs neither new value if persistence fails.
- The workflow fence now requires a positive integer generation, a positive
  integer attempt, and a nonempty owner session, all exactly matching the stored
  running claim. A changed attempt cannot reuse a valid generation/session.
- Failed-node collection shape is checked before final publication. Invalid or
  unencodable output returns an error without losing the recovery payload or
  retiring the coordinator's lease.
- Terminal jobs reject late failure/publication writes. Cancelling an already
  terminal workflow does not rewrite its receipt. Plain-job cancellation does
  not invent a workflow result.
- The public `Analysis.cancel_job/1` facade propagates a failed coordinator
  commit instead of applying a separate job-only transition. An Agent cancel
  request follows the durable cancellation; it is not a distributed transaction.
- Recovery counters record a blocked job only after the blocking state commits.
  Failed writes remain skipped work, eligible for a subsequent recovery pass.

The SQL and memory APIs retain the existing job/result facades. No schema
migration, new backend process, unbounded backup history, or compensating
result deletion is introduced in this change.

## Regression Evidence

The focused tests exercise:

- Regressive progress, stale event time, and an oversized failure message cannot
  publish a partial runtime record; corrected requests can complete or fail.
- All three terminal job states reject a late failure. Cancellation commits both
  records, preserves current progress, and rejects a late completion.
- Stale execution generations, wrong attempts, and an Orchestra lease revoked
  before entry cannot publish a new result.
- Invalid `failed_nodes` and unencodable output leave the coordinator alive and
  the execution envelope available for a corrected retry.
- A stale/missing result inside an existing lease transaction rolls back job
  progress; stale job snapshots cannot replace a result. Eight competing joint
  writers produce one matching job/result winner.
- SQLite abort triggers target each of the job and result UPDATEs for progress,
  completion, failure, and cancellation. All eight cases preserve both snapshots
  and permit retry after the injected fault is removed.
- The cancellation facade propagates a rejected commit. Recovery-block write
  failure does not inflate the blocked count; a later healthy pass can block it.
- A real publishing process is killed after job UPDATE and before result CAS
  inside a lease transaction. Both writes roll back, SQL locks are released, and
  another transaction using the still-valid logical lease can complete. This
  does not assert logical lease revocation on process death.
- Four memory filesystem failures block the next snapshot write for progress,
  completion, failure, and cancellation. Live state and primary file bytes stay
  unchanged; the store/coordinator remain alive and clean retries succeed.

Tests live under `apps/web/test/kyuubiki_web`: `orchestra/workflow_commit_test.exs`,
`orchestra/workflow_commit_sql_test.exs`, `orchestra/workflow_commit_memory_test.exs`,
`jobs/joint_snapshot_test.exs`, and `orchestra/workflow_recovery_envelope_test.exs`.
Shared fixtures/fault probes are under `apps/web/test/support`. No production
timing hooks were added. Coordinator tests use minimal workflow records, not
physical Rust solvers; the cancellation facade case is not an HTTP transport test.

Verification with warnings treated as errors:

- Full SQLite suite: 1073 tests, zero failures, 20 backend/platform/optional skips.
- Full memory suite: 1073 tests, zero failures, 48 backend/platform/optional skips.
- Focused suite with seeds 1, 42, and 1337 on each backend: 52 tests per run,
  zero failures; SQLite skips four memory-only cases, memory skips eleven
  SQLite-only cases. Skips are not counted as executed coverage.
- Compilation, formatting, diff whitespace, and project organization checks pass
  with the 800-line source / 2000-line documentation limits.
- Tensor self-tests, structure, and commands pass: 13 modules, 11 paradigms,
  zero structural gaps. The release maturity gate remains blocked: four maturity
  gaps, 16 evidence-grade gaps, and 11 P0 gaps.

The first full memory run emitted a coordinator exit while the existing snapshot
fixture manually removed the shared store. A same-seed trace did not reproduce
the log. The fixture now stops background recovery/watchdog observers before
switching its temporary storage directory and restarts them after restoration.
That isolation change is not evidence of production storage-outage tolerance.
The final same-seed full memory run passed without the coordinator-exit log.

From `apps/web`, select a disposable SQLite database using
`KYUUBIKI_STORAGE_BACKEND=sqlite` and `SQLITE_DATABASE_PATH`, or a disposable
memory directory using `KYUUBIKI_STORAGE_BACKEND=memory` and `KYUUBIKI_DATA_DIR`.
Always set `KYUUBIKI_DATA_DIR` to a disposable directory for either backend.
Run the focused suite with:

```text
mix test --warnings-as-errors --seed 42 \
  test/kyuubiki_web/orchestra/workflow_commit_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit_sql_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit_memory_test.exs \
  test/kyuubiki_web/jobs/joint_snapshot_test.exs \
  test/kyuubiki_web/orchestra/workflow_restart_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_operator_activity_test.exs \
  test/kyuubiki_web/orchestra/workflow_recovery_envelope_test.exs
```

Tests reset shared stores. Never point them at installed runtime or research data.

## Scope And Remaining Work

This is local workflow commit evidence, not PostgreSQL, distributed failover, or numerical qualification.

PostgreSQL follows the same SQL implementation but was not exercised against a
live service in this run. The process-loss test interrupts a publishing process,
not the database server or host. Neither SQL process loss nor memory write-fault
injection establishes hardware power-loss durability or exactly-once computation.

The joint boundary covers an existing workflow runtime/recovery record and its
job. It does not atomically initialize the whole job, manage external numerical
checkpoint files, combine general job/result CRUD, fix job-ID incarnation/ABA
reuse, or provide a consistent snapshot across separate reader calls. Other
writers, including the watchdog, can still terminalize jobs independently of
their workflow runtime records. Memory lease/caller process-loss interleavings
and supervised recovery while the backing store is unavailable need separate
qualification. A lease revoked before a call is not a distributed split-brain test.

The tensor claim is limited to local `contract` and `recovery` evidence for
`orchestra-control-plane`, covering workflow composition, persistence/provenance,
and validation. It does not raise a physics trust level or remote operational grade.
See also the [solver publication report](solver-atomic-publication-20260927.md).

Follow-up: [storage-outage containment](storage-outage-recovery-20260927.md)
adds local coordinator/watchdog availability tests and policy-checked resumption.
The earlier counts and remaining-work list above describe this report's checkpoint.
