# Analysis collection and composite export recovery

Source collection availability: PASS for SQLite, durable JSON and PostgreSQL. Task and
result lists, workload discovery, database export and project-bundle export now
contain known analysis-storage failures. A failed read returns an explicit
unavailable error, not an empty success or a partial downloadable bundle.
Normal response shapes, authorization and public recovery metadata are retained.

## Corrected call chains

The preceding single-record read guards did not cover collection discovery.
Stopping the actual repository reproduced exceptions in both lists and all
three composite paths. Direct control-plane collection calls also raised.

`AnalysisJobRecords.list_jobs/0` and `list_results/0` now use the existing storage
failure boundary. Successful calls still return their original maps; unavailable
storage returns `{:error, :analysis_store_unavailable}`. The Analysis facade
specifications expose that additional outcome without introducing a new codec.

Workload discovery and exports check these replies before consuming their
collections. Their outer boundary also contains known SQL failures from project,
model and version reads. They return the storage error rather than treating it
as collection data or substituting empty records. No write, resubmission or
automatic retry is introduced. Unrelated programming errors remain visible.

The five GET routes return opaque HTTP 503 JSON while unavailable:

- `/api/v1/jobs`
- `/api/v1/results`
- `/api/v1/workloads/catalog`
- `/api/v1/export/database`
- `/api/v1/projects/:project_id/bundle`

The shared map-response helper accepts GET requests only; mutating routes retain
their existing mapping. Project-bundle attachment headers are set only after a
successful full response construction, not on a failed export. Authorization
still runs before the read. Available stores with no jobs/results return HTTP
200 with genuine empty collections. An absent project still returns HTTP 404.

## Executed controls

Ten new test cases use a populated project, model, version and terminal workflow
record, then stop the actual SQL repository or durable analysis-memory store.
These are read fixtures, not solver-accuracy or scientific-qualification cases.

Five cases check successful read, opaque unavailable response without an
attachment, store restoration and unchanged original records. Only regenerated
top-level export/catalog timestamps are excluded from response comparison.
Job/result snapshots and model/version records are compared exactly.

The remaining cases check direct headless helpers, genuine empty lists,
authorization before unavailable-store access, private recovery exclusion from
result lists and exports, and an absent project's unchanged 404. Public results
retain their recovery summary but expose neither the private record nor its
owner-session identifier. Existing read, edit, CRUD, authorization, transport
security and failure-boundary tests are also included in the selections.

## Verification

- SQLite recovery/routing/API selection: 861 listed, 841 passed, 20
  storage-specific cases skipped, zero failures in 32.9 seconds.
- Isolated JSON recovery/routing/API selection: 564 listed, 553 passed, 11
  PostgreSQL-specific cases skipped, zero failures in 7.9 seconds. Project and
  library reads can succeed while the separate analysis-memory store is stopped;
  composite reads still return the error without partial data.
- Remote PostgreSQL selection: 74 tests passed, zero failures in 110.6 seconds.
  This includes the ten collection controls, existing read/edit/security cases,
  eleven typed-workflow transaction tests and five migration tests. PostgreSQL
  16.14 ran in a disposable Docker container on the Ubuntu lab host; the Elixir
  control plane and test runner used macOS source through a private tunnel.
- Strict Elixir test compilation, formatting and runtime API surface checks pass.
- Organization audit retains zero debt at 800 source / 2000 document lines.
- Documentation inventory/book, topology, function matrix, extension standard
  and version audit pass. The tensor and its self-test pass with zero structural,
  four maturity, twenty evidence-grade and fifteen P0 gaps; overall Daji
  qualification remains blocked.

Run the focused cases from `apps/web`:

```text
mix test test/kyuubiki_web/api/analysis_collection_outage_test.exs test/kyuubiki_web/api/analysis_read_outage_test.exs test/kyuubiki_web/api/storage_outage_health_test.exs test/kyuubiki_web/api/job_audit_api_test.exs test/kyuubiki_web/api/result_administration_api_test.exs test/kyuubiki_web/api/control_plane_api_test.exs test/kyuubiki_web/api/router_access_and_bar_test.exs test/kyuubiki_web/http_transport_security_test.exs test/kyuubiki_web/storage/failure_boundary_test.exs --seed 0 --warnings-as-errors
```

For JSON, provide `KYUUBIKI_STORAGE_BACKEND=json` and a fresh temporary
`KYUUBIKI_DATA_DIR`. For PostgreSQL, use a disposable database and provide
`KYUUBIKI_STORAGE_BACKEND=postgres` and `DATABASE_URL` in the environment.
Connection values and server configuration do not belong in this repository.
The disposable container, its port forward and the isolated JSON directory were
removed after verification. No Docker volume was created; other services and
shared SSH connections were not stopped.

## Qualification boundary

This verifies source service/router behavior with actual store stop/restart.
It does not qualify installed WebViews, Linux control-plane execution, independent
library/security-memory-store outages, power loss, large collection performance
or every possible mid-query failure. The outage controls alone do not prove atomic
concurrent-edit snapshots or consistent database backups. The later
[SQL snapshot follow-up](analysis-export-snapshot-20261011.md) establishes one read
transaction for the three composite SQL paths; JSON cross-store consistency and
full backups remain separate. No installed app, version metadata or Git history changes.
