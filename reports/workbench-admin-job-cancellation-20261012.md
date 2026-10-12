# Workbench selected task cancellation recovery

System/Data selected-task cancellation now uses a source-checked, receipt-checked
controller rather than an inline raw request. It shares the task/result parent
write reservation through read recovery. Only a matching valid `cancelled`
receipt confirms cancellation; an HTTP success with an active, completed or
failed job is not confirmation. This is source-client recovery with isolated
mocked services, not installed WebView or authenticated deployment acceptance.

## Reproduced boundary

The old System/Data handler sent a cancellation request for a retained backend A
record after the API target changed to B. The browser captured a POST to B for
the old task ID, and the GUI reported cancellation. Its separate historical
branch also treated any resolved raw response as cancellation and started a
history refresh. Unlike current-job cancellation, it did not inspect identity,
status or progress. The initial two browser regression assertions failed; after
the controller repair both passed.

## Source and confirmation ownership

The selected-record controller requires a current verified collection source
before endpoint capture or HTTP work. Missing, stale and unreadable sources
block without writing or reading. Cancellation acts on the saved job identity,
not editor fields, so a dirty or mismatched draft cannot change its payload.
The effective cancellation/history URL and governed-header scope is captured
before the request. Old-source confirmations and failures cannot publish into
a new backend or start a history read there. Unreadable late-source inspection
also fails closed.

Current-job and selected-record cancellation share one receipt decoder. It
checks exact job identity, the status/progress/detail contract and `cancelled`
status. Invalid or unconfirmed receipts never start recovery reads. Transport
failure and timeout retain the existing catalog and release the write reservation
for an explicit retry, without replaying writes. A confirmed cancellation remains
confirmed when the following history read fails; the existing recovery alert
owns the stale-read diagnosis.

The workspace reserves the parent job ID before the React transition. Duplicate
selected cancellation and conflicting task/result Save/Delete callbacks share
that flight rather than queue another write, including while the confirmed
history read waits and after a sidebar remount. The actual GUI disables Cancel,
Save and Delete while the parent is reserved or the displayed source is stale.
Accepted cancellation captures feedback ownership before its queued request;
new messages, navigation or manual selection suppress obsolete feedback without
discarding a same-source confirmed read.

Historical cancellation no longer chooses the current-job mutation path merely
because the IDs match. It does not clear a live result, increment its observation
token or replace its job state. The live observer independently receives backend
status through polling. Cancelling another historical record therefore cannot
erase the current calculation preview. Source guidance now says to refresh before
changing the record, covering cancellation as well as Save/Delete in all 34
existing guidance locales.

## Executed verification

The complete frontend unit suite passed 1,446 tests with zero failures or skips
in 21.4 seconds. The 41 added controls include 34 source, receipt, failure,
feedback and read-recovery cases, plus seven primary-action controls covering
draft independence, live-state isolation, shared-parent write suppression and
accepted-intent feedback. Composition tests verify the cancellation transport
reaches the primary controller. Existing current-job cancellation controls remain.

The expanded source Chromium run passed 81 tests with zero failures or skips in
299.7 seconds, including eighteen selected-cancellation controls, fifteen source
controls, fourteen parent-flight controls, thirty-two history/current-cancellation
controls and two Data/Library lifecycle controls. After adding the two live
observation cases, the final selected-cancellation suite passed all twenty tests
with zero failures or skips in 72.6 seconds. These overlapping runs are not a
single 101-test coverage claim.

The browser suite activates real System/Data controls against synthetic task
services. It checks malformed/contradictory receipts, wrong identities, transport
failure, source changes, confirmed-read failures, duplicate cancellation,
task/result mutation exclusion, remount, late export feedback, backend changes
and live-observer ownership. Backend counters distinguish read recovery from
write replay; synthetic numerical responses are not scientific validation.

Final static checks passed: frontend type checking and sensitive-storage guard,
800-line frontend guard, repository source/document organization (800/2000 lines,
zero tracked debt), runtime API surface, module topology, module/function matrix,
module extension standard, coverage tensor and self-test, desktop validation
self-test, documentation inventory/book, version contracts, Rust formatting and
whitespace checks. The coverage tensor retains four maturity gaps, twenty
evidence-grade gaps and fifteen P0 gaps; overall Daji qualification remains
blocked. Process inspection found no matching test services after completion,
and the temporary-directory check found no Workbench UI fixture directories.

Run the scoped browser suite from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-admin-job-cancellation.test.mjs
```

## Qualification limits

Source ownership compares effective URLs and governed headers, not authenticated
server identity or signed provenance. The write reservation covers selected-record
cancellation and administrative Save/Delete in one workspace; it is not a
cross-client lock or a unified flight with the separate current-job PWDT cancel
operation. Current-job source admission, continuous authority-history fencing,
linked project/version navigation, ambiguous-write reconciliation and real
deployment qualification retain their own boundaries.

Engine/Agent execution, operators, headless SDKs, backend authorization, public
API schemas and persisted formats are unchanged. No snapshot, archive, backup,
retry queue or production Node dependency is added. This round does not bump a
version, commit, push, rebuild or install the app. Native test registration,
desktop validation and the coverage tensor retain the source-client evidence
grade rather than promoting operational acceptance.
