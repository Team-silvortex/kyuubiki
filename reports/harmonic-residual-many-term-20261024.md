# Many-Term Harmonic Residual Enclosures

Date: 2026-10-24

Source base: `70340d07`, retaining the uncommitted assembly, inertia-precision
and residual-enclosure follow-ups. Package metadata and the installed runtime
remain 3.7.1. No version bump, commit, service deployment or desktop rebuild is
included. The repair stays in private Solver arithmetic; Engine and operator
interfaces remain decoupled and unchanged.

## Reproduced High-Degree False Rejection

Split the earlier oscillator's unit stiffness and half-unit damping into
65,536 identical parallel elements. Each has stiffness `2^-16` and damping
`2^-17`. The aggregate coefficients, half-unit load, unit represented angular
frequency and tiny nonzero mass are unchanged. The independently rounded
response remains `0.4 - 0.2i`; each member force amplitude is `2^-17`.

Both supplied-state validation and the public operator reject this equivalent
model before repair. Sequential outward additions inflate the relative residual
upper bound to `5.389752e-12`, exceeding the unchanged `1e-12` threshold. This
is a conservative false rejection, not silent acceptance of an invalid result.
The paired before-repair regressions retain the same numerical error.

These fixtures have two nodes for the path case and five for the dense control
case, with many parallel elements. They are not 65,536-node or million-node
models, and this local work is not a remote scalability qualification.

## Constant-State Compensated Endpoints

Normalized product intervals retain the previous outward bounds. The repair
changes only their long residual endpoint sums. Each endpoint keeps two finite
components `high`, `low`, and a nonnegative outward error `radius`.

For a new endpoint x, magnitude-ordered error-free addition gives
`high_old + x = high_new + first_tail`. A second such addition gives
`low_old + first_tail = low_new + lost_tail`. Adding `abs(lost_tail)` upward
to the existing radius therefore preserves the invariant:

`exact_endpoint_sum` is inside `high + low +/- radius`.

Finalization combines high and low, accounts for that addition's tail in the
radius and rounds both interval endpoints outward. Exact zero-radius sums stay
point intervals. A tail lost while adding compensation is never silently erased
or treated as proof of exactness. Nonfinite values, invalid intervals and
overflow fail closed. Interval updates commit only after both endpoints succeed.

The two-component complex residual still uses its L1 upper bound over the
existing conservative scale lower bound. Tolerance, pivots, coefficient admission
and ordinary compensated rows are unchanged. Only flagged rows allocate these
constant-size sums: 48 bytes per scalar interval sum and at most 112 bytes per
complex enclosure in the tested build. The ordinary equation state remains at
most 56 bytes. No wide integer accumulator per row, row copy, matrix snapshot
or term-sized storage is added. These are type-size controls, not a measured
whole-process memory bound or a speedup claim.

## Mixed Rows And Workflow Boundary

A separate model interleaves three replayed rows with two ordinary rows, using
distinct binary scales. Individually changing each row's displacement is rejected
at the corresponding original row, for both element orientations/orders. A good
replayed row does not bypass validation of an ordinary neighbor, or vice versa.

The first Engine workflow fixture exposed a different, legitimate limit: the
65,536-element JSON result exceeds the existing 500,000-node output security
budget. The numerical solve succeeds, but the workflow refuses publication.
The budget is not raised or bypassed to make the fixture green.

Engine direct dispatch is positively checked at 65,536 elements. Workflows are
positively checked at 32,768, within the output budget. A separate denial test
requires an explicit output-budget error at 65,536, not a residual error or a
successful prefix, then verifies a fresh budget-admissible workflow succeeds.
Large JSON output publication remains bounded; this is not evidence of an
unrestricted large-model workflow result transport.

## Verification Scope

Eleven new test functions cover six private units, two public Solver tests and
three Engine integrations:

- Independent signed integer endpoint sums on a `2^-100` grid cover 128
  sequences of 1,024 intervals in both orders. Half use full 53-bit significands;
  half use small coefficients across 100 binary powers. The reference and bound
  comparisons use integer significands, not an underflowed or rounded float sum.
- Long cancellation agrees with the exact represented `.4 + .1 = .5 + 2^-55`
  identity. A `2^-106` compensation tail remains enclosed, including signed and
  scaled variants. A minimum-subnormal term survives cancellation against one.
- Invalid/unordered intervals, nonfinite values, overflowing endpoint updates,
  invalid radii and radius overflow remain explicit denials. Failed updates
  cannot publish one updated endpoint paired with an old endpoint.
- Parallel counts 1, 1,024 and 65,536 certify the same independently supplied
  response, while forged amplitude and conjugated phase remain rejected.
- Public dense/path solves cover both load signs and element reversal at
  65,536 elements. Response, phase, every parallel member force and borrowed/owned
  serialized results are checked. No new reference uses the solver as its oracle.
- Public and Engine/workflow cancellation during the replayed element traversal
  prevents success, preserves the borrowed request and allows a fresh full solve.
  These are cooperative cancellation controls, not durable restart tests.

Pinned Rust 1.88.0 and locked offline dependencies reuse the macOS AArch64 cache.
Earlier source reports retain their original scopes; their counts are not
additive to this follow-up.

| Completed scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 924 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 711 | 1 ignored |
| Selected harmonic integrations and dynamic controls | 116 | Not every integration target |
| Total | 1,751 | Not a full workspace or all-physics run |

The dedicated profile executes all 11 new functions in two commands; these
overlap the table rather than adding to it. Solver/Engine all-target Clippy with
warnings denied passes. Existing tiny/large common-scale, force, assembly,
inertia, division, quotient, constraint-admission and dynamic closed-form
controls remain green. There is no new Linux, Windows or installed-payload run.

Native catalog inspection validates 105 profiles without executing them all.
Book, documentation inventory, organization, formatting and diff checks pass.
The tensor retains 13 modules and 11 paradigms, zero structural gaps, two
maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji status remains blocked.
The new claim is bounded numerical-validation and cooperative-recovery evidence,
not release qualification. Source/config and document limits remain 800 and
2,000 lines respectively.

This is bounded constant-state compensated enclosure of many-term harmonic residual replay, not a global forward-error proof, remote scale benchmark, larger workflow output budget, independent physical qualification or durable restart.
