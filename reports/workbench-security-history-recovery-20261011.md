# Workbench security history recovery

Workbench security-event history now distinguishes a pending read, a failed read
and an authoritative empty response. A failed refresh preserves the last validated
records for the same filters. A filter change clears those records and invalidates
older requests before paint, so another scope cannot masquerade as current history.
Successful explicit refresh replaces the list and clears its recovery issue.

These guarantees cover the source Workbench client and its browser control chain.
The tests use real Chromium and the real React application with isolated mocked
services; they do not qualify an installed WebView, remote runtime or solver.

## Reproduced failures

Before the fix, 19 malformed-success unit controls failed: invalid envelopes,
event arrays, records, identities, required strings, timestamps, contexts and
notes passed through the client service unchecked. A real-browser 503 control
also reproduced loss of the previously visible event list.

The client service validates the entire response before publication. Duplicate
identities or a malformed entry reject the whole history instead of silently
dropping records. Nonempty custom source, risk and status values remain supported;
the validator does not turn current UI labels into a closed operator contract.
Valid payloads retain their original references, without deep copies or backups.

## Refresh ownership

Every refresh retains its sequence until completion. Only the latest request in
the current filter scope may publish records, set loading state, or change recovery
feedback. Scope changes and unmount invalidate the sequence. Same-scope failure
preserves known records; changing filters deliberately clears the previous scope.
No read failure automatically retries a request, repeats a write or submits work.

The audit panel shows loading and failure feedback next to its refresh action.
It marks pending reads with `aria-busy`, suppresses false empty-state messages and
hides zero-data summaries/facets while the read is pending or unavailable. Existing
localized busy/failure labels remain in use. The compact global recovery alert and
the read-only PWDT `runtime/refreshAll` path remain available.

## Executed controls

Twenty-one added unit controls cover 19 malformed-success responses, valid empty
and extensible payloads, unchanged record ownership, and propagation of the original
unavailable error without retries. The two original service controls also pass.

Fourteen added browser controls cover HTTP 503, aborted network requests, malformed
envelopes, null contexts and duplicate identities; preserved records followed by
successful replacement and genuine empty history; first-read failure; pending
filters; failed filter changes; delayed successes/failures after navigation; and
all four old/new success/failure combinations during overlapping refreshes.
The controls assert no checkpoint/project writes or workflow submissions, and
exactly one read per explicit refresh in the failure and concurrency cases.

Final verification:

- Frontend unit suite: 1,075 passed, zero failures or skips, 31.5 seconds.
- Sequential checkpoint, task-history and security-history browser suites:
  56 passed, zero failures or skips, 206.4 seconds. This includes all 14 new cases.
- Type checking, the 800-line frontend guard, browser storage-security guard,
  Rust formatting and whitespace checks passed.
- The generic layout guard passed on an isolated source frontend in six viewport
  sizes, from 390 by 844 to 1440 by 900, including workflow-benchmark interactions.
  This is separate from audit failure-state behavior at 1440 by 1000. An initial
  unprivileged browser launch failed before auditing; the authorized rerun passed.
- Runtime API, topology, module/function matrix, extension standard, desktop UI
  validation self-test, documentation inventory/book, project organization and
  version alignment checks passed. The tensor and its self-test pass structurally
  with four maturity, 20 evidence-grade and 15 P0 gaps; overall Daji remains blocked.

The first browser sweep exposed a test-harness error: an aborted network request
has no HTTP response to wait for. A later sweep also read an empty-list count before
React published the target state. The final controls wait for requests and target
UI states rather than increasing arbitrary sleeps or masking failed assertions.

Run from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-checkpoint-recovery.test.mjs tests/integration/workbench-ui-history-observation.test.mjs tests/integration/workbench-ui-security-history-recovery.test.mjs
```

The Rust integration runner and desktop UI validation contract both include the
new browser suite. Its scoped tensor evidence is verified, not installed-runtime
or operational qualification. Development tests use disposable local services and
temporary frontend copies, not user databases. No persistent history cache,
automatic replay, Node backend dependency, app package or version change is added.
All test frontend copies were removed after shutdown; the temporary directory
search returned no retained Workbench UI workspaces.
