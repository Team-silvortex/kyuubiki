# Workbench saved model and project navigation recovery

Ordinary project selection and saved-model/version opening now enforce the
same source-client boundaries as linked record navigation. A retained catalog
cannot authorize a read on another backend. Returned records must match the
requested identities and parent associations before geometry, undo history or
the active result changes. Explicit project selection preserves working
geometry and detaches saved bindings rather than claiming to have loaded a model.

## Reproduced failures

The initial isolated browser baseline accepted a same-ID model response with
an unrelated project ID and loaded its geometry and association. A second
baseline used a retained model row after changing the backend and captured a
model read on the new backend. Both regression assertions failed before the
repair. Ordinary project switching already stopped that baseline's delayed
response after React committed; the new pure controls also require synchronous
intent invalidation before React commits, including same-ID selection.

The expanded browser controls exposed two further gaps. A workspace metadata
edit did not invalidate an older read's error, which overwrote current feedback.
The GUI project selector rejected a stale catalog but showed no source guidance.
Metadata edits now supersede pending loads, GUI name/material setters invalidate
feedback ownership, and project source rejection publishes the existing localized
refresh guidance before returning failure.

## Navigation behavior

The Library project selector uses `project/select`, also used by PWDT and the
immersive save interface. Identified projects require a verified current catalog.
Missing, stale or unreadable sources fail before selection/draft mutation.
An explicit `projectId: null` deselects without requiring a remote catalog.
Unknown, empty or invalid identifiers remain errors. Every accepted selection,
including the same project ID, synchronously supersedes older operations and
clears saved-model/version bindings and version history. Working geometry and
its result are not loaded, deleted or reset by project-only navigation.

Saved-model admission uses the canonical project collection and actual nested
record membership, not a copied/deferred row's source guess. Saved-version
admission requires the canonical version collection, current source and matching
selected model/project. Collection metadata is weak, outside serialized data;
confirmed-removal transforms preserve its original source instead of retagging
old rows. Version-list publication checks array shape and version/model/project
identity, retaining known rows on an invalid or failed read.

Both loaders fence effective API authority, project intent, record source and
feedback before payload publication. Models must match the chosen model and
project; versions must match version, model and project. By-ID checkpoint loading
also requires the requested version ID and identified parents even without a
caller-supplied validator. A rejected identity creates no undo entry, applies no
geometry and clears no result. Valid loads reclaim their own feedback revision
after application, and obsolete payload errors cannot overwrite a newer edit.
Final version-list completion still checks source/project authority so a
superseded operation cannot report success to a dependent caller.

## Verification

The complete frontend unit suite passed 1,534 tests with zero failures or skips
in 23.7 seconds.
This adds 43 net controls over the preceding 1,491-test suite: 26 real loader
controls, ten catalog membership/source/disposal controls, six project-selection
controls and two metadata-intent controls, replacing the previous invalid-null
selection case with explicit deselection semantics.

The source browser suite exercises wrong model/project/version envelopes, stale
catalog admission, pending successes/failures after backend or project switches,
PWDT metadata and GUI name edits, read-only same-ID source recovery, malformed
version history and successful explicit retry. Adjacent linked navigation,
Library mutation, PWDT completion and checkpoint recovery controls are included
in the expanded regression run. Synthetic model payloads qualify UI transfer
and ownership only, not numerical or material correctness.

The expanded browser run covered 113 cases: 30 new navigation controls and
83 adjacent controls (32 Library recovery, 28 linked context, 13 PWDT completion
and ten checkpoint recovery). All 83 adjacent controls passed. The run passed
111 and failed two new GUI-edit fixtures because their locator used "Model name"
instead of the actual "Model" field label. After correcting that locator, the
final complete navigation suite passed 30 tests with zero failures or skips in
70.3 seconds. No source behavior changed after the adjacent controls passed;
this records separate runs, not a claimed single clean 113-test execution.

Frontend typechecking, storage security, frontend file limits, runtime API
contracts, module topology/matrix/extension, coverage tensor and self-test,
desktop validation self-test, documentation inventory/book, repository
organization, version contracts, Rust formatting and whitespace checks passed.
The source/document limits remain 800/2000 lines with zero tracked debt, and
all 224 exact version contracts match 3.5.0. The tensor retains zero structural
gaps, four maturity gaps, twenty evidence-grade gaps and fifteen P0 gaps;
overall Daji qualification remains blocked.
No matching test service processes or Workbench UI fixture directories remained
after completion.

Run the scoped browser controls from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-saved-navigation-recovery.test.mjs
```

## Qualification limits

This remains source-client recovery against isolated mocked services, not
authenticated server provenance, continuous authority/membership history,
cross-client revision locking, scientific qualification, remote execution or
installed WebView acceptance. Weak sources and API predicates compare effective
client URLs and governed headers, not server identity. An unobserved backend
away-and-back transition is not certified. Other administrative writes and
current-job observation retain their separate admission and recovery boundaries.

No snapshot, archive, backup, automatic write replay, production Node dependency,
version bump, Git commit/push, application build or installation is introduced.
Backend APIs, persisted formats, Engine/Agent, operators and headless SDKs are
unchanged. The coverage tensor records bounded contract/recovery/UX evidence,
not an operational-grade promotion.
