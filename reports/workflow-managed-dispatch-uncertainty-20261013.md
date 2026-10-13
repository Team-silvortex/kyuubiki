# Managed native dispatch uncertainty propagation

Date: 2026-10-13. Scope: managed native workflow live dispatch and recovery.

## Reproduced gap

The dispatch journal correctly sealed `outcome_unknown` after an unverified
Agent outcome, but a successful journal write returned the original RPC result.
This could return an unverified success, expose a skippable application error or
enter transport failover despite the managed parent's retained attempt evidence.

Six red coordinator/TCP regressions (seed 68158) covered disconnect, invalid
success receipt and generic RPC error, with and without `on_error: skip`.
All six failed: invalid receipts/errors produced ordinary failed/completed parents;
disconnects entered the alternate-Agent path rather than blocking promptly.

## Fix and scope

For dispatches using the trusted `on_dispatch_reserved` callback, an unknown
observation now returns an internal tagged uncertainty. AgentClient accounts for
transport/protocol health separately, then returns the existing
`operator_task_dispatch_outcome_unknown` error without peer replay or another
progress callback that could overwrite it. The native workflow runtime maps this
to `native_workflow_recovery_dispatch_unconfirmed` and retains recovery evidence.
Concurrent cancellation preserves tagged/bare uncertainty and evidence-commit
failure instead of flattening them into ordinary cancellation.

This is deliberately scoped to managed evidence handoff, not a global TaskIR
retry-policy change. Standalone calls without that callback retain their existing
idempotent/checkpoint-required/checkpointed policies and API error contracts.
Raw solver RPC behavior is unchanged. Independent tasks remain schedulable.
Connection failure and native capacity rejection before execution remain
`not_dispatched`, and can select a different Agent. Verified executed, blocked and
failed receipts retain their existing meanings. Invalid receipts affect protocol
health; plain application errors and local progress callback faults do not.

No new journal format, result cache, retention extension, runtime dependency or
backup archive is introduced. No extra journal scan is added to dispatch.

## Verification

Added 40 regressions: six actual coordinator/TCP pipelines and 34 dispatch-policy
cases. The six pipelines also restart the journal/coordinator and verify the
original generation and attempt evidence remain blocked without peer replay.
The policy suite crosses nine faults with three retry policies, including timeout,
malformed success/failure and callback return/raise/throw/exit; controls cover
pre-execution failover, verified receipts, cancellation and unrelated work.

Existing thermal catalog API tests now require `recovery_blocked` for stale
diagnostic identity, then explicitly re-register the repaired test Agent and
submit a fresh job. They do not resume the uncertain old parent. Existing
standalone distributed-recovery probes continue to exercise their original
checkpoint-authorized replay semantics unchanged.

Focused SQLite run (seed 68163): 66 tests passed, no failures, warnings denied.
Focused durable-memory run (seed 68164): 66 tests passed, no failures, warnings
denied. It used an isolated temporary data directory, removed after the run.
Final expanded SQLite run (seed 68165): **3,054 total, 2,983 passed, 71 conditional
skips, zero failures**, warnings denied. It includes workflow, API, native/batch
recovery, dispatch, distributed recovery, Agent routing, library and storage suites.

The first expanded run exposed existing standalone replay/API contracts that the
initial global guard would change. The final guard is limited to managed evidence
callbacks; the original standalone probes and API assertions remain unchanged.
Thermal catalog tests now assert retained blocked evidence, not ordinary failure.

Formatting, whitespace, tensor validation/self-test, the 26-file HTML book,
documentation inventory and project organization checks passed. The 800-line
source and 2,000-line document limits retain zero tracked debt. Tensor maturity
is not promoted: zero structural gaps, four maturity gaps, 20 evidence-grade gaps,
15 P0 gaps, release gate still blocked.

Reproduction from `apps/web`:

```text
mix test test/kyuubiki_web/orchestra/workflow_native_dispatch_pipeline_test.exs \
  test/kyuubiki_web/playground/agent_task_uncertain_dispatch_test.exs \
  test/kyuubiki_web/playground/agent_job_cancellation_test.exs \
  test/kyuubiki_web/api/workflow_catalog_thermal_job_api_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery*test.exs \
  --seed 68163 --warnings-as-errors

mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery*test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage --seed 68165 --warnings-as-errors
```

The memory variant of the focused command sets `KYUUBIKI_STORAGE_BACKEND=memory`
and an isolated `KYUUBIKI_DATA_DIR`; no installed services or user studies are used.

## Remaining limits

This is controlled local TCP with real service coordination/storage, not a new
Rust Agent execution, remote Linux, live PostgreSQL, installed desktop, numerical
accuracy or physical power-loss qualification. The producer-completed/before-receipt
handoff availability window remains open: unknown execution blocks safely, but
does not magically recover a lost result or provide exactly-once execution.
The existing standalone replay-policy qualification is not expanded to mean
automatic continuation of managed native graphs.
