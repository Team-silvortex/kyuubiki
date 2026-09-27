# Atomic Workflow Admission Reliability

Date: 2026-09-27. Source line: daji 3.4.1. Local development evidence only.

## Defects And Contract

Asynchronous workflow submission previously created a queued job before saving
its initial runtime/recovery envelope. If the second write failed, a separate
job deletion attempted to compensate. A failed deletion could raise and leave
an orphan queued job with no executable recovery record. The old initialization
path could also replace existing runtime/results, including active execution
claims and completed receipts, without checking the job's state.

Before production changes, the initial nine initialization regression cases
failed on both SQLite and memory. A tenth SQLite case rejected both result
insertion and compensating job deletion; it raised from the deletion path.
These are failing regression cases, not ten independently classified defects.

`WorkflowRecoveryCoordinator.admit/5` now constructs the recovery envelope and
commits the queued job and initial runtime inside the existing ownership
boundary. `Store.create_with_result/2` is insert-only: an existing job or result
rejects admission rather than being replaced. Invalid job attributes or recovery
input cannot create a partial task.

The retained `initialize_runtime/5` entry point uses `Store.initialize_result/2`.
It accepts only an existing queued job with no result/runtime. Missing jobs,
nonqueued jobs, and repeat initialization return errors without modifying
either record. Callers needing a retry must use the existing dispatch/recovery
path, not overwrite the original recovery envelope.

No numerical algorithm, worker protocol, operator interface, storage schema,
new background process, or backup history is introduced. The small admission
helper keeps record construction out of the coordinator, which remains below
the 800-line source limit.

## Storage And Recovery

- SQLite commits both records in an immediate transaction, joining the existing
  Orchestra lease transaction. A rejected second write rolls back the first;
  admission no longer relies on compensating job deletion.
- Initialization conditionally locks the queued job even when no job fields
  change, then inserts the result. It cannot silently replace an existing
  result. The shared SQL implementation is used for PostgreSQL, but this run
  did not exercise a live PostgreSQL service.
- Memory validates both collections and persists one shared
  `analysis-state.json` generation before exposing the new state. A failed
  snapshot write leaves both live records and the primary file unchanged.
- A revoked Orchestra lease rejects admission before either write. Invalid
  JSON recovery input leaves the coordinator alive and allows a corrected retry.
- Persistence and dispatch remain separate steps. An admitted pending workflow
  retains its executable envelope; a coordinator restart recovers and completes
  the tested pure-transform workflow without creating a second job.

## Regression Evidence

The 24 new tests cover missing/nonqueued/duplicate initialization, active and
terminal receipt preservation, eight concurrent admissions with one exact
job/result winner, eight concurrent initializers, invalid serialization,
lease loss, filesystem failure, restart recovery, and API failure/retry.

The SQLite process-loss test kills the caller after job INSERT but before result
INSERT inside a lease transaction. Neither record survives; a subsequent
transaction using the still-valid logical lease succeeds. This checks database
transaction rollback and lock release, not logical lease revocation on death.

The memory restart test stops the coordinator and shared store, restarts the
store, and reads back the exact admitted job and initial runtime. The orphan
result case is memory-only: SQL foreign keys already prohibit that fixture.
SQL-specific fault tests are skipped on memory and vice versa.

The API regression uses the real Plug router at
`POST /api/v1/workflows/graph/jobs`: an injected persistence failure returns 422
with an error and no partial task; removing the fault permits 202, followed by a
completed job and the expected output artifact. This is in-process HTTP routing,
not a live socket, GUI, Agent RPC, or physical solver test.

Verification, with warnings treated as errors:

| Run | Tests | Failures | Skipped |
| --- | ---: | ---: | ---: |
| Full SQLite suite, seed 42 | 1122 | 0 | 23 |
| Full memory suite, seed 42 | 1122 | 0 | 50 |
| Focused SQLite, each of seeds 1, 42, 1337 | 24 | 0 | 3 |
| Focused memory, each of seeds 1, 42, 1337 | 24 | 0 | 2 |

Full-suite skips include existing backend/platform/optional cases. Skips are
not executed coverage. All runs use disposable data directories, with a fresh
SQLite database for each process; installed runtime and research data are not
test targets.

Formatting, diff whitespace, documentation book/inventory, and project
organization checks pass with the 800-line source / 2000-line document limits.
Tensor self-tests, structure, and command checks pass: 13 modules, 11 paradigms,
zero structural gaps. Its release maturity gate remains blocked, with four
maturity gaps, 16 evidence-grade gaps, and 11 P0 gaps.

From `apps/web`, set `KYUUBIKI_STORAGE_BACKEND` and a disposable
`KYUUBIKI_DATA_DIR`; for SQLite also set `SQLITE_DATABASE_PATH` to a fresh file.
Run the focused suite with:

```text
mix test --warnings-as-errors --seed 42 \
  test/kyuubiki_web/jobs/atomic_admission_test.exs \
  test/kyuubiki_web/orchestra/workflow_admission_test.exs \
  test/kyuubiki_web/orchestra/workflow_admission_sql_test.exs \
  test/kyuubiki_web/orchestra/workflow_admission_memory_test.exs \
  test/kyuubiki_web/api/workflow_admission_api_test.exs
```

Tests reset shared stores. Never point them at installed runtime or research data.

## Scope And Remaining Work

This is local workflow-admission evidence, not PostgreSQL, distributed failover, power-loss, or numerical qualification.

The evidence does not prove an exactly-once submission/dispatch protocol,
response-loss idempotency, host power-loss durability, persistent job-incarnation
fencing, or a transaction spanning external solver checkpoint files. A failure
after successful admission may still leave a durable job for the existing
recovery/dispatch policy to handle. Ambiguous client retries are not deduplicated
by this change. General job/result CRUD remains outside the admission contract.

The tensor claim is scoped to local `contract` and `recovery` evidence for
`orchestra-control-plane` across workflow composition, persistence/provenance,
and validation. It does not promote remote operational or physical solver grades.
See the earlier [workflow commit report](workflow-atomic-commit-20260927.md) and
[storage-outage report](storage-outage-recovery-20260927.md) for adjacent boundaries.
