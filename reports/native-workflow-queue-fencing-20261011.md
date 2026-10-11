# Native Workflow Queue Cancellation and Progress Fencing

Date: 2026-10-11. Source macOS ARM64 verification at commit `a20f6c8a`
(`daji 3.5.5`) with this working-tree overlay; package metadata remains `3.5.0`.
This extends [active diagnostic cancellation](native-workflow-job-cancellation-20261011.md)
to established capacity queues and the throttled node-completion boundary.
Physics and task execution remain in the existing Engine and Rust Agent.

## Reproduced Fencing Gap

Node progress is persisted at a bounded cadence rather than after every node.
The non-persisted branch previously returned `:ok` using only the coordinator's
in-memory owner flag. It did not recheck the database-backed Orchestra lease,
terminal job status or execution claim. A cancelled or superseded execution
could therefore continue across a node-completion callback until another
progress event met the persistence threshold.

The initial nine regression cases produced seven failures before repair:
completed, failed and cancelled jobs, public cancellation, a replaced claim,
a changed attempt, and a revoked Orchestra lease all incorrectly returned `:ok`.
Healthy throttled events and invalid event validation already behaved correctly.

The coordinator now reuses its existing lease/job/recovery fence even when it
does not persist an event. This check does not append progress, refresh liveness,
reset execution start or write a recovery artifact. Ownership loss still retires
the coordinator's local ownership through the existing failure boundary.

The final unit set also checks a retired recovery envelope behind an otherwise
active job. Three controlled runner cases use a 300-node fan-out graph after a
typed bar solve, native transport diagnostic or native transport score. The
first operation is cancelled while held; its late successful response reaches
the non-persisted completion branch. The runner exits before entering dependent
native scoring, leaving the cancelled job and recovery payload unchanged.
These controlled transport tests are not large physical model benchmarks.

The first full service run also exposed a timing assumption in the previous
active-diagnostic cancellation fixture: 65 tests passed and one failed while
reading `dispatch_boundary_unconfirmed` before the terminal observation arrived.
The fixture had treated every state except a nonexistent `dispatched` state as
terminal. It now waits only through the actual initial state, accepts exactly
one `observed_failed` attempt and rejects unknown or extra attempts. A separate
pure regression locks down that distinction without weakening outcome checks.

## Real Capacity Queue Scenarios

Two new service tests start isolated Orchestra instances and two capacity-one
Rust Agents. Each runs thermal, electrostatic, magnetostatic, thermo-mechanical
and Stokes chains under default handling and explicit `on_error: skip` on native
diagnostic/scoring nodes. They cover twenty cancelled graphs and twenty explicit
new recovery graphs using computed single-quad inputs.

Explicit placement tags pin graph diagnosis to the owner and scoring to the
peer. The public health snapshot proves that the target has entered the capacity
queue; the fixture does not use a sleep or pool ordering to infer admission.

- In ten diagnostic-queue cases, both Agents hold unrelated bar tasks. The graph
  diagnostic remains queued and creates no native dispatch attempt.
- In ten scoring-queue cases, only the peer holds a bar task. Diagnosis completes
  on the owner with one retained `observed_executed` attempt; scoring remains
  queued and creates no attempt. The completed diagnosis progress stays visible.
- Public cancellation removes exactly one queued request, with zero reserved or
  dispatched targets and zero Agent cancellation registrations. The receipt is
  `cancelled_before_dispatch`, not proof of arbitrary backend termination.
- Unrelated holder identities and generations stay active and uncancelled.
  Releasing capacity does not resurrect the cancelled graph or publish a score,
  objective or decision. A native SDK completed-job wait rejects the old job.
- Direct diagnosis reusing the cancelled parent identity on the queued target's
  Agent succeeds, proving no pending Agent cancellation was broadcast.
- An explicit new graph diagnoses and scores caller-retained raw input. Full
  stored output matches the Engine reductions, while the old cancelled job,
  original dispatch observation and project records remain unchanged.

The twenty rounds require exactly one hundred Agent executions: thirty unrelated
bar holders, ten completed pre-cancellation diagnoses, twenty identity-reuse
diagnoses and forty diagnostic/scoring operations in explicit recovery graphs.
The queued operations themselves never execute. Raw single-quad solutions are
computed locally by the Engine to prepare inputs; there is no additional Agent
solve or automatic replay in the recovery graphs.

Retained graph recovery records and dispatch observations are not successful
scientific result artifacts or reusable terminal-result caches. The comparisons
check routing and preservation against the same Engine implementation, not an
independent numerical reference.

## Verification

The related Elixir adapter, coordinator, heartbeat, runner, cancellation,
recovery, TaskIR and graph API suites passed 225 tests. The focused 22-test run
is a subset, not additional unique coverage. The two new real queue fixtures
passed, requiring exactly fifty Agent executions each. CLI and Agent unit tests
passed all 189 cases, and strict all-target CLI Clippy passed with warnings denied.
The corrected active-diagnostic fixture and its observation regression also passed
their focused rerun.

The final complete native Headless live suite passed all 67 tests in 155.64
seconds, including the two queue fixtures and the observation helper regression.
Focused reruns are not counted again. Rust/Elixir formatting, diff whitespace,
native topology, matrix, extension standard, runtime API surface, tensor/self-test,
documentation inventory/book and version checks passed. The version audit checked
224 exact contracts with zero mismatches. Organization checks retain zero tracked
size debt against 800-line source and 2000-line document limits. The tensor keeps
four maturity, 19 evidence-grade and 14 P0 gaps, with Daji acceptance still blocked.

## Boundaries

This is source macOS contract/recovery evidence, not installed applications,
remote/authenticated deployment, arbitrary parallel graphs, large-mesh numerical
accuracy, durable continuation or exactly-once execution. The tests establish
an existing capacity queue or an already-cancelled node-completion fence; they
do not exhaust every cancellation interleaving before admission or between
dispatch callbacks. Existing timeouts remain active. No qualification grade,
release version, installed baseline, Git commit or distribution changes are
implied by this repair.
