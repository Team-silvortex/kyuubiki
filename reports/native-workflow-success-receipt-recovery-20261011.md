# Native Workflow Successful Receipt Recovery

Date: 2026-10-11. Local macOS ARM64 source verification at `a20f6c8a`
(`daji 3.5.5`) with the working-tree overlay. Package metadata remains `3.5.0`.
This extends the [ambiguous dispatch fence](native-workflow-dispatch-recovery-20261011.md)
without granting replay authority to missing or unknown original outcomes.

## Recovery Behavior

Previously an authorized idempotent/checkpointed graph recovery started every
node again. Progress events do not retain node outputs. Even a successful native
diagnosis with an observed journal entry and a retained original Agent receipt
was executed a second time before the remaining scoring nodes could run.

The background runner now opts into bounded native receipt reuse only in a
later execution generation. This runtime option is not serialized into TaskIR
and does not change its digest, graph policy or executable protocol. Initial
generation, direct graph runs and explicit new job submissions retain normal
execution behavior. Engine physics remains independent of recovery policy.

At each supported native node, its TaskIR is rebuilt from the current graph
node, upstream input, configuration, authority context and placement hints.
A read-only journal lookup examines at most the existing 512 in-memory records
for the exact child task. A successful record can be selected only with the
actual v2 RPC parent, task digest, operator and execution-program identity.
Legacy/unbound records cannot authorize attachment. An unbound ambiguous exact
child can deny it; an explicit foreign parent cannot supply its result.

Any retained ambiguous attempt dominates a newer success. A different digest
for the same owned child blocks recovery instead of accepting changed input or
silently recalculating it. The selected success is retrieved only from its
original configured endpoint, with the existing attempt/request/process/
generation/retention-policy and completion gates. Its receipt is additionally
checked against the freshly rebuilt TaskIR summary before output publication.
There is no peer probe or fallback to another historical success.

Job, claim and Orchestra ownership are checked before retrieval and after a
valid receipt arrives. The existing node-output JSON budget, graph publication
and atomic final-result commit still apply. Receipt reads do not update the
journal, execution count or retention age. There is no duplicate input/output
archive, node snapshot cache, database migration or increased disk budget.

If a successful observation exists but its receipt is missing, expired,
oversized, contradictory, unreachable or lost on Agent restart, recovery ends
in `recovery_blocked` with its execution envelope retained. The bounded reason
contains no response payload or credential. The next action is
`restore_original_native_receipt_or_explicit_resubmit`. This block bypasses
`on_error: skip`: missing recovery evidence cannot become partial success.
An unexecuted node can still execute under the original authorized graph policy.

Failure transitions are extracted from the near-limit coordinator into a
focused helper. Both ordinary failure and native recovery blocks keep the
lease/active-job/execution-claim gates and shared job/result transaction.
Stale claims, terminal jobs, lease takeover, rejected writes and persistence
faults cannot partially retire an envelope or relabel another execution.

## Real Process Interruption

Five rounds cover thermal, electrostatic, magnetostatic, thermomechanical and
Stokes diagnostic/scoring chains using two owned Rust Agents and a real BEAM
process. A test-only admission gate pauses before the next native score reaches
journal reservation or RPC dispatch. It forwards actual Agent calls and never
fabricates a scientific receipt or output.

Each graph first executes its diagnosis, reaches the gate, and has exactly one
`observed_executed` record. The test kills BEAM while the parent is still active,
retains its SQLite database, journal and lease name, and removes the gate.
The production 15-second lease TTL is neither shortened nor forcibly expired.
Original endpoints are configured before startup recovery can read receipts;
the helper modules have no side effects when ordinary ExUnit loads them.

After takeover, generation/attempt two completes the same parent, with an
unchanged execution-envelope digest and byte-identical original dispatch file.
All retained diagnostic/quality artifacts match the Engine reference reducers,
and the public Rust SDK accepts a completed wait. The five successful rounds
require exactly ten Agent executions: one diagnosis and one score per graph.
There is no second diagnosis, extra journal attempt or substitute peer result.

A sixth round restarts the original Agent as well, discarding its memory-only
receipt cache before Orchestra recovery. The original success record stays
byte-identical; generation two becomes `recovery_blocked`, retains its envelope,
publishes no dependent summary and fails a successful SDK wait. Neither Agent
performs a new execution. That round has one original diagnosis before restart;
its restarted Agent counter is zero. The complete six-round experiment therefore
has eleven actual Agent executions across process incarnations.

Inputs are small frozen domain result fixtures, not a newly recomputed upstream
FEM solve or a scale benchmark. Reducer agreement qualifies native recovery
consistency, not independent scientific accuracy or numerical convergence.
Owned processes and scratch databases, gate files and journals are cleaned up.

## Verification

Fifty-three new controlled regressions cover all eleven bounded native built-ins,
first-run independence, absent/failed/blocked dispatches, changed TaskIR inputs,
parent/legacy identity, ambiguity dominance, selected-receipt loss, receipt
contradictions, ownership checks on both sides of retrieval and atomic blocks.
They include two SQLite write-failure cases and one JSON/memory write-failure
case. Existing branch-recovery tests also verify ordinary skip semantics.

The expanded SQLite selection passed 497 executed tests with five
backend-exclusive memory tests skipped (502 listed), in 25.6 seconds.
A separate JSON/memory selection passed all 118 tests in 1.6 seconds, including
the memory-only transaction failures. Selections overlap and are not summed.
The focused actual-process tests passed both cases in 99.62 seconds; those
cases contain the six rounds above, not just two native executions.

The final complete Headless live suite passed all 71 tests in 341.11 seconds,
including both new cases. Other cases retain their existing actual-process,
mock/fixture and fault-injection scopes; 71 is not a count of independent
scientific studies. Ordinary explicit submissions, cancellation/queue fences,
unknown-dispatch restart denial and SDK result-readback checks remain green.

Rust Headless SDK tests passed 532 unit tests, 14 composite-feedback cases and
11 composite-heat validation cases. CLI unit tests passed all 189 cases.
All-target CLI and SDK Clippy checks passed with warnings denied.

Rust/Elixir formatting and diff whitespace checks passed. Native topology,
module/function matrix, extension standard, runtime API, tensor/self-test,
documentation inventory/book, version and project-organization checks passed.
The version audit checked 224 exact contracts with zero mismatches. Source and
documentation remain within 800/2000 lines with no tracked organization debt.
The SDK manual remains 1999 newline-delimited lines (2000 in the audit).
Tensor acceptance is unchanged: zero structural gaps, four maturity gaps,
19 evidence-grade gaps and 14 P0 gaps, with Daji acceptance blocked. The new
claim registers only scoped contract/recovery evidence.

## Limits

This is bounded receipt-assisted graph recovery, not a general durable node
checkpoint protocol or exactly-once execution. The graph is still reconstructed:
local ancestors and legacy typed solves retain their existing replay behavior.
Changed upstream output fails the native digest gate instead of guessing.
Confirmed journal history can age out; absence is handled by the existing
authorized graph policy, not a guarantee that a node never executed previously.

Ambiguous dispatches remain blocked before a new claim, including idempotent or
verified-shaped checkpoint graphs. Reading their receipts through the inspection
API does not promote journal state, commit a lost node or resume that parent.
The receipt cache still expires, is bounded and does not survive Agent restart.
Endpoint/process observations are not cryptographic producer authenticity.

Arbitrary parallel graphs, legacy typed solve receipt reuse, complete per-node
durable artifacts, PostgreSQL failover, installed/remote qualification and power
loss remain open. No new replay authorization API, runtime package, version bump,
Git operation, installation or remote benchmark is performed.
