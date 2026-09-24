# Frame initial-stress reliability, 2026-09-23

This is bounded local initial-stress equilibrium validation, not general prestress or nonlinear structural qualification.

Base revision: `a7557be7` (`daji 3.3.6`). No version bump, Git commit,
deployment or app rebuild is part of this round.

## Reproduced Defects

The first six public-solver regression tests produced five failures and one
passing self-equilibrated mixed-fiber control before the correction:

- Tiny uniform prestress was accepted because the old tolerance was based on
  yield strength and had an absolute force floor.
- Fully overridden mixed fibers could be accepted after increasing an unused
  parent yield strength to `1e30`, despite unchanged unbalanced initial stress.
- A separate fully supported member could increase the global tolerance and
  hide the unbalanced initial stress at another node.
- A pure initial moment was compared with a force scale, allowing an
  unbalanced state in the tested length-unit representations.
- The failed-input/replay control could not proceed because that invalid
  initial state had incorrectly been returned as a solver result.

The replay failure reproduced the missing rejection, not observed shared
history corruption. Extreme values in the tests are numerical range controls,
not physical material properties or experimental reference data.

## Correction

Initial force assembly reuses the existing element/section response and
collects local absolute contributions as well as signed forces. For an
element, let `AN`, `AMi`, `AMj` be the absolute integration sums, each at
least the corresponding resultant's magnitude. Its translational force bound
is `AN + AMi/L + AMj/L`; its end-moment bounds are `AMi` and `AMj`.
These bounds are accumulated only at the element's incident nodes.

At each node, constrained residual components are removed. The free
translation norm is computed after division by the local force bound;
the free moment is divided by the local end-moment bound. Both ratios must
be at most `1e-9`. A zero residual and zero bound are accepted; a nonzero
residual without a positive bound is rejected. There is no yield-strength
denominator or absolute force floor. Nonfinite assembly, force and scale
values are rejected before ratios are evaluated.

The translational bound is orientation-free and deliberately norm-based;
it does not provide a separate backward-error bound for each translational
component. Absolute fiber sums permit normal integration cancellation in
self-equilibrated residual-stress fields. Forces and moments never share a
dimensional denominator. Legitimate constrained reactions need not vanish.

Both existing preflight sites use the guard: the reference buckling baseline
and the imperfect initial geometry before load stepping. Error messages
identify the node index, translation/rotation component, ratio and scale.
No physics moves into engine dispatch. Public task/result schemas, Newton
tolerances, adaptive quadrature and accepted-history commit rules are unchanged.
The extra scale vector is only assembled on the initial-stress path; ordinary
Newton force evaluations do not request it.

## Tests And Reproduction

Eighteen new tests are divided into eight kernel controls, seven public-solver
cases and three in-process Rust headless-plan cases. Kernel tests cover
finite range, node isolation, force/moment separation, rigid rotation of
translation residuals, length-unit classification, malformed maps,
nonfinite values and overflow of signed or absolute assembly sums.
Public/headless tests cover scalar and mixed-fiber input rejection, genuine
self-equilibrated residual stress, support-balanced nonzero member forces,
unused parent defaults and clean replay after an error.

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib initial_balance_tests
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test frame_2d_initial_stress_reliability -p kyuubiki-cli --test frame_2d_material_operator
./scripts/kyuubiki check-operator-validation --execute --profile frame-initial-stress-local-reliability --out tmp/frame-initial-stress-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test frame_2d_material_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification

Local macOS ARM64 results on this working tree:

- Core protocol, solver, engine and headless-SDK libraries: 1169 passed,
  7 existing ignored, no failures.
- Expanded mechanical regression: 88 passed across 14 public-solver,
  headless and workflow targets. Existing initial-stress buckling shifts,
  composite residual-stress templates, cyclic material paths, fiber adaptive
  integration and branch paths remain passing.
- All eighteen new cases passed in debug and release. The release groups
  also passed five retained material-route controls: 8 kernel, 7 public-solver
  and 8 headless-route tests in total.
- The `frame-initial-stress-local-reliability` profile executed all four
  commands successfully, totaling 57 test executions including retained
  material, composite, fiber and workflow controls. This is not execution
  of the complete validation registry.
- Strict solver all-target and CLI integration-test Clippy checks, touched
  source formatting and whitespace checks passed.
- Registry validation passed with 42 profiles. Tensor structure/command
  checks, documentation inventory and project organization audit passed.
  Source/document limits remain 800/2000 with zero tracked line-limit debt.

The global tensor still reports 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps; the Daji qualification status remains blocked. The generated
profile result is ignored under `tmp/`; no build artifact or test database
is added to the repository.

## Limits

This is local numerical-validation and fresh-replay evidence only, not a
remote Agent transport, installed GUI, performance or global physics claim.
It adds no prescribed external prestress-relaxation path and no durable
material checkpoint restart. Initial states outside the existing equilibrium
contract are rejected rather than silently relaxed into a different model.
No global Daji qualification gate is closed by this local correction.
