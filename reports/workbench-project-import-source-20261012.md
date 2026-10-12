# Workbench project import source and completion recovery

Project import now retains its effective backend and authentication scope across
file parsing, project/model/version writes and recovery reads. A changed scope
or disposed workspace stops continuation and suppresses late UI publication.
The GUI busy state lasts until the import's final history read settles, giving
the user and browser tests an observable completion signal.

## Confirmed failure

Two browser baseline controls reproduced cross-backend continuation. After
backend A returned a newly created project, the old importer posted a model to
backend B using A's project ID. After A returned a model, it posted an additional
version to B using A's model ID. The mock B rejected those requests and the old
import error appeared in the current workspace. Both assertions failed before
the source guard and passed after it.

Network-idle-only assertions had observed the chain before its next asynchronous
step and could pass early. The browser controls now require a visible busy state
while a request is held, release that request and wait for `ready` before
checking requests, records and feedback. Direct awaitable importer controls also
reproduced a new-backend catalog refresh and publication of an old-source error.

Expanded browser controls found a second completion defect at final history
recovery: UI selection setters had not yet updated the synchronous project
context, so the explicit history refresh returned early. A later React effect
started the read after the import was already ready. Both held-history controls
failed their busy assertion before the remapped context handoff and passed after
it. The importer now updates that context before requesting history, retains its
new feedback ownership and only publishes success after recovery settles.

Recovery UI assertions must wait for the recovered version rows, not only the
PWDT refresh promise: React can still be publishing state after the request
completes. The history-outage control checks visible rows before comparing their
contents and the unchanged write ledger.

## Continuation and partial records

The importer captures effective API URLs and authentication headers before the
first file read. It checks that scope and the mounted owner after parsing and
each remote write, before local preset/Store persistence, and around catalog
and history recovery. A source change stops subsequent work. Late failures do
not replace the newer workspace's message or create an import error there.

Workspace selection and backend scope remain distinct. Once remote creation
has begun, navigation within the same source can leave the import running in
its new project; the newer selection, geometry and feedback remain in place.
An obsolete selection during file parsing prevents creation altogether.

Confirmed records stay in their original backend. The importer does not delete
partial projects to compensate for interrupted continuation. After reconnecting,
the user can inspect those records; an explicit re-import creates a new project.
Local data written before a later source change is also retained. No new durable
import journal, backup or snapshot is introduced.

The existing GUI transition receives the complete async import callback,
including its final history read. The returned promise settles at the same
operation boundary. Disposed owners reject entry before parsing or starting a
transition. Ordinary write reservation and checkpoint identity behavior are
unchanged by this import guard.

A failed history-list read does not undo confirmed project/model/version writes.
The existing recovery warning remains visible, and the imported model starts with
an empty history list rather than borrowing its predecessor's versions. Explicit
runtime refresh recovers the list without repeating import creation.

## Verification

The complete frontend unit suite passed 1,685 tests with zero failures or skips
in 26.1 seconds. This adds 51 controls to the preceding 1,634-test suite:
42 combinations of seven asynchronous boundaries, three scope/owner changes
and success/failure; two initial source-publication controls; same-source
navigation; disposed entry; the awaitable busy callback and synchronous remapped
context; and four final-history failure/feedback ownership controls. The directly
affected archive/import/persisted-load suite passed all 70 tests.

A combined Chromium run passed 85 tests with zero failures or skips in
239.9 seconds: ten import-source boundary controls, explicit re-import,
22 shared Library
write-reservation controls and 52 project-boundary controls. Existing import
round trips preserve active-version mapping, workspace names and subsequent
saving, including a delayed import that completes after same-source navigation.

A follow-up Chromium run passed 22 tests with zero failures or skips in
124.2 seconds: the complete twelve-case import suite and ten checkpoint recovery
controls. The additional import case holds the final history read, confirms no
early success feedback, forces a read outage, then waits for visible recovered
rows and verifies no new project/model/version write. These two final runs cover
96 distinct browser cases (the eleven original import cases are repeated).
The temporary browser workspaces were removed after completion.

Frontend type checking, browser-storage security, source/document line limits,
documentation book/inventory, topology, function matrix, extension standard,
runtime API surface, desktop UI validation self-test, tensor check/self-test and
the 224 exact version contracts passed. Rust formatting and whitespace checks
also passed. The tensor remains at 0 structural gaps, 4 maturity gaps,
20 evidence-grade gaps and 15 P0 gaps; its Daji readiness status remains blocked.

## Evidence scope

These results exercise source Workbench code with pure effect fixtures and
isolated mocked browser services. They qualify client continuation, recovery and
UX behavior. Effective-scope comparison is not authenticated server identity or
continuous authority history; a temporary source change reverted between checks
is outside this guarantee.

Import atomicity and resumption remain separate work. The subsequent
[import admission follow-up](workbench-project-import-admission-20261012.md)
adds per-context/API-scope overlapping-import exclusion and protects the created
project against concurrent writes through recovery. Neither change establishes
cross-client locking, installed WebView acceptance or physical calculation
accuracy, and does not promote the tensor's operational evidence grade.
