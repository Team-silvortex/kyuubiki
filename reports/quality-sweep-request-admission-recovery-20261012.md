# Quality sweep request admission and fresh recovery

Date: 2026-10-12. Daji 3.6.0 source working-tree overlay at `0e1ac6cf`,
following the coupled-readiness next-round repair. No version bump, commit,
packaging, installation or remote run.

## Failure and repair

Direct quality-sweep planning still selected the first present hint and readiness
report. Nulls could hide later evidence, conflicting reports could silently win,
and a `stop` action returned before checking those reports. A blocked report could
therefore produce an ordinary sweep or a stopped plan. Native planning also
ignored a nonempty rejected-candidate list when the completion flag and hint did
not independently disable planning.

The initial 61-case fixture exposed 46 native and 39 service expectation failures.
It now contains 74 shared cases: 56 invalid cases and 18 accepted controls.
Both implementations admit the request before taking the stopped/incomplete
shortcuts or allocating sweep samples:

- Payloads must be objects. Absent/null request and seed-objective aliases are
  optional; supplied non-null values must be objects. Explicit failed convergence
  on payloads, requests, seed objectives or readiness reports rejects.
- Actions must be `continue`, `replan` or `stop`. Supplied completion/readiness
  flags must be booleans. Rejected-candidate lists must contain objects.
- `request_payload.optimization_hint` and `selected_iteration_hint` are aliases
  of one object. Non-null duplicates must be structurally equal, including metadata.
- Readiness aliases in the request, selected report, top-level report and request
  seed objective must likewise agree. Null aliases cannot mask supplied evidence.
  Top-level state, recommendation and source-contract mirrors cannot contradict it.
- A blocked report requires `replan` with `fix_coupled_readiness`; it cannot
  claim `selected_candidate_ready: true`. Warnings require a non-stopped request
  with `review_coupled_readiness`. Repair/review hints without matching evidence
  reject rather than falling back to general search. Minimal matching hints remain
  valid. Supplied source, state, recommendation, domain-list and count mirrors
  must match the report; omitted fields need not be repeated.
- Incomplete rankings, rejected candidates or `repair_rejected_candidates`
  disable the plan with zero cases. Alias/shape checks still run before this
  branch; execution-intent checks are unnecessary for the disabled plan.

The service also reports invalid planner input as a contract error instead of
misidentifying this supported operator as unsupported. One older native positive
fixture claimed a blocking count of one but omitted its domain list; it now
supplies the corresponding electrostatic domain rather than weakening validation.

Readiness reports and accepted hints remain unmodified in plan/case metadata.
The work does not move numerical algorithms into the control plane or duplicate
the native readiness generator. Repair sweeps are explicitly requested research
cases, not newly qualified candidate results.

## Verification

- Both runtimes pass the 74 shared JSON-exchanged admission cases.
- Each runtime checks 625 readiness alias placements using null, ready, warning,
  block and false-only reports at four locations. The 564 conflicting combinations
  reject; the 61 consistent combinations expand two cases each and preserve evidence.
- Each runtime runs 24 controlled graph executions: six invalid requests under
  fail/skip policies, each followed by a corrected fresh repair run. Invalid inputs
  publish no dependent plan/cases; the service also observes no batch export.
  Corrected native runs expand two cases and service runs export two compiled tasks,
  retaining the original blocked report and repair strategy. No solver is invoked.
- A separate native graph verifies that a rejected-candidate list alone disables
  planning and reaches expansion with zero cases.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
cargo clippy --locked --offline -p kyuubiki-engine --lib --tests --no-deps -- -D warnings
```

Complete Engine result: 700 total, 699 passed, one ignored, zero failures.
Engine Clippy passes for library and tests with warnings denied.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68101
```

Expanded service result: 2,285 total, 2,214 passed, 71 conditional skips, zero
failures. Ignored/skipped tests are not execution evidence.

Tensor validation and its self-test, document-book/inventory checks and project
organization audit pass. The tensor still has four maturity, 20 evidence-grade
and 15 P0 gaps; release qualification remains blocked. Source files remain below
800 lines and documents below 2,000 lines.

## Scope and remaining work

These are local controlled admission and explicit fresh-run checks, not physical
accuracy, live Agent, installed, remote, power-loss or durable recovery evidence.
Neither objective scores nor physical optimality are recomputed. Search choices
and optional focus fields remain caller-owned; this is not search-heuristic parity.
Missing reports retain metric-only shorthand and do not certify readiness.

Directly authored or modified materialized plans still need their own semantic
admission audit; bypassing the planner is not covered by this request gate.
Producer-side readiness retention through ranking, objective recomputation and
real restart recovery remain separate work. Tensor maturity and qualification
thresholds are unchanged.
