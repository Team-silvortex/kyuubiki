# Workbench Library write coordination through recovery

GUI and PWDT now share project/model write reservations within one Workbench
context and effective client API scope. A conflicting action is rejected with
`WORKBENCH_LIBRARY_WRITE_PENDING`, not queued or merged with the earlier action.
The reservation lasts through the accepted write's confirmation and subsequent
catalog/version reads. This closes cross-surface conflicts without adding a
server lock, persistent payload, backup or workspace snapshot.

## Reproduced conflicts

The isolated browser baseline reproduced both directions: a pending PWDT model
save allowed a GUI model deletion, and a pending GUI save allowed a PWDT model
deletion to report success. Both baseline assertions failed. The repair reserves
the target synchronously before the accepted controller changes its project
intent or starts its request, so a competing action cannot supersede that intent
or borrow the first operation's successful result.

The initial held-save diagnostic used a controlled response to finish cleanup
after the old code deleted the model. Final browser controls release the request
to the mock checkpoint handler and check actual fixture records and request
counts; no synthetic successful receipt is used to hide a conflicting deletion.
Independent destructive-action security audit POSTs remain distinct from Library
resource mutations.

## Reservation scope and release

Project update/deletion and model creation or Save As reserve the whole project.
Existing-model saves/deletion and version rename/deletion reserve their model,
so a parent-project mutation conflicts with any child reservation. Different
models in one project and different projects can proceed independently.
New-project creation conflicts with another new-project creation only.

The owner is held in a weak map keyed by the existing project context. GUI and
PWDT therefore share it without a second state store or new React subscription.
Only active target metadata is copied; operation payloads are not retained.
Empty authority buckets are removed on settlement. Switching client backend
scope permits independent work, while returning to an original scope with an
active reservation still conflicts. This scope check uses effective API URLs
and authentication headers, not authenticated server identity or continuous
authority history.

GUI async transition callbacks now return an awaitable completion. The
reservation survives receipt handling and catalog/history reads rather than
ending when the button handler returns. Success and failure release it for an
explicit new action; neither outcome replays the rejected action. PWDT rejection
stops dependent sequential actions. A disposed workspace refuses new work; an
effect remount reuses its owner and cannot bypass an accepted reservation.
Existing source admission, confirmation and stale-response guards stay in place.

Network idle is not a GUI completion signal. The first expanded browser run
passed 115 of 117 controls; two subsequent-rename checks ran before the GUI
async recovery finished and correctly received the pending-write rejection.
Those controls now require both the held read's release and the visible `ready`
state before an explicit rename. They still assert no extra write during the
held read and no automatic write after release.

## Verification

The complete frontend unit suite passed 1,634 tests with zero failures or skips
in 28.3 seconds. Relative to the preceding 1,586-test suite, this adds 47 pure
reservation controls and one real PWDT-controller conflict control. The pure
controls cover the 36-case target matrix, synchronous/asynchronous failure,
post-write reads, disposed/remounted owners, independent workspaces, backend
switching and return, unreadable authority, caller target edits, stale busy
feedback, action mapping and guidance availability for all 34 supported locales.
Locale availability is not linguistic or RTL visual certification.

All 22 new isolated Chromium controls passed. They cover both GUI/PWDT first
surfaces, six additional conflict pairs, catalog/history read phases, explicit
retry after failure, sequential-action stopping and same-ID backend B project
editing alongside an active A model write. The suite is registered in the native
test runner and desktop UI test contract.

The final combined Chromium run passed 117 tests with zero failures or skips
in 313.2 seconds: 22 shared-reservation controls, 40 Library write-source
controls, 32 Library record confirmation/recovery controls, ten checkpoint
recovery controls and 13 PWDT completion controls. This checks the new exclusion
alongside the earlier receipt, source-switch and lost-response recovery paths.

Frontend type checking, storage-security and file-line guards passed. Runtime
API contracts, module topology/matrix/extension checks, tensor checks and
desktop UI validation self-tests also passed, as did Rust formatting, document
inventory/book checks and the project organization audit. The 800-line source
and 2,000-line document limits have zero tracked debt. All 224 exact version
contracts still match 3.5.0. Tensor structure remains at zero gaps, with four
maturity gaps, 20 evidence-grade gaps and 15 P0 gaps; Daji status stays blocked.

## Remaining boundaries

This is source-client contract, recovery and UX evidence against isolated mocked
services. It does not prove installed WebView/mobile behavior, remote execution,
scientific accuracy, exactly-once writes, durable recovery exclusion or a mutex
across windows, clients or processes. Source validation still runs inside the
accepted operation; a busy rejection can precede another admission error.

The subsequent [import admission follow-up](workbench-project-import-admission-20261012.md)
integrates project import with a dedicated slot and a reservation for its created
project. Restore, export and checkpoint-journal acknowledgement/read-recovery
actions remain outside this reservation gate. PWDT confirmation and audit
routing occur separately and can precede the controller's busy rejection. The
change neither disables every button nor cancels an already accepted request.
Operational-grade tensor promotion and Daji readiness are not inferred from
these browser and unit controls.
