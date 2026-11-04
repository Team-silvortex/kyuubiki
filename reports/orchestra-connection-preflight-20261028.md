# Orchestra connection-permission preflight

Date: 2026-10-28. Source line: daji 3.7.1.

## Reproduced failures

Orchestra selected endpoints from registration/static routing and dispatched
without inspecting the current connection's control permission. The initial
two regression cases failed behaviorally: a discovery-only candidate received
the computation instead of the next eligible candidate, and a lone discovery-only
candidate returned success rather than a pre-dispatch rejection. The test peers
deliberately accept any computation sent to them so the test verifies controller
behavior independently of the native Agent's final gate.

After adding the probe, a further regression exposed local capacity-budget expiry
being misreported as `all_agents_failed` / `connection_preflight_timeout`. This
would incorrectly penalize peer health. It now returns `agent_capacity_timeout`
without another dispatch or a peer-failure increment.

## Contract

- New `solve_*` and execute-mode `run_operator_task_ir` dispatch to non-literal-
  loopback endpoints requires a fresh, read-only `describe_agent` observation.
  Hostnames, including `localhost`, do not bypass the probe. Strict IPv4/IPv6 and
  IPv4-mapped loopback literals preserve the existing local path.
- The probe uses a new random correlation ID and empty parameters, never task
  inputs, job IDs or claimed authority. Its deadline is at most 1500 ms, capped by
  the request timeout and remaining capacity-retry budget; the reply cap is 1 MiB.
  Existing transport framing/correlation/JSON checks remain active. Unexpected
  progress is rejected and is not forwarded to the task progress consumer.
- Only the exact three native `kyuubiki.agent-connection-access/v1` observations
  are recognized. Missing/unknown/inconsistent fields fail closed. Shared
  registration metadata cannot grant access. `authenticated` and `encrypted`
  remain false; a positive observation is not identity verification.
- The probe runs while a local capacity lease is reserved but before dispatch
  progress, registry ownership claim, task journal reservation or compute input
  transmission. Preflight rejection can safely select another candidate, and
  removes the rejected candidate from this call's later capacity retries.
  A valid discovery-only reply is reachable, not a peer outage.
- Cancellation is checked before and after probing, including failed probes.
  Controller authorization and capacity deadlines are checked before dispatch.
  Leases are released on success and failure. A new task probes current access
  again; there is no cached permission or added persistent snapshot.
- A real execution request changes the boundary: denied/unknown responses retain
  the existing non-admission and journal rules. Lost replies and forged remote
  RPC error codes cannot masquerade as local preflight failures and authorize
  cross-peer replay. Pinned inspection, cancellation and cleanup retain their
  original routing and still obey the target Agent's own gate.

## Verification

From `apps/web`, the focused suite passed 51 tests with warnings treated as errors:

```sh
mix test --warnings-as-errors --seed 281105 \
  test/kyuubiki_web/playground/agent_connection_preflight_test.exs \
  test/kyuubiki_web/playground/agent_connection_access_contract_test.exs \
  test/kyuubiki_web/playground/agent_capacity_retry_test.exs
```

The larger regression passed 622 cases, with zero failures and one backend-specific
skip (623 total):

```sh
mix test --warnings-as-errors --seed 281106 \
  test/kyuubiki_web/playground \
  test/kyuubiki_web/orchestra/distributed_recovery_test.exs \
  test/kyuubiki_web/orchestra/distributed_recovery_probe_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_*test.exs \
  test/kyuubiki_web/orchestra/workflow_dispatch_*test.exs \
  test/kyuubiki_web/orchestra/workflow_native_dispatch_*test.exs
```

The new tests cover descriptor fields, DNS/metadata bypass attempts, missing and
malformed replies, unexpected progress, wrong correlation, oversized replies,
disconnects, timeouts, cancellation during allowed/denied probes, stale controller
ownership, policy changes between tasks, capacity retry, capacity-budget expiry,
legacy dispatch and pinned cleanup. Assertions inspect wire requests, health,
local lease release and real dispatch-journal contents. Suites overlap; counts are
not a percentage of product coverage.

The additional memory-backend subset (seed 281107) passed 161 tests covering
preflight, descriptor contracts, capacity, non-admission, uncertain dispatch,
recovery classification and dispatch-journal transitions. This overlaps the
SQLite run. The subsequent
[TaskIR parameter-boundary regression](agent-taskir-parameter-boundary-20261104.md)
adds atom/string envelope consistency; the original probe tests alone did not
cover that encoding boundary.

## Scope

This is local Orchestra connection-selection and dispatch-journal evidence, not
native multi-host authentication, installed GUI/SDK acceptance, PostgreSQL takeover,
whole-host power-loss recovery or numerical qualification. Peers are simulated
local TCP responders; the journal uses isolated real temporary storage. The native
Agent gate has separately recorded tests, not an end-to-end native-Agent rerun in
this round. A consistently lying peer or a local proxy is not authenticated by a
descriptor. Permission can change between probe and execution sockets, and final
Agent-side admission remains authoritative.

No numerical engine/operator change, service restart, remote deployment, installer
rebuild, version bump, commit or push. Tests clean up their owned temporary data.
