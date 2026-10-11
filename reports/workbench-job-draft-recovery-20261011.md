# Workbench job editing recovery

Workbench task-history refresh now preserves unsaved metadata for the selected
job. Each untouched field still follows verified backend values; changing the
selection loads that record instead of transferring the previous job's draft.
Read recovery and slow save responses cannot replace newer typing. This is source
client behavior tested with real Chromium and isolated mocked services, not
installed WebView or remote-runtime qualification.

## Reproduced overwrite

The initial 14 browser controls had one pass and 13 failures. Healthy refresh
replaced all four independently edited fields: message, project, model version
and simulation case. Recovery from HTTP 503, aborted network, malformed lists,
invalid records and duplicate identities did the same. A rejected save followed
by read recovery also erased editing. A slow confirmed save replaced newer input
on either its own job or a different selected job.

The passing baseline control confirmed a write can remain successful when the
follow-up read fails. That behavior remains intact. Browser assertions wait for
newly fetched records to appear before inspecting editing; network completion
alone is not treated as React state publication.

## Editing contract

The selected job retains one in-memory baseline containing four strings. Refresh
compares each working field against that field's previous verified value. A dirty
field keeps its working value; a clean field follows the incoming value. The
baseline advances even when editing stays dirty, so reverting to the latest value
makes subsequent refresh work normally.

Explicit record selection loads verified metadata for that record. Returning to
an earlier record loads its latest value, not an archived unsaved copy. An
authoritative empty history clears selection and orphan editing. Results use the
same small string reconciliation rule while retaining their existing selected
result baseline. There is no persistent draft cache or backup history.

A save uses the record and field snapshot submitted by the user. Later typing
remains local until another explicit save. A confirmed save followed by an
unavailable read stays successful; a rejected save retains editing and does not
trigger a follow-up read. GUI Retry All and PWDT `runtime/refreshAll` only read
data and never replay either write. After a successful read reconciles the saved
value, unchanged fields can follow later backend updates.

## Controls

Twenty added unit controls exercise dirty and clean metadata/result fields,
intentional local clearing, initial selection, record switching, null selection,
advancing baselines, explicit revert and typing after submission. The complete
frontend unit suite passed 1,123 cases with zero failures or skips in 17.8 seconds.
Type checking and the 800-line frontend guard also passed.

The first repaired browser run passed 32 cases with zero failures or skips in
140.0 seconds: 14 task-edit controls, 16 result-history controls and two existing
Data/Library write/export/delete controls. The final task suite has 18 controls:
four added cases cover all old/new successful/failed response combinations, and
the confirmed-save control also checks that reconciled saved fields become clean
again. Task editing: 18 passed, zero failures or skips. Together with 32 existing
task-history observation and 10 checkpoint-recovery controls, the expanded final
invocation passed 60 cases in 197.4 seconds. Across these two repaired invocations,
78 distinct browser controls passed on the final production source; 14 task cases
were exercised in both runs and are not counted twice.

Browser storage-security, Rust formatting, runtime API, topology, module/function
matrix, extension standard and desktop UI validation self-test passed. Documentation
inventory/book, project organization and packaged-version alignment also passed.
Tensor structure and self-test pass with four maturity, 20 evidence-grade and
15 P0 gaps; the overall Daji status remains blocked.

Mutation counters distinguish the explicit task save from automatic activity.
All read-only controls assert zero project, checkpoint and result writes and zero
workflow submissions. Successful save controls allow exactly one task metadata
write; a rejected write is intercepted and counted once, without a repeated write.

Run the task controls from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-job-draft-recovery.test.mjs
```

The native integration runner and desktop UI validation contract register this
suite. Cross-record draft retention, multi-user conflict resolution, ambiguous
write acknowledgments and backend-authority transitions remain separate work.
The tests do not use the installed application's database or package an app.
No production Node dependency, version change or Git commit is introduced.
After shutdown, checks found no retained Workbench test frontend directories or
matching test/frontend processes. No package, retained database or backup was
created by these controls.
