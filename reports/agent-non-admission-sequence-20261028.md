# Agent non-admission response sequence regression

Date: 2026-10-28. Source line: daji 3.7.1.

## Reproduced failures

The Orchestra transport validated individual RPC envelopes but did not remember
whether a terminal rejection followed progress or heartbeat on the same request.
Consequently, `agent_at_capacity` could still enter capacity retry after such
telemetry. The regression observed a managed TaskIR task execute on a second TCP
peer and return success instead of remaining uncertain, including under
`checkpoint_required`. The legacy solver path had the same issue.

Separately, a plain native `agent_lan_control_requires_explicit_trust` rejection
was treated as unknown by the managed dispatch journal, obscuring a definite
policy denial behind an execution-recovery error. In the initial working harness,
19 of 23 cases failed before production changes; these were behavioral assertions,
not build failures. The final suite was expanded to 33 cases.

## Changes

- The transport tracks one request-local boolean for received valid progress or
  heartbeat. Local queue/dispatch callbacks do not set this flag; no progress
  history is retained. Existing framing, deadlines, duplicate-key rejection and
  correlation checks remain in place.
- Only the exact native capacity and explicit-LAN-trust rejection codes qualify
  as non-admission. If either follows telemetry or carries an
  `operator_task_failure_receipt` field, transport reports a protocol conflict
  (`contradictory_non_admission`), not an ordinary RPC rejection.
- Legacy recovery gives that conflict the diagnostic code
  `agent_non_admission_conflict` and disallows automatic retry for every replay
  policy, including default idempotency and a verified-checkpoint hint. Normal
  transport failure policy outside this conflict is unchanged.
- Managed TaskIR preserves `outcome_unknown`, its original attempt identity and
  workflow replay fence. Journal reopen does not change the observation; healthy
  distinct work can proceed without overwriting the uncertain record.
- A plain consistent LAN denial is recorded as `not_dispatched`, with the original
  permission error returned. It does not automatically wait, retry or select a
  different Agent, and does not mark a reachable Agent unhealthy. Pure capacity
  rejection still permits existing bounded waiting/selection.
- Unknown errors cannot claim non-admission through their text or a generic
  `dispatched=false` detail. A late permission denial cannot overwrite an already
  sealed unknown observation; failed journal acknowledgement stays uncertain.
- The controlled test peer now supports explicitly emitting no progress. The old
  capacity-positive fixture incorrectly emitted progress before its supposed
  pre-execution rejection; it now models the native rejection order.

## Verification

From `apps/web`, default SQLite-backed tests ran with warnings treated as errors:

```sh
mix test --warnings-as-errors --seed 281029 \
  test/kyuubiki_web/playground \
  test/kyuubiki_web/orchestra/distributed_recovery_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery_probe_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_*test.exs \
  test/kyuubiki_web/orchestra/workflow_dispatch_*test.exs \
  test/kyuubiki_web/orchestra/workflow_native_dispatch_*test.exs
```

Result: 600 cases, zero failures, one backend-specific skipped case (599 passed).
An additional memory-backend run with seed 281030 passed 204 cases across the
new rejection suite, uncertain dispatch, capacity retry, dispatch observations,
recovery classification, journal, journal transitions and workflow dispatch
recovery. The earlier focused 114-case run also passed; these suites overlap and
must not be represented as distinct product coverage or a coverage percentage.

The new TCP tests cover both progress and heartbeat, both rejection codes,
managed and legacy paths, and idempotent/checkpointed/checkpoint-required policies.
They assert requests sent to each path, capacity/queue release, health accounting,
persisted attempt state, replay fences, later healthy tasks and journal reopen.
Test peers are local simulated RPC responders; the persistent journal is real and
isolated in temporary test storage.

## Scope and remaining work

This is local Orchestra TCP-response consistency and dispatch-journal evidence,
not installed GUI, real multi-host/native-Agent execution, authenticated identity,
PostgreSQL takeover, whole-host power loss or numerical qualification. A peer can
still lie consistently; validating response order is not cryptographic proof.
No engine/operator algorithm or task input format changed. Rust Headless SDK
service calls use these backend routes, but SDK-to-installed-service acceptance
was not rerun in this round. This patch fixes response handling, not pre-dispatch
selection. The subsequent [connection-preflight regression](orchestra-connection-preflight-20261028.md)
covers that separate path with local simulated TCP peers and a real journal.
No rebuild/install, remote deployment, commit or push.
