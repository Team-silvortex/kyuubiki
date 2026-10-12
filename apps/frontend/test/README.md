# Frontend Test Notes

The frontend now has a lightweight native unit-test layer built on Node's
`node:test` runner with `--experimental-strip-types`.

Current safety rails include:

- `npm run build`
- `npm run typecheck`
- `npm run test:unit`
- `npm run test:unit:workflow`

Organization rules:

- Put reusable fixtures and helpers under `test/support`.
- Group tests under `test/<domain>` so workflow, installer, and hub behavior can
  grow independently.
- Keep browser and smoke coverage separate from pure unit tests; integration
  flows still live under the repo-level `tests/integration`.
- The unit-test runner registers a local `@/` alias loader, so tests may import
  real frontend source modules directly instead of copying logic into test-only
  wrappers.

## Modeling performance

The `workbench-model-batch` filter covers grouped selection, topology-preserving
arrays, load distribution, member assignment, deletion and atomic history edits.
The `workbench-model-transform` filter covers right-hand rotation, selection and
explicit pivots, scaling, reflection, grid snapping, copy identity/boundary rules,
72 rotation round trips and bounded 200k-node transforms. These are geometry
invariants and operation-count checks, not solver or interactive FPS benchmarks.
The real-browser GUI/PWDT boundary test is
`tests/integration/workbench-ui-model-batch.test.mjs` (run from the repository
root with `node --test`). Its backend is isolated and mocked; it is not a solver
accuracy or native WebView qualification. See the
[batch modeling tutorial](../../../docs/tutorial-pwdt-automation.html#batch-modeling).

`tests/integration/workbench-ui-immersive-modeling.test.mjs` exercises real Chromium
fullscreen, sidebar/dock draft transfer, legacy quick edits, multi-selection,
undo/redo, study navigation, rejected fullscreen requests and five viewport sizes.
It also covers query-to-canvas selection, fullscreen save/save-as/retry with mocked
storage, duplicate-submit prevention across tool-tab remounts, and bounded pointer /
keyboard dock resizing, cancellation and reset. Pure layout and selection tests
cover invalid dimensions and 200k-node selection without scanning connectivity.
Its storage service is mocked. `workbench-model-batch-draft` and
`workbench-script-state-controller` add draft-scope and failure-atomicity checks.
See the [fullscreen editing contract](../../../docs/tutorial-pwdt-automation.html#immersive-modeling).

Missing-API fallback, editable-form shortcuts and cold-load exit ownership are
covered separately from actual browser fullscreen. Projection tests keep default
3D presets inside the canvas and verify proportional fitting and drag inversion.
For native acceptance, run `tests/manual/pwdt-immersive-truss3d.py` in Workbench PWDT,
then follow the [installed macOS acceptance record](../../../docs/acceptance-immersive-modeling.html).
After saving the tall variant and restarting the managed runtime, reopen it and
run `tests/manual/pwdt-immersive-recovery.py`. These use real services and create a
small isolated project; they are not standalone Python/headless-SDK tests.

`tests/integration/workbench-ui-checkpoint-recovery.test.mjs` covers checkpoint
publication versus history-read failures, read-only recovery without duplicate
writes, model-scoped history, and delayed responses after navigation. The installed
backend's checkpoint endpoint already commits model metadata, geometry, latest
pointer and version together; neither GUI nor PWDT may precede it with a model
PATCH. SQLite rollback injection is covered independently in
`apps/web/test/kyuubiki_web/library/checkpoint_transaction_test.exs`.

The same browser suite aborts responses after committed Save and Save As operations,
then exercises GUI buttons and PWDT retries without duplicate versions. Unit checks
in `workbench-checkpoint-retry` cover request-key retention, concurrent submissions,
authority changes, explicit keys and bounded-cache refusal. The project-library
service also rejects malformed success envelopes without discarding the retry key.
Backend request/conflict/deletion and SQL receipt-transaction tests live in
`apps/web/test/kyuubiki_web/library/checkpoint_request_test.exs` and
`apps/web/test/kyuubiki_web/library/checkpoint_receipt_transaction_test.exs`.
These require data revision 2 for SQL stores; use disposable test databases, never
the installed application's state database.

`tests/integration/workbench-ui-security-history-recovery.test.mjs` covers the real
audit refresh button and PWDT recovery path with isolated mocked services. It
preserves validated same-filter history on failure, rejects malformed successful
responses, clears prior filter data, and discards obsolete responses without
replaying writes. Loading/unavailable states cannot claim empty history or empty
facets. Unit controls live in `workbench-security-event-backend-service`; see the
[scoped recovery results](../../../reports/workbench-security-history-recovery-20261011.md).

`tests/integration/workbench-ui-result-history-recovery.test.mjs` exercises the
Results refresh and runtime Retry All buttons, PWDT read recovery, malformed
success, overlapping responses, and confirmed or rejected writes. Failed reads
retain known records and selection; recovery cannot overwrite unsaved result
drafts. Loading/unavailable is distinct from empty history. Unit controls live in
`workbench-admin-data-backend-service` and `workbench-admin-result-controller`.
See the [result recovery controls](../../../reports/workbench-result-history-recovery-20261011.md)
for source-client scope, write counters, and disposal requirements.

`tests/integration/workbench-ui-job-draft-recovery.test.mjs` covers task metadata
editing through GUI Retry All and PWDT read recovery, malformed history, rejected
or slow saves, record switching and overlapping responses. Dirty fields remain
local while clean fields follow verified data; read recovery never replays writes.
The `workbench-admin-security-state` unit filter covers the shared string-draft
rule. See the [job editing controls](../../../reports/workbench-job-draft-recovery-20261011.md)
for selected-record scope and disposable-service requirements.

`tests/integration/workbench-ui-job-record-recovery.test.mjs` checks explicit null
for clearing an optional version, required-field rejection, malformed write
acknowledgements, and job/result removal after confirmed deletion. Four stale-read
combinations cannot resurrect deleted records while follow-up reads fail or wait;
other selections and drafts survive. Read recovery never repeats writes.
The `workbench-admin-job-controller` and `workbench-admin-data-backend-service`
unit filters check the same confirmation boundary. See the
[job record controls](../../../reports/workbench-job-record-recovery-20261011.md)
for the separate source-browser and real backend API results.

`tests/integration/workbench-ui-admin-record-flight.test.mjs` checks the task/result
GUI Save/Delete reservation from write submission through follow-up reads. Rapid
activation cannot queue conflicting writes for the same task; paired-record views
and lazy sidebar remount retain the guard, while other records stay editable.
Confirmed deletion removes deferred rows even while refresh waits. Failed reads
do not replay writes or discard another record's draft. Unit controls live in
`workbench-admin-record-mutations` and `workbench-primary-actions-controller`.
The guard is published before React starts the async transition, not deferred
until its completion. See the [write recovery controls](../../../reports/workbench-admin-record-flight-20261011.md)
for source-client scope; backend and installed acceptance remain separate.

`tests/integration/workbench-ui-admin-feedback-recovery.test.mjs` checks late
task/result Save/Delete success and failure after newer download feedback,
manual selection away and back, blocked follow-up reads and effective backend
changes. Confirmed writes still refresh at the same backend; an old response may
not remove a same-ID record or start reads on a new backend. Job/result reads
discard old-authority replies even without a newer read. Unit controls live in
`workbench-feedback`, `workbench-api-auth-context`, `workbench-admin-record-mutations`
and `workbench-primary-actions-controller`. Feedback ownership starts at accepted
intent, before the queued write; duplicates cannot replace it. See the
[feedback and backend controls](../../../reports/workbench-admin-feedback-recovery-20261011.md)
for effective-scope, source-browser and authenticated-deployment boundaries.

`tests/integration/workbench-ui-admin-record-source.test.mjs` checks that retained
tasks/results and their dirty drafts cannot authorize Save/Delete on another
backend before explicit valid refresh. The GUI refresh button and PWDT recovery
load new-source values, including same-ID records; same-source reads retain dirty
editing and old result drafts remain locally exportable. Source-inspection faults
block writes without a data-panel exception. Connection-setting read faults keep
the retained-record GUI alive and block HTTP without an implicit fallback;
restoring the same source and explicitly refreshing permits a new save. Unit
controls live in `workbench-backend-target`,
`workbench-admin-record-source`, `workbench-admin-security-state` and
`workbench-primary-actions-controller`, including all 34 built-in/catalogued
guidance locales. Weak metadata is per collection, not per record, and deletion
filters retain its original source. See the
[record and draft source controls](../../../reports/workbench-admin-record-source-20261012.md)
for the four-write, retained-record and authenticated-deployment limits.

`tests/integration/workbench-ui-admin-job-cancellation.test.mjs` covers the
System/Data selected-task cancellation control: exact cancelled receipts,
record-source admission, confirmed-read failure, no automatic write replay,
shared-parent Save/Delete exclusion, remount, late feedback and backend changes.
Historical cancellation cannot clear an unrelated live preview or take over its
observation token; the live observer polls its own backend state. Pure controls
use `workbench-admin-job-cancellation`, `workbench-primary-actions-controller`
and the existing `workbench-job-history-controller` tests. See the
[selected cancellation controls](../../../reports/workbench-admin-job-cancellation-20261012.md)
for source-client and separate current-job flight limits.

`tests/integration/workbench-ui-linked-context-recovery.test.mjs` covers the shared
GUI/PWDT linked project/version/Apply Context route: current task/result sources,
actual result membership, version/project association, project-only geometry
binding, delayed payload/list reads, dependent-write suppression and project
catalog recovery. Pure controls use `workbench-admin-data-controller`,
`workbench-admin-record-source`, `workbench-script-macro-data-controller` and
`workbench-project-store-bundle-roundtrip`. See the
[linked-context controls](../../../reports/workbench-linked-context-recovery-20261012.md)
for source-client, generic navigation and installed-acceptance boundaries.

`tests/integration/workbench-ui-saved-navigation-recovery.test.mjs` covers ordinary
project selection and direct saved model/version opening: canonical source and
membership admission, exact envelope/parent identity, synchronous project intent,
explicit deselection, delayed payload/errors, metadata/GUI name editing,
same-ID new-source read recovery and malformed version-list retention.
Pure controls use `workbench-saved-navigation-recovery`,
`workbench-project-storage-controller` and `workbench-script-project-model-controller`.
An explicit project selection retains geometry but detaches saved bindings even
when its ID is unchanged. See the
[saved navigation controls](../../../reports/workbench-saved-navigation-recovery-20261012.md)
for source-client and installed-acceptance limits.

`tests/integration/workbench-ui-library-write-source.test.mjs` covers seven GUI/PWDT
Library write forms, stale catalog admission, late confirmations/errors after
verified same-ID source switching, new-project authority and explicit read recovery.
It classifies independent security audit POSTs separately while asserting that
Library writes and recovery reads cannot cross the admitted source. Pure controls
use `workbench-library-write-source`, `workbench-project-storage-controller`,
`workbench-script-project-model-controller` and `workbench-flow-composition`.
Version history must remain canonical through the invoker; copied/unmarked rows
cannot authorize writes. See the
[Library write source controls](../../../reports/workbench-library-write-source-20261012.md)
for source-client, payload provenance, write exclusion and audit-routing limits.

`tests/integration/workbench-ui-result-record-recovery.test.mjs` checks malformed
result receipts and response bodies, invalid object drafts, slow saves, record
switching, old reads after confirmed deletion, and successful or failed downloads.
Read recovery never replays writes; newer typing survives response publication.
Export uses the working draft rather than silently reading the saved result.
The `workbench-result-record-confirmation` unit filter checks receipt gates,
numeric overflow, failure classification and temporary download URL cleanup.
See the [result record controls](../../../reports/workbench-result-record-recovery-20261011.md)
for mock-service source-browser scope and separate real backend API results.

`tests/integration/workbench-ui-library-record-recovery.test.mjs` checks ordinary
project/model/version mutations through GUI and PWDT, including unconfirmed
receipts, confirmed deletions followed by unavailable reads, late success/failure
responses and version refresh while catalog reads wait. Explicit read recovery
never repeats writes. The live model remains available after deletion detaches
its stored identity; unrelated selection and newer feedback remain protected.
The `workbench-library-record` unit filter covers receipt gates, immutable local
removal, affected read generations, disposal and linear deferred-list membership
filtering. Deferred additions may wait, but removed rows cannot remain visible
through a blocked catalog read. Existing project-boundary and
checkpoint tests verify that idempotent save recovery remains unchanged. See the
[library record controls](../../../reports/workbench-library-record-recovery-20261011.md)
for source-browser scope and separate real API/storage results.

`workbench-modeling-performance` and `workbench-truss3d-webgl` unit filters cover
immutable batch edits, linear selection/adjacency work, 200k-node bounds, scene
colors, and GPU-resource lifecycle. Run the opt-in CPU benchmark from this directory:

```sh
node --import ./test/support/register-alias-loader.mjs ./test/benchmarks/workbench-modeling.bench.ts
```

The benchmark emits a small JSON summary, never generated mesh files. Timing is
informational; deterministic operation-count tests provide the CI regression gate.
The real-browser counterpart is
`tests/integration/workbench-ui-modeling-performance.test.mjs` at repository root.
See [measured scope and limitations](../../../docs/rendering-roadmap.html#modeling-performance).
