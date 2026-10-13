# Native workflow failure finalization

Date: 2026-10-13. Scope: ordinary coordinator failure and workflow runner deadlines.

Follow-up: the separate periodic watchdog gap recorded below is now addressed by
the [periodic finalization report](./workflow-watchdog-finalization-20261013.md).
The results and remaining-limit list below describe this earlier checkpoint;
the new follow-up does not retrospectively qualify physical cancellation or
identify the earlier intermittent lease-admission failure.

## Reproduced gap

The workflow runner deadline killed its local execution task and called ordinary
failure finalization. An Agent could already have accepted the native task, with
its journal still at `dispatch_boundary_unconfirmed`. Ordinary failure pruned the
parent envelope and private native evidence without checking that observation.
This bypassed the managed live-RPC uncertainty guard because the task carrying
that guard was killed before returning.

The controlled-TCP pipeline regression failed before the fix (seed 68166):
9 tests, 1 failure. The deadline produced `failed` recovery instead of preserving
`recovery_blocked`, even though the fixture had received the native request.

## Implementation

`WorkflowRecoveryFailure` now verifies the recovery envelope and the shared native
dispatch fence before ordinary string-error cleanup. Unconfirmed dispatches,
unavailable/missing/corrupt journal history or invalid evidence retain the private
recovery record as blocked. The original failure message stays on the job and in
recovery history; a bounded reason and inspection action explain the block.

The existing lease, execution claim and atomic job/result write remain mandatory.
Rejected progress, invalid failure text and stale claims cannot partially publish
a blocked record or erase evidence. This is a read-only dispatch verification,
not a new claim, replay authorization, journal rewrite or receipt promotion.
Verified executed/failed/blocked observations and confirmed non-dispatched work
still allow ordinary failure cleanup. Local-only graphs do not access the journal.

Before stopping a deadline-expired local task, the runner uses the existing
dispatch gate to capture its actual endpoints, then makes a best-effort targeted
cancellation request. A cancellation acknowledgement is not proof of termination.
It does not convert the original dispatch observation into success or authorize
another execution. A late reply cannot publish a result into the blocked parent.

No new storage bucket, backup, runtime dependency or format migration is added.
The engine/operator interface is unchanged. Ordinary failure cleanup adds one
bounded replay-fence verification for native graphs, not per-progress scanning.

## Verification

Added 20 regressions: 19 focused failure-retention tests and one actual
coordinator/TCP deadline pipeline. Coverage includes both unresolved journal
states, four confirmed terminal states, unavailable/missing/empty/corrupt journal
storage, invalid or absent private evidence, a damaged envelope, local-only work,
native graphs before dispatch, stale claims, invalid messages and rejected job
progress. Restarting the coordinator and journal retains blocked evidence without
changing the attempt or sending another execution request.

The deadline pipeline holds an accepted request past the runner timeout, checks
the exact targeted cancellation, returns a late frame and restarts the recovery
components. The original boundary observation and evidence remain intact; no
output or additional execution is published, and the healthy Agent is not penalized.

Focused SQLite run (seed 68168): **93 total, 84 passed, 9 conditional skips,
zero failures**, warnings denied.
Focused durable-memory run (seed 68170): **93 total, 72 passed, 21 conditional
skips, zero failures**, warnings denied, in an isolated temporary data directory.

The first expanded SQLite run (seed 68169, max_cases 16) found **10 failures among
3,170 tests, with 73 conditional skips**. These were lease-lost/standby admission
errors in `ProjectWorkflowContextApiTest` and `WorkflowAsyncBridgeApiTest`, not
failed assertions in the new deadline/retention cases. Two diagnostic runs using
`--trace` passed all non-skipped cases, but trace mode forced `max_cases: 1` even
when a larger value was requested. These serial passes do not establish the cause
or resolution of the original lease failures. Keep this timing-sensitive admission
instability open rather than masking it with retries or longer lease settings.

The final non-trace SQLite run, again seed 68169 with `max_cases: 16`, passed:
**3,170 total, 3,097 passed, 73 conditional skips, zero failures**, 57.4 seconds,
warnings denied. A passing rerun is not treated as a fix for the initial lease
instability. Formatting, whitespace, tensor validation/self-test, the 26-file HTML
book, documentation inventory and project organization checks pass. Source files
stay below 800 lines and documents below 2,000, with zero tracked organization debt.
The tensor remains at zero structural gaps, four maturity gaps, 20 evidence-grade
gaps and 15 P0 gaps; its release gate remains blocked.

Reproduce focused coverage from `apps/web`:

```text
mix test test/kyuubiki_web/orchestra/workflow_native_failure_retention_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_dispatch_pipeline_test.exs \
  test/kyuubiki_web/orchestra/workflow_commit*test.exs \
  test/kyuubiki_web/orchestra/workflow_storage_outage_test.exs \
  test/kyuubiki_web/playground/agent_job_cancellation_test.exs \
  --seed 68168 --warnings-as-errors
```

The memory variant sets `KYUUBIKI_STORAGE_BACKEND=memory` and an isolated
`KYUUBIKI_DATA_DIR`. Expanded coverage adds `workflow_*test.exs`, the API directory,
Orchestra workflow/operator-task/dispatch suites, the headless native recovery
gate, distributed recovery, Playground, Library, Storage and Jobs directories.

## Remaining limits

- Separate periodic watchdog reconciliation remains open. `Jobs.Watchdog` writes
  job failure through its own snapshot-checked path, not this coordinator failure
  finalizer; matching recovery state and execution shutdown need their own proof.
- Cancellation here is best effort, not a durable cancellation receipt or proof
  that a remote computation has stopped. Explicit user cancellation is unchanged.
- The producer-completed/before-receipt gap still blocks rather than recovering
  a missing result. No exactly-once or atomic Agent-to-database guarantee is claimed.
- Controlled local TCP and recovery-component restarts are not real Rust Agent,
  remote Linux, installed desktop, live PostgreSQL, physical power-loss or numerical
  accuracy qualification. No module maturity or release readiness is promoted.
