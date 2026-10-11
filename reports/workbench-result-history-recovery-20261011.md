# Workbench result history recovery

Workbench result history distinguishes pending, unavailable and authoritative
empty reads. Failed refresh retains verified records, the current selection and
editing. Successful read recovery preserves unsaved drafts for the same result;
explicit record selection loads its latest verified contents. Results refresh,
runtime Retry All and PWDT read recovery never replay writes or computations.

The scope is the source Workbench client and its real browser control chain with
isolated mocked services. This does not qualify installed WebViews, remote runtime
outages, solver accuracy, large result histories or authenticated provenance.

## Reproduced failures

The initial service/controller selection had 28 cases: five passed and 23 failed.
Sixteen malformed-success controls exposed unchecked envelopes, arrays, identities,
result objects, timestamps and duplicates. Controller controls reproduced loss of
records/selection, missing recovery feedback and loading ownership, and deletion
clearing unrelated results after a failed follow-up read.

A later browser sweep also reproduced five failures after read recovery: the
history became available, but each successful refresh silently replaced the
unsaved result draft. Those controls now assert preservation after the newly
fetched records are actually visible, rather than treating network completion as
React state publication. An initial test used the wrong recovery subpage and an
incorrect success-message word; those test expectations were corrected without
weakening application-state assertions or adding sleeps.

## Result publication and editing

The client service validates the whole list before publication. Duplicate ids or
a malformed record reject the response instead of publishing a partial list.
Valid empty objects, custom result fields and missing optional timestamps remain
supported. Valid payloads keep their original references; result fields are not
copied or traversed as a physical-solver schema.

Only the latest refresh can publish records, finish loading or change the `results`
recovery issue. Older successes/failures cannot erase newer data or warnings, and
clearing a result warning never clears another backend channel. Failed reads keep
the previous list and selection. The local panel exposes loading/error feedback,
`aria-busy` and a refresh button; it suppresses false empty states and current count
claims until the read succeeds.

The editing state retains one in-memory baseline for the selected result. Clean
drafts follow refreshed data; dirty same-record drafts stay editable. Explicit
selection changes load that record's verified contents, and confirmed deletion
clears the deleted selection. No persistent draft cache, history files or backups
are added. These controls do not introduce cross-record draft retention or a
multi-user edit conflict resolver.

Confirmed save/delete and follow-up reads have distinct feedback. A confirmed
save remains successful when list refresh fails; a confirmed delete removes only
its local record and leaves other verified records intact. Rejected writes retain
the draft and selection and do not pretend to refresh a committed change. Explicit
read recovery neither repeats a rejected write nor duplicates a committed one.
Automatic retry and ambiguous write-response reconciliation remain separate.

## Executed controls

Twenty-eight added unit controls cover 16 malformed histories, valid unchanged
payload ownership, transport failure identity, preserved failed reads, all four
overlapping old/new success/failure combinations, stale completions while a newer
read is pending, unrelated warnings, and confirmed save/delete followed by failed
reads. The three original service controls remain intact.

Sixteen browser controls use actual Results and Retry All buttons and the PWDT
`runtime/refreshAll` path. They cover HTTP 503, aborted network, invalid envelopes,
invalid records, duplicate ids, first-read failure, pending first read, read
recovery, dirty/clean drafts, explicit selection, four response-order combinations,
confirmed writes with failed follow-up reads and rejected writes. Mutation counters
assert no project/checkpoint/job writes or workflow submissions; only the two
explicit successful save/delete controls allow their single result write.

Verification on the final source:

- Full frontend unit suite: 1,103 passed, zero failures or skips, 24.2 seconds.
- Result-history browser suite: 16 passed, zero failures or skips, 59.8 seconds.
- Sequential checkpoint, task-history, security-history and Data/Library browser
  suites: 58 passed, zero failures or skips, 233.9 seconds. Together the two final
  invocations cover 74 browser cases on the final source.
- Type checking, the 800-line frontend guard, browser storage-security guard,
  Rust formatting and whitespace checks passed.
- Runtime API, topology, module/function matrix, extension standard, desktop UI
  validation self-test, documentation inventory/book, project organization and
  version alignment checks passed. Tensor structure and self-test pass with four
  maturity, 20 evidence-grade and 15 P0 gaps; overall Daji remains blocked.

Run the result controls from the repository root:

```text
node --test --test-concurrency=1 tests/integration/workbench-ui-result-history-recovery.test.mjs
```

The native integration runner and desktop UI validation contract include this
suite. Its tensor claim is scoped verified client recovery, not operational or
installed-runtime qualification. Test services and frontend copies are disposable;
tests do not use the installed application's database or create app packages.
No production Node runtime dependency, version change, Git commit or push is added.
After final shutdown, the temporary-directory search found no retained Workbench
UI copies, and the process check found no matching test/frontend processes.
