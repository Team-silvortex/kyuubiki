# Native Workflow Job Cancellation

Date: 2026-10-11. Source macOS ARM64 verification at commit `a20f6c8a`
(`daji 3.5.5`) with this working-tree overlay; package metadata remains `3.5.0`.
The public graph-job cancellation path now has bounded cross-domain service
evidence, extending the [native diagnostic chains](native-domain-diagnostic-chains-20261008.md)
without adding physics implementations or changing the installed baseline.

## Reproduced Gaps and Repair

Two adapter regressions failed before repair: native diagnostic/scoring activity
was not forwarded, and an invalid activity consumer was silently ignored. The
existing Agent client now receives the validated progress callback, while the
Orchestra engine wraps solver, native extraction and native scoring activity
through the same node-bound callback. Identity and completion gates remain in
their existing TaskIR paths.

Three additional regressions reproduced a cancellation timing gap for solver,
native diagnostic and native scoring nodes. After the coordinator commits a
cancelled job, the next heartbeat used to abort its local dispatch owner before
targeted backend cancellation could capture that owner's capacity lease. The
runner now drains activity for that already-cancelled job without changing its
state or progress. It retains the owner until the backend terminal response or
existing timeout boundary. Node completion/failure and result publication still
fail the cancelled-job fence; other ownership errors are not ignored.

Controlled heartbeat tests require liveness refresh without completed nodes,
iteration advancement or execution-start reset. Missing heartbeats still fail
the stale-work guard, and fresh heartbeats cannot renew the execution deadline.
After cancellation, state stays unchanged, the exact dispatch endpoint remains
available to targeted cancellation, and even a late successful reply cannot
complete the node or rewrite the cancelled recovery record.

## Executed Service Scenarios

The new live fixture starts an isolated Orchestra and two capacity-one Rust
Agents bound to it. It covers thermal, electrostatic, magnetostatic,
thermo-mechanical and Stokes diagnostics under both default error handling and
`on_error: skip`: ten cancellations and ten explicit recovery graph jobs.

Inputs are computed single-quad results whose node and element records are each
repeated to length 129. This exercises diagnostic reduction, not a larger valid
physical mesh. Explicit placement tags select the held owner and bystander;
the fixture does not depend on incidental pool ordering. The diagnostic is held
at `result_diagnostics` after exactly 64 records, using the existing explicitly
enabled task-scoped fault injector.

The public `/api/v1/jobs/{job_id}/cancel` route registers cancellation only on
the Agent that owns the graph's diagnostic dispatch. Its acknowledgement remains
a request, not terminal-execution proof. Subsequent observations require:

- The owner returns to accepting work, retaining a cancelled failure with the
  same request, generation and parent job and the held diagnostic-stage cause.
- The bystander keeps its original active request/generation and is not cancelled.
- The graph diagnostic has one retained `observed_failed` TaskIR attempt and no
  quality dispatch. Inspection does not authorize automatic replay.
- Diagnosis, quality, objective and decision have no completed node, completion
  progress event or summary artifact in the cancelled graph runtime.
- A native SDK completed-job wait rejects the cancelled job even though recovery
  diagnostics are retained. `has_result` here denotes that retained record, not
  successful physical computation.
- A direct diagnostic using the same parent identity on the bystander succeeds;
  no stray broadcast cancellation poisons its next admission.
- An explicit new graph job diagnoses and scores the retained healthy raw data.
  Its full stored outputs match the Engine reductions; the old job and failed
  attempt remain cancelled/failed, and project records stay unchanged.

Recovery uses caller-retained raw input, not a physical result recovered from
the cancelled graph's diagnostic runtime record.

The ten rounds require exactly sixty Agent executions: twenty deliberate slot
holders, ten cancelled graph diagnostics, ten bystander identity-reuse checks,
and twenty native diagnostic/scoring executions in explicit recovery graphs.
There is no additional solve or automatic replay in those recovery graphs.
CLI previews intentionally compact large record arrays; physical comparisons
read the complete retained result rather than interpreting previews as full data.

## Verification

| Suite | Result |
| --- | --- |
| Related Elixir adapter, heartbeat, graph, TaskIR, cancellation and recovery suites | 212 passed |
| CLI and Agent unit regressions | 189 passed |
| New five-domain two-policy cancellation/recovery service fixture | passed, 60 executions |
| Complete native Headless live suite, including the new fixture | 64 passed in 163.94 seconds |

The fixture is included in the full-suite count; focused reruns and unit subsets
are not summed as additional unique coverage. Strict all-target CLI Clippy
passed with warnings denied. Rust/Elixir formatting and diff whitespace checks
passed. Native topology, matrix, extension standard, runtime API surface,
tensor/self-test, documentation inventory/book, version and project-organization
checks passed, with no tracked size debt against 800-line source and 2000-line
document limits. The tensor still has four maturity, 19 evidence-grade and
14 P0 gaps, with Daji acceptance blocked; this evidence does not promote them.

## Boundaries

This qualifies the source public graph-job route while one of these five native
diagnostic nodes is active, not arbitrary parallel graph dispatch, all solver or
plugin cancellation, remote/authenticated deployment or installed applications.
Activity draining is not unlimited execution: existing RPC and runner deadlines
remain active. The 64-record cadence is not a wall-clock latency bound. It does
not cover JSON processing preemption, large physical meshes, durable continuation,
numerical accuracy, exactly-once execution or qualification promotion.
