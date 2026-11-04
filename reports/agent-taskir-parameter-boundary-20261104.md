# TaskIR control-envelope encoding boundary

Date: 2026-11-04. Source line: daji 3.7.1.

## Reproduction

The generic Elixir Agent client accepted map parameters. JSON encoding converts
atom keys and ordinary atom values to strings, but permission probing and dispatch
journaling inspected string-keyed parameters with the exact string `execute`.
An atom mode key, atom mode value or atom envelope could therefore reach an
execution-capable TCP responder without the expected probe and journal path.
The typed high-level builder already emitted string parameters; this defect was
in the generic entry point, not evidence that every normal workflow bypassed it.

Four new integration cases failed before production changes: three parameter
variants selected a discovery-only responder instead of the permitted responder,
and an ambiguous atom/string envelope was sent rather than rejected locally.
These were behavioral failures, not build errors. Responders intentionally accept
computations to expose controller behavior independently of the native final gate.

## Fix

- `AgentRpcParams` canonicalizes outer TaskIR parameter keys and ordinary atom
  mode values before digest capability routing, job metadata, permission preflight,
  journaling and final JSON encoding. Nil/boolean modes retain their JSON values.
  No new atoms are created from caller input.
- Atom/string name collisions fail locally as `request_encoding_failed`, even
  for identical values or extension fields. Diagnostics are fixed messages and
  do not include caller values. Invalid key types and parameter structs fail
  before routing, rather than relying on custom JSON encoder behavior.
- Only the outer parameter map is rebuilt. Embedded task/model data is not
  traversed or rewritten; task digest rules, task formats, numerical algorithms
  and legacy solver/recovery envelope representations remain unchanged.
- All downstream control decisions use that same normalized envelope. Atom task
  keys retain typed digest capability requirements. Lost execution replies retain
  an unknown journal observation and do not reselect a peer, even with an
  idempotency hint. Key conflicts open no socket and reserve no journal attempt.

## Verification

The new focused cases include direct parameter contracts and integration checks
with controlled local TCP responders and an isolated real dispatch journal.
The initial focused rerun passed 27 tests before adding the five parameter unit
tests and two capability/unknown-outcome regressions.

Memory-backend cross-check: 172 tests passed, zero failures (seed 41104). From
`apps/web`:

```sh
env KYUUBIKI_STORAGE_BACKEND=memory mix test --warnings-as-errors --seed 41104 \
  test/kyuubiki_web/playground/agent_rpc_params_test.exs \
  test/kyuubiki_web/playground/agent_connection_preflight_test.exs \
  test/kyuubiki_web/playground/agent_connection_access_contract_test.exs \
  test/kyuubiki_web/playground/agent_capacity_retry_test.exs \
  test/kyuubiki_web/playground/agent_pre_dispatch_rejection_test.exs \
  test/kyuubiki_web/playground/agent_task_uncertain_dispatch_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_journal_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_journal_transitions_test.exs
```

The earlier expanded SQLite run passed 631 cases with one backend-specific skip
(632 total, zero failures, seed 41103), before the final two integration cases.
The final expanded SQLite run passed 633 cases with one backend-specific skip
(634 total, zero failures, seed 41105):

```sh
mix test --warnings-as-errors --seed 41105 \
  test/kyuubiki_web/playground \
  test/kyuubiki_web/orchestra/distributed_recovery_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery_probe_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_*test.exs \
  test/kyuubiki_web/orchestra/workflow_dispatch_*test.exs \
  test/kyuubiki_web/orchestra/workflow_native_dispatch_*test.exs
```

These runs overlap; they are not independent product coverage or a percentage.
Scoped formatting and `git diff --check` passed. The coverage-tensor check passed
structural/command validation with zero structural gaps. It still reports two
maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; overall daji status remains
blocked. The new verified local claim does not close qualification obligations.

## Scope

This is local TaskIR control-envelope and dispatch-journal evidence, not
authenticated multi-host Mesh recovery, installed GUI/SDK acceptance, PostgreSQL
takeover, host-power-loss or numerical qualification. The native Agent was not
rerun end-to-end in this round. Its final connection gate remains required;
descriptor observations are not identity verification. No recursive JSON
canonicalizer or general-purpose task-data migration is introduced.

Existing uncommitted work is retained. No deployment, installation, version bump,
commit or push. Temporary data belongs to the tests and is cleaned by their
existing fixtures.
