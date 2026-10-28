# Workflow Terminal Diagnostic Isolation

Date: 2026-10-28. Source line: daji 3.7.1. Local development evidence.

## Reproduced Failure

Terminal recovery reconciliation read the final history entry with `List.last/1`
without checking the stored field's shape. Null, object, string, number and
boolean histories raised `FunctionClauseError` inside the recovery scan. The
storage outage boundary deliberately does not swallow programming exceptions,
so a malformed diagnostic could terminate the coordinator and recur at startup.

A second path passed oversized messages directly to the job progress contract.
The 4096-byte gate rejected the update, leaving the active job unreconciled.
Reason text could fit by itself but exceed the limit after adding its prefix.
Empty or whitespace-only messages also obscured the terminal failure reason.

The initial direct regression ran 35 cases before the production fix: 19 failed.
These are failing inputs across related boundaries, not 19 independent defects.
Four coordinator-level cases were excluded from that red run to avoid repeatedly
restarting the application's supervisor against intentionally malformed data.

## Changed Boundary

`WorkflowRecoveryScan` now reads the last entry only when history is a list.
For failed or recovery-blocked records it selects a nonblank, valid UTF-8 message,
then a usable reason from that same final entry, then the existing state-specific
fallback. It does not search older entries for a potentially stale explanation.

The final prefixed text must fit `Jobs.ProgressEvent.message_max_bytes/0`, sharing
the existing 4096-byte progress limit rather than introducing a second limit.
Exact-limit text is preserved; oversized text is not truncated or written back.
The raw runtime, recovery envelope, history and cancellation evidence remain
unchanged. Completed and cancelled reconciliation retain their existing behavior.
No schema migration, additional backup, broad exception rescue, solver replay or
new cancellation authority is introduced.

## Regression Coverage

`apps/web/test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs`
contains 46 cases:

- Missing, null, wrong-type, empty and malformed-last-entry histories in both
  failed and recovery-blocked records.
- Empty, blank and oversized messages/reasons, including UTF-8 byte limits and
  the reason prefix crossing the final message limit.
- Message precedence, same-entry reason fallback and exact-limit preservation.
- Completed/cancelled controls independent of diagnostic history.
- Manual scans and coordinator restarts with a malformed or oversized record
  ordered before a healthy sibling. Both jobs reconcile, no runner is tracked,
  raw runtime records stay unchanged and the next scan sees no active jobs.

The initial focused SQLite pass, before the seven extra text-boundary cases,
passed 68 tests including storage outages, restart recovery, envelope validation
and the progress-event contract (seed 281902).

Final verification:

| Run | Result |
| --- | --- |
| Complete Web suite, disposable SQLite, seed 281903 | 4023 tests, 0 failures, 77 existing skips; 211.9 seconds |
| Focused selection, isolated memory/JSON, seed 281904 | 75 tests, 0 failures; 2.1 seconds |
| Elixir formatting and diff whitespace | Passed |
| Documentation book and inventory | Passed |
| Project organization and tensor checks, including self-tests | Passed |

The 46 added cases ran in both final selections. The memory/JSON directory was
removed after its test process exited; no test store or backup is retained.
The tensor claim is limited to local contract/recovery evidence. Overall tensor
status remains blocked: 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

To run the focused selection from `apps/web`:

```text
mix test --warnings-as-errors --seed 281902 \
  test/kyuubiki_web/orchestra/workflow_terminal_reconciliation_test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs \
  test/kyuubiki_web/orchestra/workflow_restart_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_recovery_envelope_test.exs \
  test/kyuubiki_web/jobs/progress_event_test.exs
```

Tests reset their stores. Use the test harness's disposable SQLite database or a
fresh disposable `KYUUBIKI_DATA_DIR` with `KYUUBIKI_STORAGE_BACKEND=memory`, never
installed runtime or research data. Remove owned temporary stores after exit.

## Scope

This is local terminal-diagnostic isolation evidence, not PostgreSQL, distributed
failover, power-loss, concurrent terminal-write atomicity or numerical qualification.
The malformed record is deliberately retained, not repaired or authenticated.
This does not change which terminal records the existing coordinator trusts, or
guarantee recovery completion for unrelated storage/write failures.
