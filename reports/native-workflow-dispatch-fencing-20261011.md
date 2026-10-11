# Native Workflow Dispatch Admission Fencing

Date: 2026-10-11. Source macOS ARM64 verification at commit `a20f6c8a`
(`daji 3.5.5`) with this working-tree overlay; package metadata remains `3.5.0`.
This extends [queue cancellation](native-workflow-queue-fencing-20261011.md)
to the handoff before capacity acquisition and the final dispatch admission check.
It preserves existing Engine physics, Rust Agent execution and in-flight receipt draining.

## Reproduced Admission Gaps

Cancelled workflows deliberately drain operator activity so an already-running
dispatch owner remains available for targeted cancellation and its terminal
receipt. However, queue and reservation callbacks also use that activity path.
If cancellation happened after the last completed-node fence but before capacity
acquisition, a later reservation could still enter the operator client.

The workflow facade now supplies the existing `before_dispatch` hook for typed
solves and the bounded native diagnostic/scoring whitelist. An internal
`operator_dispatch` event checks the Orchestra lease, active job and current
execution generation/attempt. Unlike operator activity, it is never drained
after cancellation and never writes progress or renews liveness. Legacy
one-arity solve clients also execute the guard before invocation. Invalid guard
arity cannot silently disable the fence; functions remain outside TaskIR JSON.

The initial 48-test focused set produced 15 failures before wiring: three
controlled pre-acquisition cancellations still entered their clients, seven
new admission checks were unrecognized, and five adapter/API guard cases failed.
The reservation cases were already protected by the execution gate. A preliminary
fixture correction distinguished reserved cancellation from actual dispatched
Agent targets; reserved work correctly has zero remote cancellation targets.

Actual TCP regression then exposed a second omission: the TaskIR Agent client
filtered `before_dispatch` out of its control options. All sixteen new native
diagnostic/scoring admission-failure cases failed despite the adapter tests
passing. Adding the hook to that existing allowlist closes the real client path;
it does not add a function to wire payloads or change task digests.

## Controlled Boundary Coverage

Forty-eight new regressions cover the following source contract/recovery cases:

- Seven coordinator cases check unchanged healthy state, completed/failed/cancelled
  jobs, retired envelopes, stale attempts/generations and revoked Orchestra leases.
  Healthy admission does not change progress or the activity-throttle cache.
- Five adapter cases check eleven native built-ins, typed solve forwarding,
  legacy client rejection and malformed guards. Serialized TaskIR identity and
  integrity remain unchanged by control callbacks.
- Twelve runner cases hold typed bar solve, native transport diagnosis or native
  transport scoring immediately before acquisition or before dispatch. Both
  default and explicit `on_error: skip` policies use public job cancellation.
  Pre-acquisition receipts have zero active targets; reservations report one
  local cancellation and no remote registration. Resuming cannot start the
  operator, publish output or modify retained cancelled data; leases are released.
- Twenty-four actual-client TCP cases cover the same three routes, returned
  errors, throws, exceptions and exits, both initially and after capacity rejection.
  Rejection before admission creates no TCP request or native journal attempt.
  Retrying capacity performs a second admission check and cannot reach a peer.
  A successful first callback returning a `send` result is normalized to `:ok`.
  Local guard failures release capacity and do not penalize either Agent's health.

The TCP capacity fixtures do not execute physics. In the twelve retry rounds,
exactly twelve requests are rejected as `agent_at_capacity` before execution.
Only eight native rounds create an initial `not_dispatched` journal observation;
the guard failure never creates a second attempt. The other twelve rounds admit
zero requests. These counts describe requests and observations, not Rust Agent
execution counts or independent numerical evidence.

## Verification

The enlarged adapter, coordinator, runner, watchdog, cancellation, recovery,
TaskIR, dispatch-journal, TCP transport and graph API selection passed all 309
Elixir tests in 21.2 seconds. Focused reruns are subsets, not additional coverage.

The complete native Headless live suite passed all 67 tests in 157.44 seconds,
including the five-domain active and queued cancellation/recovery chains. CLI
and Agent unit tests passed 189 cases; strict all-target CLI Clippy passed with
warnings denied. Rust and Elixir formatting and diff whitespace checks passed.

Native topology, matrix, extension-standard, runtime API surface, tensor/self-test,
documentation inventory/book, version and organization checks passed. The version
audit checked 224 exact contracts with zero mismatches. Source/document limits
remain 800/2000 lines with zero tracked size debt. The tensor still reports zero
structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
Daji acceptance remains blocked. No maturity grade was promoted by these tests.

## Remaining Boundaries

The new deterministic admission tests use controlled clients and loopback TCP
capacity responses, not an installed application, authenticated remote cluster
or a numerical benchmark. The existing native Headless service tests separately
exercise actual Rust Agent diagnostic, scoring, cancellation and explicit recovery.
The new fence applies to typed solves and the bounded native whitelist, not every
local transform, plugin or arbitrary parallel graph. It does not establish
exactly-once execution, durable continuation or exhaustive scheduler interleavings.
Ownership checks cannot retroactively revoke an already-accepted computation;
existing targeted cancellation, receipt recovery and deadlines remain responsible
for that path. No release version, installed baseline or qualification promotion
is implied, and Git submission remains caller-controlled.
