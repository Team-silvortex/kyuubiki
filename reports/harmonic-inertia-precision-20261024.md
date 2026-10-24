# Harmonic Inertia Product Precision

Date: 2026-10-24

Source base: `70340d07`, retaining the uncommitted coefficient-addition follow-up.
Package metadata and the installed runtime remain 3.7.1. This is a private Solver
arithmetic change; Engine, Agent, operator descriptors and TaskIR are unchanged.
No version bump, commit, service deployment or desktop rebuild is included.

## Reproduced Upstream Rounding

Let `d = 2^-27`, represented angular frequency `w = 1+d`, mass `m = 1+d`, and
grounded stiffness `k = 1+3d+2^-51`. The exact represented product is
`w*w*m = 1+3d+3d^2+d^3`. The dynamic diagonal is therefore
`L = 5*2^-54 - 2^-81`. Applying real load L gives displacement one.

The previous two-multiplication order rounds the square before mass joins it.
It produces inertia `1+3d`, and the subsequently exact addition collector gets
the wrong operand. Public solving succeeds with amplitude
`0.6249999990686774`, about 37.5 percent too small. One coefficient test and one
public test fail before repair. This remains a highly conditioned represented
input example, not a physical material qualification.

Frequency/mass powers are paired so `w` scales by `2^p` and mass by `2^-2p`.
The reference stays unchanged for p from -500 through 500. The public fixture
asserts that conversion from its supplied frequency yields the intended
represented angular frequency. It does not treat mathematical pi as exact.

## Carry Product Parts Until Assembly

The inertia kernel decomposes a normalized frequency square and both subsequent
mass products with FMA residuals. Four parts share one binary exponent. A fast
exact-product control is admitted only when both zero FMA residuals have a safe
range: a floor of `2^-968` prevents an underflowed residual from masquerading as
exact zero. Direct tests reproduce zero-residual false certificates at tiny
scales.

Outward-padded bounds admit ordinary normal-range expansions without a wide
integer range check. Near output limits, a checked exact sum decides admission.
The kernel retains the full parts instead of publishing that rounded range-check
result as the assembly operand.

Assembly uses its existing finite correction buffer when every part restores
exactly. Otherwise it retains scaled parts in a separate sparse buffer. The
finite buffer's entry width is not increased for every coefficient. Corrected
scalar groups that need the wide path use a 68-limb accumulator with a `2^-3222`
lattice, covering the minimum binary64 triple-product quantum, finite coefficient
range and a usize-bounded carry. The existing 18- and 34-limb flavors keep their
own storage and admission rules.

Final normal coefficients round once; nonexact subnormal publication, nonzero
underflow, true overflow, invalid alignment and capacity loss fail closed.
Boundary tests include a sticky bit far below the ordinary float quantum,
normal/subnormal transitions, overflow midpoints, exact zeros and poisoned state.
The wide accumulator occupies at most 1,120 bytes. This is a type-size bound,
not a whole-solve stack or memory measurement.

The original-equation visitor receives every nonzero part with its exponent;
it no longer substitutes the old rounded inertia coefficient. Existing
backward-error tolerance, row normalization and pivot floors remain unchanged.
Velocity/acceleration postprocessing and damping-product rounding are separate
and retain their existing order.

## Regression Feedback And Limits

An initial candidate restored each part to binary64 separately. The existing
`1e-300` common-model-scale test caught a false rejection. Scaled deferred parts
repair that regression without deleting the test or relaxing its checks.
The corresponding `1e300` control also passes.

A new deliberately extreme cross-coefficient-scale fixture still exceeds the
original residual alignment range. It remains an explicit rejection control,
followed by clean reuse. The positive below-quantum-tail fixture uses a common
small coefficient scale instead; its final coefficient is normal and its unit
response is independently derived. No all-contrast acceptance is claimed.

The first dense frequency-scaling fixture left its isolated control masses
unscaled, generating a separate extreme-fill range rejection at the largest
frequency. Scaling those control masses by the same frequency/mass similarity
preserves the intended dimensionless control problem. A separate integer-oracle
fixture also initially constructed a zero mass by evaluating an excessively
small power before multiplying its integer significand. It now normalizes that
significand before applying the common binary scale. These fixture issues are
not counted as production numerical defects.

Scratch storage remains proportional to retained finite/scaled parts, not a
matrix snapshot or wide accumulator per model cell. Grouping is comparison
sorting and is cancellable only before/after each sort, not during it. Replay
and coefficient scans have cooperative checks. No speedup, production hit rate
or million-node performance qualification is claimed.

## Verification

Nineteen new functions cover 11 private units, five public Solver controls and
three Engine integration tests:

- Independent cubic identities, small integer products and 256 full u128 product
  and weak-difference tuples at three common binary scales.
- Dense/path assembly, signs, orientation, borrowed/owned equality, amplitude
  and member-force readback, plus explicit original-equation term visitation.
- Scaled sticky bits, zero-FMA underflow, checked lattice alignment, rounding,
  range admission, capacity poisoning and the original common-scale controls.
- Cancellation after inertia preparation, during grouping/replay and before
  handoff/residual checks; unchanged borrowed requests and fresh complete reuse.
- Serialized Engine dispatch and workflow output, late-frequency failure and
  invalid subnormal admission without successful prefix publication.

Pinned Rust 1.88.0 and locked offline dependencies reuse the macOS AArch64 cache:

| Completed final-source scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 910 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 711 | 1 ignored |
| Selected harmonic integrations and dynamic controls | 105 | Not every integration target |
| Total | 1,726 | Not a full workspace or all-physics run |

All-target Solver/Engine Clippy with warnings denied passes. The local profile
selects the 19 new tests and four unchanged frequency-product controls. These
23 are a subset of the table, not an additional count. The preceding assembly
report retains its original source scope; its test totals are not additive.

The source claim is registered in the numerical-validation and recovery
coordinates without promoting release qualification. Native catalog inspection
validates 103 profiles without executing them all. The new profile executes all
three commands and passes 23 selected tests. Book, inventory, formatting,
whitespace and the 800/2,000-line organization checks pass with no tracked debt.
Tensor checks retain 13 modules, 11 paradigms, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps; overall qualification remains blocked. Linux, Windows, installed
runtime, SDK/service research, remote scale and forced-kill recovery are not
performed here.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_inertia_precision
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_inertia_precision
kyuubiki-script-runner check-operator-validation --profile harmonic-inertia-precision-local-reliability --execute --out tmp/harmonic-inertia-profile.json
```

## Scope Boundary

This is bounded retention of admitted represented angular-frequency squared times mass through harmonic assembly, not exact angular-frequency conversion, exact damping/postprocessing, a whole-solver forward-error certificate, independent physical qualification or durable restart.

The reference uses represented model operands. Physical parameter uncertainty,
unrepresented input precision, unrelated products, factorization roundoff and
residual-alignment limits retain separate boundaries. Cooperative failure tests
do not qualify process termination, host power loss or durable continuation.
