# Project creation receipts and revision-three recovery

## Confirmed gap and scope

Project creation previously ignored `request_id`. The nine initial regression
controls all failed against that implementation: replay created another project,
changed input was accepted, invalid identities wrote records, concurrent callers
all created projects, and deletion did not prevent resurrection by an old request.

This round adds an opt-in backend foundation, not automatic GUI import recovery.
`POST /api/v1/projects` accepts the existing request-ID syntax: 16-128 ASCII
letters, digits, underscores or hyphens. Callers should allocate an unpredictable
ID for one logical creation and retain it for retry. The identity is scoped to
one Library datastore, not inferred from project names or shared across servers.
Write authentication applies before creation and replay alike.

The same identity with the same normalized name/description returns the original
creation receipt. Replays never rewind later project edits or remove children.
Changed input returns `409 project_request_conflict`; a deleted result returns
`409 project_result_deleted`. Missing IDs continue to create independent projects.
This is deliberately not same-name deduplication or a project update operation.

## Storage and migration

SQLite commits project creation and the receipt in one immediate transaction.
PostgreSQL code uses a transaction-scoped per-request advisory lock, on the same
dynamic repository as the transaction. This PostgreSQL path is not executed in
this round. Memory Library commits both maps in one Agent update and retains
deletion tombstones only for that process lifetime.

Data revision 3, `runtime-project-requests-0003`, adds a separate
`kyuubiki_project_requests` table. Revision-1/2 recipes and checksums are unchanged.
Receipts store hashed request identity/content, project ID and original project
metadata, never model meshes or a copy of the import bundle. There is deliberately
no cascading foreign key: deleted projects retain a replay tombstone. Receipts
are not expired automatically; an explicit full Library reset or fresh disposable
database discards them. Logical research exports do not preserve this private
retry history. Full SQLite snapshots include it.

Existing revision-1/2 databases report `upgrade_required`. Startup refuses rather
than modifying active data. The existing explicit candidate-copy/verify/service-
switch procedure remains required; disposable development data may instead use a
fresh empty database. Old revision-1/2 snapshot receipts remain verifiable. Empty
databases initialize all three revisions atomically. No installed database was
migrated, backed up or switched during this work.

## Verification

- Targeted SQLite Library/migration/lifecycle suite: 73 tests, zero failures or skips.
- Memory Library and snapshot controls: 66 tests, zero failures, 15 SQL-only skips.
- Full frontend unit suite: 1,737 tests, zero failures or skips.
- Explicit API/client identity controls: 11 tests, zero failures or skips, included
  in the full frontend result rather than additional coverage.
- SQLite failure injection proves a rejected receipt insert leaves no project;
  repairing storage and replaying commits once. Eight tasks sharing two independent
  connections commit one project/receipt. Separate database close/reopen controls
  retain committed and deleted receipts without an in-memory retry cache.
- Candidate upgrade preserves old ledger rows and research payloads; late DDL
  failure rolls back. A future fourth ledger row after the supported prefix is
  rejected. Old snapshot digests remain unchanged after upgrading another file.

The final expanded API/Library/storage sweep is **not green**: 503 tests,
453 passed, 3 failed and 47 skipped. Both thermal catalog job cases and the coupled
electrostatic/heat/thermo diagnostics case still fail. A focused diagnostic rerun
locates the failures at `extract_thermal_diagnostics` or
`extract_electrostatic_diagnostics`: `operator_task_execution_receipt_invalid`
for `task_id`, or the finite fake-agent session queue has ended and returns
`agent_process_unavailable`. Their static solver response queues do not cover the
current diagnostic task calls. Those fixtures and diagnostic integration need a
separate correction; no production guard was relaxed to make them pass.

Follow-up: [native thermal catalog recovery](thermal-catalog-native-recovery-20261012.md)
corrects these fixtures and the actual native diagnostic metric/decision gaps they
uncovered. Its expanded selection passes 550 tests with 47 conditional skips.
It also records a DELETE-mode concurrency fixture failure and the correction to
runtime WAL with journal initialization before the two-connection pool starts.
The results above describe this report's original run, not that later rerun.

Frontend type checking, Elixir formatting, source/document size limits and runtime
API surface checks passed. The data-lifecycle JSON schema is updated for revision
3 while retaining reader targets 1 and 2. PostgreSQL, installed WebView, real
network-disconnect, power-loss and whole-bundle recovery are not qualified here.

## Remaining recovery work

The frontend API and Library service accept and preserve an explicit caller-owned
project request ID. They do not invent one, automatically retry, or add a second
retry mechanism beside the existing model/version journal. GUI manual creation
and import still need durable project-identity allocation, read-only receipt
reconciliation, source ownership and partial-import continuation. In particular,
an explicit GUI re-import still creates a new project. Backend support alone must
not be reported as a repaired end-to-end import retry path.

Evidence remains `verified` contract/execution/recovery support for the control
plane, not operational or numerical qualification. Product version remains 3.5.0;
no commit, push, application build or installation was performed.
