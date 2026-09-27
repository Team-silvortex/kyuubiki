# Atomic Result Administration Reliability

Date: 2026-09-27. Source line: daji 3.4.1. Local development evidence only.

## Defects And Contract

Public result editing previously performed separate job/result reads, an upsert,
and a final reread. A missing result could be created, and the memory backend
could create a result without a parent job. Ordinary results accepted injected
`_workflow_recovery` metadata. The read-only check covered only an active job
with a pending/running recovery record: inconsistent, malformed, or legacy
workflow records could bypass it. Result replacement/deletion did not check the
Orchestra lease, and a terminal replacement dropped its top-level workflow ID.

The initial 20 regression cases produced 20 failures on SQLite and 17 failures
with two SQL-only skips on memory. These are variants across actions, record
shapes, states, and backends, not 20 independent defects.

`Analysis.update_result/2` and `Analysis.delete_result/1` now pass through
`WorkflowRecoveryCoordinator.edit_result/2` and its existing lease guard.
`Store.edit_result/2` checks the parent, result, and mutation policy in the same
storage commit as the change. It never inserts a missing result. The returned
payload comes from that mutation, not a separate post-commit read.

- Missing parent/result produces the corresponding not-found error; both API
  routes return 404. A deleted result cannot be resurrected with PATCH.
- An incoming internal recovery key, either a string or an atom, is rejected.
- An active job with workflow identity stays read-only, including legacy
  records. A terminal job does not make a running, malformed, or unknown private
  recovery claim editable. Both sources of state must permit the change.
- Terminal workflow edits preserve the exact private recovery record and
  workflow ID. The public recovery summary is derived from that retained record,
  not accepted from the replacement payload. Failed/blocked receipts remain
  manageable without losing their retained envelope during an edit.
- Ordinary result editing, including public solver `recovery` fields, remains
  supported. Result-only deletion leaves the parent job unchanged.

The pure mutation policy is shared by both storage implementations. SQLite uses
an immediate transaction; the SQL backend locks the observed job with a
conditional no-op UPDATE, then replaces/deletes the exact result snapshot.
Memory applies the policy and mutation inside its shared analysis-state Agent
transaction and persists one complete generation. Storage errors leave the
previous job/result state intact and propagate to the caller.

This adds no schema migration, runtime process, backend Node dependency, or new
solver/engine coupling. Trusted low-level result-store put/delete operations
retain their existing semantics; public administration uses the guarded path.

## Regression Evidence

The 46 focused cases include the following checks:

- String/atom private-key injection; missing job/result; active, legacy, malformed,
  inconsistent and terminal recovery states; exact identity retention;
  unencodable edits; invalid storage mutation requests; revoked authority.
- Eight concurrent result deletions have exactly one winner. Replacement races
  deletion over twelve iterations without recreating the removed record.
- A SQL result read is paused while a separate publisher attempts a workflow
  claim. Editing/deletion and that claim cannot both consume the old mutable
  snapshot. The old code deterministically allowed both operations to succeed.
- Four SQLite trigger faults reject job/result writes for both actions. The
  coordinator survives, records remain exact, and retry works.
- The mutation process is killed after result UPDATE/DELETE but before the lease
  transaction commits. Rollback preserves both records and releases SQL locks;
  retry succeeds with the still-valid logical lease.
- Memory filesystem faults preserve live state and durable primary bytes.
  Successful edits/deletions survive a fresh storage process with the job intact.
- In-process Plug requests check honest 404/422/200 responses, ordinary editing,
  GET/PATCH round-tripping, forged recovery summaries, and active-runtime refusal.

Verification uses warnings as errors and fresh disposable storage per process:

| Run | Tests | Failures | Skipped |
| --- | ---: | ---: | ---: |
| Full SQLite suite, seed 42 | 1191 | 0 | 30 |
| Full memory suite, seed 42 | 1191 | 0 | 63 |
| Focused SQLite, each of seeds 1, 42, 1337 | 46 | 0 | 4 |
| Focused memory, each of seeds 1, 42, 1337 | 46 | 0 | 8 |

Focused SQLite skips four memory-only cases; memory skips eight SQL-only cases.
Full-suite skips include pre-existing backend/platform/optional checks. Skips
are not executed coverage. These counts are not line or branch coverage.

Formatting, diff whitespace, documentation book/inventory, project organization
(800-line source / 2000-line document limits), and tensor self-tests/checks pass.
The tensor retains 13 modules, 11 paradigms, zero structural gaps, four maturity
gaps, 16 evidence-grade gaps, and 11 P0 gaps. The release gate remains blocked;
local administration evidence does not remove those operational gaps.

From `apps/web`, set `KYUUBIKI_STORAGE_BACKEND`, a disposable
`KYUUBIKI_DATA_DIR`, and a fresh `SQLITE_DATABASE_PATH` for SQLite, then run:

```text
mix test --warnings-as-errors --seed 42 \
  test/kyuubiki_web/results/administration_test.exs \
  test/kyuubiki_web/results/administration_sql_test.exs \
  test/kyuubiki_web/results/administration_memory_test.exs \
  test/kyuubiki_web/api/result_administration_api_test.exs
```

Tests reset shared stores. Never use installed runtime or research data.

## Scope And Remaining Work

This is local result-administration evidence, not PostgreSQL, distributed failover, power-loss, or numerical qualification.

No live PostgreSQL service, remote Agent, GUI, or physical solver was exercised
by these new focused cases. API tests use the real router in-process rather
than a listening HTTP server. Process-loss tests kill a transaction owner, not
the host; the authority tests revoke a lease before entry, not during a network
partition. Terminal legacy records have no private recovery envelope to retain.
Large-result administration throughput was not benchmarked in this round.

Result-only deletion intentionally removes the result/recovery receipt, not
external artifacts or checkpoints. Existing whole-generation fallback may
restore an older complete snapshot if the newer primary is later corrupted;
this is not irreversible erasure. Explicit job-ID reuse, persistent incarnation
fencing, hardware durability, and remote execution cancellation remain separate
contracts. No new backup history or operator trust-level promotion is added.

The tensor claim is limited to local `contract` and `recovery` evidence for
`orchestra-control-plane` across workflow composition, persistence/provenance,
and validation. It does not upgrade remote operational evidence.
See the [admission](workflow-admission-reliability-20260927.md) and
[deletion](workflow-deletion-reliability-20260927.md) reports for adjacent paths.
