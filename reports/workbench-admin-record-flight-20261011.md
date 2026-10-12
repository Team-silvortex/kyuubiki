# Workbench task and result write recovery

Workbench administrative Save and Delete now share one in-flight operation per
parent task identity. The reservation starts before the request and remains until
follow-up reads settle. It blocks duplicate or conflicting actions for the task
and its result without blocking other records, draft editing, navigation or result
export. Source-browser controls use isolated mocked services, not installed
WebViews or real persistence backends.

## Reproduced defect and repair

The four selected browser baseline controls failed because neither task nor result
Save disabled competing writes while its request waited. The action controller
discarded the asynchronous promise inside the React transition. The data panel
had no pending-write inputs, and recreating a controller provided no shared
reservation across task/result views.

The first repair reserved identities inside the asynchronous transition. All 14
expanded controls still failed at the pending-button assertion. Although the pure
reservation tests passed, its React state update could wait for the transition's
asynchronous completion. This was a real integration defect, not a timeout to relax.

The final controller reserves the identity and publishes its pending state before
starting the transition. React receives the asynchronous operation instead of a
discarded promise. The four selected Save controls then passed. Unit assertions
also require reservation publication before the transition callback executes.

## Write and rendering boundaries

One in-memory reservation belongs to the Workbench session rather than its lazy
sidebar panel. Task and Result actions use the same parent job ID. Synchronous
duplicate activation shares the existing promise; a conflicting callback is not
queued or run after the first action. Other identities and other Workbench
sessions remain independent. Both success and rejection release the reservation,
and a later write requires a new explicit action.

Follow-up reads are part of the reservation. A confirmed write with a failed read
retains its confirmation and does not replay the mutation. Saving and deleting
the selected record are disabled while it is reserved, with the existing localized
busy label. Draft inputs, navigation and export stay available. Switching to the
paired record or remounting the sidebar retains protection; another task remains
writable. This is not backend locking, cross-client serialization or exactly-once
execution, and it adds no write retry queue.

Deferred task/result lists now filter against current record identities, reusing
the same membership rule as model/version lists. Confirmed deletion cannot keep
an obsolete deferred row visible while its follow-up read waits. Task deletion
removes its task/result pair; result deletion keeps the task. New rows may still
render later, and membership filtering remains linear. Verified read failures
retain unrelated records, selection and working drafts under the existing rules.

Effect disposal suppresses reservation-state publication and new actions through
that disposed scope; it does not claim to cancel an already accepted operation.
Effect remount restores pending identities without starting another operation.
The existing mutation controllers still own their messages and local effects.
This change does not add late-message ownership fencing or backend-authority
qualification to those controllers.

Engine/Agent execution, operator code, headless SDKs, public APIs and data schemas
are unchanged. The protection is for the Workbench administrative GUI actions,
not arbitrary independent clients. No persistent draft cache, backup, tombstone,
snapshot, retry journal or production Node dependency is introduced.

## Executed controls

- Final frontend unit suite: 1,319 passed, zero failures or skips in 32.9 seconds.
  The 170-case focused selection and final 37-case action/composition selection
  also passed. Type checking and the 800-line frontend guard passed.
- Final source Chromium selection: 139 passed, zero failures or skips in 507.7
  seconds. It includes 14 new write-flight controls, 18 task-draft controls,
  22 task-record controls, 16 result-history controls, 25 result-record controls,
  32 library-record controls, ten checkpoint controls and two Data/Library action
  controls. Startup and disposal are included; this is not a performance benchmark.

The 21 added frontend controls comprise eight reservation controls, twelve action
controller controls and one deferred job-ID membership control. They cover
synchronous duplicates, conflicting actions, independent identities/workspaces,
write and read waiting, synchronous/asynchronous failures, explicit later actions,
disposal, effect remount and reservation publication before React transitions.
The flow-composition control additionally checks that the shared reservation
reaches the action controller unchanged.

Fourteen new source Chromium controls exercise actual GUI Save/Delete, switching
between task/result views, sidebar remount, another record's concurrent write,
later typing, failed writes, blocked/failed follow-up reads and explicit PWDT read
recovery. They check button state, visible membership, async busy state, request
counts and independent write/submission counters. They do not count HTTP status
alone as successful recovery.

Run the expanded source-browser selection from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-admin-record-flight.test.mjs tests/integration/workbench-ui-job-record-recovery.test.mjs tests/integration/workbench-ui-result-record-recovery.test.mjs tests/integration/workbench-ui-job-draft-recovery.test.mjs tests/integration/workbench-ui-result-history-recovery.test.mjs tests/integration/workbench-ui-data-library-actions.test.mjs tests/integration/workbench-ui-library-record-recovery.test.mjs tests/integration/workbench-ui-checkpoint-recovery.test.mjs
```

The new browser file is registered in the native UI integration runner and desktop
validation contract. This remains scoped source-client evidence. Installed WebView
acceptance, real persistence-backend regression, authenticated provenance,
ambiguous-write reconciliation, cross-client conflicts, large-history performance,
power-loss recovery and backend-authority transitions remain separate work.
There is no version bump, app rebuild/install or Git commit in this round.

Storage-security, Rust formatting, runtime API, topology, module/function matrix,
extension standard, desktop-validator self-test, documentation inventory/book,
project organization, version alignment and diff whitespace checks passed. The
tensor and its self-test pass structurally with zero structural gaps, four maturity
gaps, 20 evidence-grade gaps and 15 P0 gaps. Overall Daji remains blocked; this
verified source-client evidence does not promote any operational acceptance grade.
The full desktop validator and real backend storage tests were not rerun this round.

After shutdown, checks found no matching browser/test frontend processes or
retained Workbench runtime copies in the fixture temporary root. No installed
data, unrelated process, backup or historical evidence was removed.
