# Modal Joint Neighborhood Comparison

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only joint-neighborhood diagnosis, not new runtime admission or general modal qualification.

The unresolved tiny-coordinate first-mode physical-publication failure is
retained. A three-coordinate local search improves two passing scale samples
but does not resolve the failing one. Directly constructing candidate columns
from physical stiffness and mass also does not change the sampled outcomes.
Neither experiment is promoted into production. Acceptance tolerance, solver
contracts, eigenvalue, dependencies and version remain unchanged.

## Controlled Comparisons

Each 128-element fixture uses actual rounded segment lengths, production
element stiffness/mass and 256 active bending coordinates. Every method at
a given scale starts from the identical best private shape produced by the
actual four-sweep physical polisher and retains the same checked eigenvalue.
Here `D` is the diagonal inverse square root of the reduced mass entries.

| Direction construction | Local candidate family |
| --- | --- |
| Existing columns recovered from the rounded mass-normalized matrix | Existing automatic fine partition and single/pair search |
| Existing recovered columns | Same partition, single/pair search plus bounded triples |
| Direct physical `D * (K - lambda M)` columns | Existing automatic fine partition and single/pair search |
| Direct physical columns | Same partition, single/pair search plus bounded triples |

Direct construction uses test double-double subtraction/multiplication before
rounding each direction to f64. It is not full-precision projection, a new
acceptance oracle or a change to the physical operator. Both direction
families are only approximate proposal data. The real production compensated
physical product and residual decide which full candidate is retained.

The triple graph nominates at most six groups per coarse coordinate from its
four strongest retained graph neighbors. Sorted, deduplicated nominations
retain at most four times the coarse-coordinate count, with deterministic tie
ordering. Each group tests at most 26 nontrivial combinations of the current,
next-up and next-down f64 coordinates, sweeping forward and backward.

Triples share the existing fine factor, fine fits and four outer iterations.
They do not receive an extra true-residual budget. Initial/final measurements
and both fine fits consume at most 74 callbacks in the same 80-check counter.
Only the retained candidate is returned, after a fresh final measurement and
validation cancellation poll. Failed or cancelled searches return an error,
not a partial shape.

## Numerical Outcomes

The physical residual gate remains `1e-8`.

| Candidate family | Segment length 1 | Segment length 1e14 | Segment length 1e-10 |
| --- | --- | --- | --- |
| Existing single/pair, either direction construction | Pass, 9.205252983779463e-9 | Pass, 7.362945994602734e-9 | Reject, 1.7404270550916884e-8 |
| Single/pair plus triples, either direction construction | Pass, 8.397853059584172e-9 | Pass, 7.329754180186009e-9 | Reject, 1.7404270550916884e-8 |

All passing comparisons also check a constructed complete modal result record:
unit shape norm, exact eigenvalue/shape bits through typed JSON readback, and
a separately assembled closed-entry physical residual on the actual request
geometry. These are test-only candidate records, not new public solver,
Headless, Agent or installed admission for triples. Failure bands and error
categories are explicit assertions; a failed solve is not silently skipped.

A separate four-coordinate analytic example gives three coarse directions
with pairwise opposing components and a shared small component. The retained
single/pair sweeps cannot change the candidate in this fixture. Three
coordinated upward moves across the forward/reverse sweep
remove that residual exactly in f64. Independent closed-entry products verify
the result at the same `1e-8` gate, without trusting the projected score. This
demonstrates the local method's purpose, not its sufficiency for the beam.

The unchanged failing residual does not prove that no f64 mode shape exists.
It only excludes these sampled direction/neighborhood changes as a remedy.
Higher-precision fine projection and more general representable-vector
proposals remain hypotheses requiring independent physical, spectral and
multimode checks before any runtime promotion.

## Resource And Recovery Guards

A separate test-only plan adds normalized graph storage, nominations, triples
and scratch to the existing fit model. Its extra component-visit allowance is
`5000 * n^2`, covering four forward/reverse triple sweeps, bounded trials and
graph bookkeeping. At 256 active coordinates the total modeled work is
663,224,320 component visits, under a separate 700,000,000 ceiling. The total
modeled numeric/container payload is 4,980,736 bytes, under 8 MiB. These are
conservative code-work/payload models, not measured runtime, allocations,
peak RSS or a performance qualification. The original runtime plan does not
cover this additional experiment, and production never runs it.

Retained controls exercise dimensions 2/4/128/256, rejection above the bound,
graph cardinality/uniqueness/replay, known physical column entries, nonfinite,
ragged, asymmetric and out-of-range inputs, and nonzero-coupling underflow.
A decreasing synthetic certificate sequence forces all 74 checks and confirms
one monotonic counter; it is a control fixture, not numerical evidence.

An initially passing certificate followed by an above-gate or malformed final
certificate must fail. Actual-operator errors propagate. Cancellation is
checked at preparation, graph scanning, vector updates, iteration, residual
search and final validation. Borrowed seed shapes and physical matrices remain
unchanged, and the same cached factor or borrowed model can be replayed with
a fresh control after cancellation.

## Verification

All listed commands completed successfully on this local source line. The
retained `joint_rounding_` filter passes six tests. Full solver regression
passes 1,424 tests across 186 result groups, with zero failures and nine
pre-existing ignored tests. The six new tests are included in that total,
not added to it. The Headless modal integration suite passes all 37 tests.
Strict Clippy reports no warnings, and formatting/whitespace checks pass.

Tensor structure and self-tests pass with 13 modules and 11 paradigms. The
broader readiness gaps remain unchanged: four maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps; daji qualification remains blocked. Operator registration
and self-tests pass for 59 profiles with `executed=false`. Those registry
validators verify registrations and evidence anchors, not the numerical
commands themselves. Documentation book and inventory checks pass, including
26 HTML files. Organization checks retain zero tracked line-limit debt, and
the new untracked source/report files also remain below their applicable limits.

Commands from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib joint_rounding_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

Native validation commands from the repository root:

```text
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor --self-test
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --self-test
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation
workers/rust/target/debug/kyuubiki-script-runner check-doc-book
workers/rust/target/debug/kyuubiki-script-runner check-doc-inventory
workers/rust/target/debug/kyuubiki-script-runner audit-project-organization
git diff --check
```

The source and documentation limits remain 800/2,000 lines. No remote or
installed testing, live Agent qualification, packaging, version bump or Git
submission is claimed by this report.
