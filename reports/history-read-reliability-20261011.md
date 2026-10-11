# Independent history read reliability

Project, model, version and checkpoint reads, plus security-event history and
JSON/CSV exports, now contain known storage failures without replacing history
with empty success or false not-found responses. Eleven HTTP routes retain their
successful shapes, authentication policy and genuine empty/missing responses.
SQL project/model details and checkpoint reconciliation read one generation per
call; single memory-store reads keep their existing atomic operation.

These are source service guarantees. They do not create one snapshot across
separate HTTP requests, persist the volatile project library, or qualify installed
GUI behavior and large-history performance.

## Reproduced failures

Actual library/security-store stops caused eleven independent routes to exit
instead of returning an unavailable response. Direct helpers and authenticated
reads failed in the same way. Genuine empty-history controls already passed.

Four second-connection SQLite cases also reproduced mixed records: project lists
and details retained a deleted project with an empty embedded model list; model
detail kept an old latest-version pointer alongside a new version list; and
checkpoint reconciliation combined an old receipt with its later atomic deletion.
The writers committed between real queries, not through mocked database responses.

## Read boundaries

Library reads use the existing failure boundary. The multi-query SQL calls
additionally use `ReadSnapshot`; memory reads need no additional whole-store
capture when called independently. Existing composite JSON exports still reuse
their captured state. No write, dispatch or task execution is retried.

SQL nested reads may reuse only the boundary's established snapshot in the same
process and dynamic repository. Unowned ambient transactions and a different
selected repository are refused. Success, errors and exceptions release the
snapshot context. PostgreSQL uses read-only repeatable read; SQLite keeps its
deferred WAL snapshot. SQLite callbacks must remain read-only application code.

Security-event list and export projections contain known store failures outside
their read callback. JSON errors remain opaque. A failed CSV request returns
HTTP 503 JSON rather than a successful CSV body containing an error. Checkpoint
lookup keeps `no-store`. Version-context derivation propagates storage loss
without confusing it with a missing version or publishing a task.

## Partial export rejection

Version-query errors no longer become empty version history in bundles, workload
catalogs or database exports. Security-query failure no longer becomes a partial
database map. Only database export depends on security events; a security outage
does not block bundles or catalogs.

The first remote PostgreSQL sweep listed 128 cases and found two failures in
the missing-table controls. A nested boundary contained the query error, but
PostgreSQL had already aborted the transaction; the outer commit then replaced
the original unavailable error with a generic rollback response, producing 422.
The read boundary now explicitly rolls back an error result with its original
reason. A dedicated nested-query control verifies the error survives rollback
and subsequent reads/writes work. No production timeout or retry limit was relaxed.

## Executed controls

The 29 added cases comprise 16 API/helper controls, eight SQL concurrent-history
controls, two missing-table export controls and three SQL nesting/rollback controls.
They cover both model and version receipts, real stop/restart, authorization,
CSV content type, security filters and summary counts, genuine empty/missing
records, unchanged execution records and task-count preservation.

SQL concurrent writes run on a second selected repository connection between
queries, through direct helpers and HTTP routes. Missing-table tests temporarily
rename a table only in the disposable database and restore it in `after`.
SQL and durable security-event restarts retain their original records. Restarting
the lightweight JSON library loses its volatile records under its existing policy;
the controls check that it becomes usable again, not fabricated record recovery.

## Verification

- Final JSON recovery/routing/API/library selection: 730 listed, 682 passed,
  48 backend/environment-specific cases skipped, zero failures in 10.8 seconds.
- Final SQLite recovery/routing/API/library selection: 1,027 listed, 955 passed,
  72 backend/environment-specific cases skipped, zero failures in 33.7 seconds.
- Remote PostgreSQL 16.14 final regression: 129 listed, 98 passed, 31
  backend-specific cases skipped, zero failures in 142.9 seconds. The database
  runs on the Ubuntu lab host through a private loopback tunnel; the source
  control plane runs on macOS. This is not Linux control-plane qualification.
- Different-order PostgreSQL follow-up with seed 2718: all 35 focused cases
  passed, zero failures or skips in 49.2 seconds.
- Strict test compilation and formatting checks passed.
- Runtime API, topology, module/function matrix, extension-standard, documentation
  inventory/book, project organization and version-line checks passed. The tensor
  and its self-test passed structurally: zero structural gaps, four maturity gaps,
  20 evidence-grade gaps and 15 P0 gaps. Overall Daji qualification stays blocked.

Run the focused cases from `apps/web` with an isolated test backend:

```text
mix test test/kyuubiki_web/api/history_read_outage_test.exs test/kyuubiki_web/library/history_read_snapshot_test.exs test/kyuubiki_web/storage/history_partial_export_test.exs test/kyuubiki_web/storage/read_snapshot_test.exs --seed 0 --warnings-as-errors
```

For JSON, use `KYUUBIKI_STORAGE_BACKEND=json` and a fresh temporary
`KYUUBIKI_DATA_DIR`; SQL-only cases skip. For SQLite, use
`KYUUBIKI_STORAGE_BACKEND=sqlite` with the runner's disposable database. PostgreSQL
requires `KYUUBIKI_STORAGE_BACKEND=postgres` and an isolated `DATABASE_URL`.
Never rename tables or run destructive fixtures against installed user data.
Host addresses, credentials and server configuration remain outside Git.

The remote container uses 256 MiB tmpfs, 512 MiB memory, two CPUs and a 128-process
limit. It creates no volume and binds only to loopback. No retained backup,
snapshot history or new storage format is introduced.
After verification, the test container, its private port forward and the local
disposable JSON directory were removed. The container label query returned no
instances, the forwarded port had no listener and the local directory was absent.
Shared SSH connections and unrelated services were left running.

## Qualification boundary

The coverage is scoped to these source service reads and their public projection.
Independent HTTP requests can observe different generations. It does not certify
installed GUI refresh/retry behavior, distributed Agent state, external result
files, Kcore artifacts, power-loss recovery, full backups or large-history
memory/latency. Low-level backend functions are internal and keep their original
failure behavior; the protected public boundary is Library/Analysis and the router.
Unexpected programming errors are not masked. No app package, version metadata
or Git history changes.

The [Workbench security-history follow-up](workbench-security-history-recovery-20261011.md)
adds scoped real-browser client controls with mocked services. It does not turn
these service results into installed-GUI or remote-outage qualification.
