# Atomic Workflow Deletion Reliability

Date: 2026-09-27. Source line: daji 3.4.1. Local development evidence only.

## Defects And Contract

Job deletion previously called cancellation, result deletion, and job deletion
separately. It ignored cancellation and result-deletion error tuples. A revoked
Orchestra lease could therefore reject cancellation but still allow the public
facade to delete both records. SQL delete exceptions escaped to the caller, and
a failure after result deletion could leave a job without its recovery record.

On memory storage, cancellation, result deletion, and job deletion also created
multiple snapshot generations. The retained previous generation could contain
the cancelled job with no result. Finally, successful deletion did not stop a
tracked local workflow runner blocked inside an operator call.

Before production changes, the first 13 regression cases yielded six failures
and two skips on SQLite, and three failures and four skips on memory. Four SQL
failures were fault-injection variants across job/result deletion and active/
terminal jobs; these counts do not represent six independent defects.

`Store.delete_with_result/1` now removes a job and its optional result in one
storage commit. `WorkflowRecoveryCoordinator.delete/1` owns that operation inside
the existing lease boundary. The public `Analysis.delete_job/1` facade propagates
errors instead of executing independent cleanup after a rejected write.

The API retains its job snapshot plus `deleted: true` response. The snapshot now
represents the job actually removed; deleting an active job does not fabricate
a `cancelled` transition first. Explicit cancellation remains a separate API.
Missing jobs return not found and do not authorize deleting unrelated/orphan
results. Invalid storage API identifiers return a typed error.

## Storage And Execution

- SQLite uses one immediate transaction, joining an existing lease transaction.
  A conditional job UPDATE locks the exact observed snapshot before DELETE.
  The existing foreign-key cascade removes its result inside that transaction;
  either a job-delete or result-delete trigger failure rolls everything back.
- The shared PostgreSQL implementation uses the same conditional snapshot
  boundary. A competing update can return a stale-snapshot error instead of
  silently deleting a newer job snapshot. No live PostgreSQL service was tested.
- Memory removes both map entries in one shared `analysis-state.json` generation.
  Persistence failure leaves live state and primary file bytes unchanged. A
  normal store restart observes neither half of a successfully removed task.
- The coordinator sends shutdown only to the tracked local runner after a
  successful storage commit. Its existing monitor removes tracking state.
  A rejected persistence write leaves the runner active; authority loss keeps
  the existing safety behavior of retiring locally tracked runners.
- Late workflow progress/completion cannot recreate the deleted job or result.
  The tested recovery scan does not redispatch the deleted task, and another
  live workflow remains running when its neighbor is deleted.

This adds no schema migration, new backend process, solver dependency, or extra
backup history. Physical operators and the execution engine retain their existing
interfaces. General low-level `Store.delete/1` behavior is unchanged; the public
analysis deletion path uses the new joint operation.

## Regression Evidence

The 23 tests exercise ownership loss, missing/invalid identifiers, active and
terminal jobs, unchanged unrelated work, post-delete late publication, tracked
runner shutdown, and the following storage/API cases:

- Eight simultaneous deletion requests produce one successful removal. Deletion
  races with result initialization and final solver publication across twelve
  iterations per operation without leaving an orphan result.
- Four SQLite trigger failures cover both delete targets and active/terminal
  jobs. Both snapshots remain exact, the coordinator survives, and retry works.
- A caller is killed after SQL DELETE but before its enclosing lease transaction
  commits. Both records are restored by rollback; SQL locks are released and
  a retry with the still-valid logical lease succeeds.
- Memory write failure preserves durable bytes and live records. A successful
  delete advances directly from the complete original pair to neither record,
  leaving that complete original pair as the previous generation.
- Through the real Plug router, rejected deletion returns 422, preserves the
  running task and its records, and permits retry. Successful retry returns 200;
  subsequent job/result reads and a repeat delete return 404. Lost authority is
  reported as an error, not a false 200 success.

The runner tests use the existing held in-process solver fixture. They exercise
the real workflow runner/coordinator and linked execution path, not a Rust
numerical solver or remote Agent. API tests use in-process Plug requests rather
than a listening HTTP server or GUI.

Verification with warnings treated as errors:

| Run | Tests | Failures | Skipped |
| --- | ---: | ---: | ---: |
| Full SQLite suite, seed 42 | 1145 | 0 | 26 |
| Full memory suite, seed 42 | 1145 | 0 | 55 |
| Focused SQLite, each of seeds 1, 42, 1337 | 23 | 0 | 3 |
| Focused memory, each of seeds 1, 42, 1337 | 23 | 0 | 5 |

Focused SQLite skips the two memory snapshot cases and the memory-only orphan
fixture; memory skips five SQL cases. Full-suite skips also include pre-existing
platform/backend/optional cases. Skips are not executed coverage.

Formatting, diff whitespace, documentation book/inventory, and project
organization checks pass under the 800-line source / 2000-line document limits.
Tensor self-tests, structure, and command checks pass: 13 modules, 11 paradigms,
zero structural gaps. The release maturity gate remains blocked with four
maturity gaps, 16 evidence-grade gaps, and 11 P0 gaps; this local claim does not
promote the remote operational evidence grade.

All runs use disposable data directories and a fresh SQLite database per test
process. From `apps/web`, set `KYUUBIKI_STORAGE_BACKEND`, a disposable
`KYUUBIKI_DATA_DIR`, and for SQLite a fresh `SQLITE_DATABASE_PATH`, then run:

```text
mix test --warnings-as-errors --seed 42 \
  test/kyuubiki_web/jobs/atomic_deletion_test.exs \
  test/kyuubiki_web/orchestra/workflow_deletion_test.exs \
  test/kyuubiki_web/orchestra/workflow_deletion_sql_test.exs \
  test/kyuubiki_web/orchestra/workflow_deletion_memory_test.exs \
  test/kyuubiki_web/api/workflow_deletion_api_test.exs
```

Tests reset shared stores. Never point them at installed runtime or research data.

## Scope And Remaining Work

This is local workflow-deletion evidence, not PostgreSQL, distributed failover, power-loss, remote Agent cancellation, or numerical qualification.

Stopping a local runner is not an acknowledged remote-computation cancellation.
Deletion does not transactionally erase external solver checkpoints, artifacts,
Agent caches, or library data. A process loss after commit but before local
shutdown is also outside the atomic storage boundary. Normal late publications
are rejected, but job-ID reuse still requires separate persistent incarnation
fencing. Result-only administrative edits/deletion remain separate operations.

The memory previous-generation recovery policy is unchanged: if the new primary
is subsequently corrupted, fallback can restore the earlier whole generation,
including a previously deleted job. This change does not add deletion tombstones,
irreversible erasure, or hardware power-loss durability. The authority test revokes
the lease before entry; it is not a distributed split-brain test.

The tensor claim is limited to local `contract` and `recovery` evidence for
`orchestra-control-plane` across workflow composition, persistence/provenance,
and validation. See the [admission report](workflow-admission-reliability-20260927.md)
and [workflow commit report](workflow-atomic-commit-20260927.md) for adjacent
boundaries; remote operational and physical qualification grades are not raised.
