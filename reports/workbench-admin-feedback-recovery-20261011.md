# Workbench administrative feedback and backend ownership

Workbench administrative task and result Save/Delete now distinguish feedback
ownership from data ownership. A late response cannot replace a newer message or
publish task/result changes against a different effective backend. Confirmed
writes at the same backend still clean up their own records and refresh history
after navigation. These are source-client controls with isolated mocked services,
not installed WebView or authenticated deployment acceptance.

## Reproduced feedback defect

A held task Save request returned an error after the user downloaded a result.
The baseline browser control observed `old administrative write failed` replacing
`Analysis result JSON downloaded.` The defect was unconditional message publication
after the write, not an unavailable download or missing click handler.

Each workspace now owns a small in-memory feedback revision. Accepted task/result
mutation intent captures the revision synchronously, before its queued write
starts. Suppressed duplicate or conflicting actions do not claim another revision.
This preserves the existing reservation-before-transition rule.

Every message publication invalidates older owners, including identical text that
React might otherwise ignore. Built-in sidebar/system navigation, manual record
selection and workspace disposal also invalidate feedback. Selecting another
record and then returning does not revive the old owner. Automatic selection
reconciliation after a confirmed deletion uses raw setters, so an operation does
not accidentally invalidate its own success feedback.

This guard does not cancel or roll back a backend write. It suppresses only a
message that no longer belongs to the active interaction. Same-backend deletion
still removes the confirmed record, keeps unrelated selection and working drafts,
and runs its follow-up reads. Typing without navigation or newer feedback retains
the existing confirmation message behavior.

## Backend ownership boundary

Task/result writes capture their effective endpoint URLs and governed request
headers. After acknowledgement or rejection, only the same effective authority
can receive their local effects and messages. Confirmed deletion from backend A
cannot remove a same-ID record already loaded from backend B, and an old write
cannot start follow-up reads at B. Task deletion checks both affected collections.
Job/result reads also check authority after awaiting the response, independently
of their existing latest-read sequence guards. Old-authority success cannot
replace known history; old-authority failure cannot publish or clear recovery
warnings. A discarded result read releases only its own pending state.

The comparison is held only for the request lifetime in memory. Credentials are
not written into browser storage, reports, logs or a new recovery journal. URL
query precedence and applicable header rotation/removal are covered; unrelated
settings or unused credentials do not supersede an authority unnecessarily.

This is an effective URL/header equality check, not a continuous authority event
history. An A-to-B-to-A transition with identical final URL and headers is not
certified as distinct, and server identity changes behind an unchanged address are
not authenticated by this comparison. Known history remains a retained view until
an explicit current-backend refresh; this change does not implement cross-client
locking, ambiguous-write reconciliation or automatic write replay.

## Executed regression controls

Verification ran October 11-12, 2026.

- Frontend unit suite: 1,335 passed, zero failures or skips in 20.9 seconds.
  Sixteen added controls cover seven feedback revision cases, three API authority
  cases, one accepted-intent reservation case and five action-controller cases,
  including an invalid result draft queued before newer feedback.
  Flow composition additionally verifies the workspace feedback owner reaches
  the administrative actions unchanged.
- New source Chromium suite: 24 passed, zero failures or skips. Eight controls
  cover late Save/Delete success/error after export; four cover manual selection
  away and back; four cover blocked post-write reads; four cover old confirmed
  writes after current-backend same-ID reads; four discard old read success/error
  after backend change without a newer read.
- Final expanded source Chromium selection: 163 passed, zero failures or skips
  in 521.0 seconds. It includes the 24 new controls and 139 task/result write-flight,
  task-draft, task-record, result-history, result-record, library-record,
  checkpoint and Data/Library action controls. Fixture startup and disposal are
  included; this is not a performance benchmark.

The browser tests exercise actual GUI Save/Delete, record selection and result
download, plus built-in PWDT navigation and explicit read recovery. Backend
transitions use controlled URL-query overrides, not external hosts or real
credentials. Mutation/submission counters and new-authority read counters reject
write replay and unintended follow-up work. Completion and record visibility are
checked separately from HTTP acknowledgement.

The first expanded 163-case run passed 159 controls and failed four obsolete
message expectations after switching from tasks to results. Task/result removal,
older-read isolation and the current recovery warning had already passed in
those cases; the timeout waited only for the old task-deletion message. The four
controls now wait for transition completion and require the newer page's feedback
to remain unchanged. Record visibility, selection, warning and no-replay assertions
remain intact; no timeout was increased or data assertion removed.

Run the new suite from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-admin-feedback-recovery.test.mjs
```

The native UI integration runner, desktop validation contract and coverage tensor
register this source-client evidence. Engine/Agent execution, physical operators,
headless SDKs, API schemas and data formats are unchanged. No persistent cache,
snapshot, backup, retry queue or production Node dependency is added. There is no
version bump, Git commit, app rebuild or installation in this round. Installed
WebViews, real persistence stores, authenticated remote authority transitions,
cross-client conflicts and power-loss recovery retain their separate scope.

Type checking, storage-security, Rust formatting, runtime API surface, topology,
module/function matrix, extension standard, desktop-validator self-test,
documentation inventory/book, project organization, version alignment and diff
whitespace checks passed. The source limit remains 800 lines and documentation
limit 2000 lines, with zero tracked line-count debt. The tensor and its self-test
pass structurally with zero structural gaps, four maturity gaps, 20 evidence-grade
gaps and 15 P0 gaps. Overall Daji remains blocked; this source-client evidence
does not promote operational qualification. The full desktop validator and real
backend storage tests were not rerun.

After shutdown, no matching test/frontend processes or Workbench fixture runtime
directories remained in the checked temporary root. No installed data, unrelated
process, backup or historical evidence was removed.
