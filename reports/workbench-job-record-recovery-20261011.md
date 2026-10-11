# Workbench job record confirmation and recovery

Workbench job editing now distinguishes a confirmed mutation from unavailable
follow-up history. Optional model-version clearing sends explicit null; required
project/case edits reach the backend for validation instead of silently becoming
omitted fields. Confirmed deletion removes only its job/result pair and invalidates
older reads without losing other records or editing. These are source-client
controls with isolated mocked services and separate real JSON/SQLite API tests,
not installed WebView or remote-runtime acceptance.

## Reproduced defects

The initial 16 browser controls passed two cases and failed 14. Empty optional
version editing retained the old association; empty required project/case editing
returned success while leaving the old values. Confirmed deletion followed by
unavailable list reads retained the deleted selection and records locally.
Malformed HTTP 200 mutation bodies were also accepted without checking the target
job or deletion flag. Multiple failing controls exercise these same defects;
they are not 14 independent bugs. Slow deletion with another selected record and
HTTP failure preservation already passed and remain protected.

The first real API selection listed 42 cases and found two failures: whitespace-only
project/case IDs returned HTTP 200 and were stored. Shared required-string
validation now rejects whitespace-only job, project and case identities. Nonblank
identifiers are preserved exactly rather than silently trimmed. New direct model
and real API controls verify this distinction at creation and metadata update.

## Mutation boundaries

The source admin-data service requires a matching job identity and valid lifecycle
contract for save/delete responses. Deletion additionally requires `deleted` to
be the boolean `true`. A malformed success body raises
`JOB_RECORD_MUTATION_UNCONFIRMED`; it cannot remove local records or trigger an
automatic follow-up read. The backend may nevertheless have committed an ambiguous
write, so the diagnostic directs the user to refresh before retrying. These
checks do not authenticate the receipt or validate every returned metadata value.

Saving captures the selected job and current fields before awaiting the write.
An empty optional model-version field sends null. Required project/case fields
remain present in the patch; null, empty and whitespace-only identities receive
422 at the backend without changing either record. An omitted optional version
still means unchanged. Shared draft reconciliation preserves later typing.

After confirmed deletion, Workbench invalidates older job and result list reads,
filters only the matching records and clears only selections for that job. Another
selected job/result and its unsaved editing remain intact. Both follow-up reads
start without waiting for each other. Read failure cannot revoke a confirmed
mutation; the read controllers retain their own recovery warnings.

Four browser controls hold pre-deletion reads and deliver every old job/result
success/failure combination after confirmation. The new job read stays blocked
while the result read fails, proving result recovery starts independently. Old
success cannot resurrect deleted records, and old failure cannot replace the new
result warning. Explicit PWDT read recovery never repeats the mutation.

## Executed controls

- Frontend unit suite: 1,159 passed, zero failures or skips in 22.9 seconds.
  Thirty-six added cases comprise 25 receipt-validation and 11 mutation-controller
  controls. Type checking and the frontend 800-line guard passed.
- Final source Chromium selection: 58 passed, zero failures or skips in 200.2
  seconds. It includes 22 new job-record controls, 18 existing task-draft controls,
  16 result-history controls and two Data/Library action controls. An earlier
  52-case run and a 20-case job-record run also passed; repeated cases are not
  counted as new coverage.
- Real JSON targeted selection, seed 0: 59 passed, zero failures or skips.
- Real SQLite targeted selection, seed 117: 59 listed, 58 passed, one memory-only
  atomic-deletion case skipped, zero failures.
- Expanded entire task domain and related APIs on JSON, seed 0: 141 listed,
  116 passed, 25 backend-specific cases skipped, zero failures in 1.4 seconds.
- The same expanded selection on SQLite, seed 117: 141 listed, 139 passed,
  two memory-only cases skipped, zero failures in 0.9 seconds.

The 19 added backend cases comprise ten real API controls and nine direct task
contract controls. Real storage stop/restart follows a confirmed deletion and
returns opaque unavailable responses, not empty successful histories. Other
jobs/results stay unchanged during normal joint deletion; repeat deletion is 404.
Browser assertions wait for published GUI state, not just network completion.
Read-only controls assert no project, checkpoint, result or workflow writes;
confirmed job mutations occur once, and rejected/ambiguous attempts are not replayed.

Run the source browser controls from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-job-record-recovery.test.mjs
```

Run the expanded backend selection from `apps/web` with an isolated backend:

```text
mix test test/kyuubiki_web/jobs test/kyuubiki_web/api/job_record_recovery_api_test.exs test/kyuubiki_web/api/job_audit_api_test.exs test/kyuubiki_web/api/result_administration_api_test.exs test/kyuubiki_web/api/history_read_outage_test.exs test/kyuubiki_web/api/analysis_collection_outage_test.exs --warnings-as-errors --seed 0
```

Use a fresh temporary `KYUUBIKI_DATA_DIR` for JSON. SQLite test mode generates its
own disposable database when no explicit database path is supplied. These controls
never use the installed application's database. The browser suite is registered
in the native integration runner and desktop UI validation contract.

Storage-security, strict test compilation, formatting, runtime API, topology,
module/function matrix, extension standard, desktop UI validation self-test,
documentation inventory/book, project organization and version alignment passed.
The tensor and its self-test pass structurally with zero structural gaps, four
maturity gaps, 20 evidence-grade gaps and 15 P0 gaps; overall Daji remains blocked.

After shutdown, checks found no matching browser/test frontend processes or
retained Workbench runtime copies. The disposable JSON directory was removed.
The successful SQLite runs cleaned their owned databases automatically; the
interrupted test-compilation run's temporary database was separately verified
unused and removed. No retained backup, package or installed-data change was made.

## Scope

No production Node dependency, persistent tombstone, draft archive, backup system,
version change or Git commit is introduced. Ambiguous-write outcome reconciliation,
multi-user conflicts, authority transitions, installed GUIs, remote execution,
power-loss recovery, exactly-once guarantees and large-history performance remain
separate acceptance work. The live modeling preview is not discarded when an
administrative history record is removed. Engine and operator execution are
unchanged.
