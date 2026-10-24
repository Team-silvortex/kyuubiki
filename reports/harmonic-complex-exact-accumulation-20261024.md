# Harmonic Complex Aligned-Component Accumulation

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted
[RHS range](harmonic-complex-rhs-range-20261024.md),
[coefficient normalization](harmonic-coefficient-normalization-range-20261024.md),
[zero-cancellation](harmonic-complex-zero-cancellation-20261024.md) and
[near-cancellation](harmonic-complex-near-cancellation-20261024.md) follow-ups.
This changes only the exceptional harmonic Solver accumulation path. Engine
production code, Agent, TaskIR, operator contracts and installed runtimes are
unchanged. No release bump, publication or installation is included.

## Reproduced Defect

All four initial new counterexample tests failed before this change. Entering
replay was insufficient: its floating-point correction balance could itself
lose a smaller term before larger terms cancelled.

- For `L=2^60` and `t=2^-60`, the exact expression `L+1+t-L-1` is `t`.
  The previous replay returned zero. Signed real/imaginary controls repeat this
  with exact power-of-two scales `2^-500`, one and `2^500`.
- A product-tail expression with `d=2^-27`, `t=2^-100` is
  `L+(1+d)*(1-d)+t-L-1+d*d = t`. Its FMA tail entered replay correctly, but
  the single correction balance still returned zero.
- A two-complex-term row with real RHS `L`, terms `(-1+i)*(1+i*t)` and
  `(L-i)*(1+i)`, requires real numerator `t`. Its imaginary numerator is
  `-L+t`, which rounds to `-L`; the components must not share a scale or error
  balance. The real component was previously erased even inside replay.
- A manufactured six-row unit-diagonal triangular system with first-row unit
  couplings and remaining RHS `L,1,t,-L,-1` requires first unknown `-t`.
  Dense back substitution returned zero instead.

These are internal arithmetic and manufactured-system counterexamples, not new
evidence that a public physical model previously published an incorrect force.
Existing public force and Engine dispatch/workflow fixtures remain consumer
regressions and are not counted as new physical reference validation.

## Bounded Integer Sum

The existing first pass still selects a common binary exponent. The second
pass still checks that each aligned high/FMA-low part is reversibly represented
in binary64. Alignment failures remain errors; this change does not silently
drop terms that cannot be aligned.

Each accepted normalized part has magnitude at most four and is an integer
multiple of `2^-1074`. Separate positive and negative arrays of 18 `u64` words
accumulate those integer units with checked carries. Their difference retains
all accepted parts without a rounded floating-point error balance. Final
conversion uses significand, half and sticky bits with ties-to-even rounding.
Thus the aligned sum is rounded once; subsequent exponent restoration and
division are separate operations, not an exact end-to-end solve.

The accumulator has fixed storage, no heap allocation or dependency, and its
type is checked to fit within 320 bytes. This is not a bound on the entire
solver call stack. Each sign has a finite 1,152-bit capacity corresponding to
magnitudes below `2^78` on the normalized lattice. Capacity is checked rather
than assumed: overflow or an invalid normalized part poisons the accumulator,
and it cannot publish a wrapped or partial sum afterward. Capacity-overflow
tests inject synthetic near-full arrays; they are not large-scale benchmarks.

Ordinary fast-path arithmetic and replay triggers are unchanged. No matrix or
row copy, alternate factorization, pivot relaxation, new stage ID, persistent
state or automatic retry is added. Original-equation admission remains required.
Fixed integer accumulation costs work on replay; no throughput improvement or
performance neutrality is claimed without a separate benchmark.

## Controls And Validation

Ten new tests cover the four reproductions, cancellation/reuse and five
accumulator groups:

- A separate `i128` dyadic-lattice oracle checks 128 sequences of 64 exactly
  representable signed inputs, each in forward and reverse order. Production
  uses fixed `u64` arrays; the oracle uses native integer sums and integer-to-f64
  conversion rather than production decomposition or rounding helpers.
- Explicit midpoint, sticky-bit, odd/even and significand-carry controls cover
  both signs, including a minimum subnormal quantum on either side of a normal
  midpoint. These complement rather than replace the integer oracle.
- Subnormal/normal boundary carries, long borrows, independent quantum terms
  and exact cancellation to zero have exact bit/value references.
- Deterministic reorderings retain deeply nested tails with fixed storage.
- Nonfinite/out-of-domain parts and injected capacity overflow reject; both
  further additions and final conversion remain invalid after the error.
- A padded 2,050-term replay cancels at bounded entry/interior/pass/component
  checkpoints. Fresh identical-input calls retain the correct signed tiny
  components afterward. This is cooperative in-process reuse, not durable restart.

Rust 1.88.0, locked offline dependencies and existing build directories are used
on macOS AArch64 and Linux x86_64. Linux reuses the prior source-overlay directory,
not a clean Git checkout or installed runtime. Two preexisting files matched
their preceding recorded digests before transfer; the three new files were
absent. All five transferred files matched final local SHA-256 digests. No
server deployment configuration or credentials changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 827 | 827 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Engine harmonic integration target | 16 | 16 | Existing serialized/workflow regression scope |
| Public complex-force integration target | 7 | 7 | Existing public regression scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets only |
| Total | 1,591 | 1,591 | Not full workspace or all-physics coverage |

The new profile selects ten new units, ten existing complex-range/bit units and
23 existing public/Engine integration tests. These 43 tests are already included
in the table, not additional tests to sum again. Historical reports retain
their own source provenance, algorithms and counts.

The native catalog validates 93 profiles without executing them; the new profile
separately executes all three commands and its 43 selected tests successfully.
macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. The 26-file HTML book, documentation inventory and
800/2,000-line organization audit pass with zero tracked line-limit debt.
Tensor structure and command checks pass for 13 modules and 11 paradigms;
qualification remains blocked with zero structural gaps, two maturity gaps,
23 evidence-grade gaps and 18 P0 gaps. No release-grade promotion or threshold
change is attributed to this local evidence.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_accumulation
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_complex_force_range --test harmonic_constraint_admission
```

Profile: `harmonic-complex-exact-accumulation-local-reliability`, in the registered
`harmonic-cancellation.json` shard. The implementation and tests are separate
small files below the 800-line source/config limit.

## Remaining Boundary

This is bounded exact accumulation of aligned harmonic components, not exact assembly or division, arbitrary precision, a forward-error certificate, independent physical qualification or durable restart.

It cannot recover information already lost in assembly, row normalization,
earlier factor updates or stored unknowns. Reversible alignment/range admission,
final restoration and solution division retain their existing policies. This
is not formal verification of the entire accumulator or solver. Physical and
material reference comparisons, other numerical consumers, scale performance,
installed/SDK journeys and persisted process/power-loss continuation remain
separate work.
