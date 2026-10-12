# Direct quality sweep plan admission and fresh recovery

Date: 2026-10-12. Daji 3.6.0 source working-tree overlay at `0e1ac6cf`,
following direct sweep-request admission. No version bump, commit, packaging,
installation or remote run.

## Failure and repair

The materialization entry accepted plans without checking whether enablement,
action, readiness, optimization hints and repair metadata agreed. A plan claiming
both `sweep_enabled: true` and `sweep_action: stop` could expand cases. Missing or
contradictory repair intent, incomplete-candidate markers and failed convergence
could likewise survive into case metadata. Disabled plans bypassed even metadata
shape checks. The initial 67-case fixture exposed 51 native and 46 service
expectation failures.

Materialization now shares the request admission module's hint checks, readiness
rules and repair-strategy mapping; there is no separate numerical implementation.

- Supplied payloads, reports, hints and focus objects must have valid shapes.
  Explicit failed convergence and malformed flags reject before disabled returns.
- Enabled plans cannot claim `stop`, an incomplete source ranking, rejected
  candidates, a sweep-blocking reason or `repair_rejected_candidates`.
- Enabled coupled repairs/reviews must carry matching evidence and intent. A
  missing action remains shorthand: a fix-coupled hint implies `replan`; ordinary
  enabled plans imply `continue`. An explicitly contradictory action rejects.
- Optional repair-strategy and focus mirrors must match the supplied hint/report.
  Metadata is not silently rewritten. Disabled plans can retain valid repair
  diagnostics without producing cases, but malformed metadata still rejects.
- The service dispatch reports bad materialization input as a contract error,
  not as an unsupported operator.

Native materialization previously recalculated concrete stale counts but trusted
declared totals for upstream-blocked plans. It now agrees with service strictness:
supplied totals and per-axis counts must match the axis product. Deferred axes
require an upstream budget block and a positive whole count, while missing counts,
invalid paths/flags and product overflow reject. Counting is iterative and does
not allocate sample arrays. Raising a downstream budget does not release a
deferred upstream block. Prefix and case-limit options are checked even for
disabled plans.

Two older positive metadata fixtures supplied focus mirrors without their source
hint fields; those fixtures now provide the matching fields. A blocked-count
fixture now explicitly describes 27 deferred values instead of claiming 27 while
carrying three. The former stale-count normalization test now asserts rejection
and verifies that a corrected plan still respects the case budget.

## Verification

- Both runtimes pass 74 shared cases: 61 invalid cases and 13 accepted controls,
  with JSON exchange before admission.
- Each runtime rejects 21 single-field mutations of generated ready/warning/block
  plans, then expands the three unchanged controls with readiness and strategy
  preserved. These controls use the actual next-round and planner operators.
- Each runtime completes 24 controlled graph executions: six invalid plans under
  fail/skip policies, each followed by a corrected fresh repair run. Rejection
  publishes no dependent expansion/cases; service observers see no task-batch
  export. Fresh native runs expand two cases; service runs export two compiled
  tasks retaining readiness, hint, strategy and focus. No solver is invoked.
- Four controls per runtime keep stopped, disabled-repair, incomplete and
  billion-sample deferred plans at zero expanded cases. They demonstrate guarded
  planning without sample allocation, not billion-case simulation capacity.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
cargo clippy --locked --offline -p kyuubiki-engine --lib --tests --no-deps -- -D warnings
```

Complete Engine result: 704 total, 703 passed, one ignored, zero failures.
Engine Clippy passes for library and tests with warnings denied.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68102
```

Expanded service result: 2,362 total, 2,291 passed, 71 conditional skips, zero
failures. Ignored/skipped tests are not execution evidence.

Document-book/inventory, tensor self-test/validation and organization checks pass.
The tensor retains four maturity, 20 evidence-grade and 15 P0 gaps; release
qualification remains blocked. Source and documentation limits remain 800 and
2,000 lines respectively, with no tracked organization debt.

## Scope and remaining work

This is local controlled admission and explicit fresh-run recovery, not physical
accuracy, live Agent, installed, remote, power-loss or durable continuation.
Optional evidence is not authenticated provenance: deleting all evidence and
submitting a new minimal plan does not establish research validity.

Direct low-level expansion envelopes and caller-authored execution batches need
separate semantic/provenance audits; this check covers the quality materializer
boundary. Producer-side readiness retention through ranking and objective
recomputation remain open. Validation-repair hints retain their metadata, but
this change does not independently re-evaluate their original numerical evidence.
Tensor maturity and release-qualification thresholds are unchanged.
