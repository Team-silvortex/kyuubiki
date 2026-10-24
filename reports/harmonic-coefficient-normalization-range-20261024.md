# Harmonic Coefficient Normalization Range Admission

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, on top of the uncommitted
[complex RHS repair](harmonic-complex-rhs-range-20261024.md).
This is Solver preparation hardening. Engine production code, Agent, TaskIR,
operator inputs, result formats and installed runtimes are unchanged.

## Reproduced Failures And Existing Containment

Two new unit controls and one public preparation control failed before this guard:

- The triangular system with diagonal `1e308, 1`, upper coupling `1e-308`
  and RHS `0, 1e308` returned first unknown zero in both complex factorization
  paths. Its original equation requires approximately `-1e-308`. Dividing the
  first row by `1e308` silently erased its upper coefficient. An imaginary-only
  weak coefficient produces the corresponding missing imaginary unknown.
- Dividing five minimum subnormal quanta by two rounded the coefficient to two
  quanta; multiplication by two restored four, not the original five. A zero-RHS
  control showed that input-matrix corruption was admitted even when no later
  residual could expose it from the zero state.
- A public grounded hard/soft spring fixture reached elimination and then
  failed original-equation backward-error admission. The new control requires
  the failure before factorization or complete-state admission begins.

The public fixture did **not** previously publish an incorrect successful force:
the original physical-equation guard already contained the bad candidate.
The defect was silent mutation inside the private complex solve and late,
less-local public diagnostics. This change is not a claim of newly solving the
extreme-contrast physical model.

## Preparation Contract

Dense and tridiagonal row normalization now use one checked complex coefficient
operation. Each finite component still uses its existing `value / row_scale`.
A zero or subnormal scaled component must multiply back to the same original
binary64 coefficient; otherwise the entire solve returns a numerical-range error.
Nonfinite operands, invalid row scales and nonfinite normalized values also reject.
Ordinary normal division keeps its previous order and bits. This is a binary64
round-trip range check, not exact rational division or a proof of all rounding.

Errors identify the prepared reduced row, column and real/imaginary component.
Path indices belong to the prepared path ordering, not necessarily the original
physical node numbering. Public entry points retain the frequency index/value.
Diagonal, lower-band and upper-band coefficients share the same policy.

No matrix copy, alternate solver, widened coefficient representation, new stage
ID, pivot-floor relaxation, regularization or fallback is added. Existing RHS
range recovery and complete original-equation checks remain independent. Runtime
predicates add work; no speedup, unchanged throughput or new scale qualification
is claimed. Preparation errors discard private partial factors/state.

## Positive, Failure And Recovery Controls

Twelve new tests comprise seven Solver units, two public Solver integrations
and three Engine integrations:

- Dense/path erasure and partial-subnormal-rounding rejection; diagonal and both
  bands retain exact row/column and component error identity.
- Exactly round-tripping subnormal couplings remain admitted and produce nonzero
  minimum-quantum real/imaginary unknowns under signed inputs in both paths.
  Ordinary divisions and signed-zero bits are checked separately.
- Invalid scales and nonfinite real or imaginary components remain errors.
- A 2,050-row path and 32-row dense control cancel during preparation before a
  later bad coefficient. Uncancelled replay reproduces the specific range error;
  explicit input correction produces a clean solve.
- Public hard/soft path and cyclic fixtures check signed loads, reversed elements
  and borrowed/owned error equality. Neither factorization nor original-state
  admission is entered for the bad normalized matrix. Corrected contrast solves
  retain an independently computed hard-node displacement control.
- A static frequency completes privately before a later damped frequency loses a
  real diagonal component. Public and workflow callers receive an error, not the
  earlier successful frequency. An explicit damping correction admits both.
- Serialized direct dispatch and workflow calls preserve the specific error.
  Entry cancellation remains cancellation; a fresh invalid call still returns
  its range error, and a separately corrected model succeeds.

Two existing tests that previously expected a late backward-error failure now
expect the precise earlier preparation failure. Their expectations were not
weakened to accept arbitrary errors. The direct injected-state test
`harmonic_original_equations_reject_erased_incident_force` remains unchanged and
continues to exercise original-equation rejection independently.

These are bounded arithmetic and in-process recovery controls, not physical
material validation, durable continuation or permission to retry unknown work.

## Validation

Pinned Rust 1.88.0 with offline locked dependencies and existing build directories.
The Linux x86_64 run reused the previous source-overlay test workspace, not a
clean Git checkout or installed runtime. Four preexisting files matched the
previous overlay/base before transfer; the new scaling-test file did not exist.
All five changed/new source files matched local SHA-256 digests after the final
test update. No server deployment configuration or runtime installation changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 801 | 801 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Engine harmonic integration target | 16 | 16 | Includes the three new Engine tests |
| Public complex-force integration target | 7 | 7 | Includes the two new public tests |
| Existing frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected targets only |
| Total | 1,565 | 1,565 | Not the full workspace or all-physics suite |

The new validation profile's three commands select the 12 new tests; these are
a subset of this table, not additional tests to sum again. Historical reports
and the 3.7.0 release snapshot keep their original counts and source provenance.
The native catalog validates 90 profiles without executing them; this profile
separately executes all three commands successfully. macOS Solver/Engine
all-target Clippy with warnings denied, Rust formatting, diff whitespace,
26-file HTML book, documentation inventory and 800/2,000-line organization
checks pass. The tensor remains blocked at zero structural, two maturity,
23 evidence-grade and 18 P0 gaps; this guard does not promote qualification.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_coefficient_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test harmonic_complex_force_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-engine --test harmonic_constraint_admission
```

Profile: `harmonic-coefficient-normalization-range-local-reliability`.
The tensor records scoped verified numerical/recovery evidence, not a new
operational or physical qualification grade.

## Remaining Boundary

This is bounded harmonic coefficient-normalization admission, not a wider-range complex factorization, arbitrary precision, independent physical qualification or durable restart.

Some finite original systems cannot be represented faithfully by the current
normalized coefficient matrix. They now reject explicitly rather than continuing
with changed coefficients. Supporting them needs a separately verified
equilibration/representation strategy, not removal of the range guard. Assembly
roundoff, general conditioning, other complex operators, modal families, physical
reference comparisons, large-scale performance and installed/SDK journeys remain
separate work.
