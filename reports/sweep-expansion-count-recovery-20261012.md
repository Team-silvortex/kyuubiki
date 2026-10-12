# Sweep expansion count consistency and fresh recovery

Date: 2026-10-12. Daji 3.6.0 source working-tree overlay at `0e1ac6cf`, following
direct expansion quality-metadata admission. No version bump, commit, packaging,
installation or remote execution.

## Failure and repair

Native direct expansion counted supplied values but ignored `value_count` and
`values_deferred`. Both runtimes returned stopped/budget-blocked diagnostics
before validating supplied axes and count mirrors. Stale totals, contradictory
budget estimates and invalid deferred counts could survive as apparently valid
diagnostics; native expansion could execute concrete values marked deferred.
The initial 85-case fixture exposed 70 native and 55 service expectation failures.
These are related boundary variants, not 125 independent defects.

The native materializer and direct expander now share one declared-axis counter.
The service reuses its existing counter with the same additional deferred-value
rule. No numerical solver or Agent execution protocol was changed.

- Concrete axes require nonempty values and matching optional `value_count`.
  Deferred flags must be boolean. Deferred axes require an explicit budget-blocked
  envelope and a positive whole count; they cannot also contain a `values` field,
  including null or an empty array. Counts use checked multiplication and do not
  materialize deferred samples. The materializer also rejects mixed declarations.
- Expansion selects axes using existing precedence: body, caller config, embedded
  config. It validates the selected product before any stopped or blocked return.
  A caller replacing config axes must reconcile count mirrors, not rely on stale
  estimates from the old selection.
- Supplied `case_count_estimate` in envelopes, bodies, quality metadata and their
  budgets must agree with that product and each other. Missing optional counts
  remain allowed; explicit null, strings, fractional or out-of-range values reject.
  Integral float representations remain supported.
- Every supplied budget limit is validated. A budget cannot claim unblocked when
  its known count exceeds its own limit. Active expansion rejects a block carried
  by a raw body or metadata rather than ignoring it. A lower effective caller
  limit still blocks before generation; increasing it does not release an upstream
  block. Budget limits from different stages are not silently equated or rewritten.
- Minimal stopped/blocked diagnostic envelopes may omit executable axes. Their
  supplied count mirrors still have to agree. If axes are supplied, even for a
  stopped envelope, malformed or inconsistent declarations reject.
- Ordinary custom metadata without reserved quality fields remains caller-owned;
  a custom `case_count_estimate` note alone does not become plan evidence.

Admission returns the checked product for expansion to reuse. Native input bodies
are borrowed rather than cloned into a fabricated plan. This is not a measured
performance or process-memory claim.

An older native positive test claimed 27 cases while supplying three values. It
now first verifies rejection of that stale envelope, then checks that a corrected
count of three still returns the intended structured budget block and diagnostics.
No invalid fixture was simply removed.

## Verification

- Both runtimes pass 85 shared cases: 71 invalid cases and 14 accepted controls.
- Each runtime checks 125 count-mirror placements after JSON exchange, including
  absent counts, integral floats, nulls and stale values. There are 27 accepted and
  98 rejected placements.
- Each runtime completes 32 controlled graph executions: eight invalid inputs
  under fail/skip policies, each followed by a corrected fresh run. Invalid counts
  emit no dependent native cases or service task-batch export. Fresh runs yield
  two native cases or two compiled service tasks with budget metadata intact.
- Real planner/materializer outputs supply two valid controls and eight mutated
  count declarations per runtime. Three additional materializer controls reject
  deferred axes carrying null, empty or populated concrete values.
- A billion-sample deferred plan remains descriptive through materialization and
  expansion with the downstream limit raised to two billion. No values array is
  generated and zero cases are emitted. This is not billion-case simulation.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
cargo clippy --locked --offline -p kyuubiki-engine --lib --tests --no-deps -- -D warnings
```

Engine result: 712 total, 711 passed, one ignored, zero failures. Engine Clippy
passes with warnings denied.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68102
```

Expanded service result: 2,523 total, 2,452 passed, 71 conditional skips, zero
failures. Mix needed permission for its local TCP sockets after sandbox denial.
Skipped and ignored cases are not execution evidence.

Formatting, diff-whitespace, document-book/inventory, tensor self-test/validation
and organization checks pass. Source/document caps remain 800/2,000 lines.
Tensor qualification remains blocked: four maturity, 20 evidence-grade and 15 P0
gaps are unchanged.

## Scope and remaining work

This is local controlled admission and explicit fresh-run recovery, not physical
accuracy, live Agent, installed/remote acceptance, power-loss recovery or durable
continuation. Caller-owned count agreement is not authenticated research evidence
or numerical convergence. Bounds use each runtime's supported count range; this
run is not new Windows or 32-bit platform qualification.

Caller-authored execution-batch admission and producer-side readiness retention
through ranking remain the next boundaries. Objective recomputation, full budget
diagnostic-label semantics and end-to-end research validity remain separate work;
this check does not certify every field of a user-authored workflow artifact.
