# Workbench linked record navigation recovery

System/Data and PWDT linked project, version and Apply Context actions now use
one guarded entry point. Task links require a verified current collection source.
Result links additionally require an actual result record and a matching task
collection source. Version loading validates the referenced version ID, nonempty
model/project IDs and the task's project association when supplied, before
applying geometry or resetting the active preview. This is source-client
recovery against isolated mocked services, not authenticated or installed-app
qualification.

## Reproduced failures

A retained backend A task could open its version ID on newly selected backend B.
The browser baseline captured both version and version-list reads on B and
loaded B's payload as the working model. A second baseline supplied a same-ID
version with an unrelated project ID; the old loader accepted it and changed the
project association and model name. Both regression assertions failed before
the repair and passed after the guarded entry was connected.

The first integration rerun also caught a missing language dependency passed
to the session layer. The new translated source guidance exposed that missing
binding during server rendering. The binding was corrected and the two original
browser assertions passed without changing their expectations.

## Source and loaded model ownership

GUI actions and `data/openLinkedContext` resolve links from canonical task/result
collections rather than editor fields. A result ID cannot borrow a same-ID task
when its result is absent. Missing, stale, mixed or unreadable source metadata
blocks before a backend read or selection/filter mutation. The effective API
URL and governed-header predicates remain client equality checks, not server
identity proof or continuous authority-history tracking.

Linked version loading carries source and feedback ownership into the existing
persisted-version loader. Before fetch/application, obsolete source, workspace
intent or feedback blocks publication. Invalid version associations cannot
create an undo entry, reset a preview or apply context filters. After valid
application the loader reclaims its own feedback revision; newer messages during
the final version-list read still suppress an obsolete Apply Context message.
Current-project ownership and effective API scope are checked again after that
read, and PWDT reports failure so dependent saves cannot continue from a
superseded load. A valid payload already applied before a later switch is not
rolled back or archived.

Project-only navigation selects the verified project catalog entry and detaches
saved-model/version associations while preserving working geometry. It no longer
selects the first saved model or binds an unloaded latest version, nor reads that
model's version list just to open a project. Apply Context commits filters with
a validated payload or project navigation, not with an optional success message
after the final list read. Later list completion cannot repeat those setters
over newer editing. Errors are structured
operation failures; disposed owners cannot start project navigation.

Project catalogs receive one weak source entry per collection outside serialized
data. Project/version-list reads discard replies from an obsolete effective
backend, including stale recovery errors. Bootstrap cannot act on a response
after its authority changed. Confirmed project/model/version removal preserves
the original project collection source through filtering/mapping; it does not
retag old records as a new backend. Source marking and lookup remain constant
time; deletion mapping retains the existing linear traversal and adds no payload
copy, snapshot, backup or persistent source field.

## Verification controls

The final complete frontend unit suite passed 1,491 tests with zero failures or
skips in 24.3 seconds. Relative to the preceding 1,446-test suite, this adds 45
net controls: 38 linked-service controls replace three older navigation controls,
one source-preserving map case, three PWDT forwarding cases and six real
version-loader scope cases.

The final source Chromium run passed all 93 tests with zero failures or skips
in 252.4 seconds: 28 linked-context controls, 13 PWDT completion controls,
20 selected-task cancellation controls and 32 Library record recovery controls.
The linked suite includes twelve stale-source GUI/PWDT combinations, six valid
navigation cases, three missing-result cases, four delayed/superseded load cases,
one project-catalog recovery case and the two original reproductions.

Two intermediate 93-test browser runs each passed 92 and failed one. The first
fixture made health unavailable along with the project catalog, so the offline
UX guard blocked before the intended catalog-source check. The fixture now keeps
health valid while independently failing project reads and still asserts no
backend writes. The second run exposed a normal GUI Apply Context load with
missing filters. Filter publication was coupled to final-message ownership; it
now commits once with the validated payload. A new unit control verifies that
a later list response cannot repeat the context setters over newer editing.

Final checks passed for frontend types, the sensitive-storage and 800-line
frontend guards, repository organization (800-line source/2000-line documents,
zero tracked debt), runtime API contracts, module topology, module/function
matrix, module extension standard, coverage tensor and self-test, desktop
validation self-test, documentation inventory/book, version contracts, Rust
formatting and whitespace. The tensor retains four maturity gaps, twenty
evidence-grade gaps and fifteen P0 gaps; overall Daji qualification remains
blocked. No matching test service processes or Workbench UI fixture directories
remained after completion.

Pure controls cover canonical membership, both task/result paths, all three
modes, missing/stale/unreadable sources, missing results, version identity and
project mismatches, project-only binding, delayed/failed version loads, disposed
ownership and unexpected opener rejection. The real version-loader controls
reject late payloads/errors before validation, geometry, undo or preview reset.
PWDT controls verify the original record identity reaches the shared service.

The scoped browser suite covers GUI/PWDT source rejection, valid navigation,
missing result records, wrong-project versions, delayed payload/list responses,
superseding model edits or backend changes, dependent-write suppression and
task-only recovery with an unavailable project catalog. Synthetic solver/model
payloads test UI transfer and operation ownership, not scientific accuracy.

Run the browser suite from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-linked-context-recovery.test.mjs
```

## Qualification limits

This change does not establish authenticated provenance, cross-client revision
locks, continuous record-membership or authority-history fencing, scientific
validity, remote operation or installed WebView acceptance. General saved-model
opens, ordinary project selection and other administrative workflows retain
their own source/admission boundaries. Backend permissions, API schemas,
persisted formats, Engine/Agent, operators and headless SDKs are unchanged.

No write replay, archive, snapshot, backup, production Node dependency, version
bump, Git commit/push, application rebuild or installation is introduced.
Native test registration, desktop validation and the coverage tensor record the
bounded source-client evidence without promoting operational acceptance.
