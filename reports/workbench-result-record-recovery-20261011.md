# Workbench result record confirmation and recovery

Result save/delete now requires a matching identity and object receipt before
Workbench reports success or starts follow-up reads. Save/export reject nonobject
drafts and parsed numeric overflow. Backend and download failures are separated
from draft-input errors, and download activation releases its temporary URL even
on failure. These are source-client controls with isolated mocked services and
separate real JSON/SQLite API tests, not installed WebView or remote acceptance.

## Reproduced defects

Before the production fixes, the initial unit selection listed 48 cases: six
passed and 42 failed. Multiple failing cases exercise the same small set of
defects; they are not 42 independent bugs. The three selected source Chromium
baseline controls all failed: a null save receipt reported success, a null draft
was exported successfully, and a blocked download was blamed on invalid JSON.
The remaining browser controls were not run as a baseline.

The first expanded repaired browser run passed 63 of 65 cases. The two failures
were overly specific assertions expecting a JSON parsing error for a malformed
HTTP 200 response. The existing HTTP reader tolerates JSON decoding failure as
an empty payload; the new service correctly rejected this with
`RESULT_RECORD_MUTATION_UNCONFIRMED`. The assertions now require that diagnostic,
retained selection/draft, zero automatic reads and one explicit read recovery
without write replay. No production protection was weakened to pass them.

## Confirmation and input boundaries

The admin-data service requires the requested `job_id` and an object `result`.
Deletion additionally requires `deleted` to be the boolean `true`. Missing,
null, array, wrongly typed or mismatched receipts cannot confirm the mutation.
The returned object is preserved, not rewritten to equal the submitted draft:
the real backend may derive public workflow metadata. Transport errors remain
visible. A malformed receipt does not start automatic follow-up reads or change
local selection/records. The backend may nevertheless have committed, so users
must read the current state before deciding whether to retry an ambiguous write.
These checks do not authenticate the receipt or validate every nested value.

Save/export share a draft parser requiring a non-null JSON object. Numbers that
parse to non-finite values are rejected at any depth, before a write or download.
This prevents `1e400` from silently becoming `null` during JSON serialization.
Empty objects and nested arrays, nulls, strings, booleans and finite numbers remain
valid. This is not arbitrary-precision, underflow protection or lossless numeric
interchange; JavaScript number semantics otherwise remain unchanged.

Parsing, backend mutation and download activation have separate error boundaries.
A backend `SyntaxError` is not blamed on a valid draft, and download failures
retain their actual diagnostic. The shared download helper revokes its owned
object URL in `finally`. A successful activation still does not prove that an
installed WebView or operating system completed a file write.

Export uses the current working draft, which may be unsaved, and never reads or
writes the backend. Slow saves publish the originally submitted draft while
preserving later typing on the same or another selected record. Confirmed result
deletion preserves its parent job and other records. Existing local removal and
read-generation fencing are now additionally exercised against old successful
and failed reads; neither can resurrect a deleted record or finish a newer read.

## Executed controls

- Frontend unit suite: 1,209 passed, zero failures or skips in 30.3 seconds.
  Fifty added controls cover result receipts, draft validity, original errors,
  no-selection behavior and download URL cleanup. The targeted 128-case frontend
  selection, type checking and frontend 800-line guard also passed.
- Final source Chromium selection: 65 passed, zero failures or skips in 200.8
  seconds. It includes 25 new result-record controls, 16 existing result-history
  controls, 22 job-record controls and two Data/Library action controls. Browser
  startup and disposal are included; this is not a performance benchmark.
- Expanded task domain and related APIs on JSON, seed 0: 151 listed, 126 passed,
  25 backend-specific cases skipped, zero failures in 4.6 seconds.
- The same expanded selection on SQLite, seed 2718: 151 listed, 149 passed,
  two memory-only cases skipped, zero failures in 3.6 seconds.

Ten added real result API controls join five existing controls. They verify
nonobject rejection without mutation, matching valid-object receipts, parent-job
preservation, other-record preservation and repeat-deletion 404. Save/delete is
checked as committed before real storage loss; the subsequent list returns opaque
`analysis_store_unavailable` 503, not empty success. Existing workflow metadata,
active-workflow and no-resurrection controls remain covered. This follow-up adds
backend tests, not new backend production behavior.

The source browser uses PWDT for navigation/read recovery and actual GUI buttons
for save, delete and export. Assertions inspect published state, selection,
drafts, pending/warning ownership and actual downloaded JSON, not just HTTP
status. Explicit writes occur once; unconfirmed/rejected attempts are never
replayed. Read/export-only controls assert no project, checkpoint, job, result
or workflow writes. Browser fixtures do not qualify real parent-job validation;
real backend API tests provide that separate boundary.

Run the source browser controls from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-result-record-recovery.test.mjs
```

Run the expanded backend selection from `apps/web` with an isolated backend:

```text
mix test test/kyuubiki_web/jobs test/kyuubiki_web/api/job_record_recovery_api_test.exs test/kyuubiki_web/api/job_audit_api_test.exs test/kyuubiki_web/api/result_administration_api_test.exs test/kyuubiki_web/api/history_read_outage_test.exs test/kyuubiki_web/api/analysis_collection_outage_test.exs --warnings-as-errors --seed 0
```

Use a fresh temporary `KYUUBIKI_DATA_DIR` for JSON. SQLite test mode generates its
own disposable database when no explicit database path is supplied. Neither uses
installed application data. The browser suite is registered in the native
integration runner and desktop UI validation contract. Earlier runs are not
counted as additional coverage.

Storage-security, strict backend test compilation, formatting, runtime API,
topology, module/function matrix, extension standard, desktop UI validation
self-test, documentation inventory/book, project organization and version
alignment passed. The tensor and its self-test pass structurally with zero
structural gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps;
overall Daji remains blocked. These scoped controls do not close the broader
operational qualification gaps.

After shutdown, checks found no matching browser/test frontend processes or
retained Workbench runtime copies. The unused disposable JSON directory was
removed; successful SQLite tests cleaned their owned databases automatically.
No retained backup or installed-data change was made.

## Scope

No production Node dependency, persistent tombstone, draft archive, backup,
version bump, installed package change or Git commit is introduced. Engine and
operator execution are unchanged. Ambiguous-write outcome reconciliation,
multi-user conflicts, authority transitions, installed GUIs, remote execution,
power-loss recovery, exactly-once guarantees and large-history performance remain
separate acceptance work. Removing an administrative result record does not
discard the live modeling preview.
