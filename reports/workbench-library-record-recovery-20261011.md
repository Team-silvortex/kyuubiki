# Workbench library record confirmation and recovery

Ordinary project/model/version writes now require usable matching record receipts
before Workbench reports success or starts follow-up reads. Confirmed deletion
removes its local records and invalidates older reads before refresh, while the
live modeling copy remains available. GUI and PWDT share these controls; existing
idempotent model/version checkpoint saves remain unchanged. Source Chromium uses
isolated mocked services; separate real JSON/SQLite API tests check storage.
Neither is installed WebView or remote acceptance.

## Reproduced defects and repair

The initial 76-case receipt unit selection passed 12 and failed 64. These failures
exercise several receipt boundaries, not 64 independent product bugs. The first
selected source-browser baseline reproduced false success for a null project-edit
receipt and false model deletion confirmation. Its third failure was a test-harness
path recomputed after deletion, not a product defect. After capturing the path
before mutation, the version baseline reproduced a real retained deleted row:
`Latest` remained beside `Earlier` when the confirmed delete's follow-up reads failed.

An initial repaired expanded browser run passed 88 cases. Further expansion added
old failed responses and blocked-catalog independence. That 96-case run passed
95 and failed one: the GUI version row remained visible even though its stored
selection was detached and an independent version-read warning had appeared.
A bounded five-second wait with the catalog still blocked reproduced the failure;
the same PWDT control passed. This was not dismissed as a timing flake.

The model/version lists use deferred rendering. Their old values could remain
visible while the GUI's asynchronous transition awaited a catalog read. They now
filter deferred rows against current verified/local-confirmed identities. Removed
rows and previous-workspace rows cannot survive the pending transition; new rows
may still render later. Membership filtering is linear, not a nested scan. This
keeps deferred additions without making confirmed deletions wait for unrelated
reads. The repaired blocked-catalog GUI and PWDT controls both passed, followed
by the complete final browser regression.

The first real backend run had zero business failures but failed strict validation
because generated tests compared literal disjoint atom types. Moving parent checks
into pattern-matched helpers removed those compiler warnings; warnings remain
errors. Unit reference-preservation assertions also caught unnecessary recreation
of unrelated project objects; local removal now preserves their identity instead
of weakening the assertions.

## Confirmation and recovery boundaries

The shared library service validates project creation and ordinary metadata edit
or deletion receipts. Envelopes and returned records must be non-array objects.
Primary identities and names must be nonblank strings; edit/delete identities
must match the requested record. Models/versions require project identities and
object payloads. Versions also require a model identity and a positive integer
version number. Valid receipt objects and original transport errors are preserved.
Library DELETE returns the removed record, not a separate boolean deletion flag.

Malformed success responses raise `LIBRARY_RECORD_MUTATION_UNCONFIRMED`, preserve
selection and the working copy, and do not start automatic reads or replay writes.
The server may nevertheless have committed an ambiguous mutation. Read current
records before deciding whether to retry; ordinary writes have no newly invented
retry or exactly-once guarantee. These checks do not authenticate receipts, verify
every record field, or bind a returned parent identity to an expected parent.

Confirmed deletion immutably removes only matching local project/model/version
records and their listed descendants. It invalidates older catalog reads and only
the affected visible version-read scope. A disposed workspace publishes nothing.
Selection detaches only if it still points at the removed identity. The live
modeling payload/name and other selections remain intact. Removing the known latest
version clears its local latest ID/number; a partial list is not used to invent a
replacement. A later verified backend read can provide the actual latest version.

Local removal and detachment happen before follow-up reads. Catalog and version
refresh start independently for version deletion. Contained read failures remain
visible without revoking confirmation or resurrecting removed records. Late old
successes/failures cannot publish stale lists or replace newer recovery warnings.
Feedback ownership is captured after local detachment; navigation before the write
returns or while follow-up reads wait cannot publish an old success message over
the new workspace. Explicit read recovery does not repeat ordinary mutations or
checkpoint/execution writes.

No persistent tombstone, draft archive, backup or extra stored snapshot is added.
The shared local rule stays in the frontend; Engine/Agent and operator execution
are unchanged. The helper preserves unrelated project objects and model payload
references, not every top-level array reference. Existing checkpoint request IDs,
retry journal, lost-response reconciliation and save-as behavior are unchanged.

## Executed controls

- Final frontend unit suite: 1,298 passed, zero failures or skips in 35.6 seconds.
  The 89 added controls cover 76 ordinary receipt boundaries, ten local/deferred
  removal controls and three navigation-during-refresh cases. The focused 181-case
  selection, type checking and frontend 800-line guard also passed.
- Final source Chromium selection: 96 passed, zero failures or skips in 323.6
  seconds. It includes 32 new library-record controls, ten checkpoint-recovery
  controls, two Data/Library action controls and 52 project-boundary controls.
  Startup/disposal are included; this is not a performance benchmark.
- Expanded real library and related APIs on JSON, seed 0: 65 listed, 54 passed,
  11 backend-specific cases skipped, zero failures in 1.7 seconds.
- The same selection on SQLite, seed 2718: 65 passed, zero failures or skips in
  1.4 seconds. Both runs enforce compiler warnings as errors.

Ten added real API controls verify project creation, metadata edits and matching
usable deletion receipts, including parent identities, object payloads and version
numbers. Removed records return 404 on reads, repeated deletion and metadata edits;
remaining parents, unrelated models/projects and existing execution records stay
unchanged. Confirmed deletion is checked before actual storage loss; subsequent
catalog reads return opaque `analysis_store_unavailable` 503, not empty success.
JSON's lightweight library restart is volatile: these outage cases do not qualify
durable memory-store recovery. This adds backend tests, not backend production behavior.

The browser controls use actual GUI buttons and PWDT actions, not just HTTP status
checks. Assertions inspect published selection, model name, visible rows, warnings,
request counts and write/submission counters. Twelve controls reject unconfirmed
receipts; eighteen combine confirmed project/model/version deletion with failed
follow-up reads and absent, old successful or old failed reads. Two keep catalog
requests blocked while version history recovers independently. Read recovery never
replays writes; ordinary mutations do not create task/results or submit workflows.

An additional full desktop-validator attempt in the restricted sandbox could not
start browser fixture listeners (`listen EPERM`). Its summary was 376 listed,
10 passed and 366 failed, dominated by setup/hook failures. This is not a product
verdict or qualified evidence and was not counted among the successful controls.
The complete desktop suite was not rerun with expanded permissions; the 96-case
scoped browser regression above ran with authorized isolated local test services.

Run the source browser selection from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-library-record-recovery.test.mjs tests/integration/workbench-ui-checkpoint-recovery.test.mjs tests/integration/workbench-ui-data-library-actions.test.mjs tests/integration/workbench-ui-project-boundaries.test.mjs
```

Run the expanded real backend selection from `apps/web` with an isolated backend:

```text
mix test test/kyuubiki_web/library test/kyuubiki_web/api/library_record_recovery_api_test.exs test/kyuubiki_web/api/project_model_api_test.exs test/kyuubiki_web/api/project_version_chain_api_test.exs test/kyuubiki_web/api/project_workflow_context_api_test.exs test/kyuubiki_web/api/history_read_outage_test.exs --warnings-as-errors --seed 0
```

For JSON, select `KYUUBIKI_STORAGE_BACKEND=json` and a fresh temporary
`KYUUBIKI_DATA_DIR`. For SQLite, select `KYUUBIKI_STORAGE_BACKEND=sqlite` without an
explicit database path so test mode owns a disposable database. These tests never
use installed application data. The browser file is registered in the native UI
integration runner and desktop UI validation contract. Earlier iterations are not
counted as additional final coverage.

Storage-security, type checking, Rust/backend formatting, runtime API, topology,
module/function matrix, extension standard, desktop-validator self-test,
documentation inventory/book, project organization, version alignment and diff
whitespace checks passed. The tensor and its self-test pass structurally with zero
structural gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps. Overall
Daji remains blocked; this scoped verified evidence does not close operational
qualification gaps or certify the full desktop suite.

After shutdown, checks found no matching browser/test frontend processes or
retained Workbench runtime copies. Both owned disposable JSON directories were
removed after confirming no open handles; successful SQLite runs cleaned their
owned databases automatically. The sandbox-blocked temporary desktop report was
discarded, with its failed outcome recorded above. No installed data or backup
was changed.

## Remaining scope

Ambiguous-write outcome reconciliation, cross-record persistent drafts, multi-user
conflicts, authenticated provenance, backend-authority transitions, installed GUIs,
remote deployment, large-history performance, power-loss recovery and exactly-once
execution remain separate acceptance work. Deferred filtering covers model/version
lists, not a general upgrade of every deferred administrative surface. No production
Node dependency, version bump, app rebuild/install or Git commit is introduced.
