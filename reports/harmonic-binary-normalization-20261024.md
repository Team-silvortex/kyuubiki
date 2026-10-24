# Harmonic Binary Row Normalization

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted
[RHS range](harmonic-complex-rhs-range-20261024.md),
[coefficient range](harmonic-coefficient-normalization-range-20261024.md),
[zero](harmonic-complex-zero-cancellation-20261024.md),
[near-cancellation](harmonic-complex-near-cancellation-20261024.md) and
[aligned accumulation](harmonic-complex-exact-accumulation-20261024.md) follow-ups.
This changes Solver preparation and pivot-scale bookkeeping. Engine production
code, Agent, TaskIR, operator contracts and installed runtimes are unchanged.
No version bump, package publication or desktop rebuild is included.

## Reproduction Before Repair

The initial precision test failed; the initial pivot-floor control passed.
For `d=2^-27`, consider the manufactured triangular system
`3*x0+(1-d)*x1=1`, `x1=1+d`. Its exact weak response is `x0=d*d/3`.
Independently dividing the first coefficient and RHS by three rounds
`(1-d)/3` and `1/3` separately. Even exact downstream accumulation then evaluates
the altered expression, producing approximately `-1.8503717215e-17` instead of
the positive `1.8503717077e-17`. Signed controls expose the opposite sign error.

The matrix infinity-norm condition number is below four for these positive
`d` values, but the individual tiny response remains sensitive to cancellation.
This demonstrates componentwise information loss, not a large normwise error
of the whole solution: the other unknown is order one. No new incorrect
successful public physical model is claimed; these are internal manufactured
systems with independent closed-form references.

The final test repeats forward/backward triangular arrangements in both dense
and path implementations, real/imaginary RHS components, both signs, common
scales `2^-500`, one and `2^500`, and `d=1,3,5,7` times `2^-27`. The reference
`sign*d*d/3` uses the exact dyadic numerator and one final division; comparisons
allow four binary64 epsilons of relative error.

## Preparation Policy

For each row, preparation first considers the finite power of two at the
exponent of its original maximum component. A bounded preflight checks every
real/imaginary coefficient component. That divisor is selected only if each
quotient remains finite and multiplication by the divisor restores the original
binary64 value. The same chosen divisor then scales coefficients and RHS.
Normal binary shifts avoid the independent non-dyadic rounding in the example.

A binary shift is not always the best representable range choice. For example,
five minimum subnormal quanta divided by four cannot round-trip, while division
by five produces one quantum and restores the original five. In such a row the
legacy maximum-component divisor remains available. Its existing coefficient
range guards still run with prepared row/column/component diagnostics. Neither
candidate is permission to erase a tiny coefficient. Some rows rejected under
the old divisor may instead become representable under the binary divisor;
unchanged original-equation admission is still required for public publication.

The scalar `Complex::scaled` contract itself is unchanged. Preparation chooses
a different divisor, so whole solves are not promised to retain previous bits
or identical pivot histories. The legacy fallback also retains normal division
roundoff; exact normalization is not asserted for every supported row.

## Pivot Units, Storage And Cancellation

Binary normalization can put a row maximum between one and two rather than
exactly one. Comparing directly with the old pivot floor would silently change
admission units. Each row therefore retains `weight=original_maximum/divisor`.
Pivot scores are `complex_amplitude/weight`, expressed on the original
max-normalized scale. Dense and adjacent path swaps move the weight together
with matrix rows and RHS identity. The constant remains `32*epsilon*size`;
no deliberate pivot-floor relaxation or regularized fallback is introduced.
Floating-point boundary decisions are not claimed bitwise identical for every
possible input.

The path implementation reuses its existing row-scale vector for weights after
preparation. Dense solving adds one `f64` per row, not another matrix. Preflight
adds a row scan and bounded preparation checkpoints. There is no matrix snapshot,
new persistent state, stage ID, retry policy or Engine-owned numerical algorithm.
Additional scanning, scoring and metadata have costs; no performance neutrality
or speedup is claimed without separate measurement.

## Controls And Validation

Eight new tests cover:

- Signed/scaled weak-response closed forms before forward/backward cancellation.
- Rejection at 0.9 times the original pivot floor and admission at 1.1 times,
  in dense/path cases with and without row swaps.
- Exact binary scale selection with explicit weights, and a separate reversible
  legacy subnormal-coupling fallback.
- Invalid maxima and single-unknown controls from a minimum subnormal through
  the largest finite coefficient, including complex coefficients and both signs.
- A 2,050-entry preflight cancelled at bounded checkpoints, followed by fresh
  same-input selection. Partial preparation is private and never returned.
- Five different complex row phases with non-power-of-two maxima, an initial adjacent
  pivot and a generated second superdiagonal. Both solvers recover a prescribed
  real solution. Integer/half-integer matrix products construct an exact RHS
  independently of preparation and factorization.

Pinned Rust 1.88.0 and locked offline dependencies use existing build directories
on macOS AArch64 and Linux x86_64. Linux reuses the previous source-overlay
workspace, not a clean Git checkout or installed runtime. The existing linear
source matched its preceding digest before transfer; the two new files were
absent. All three final transferred files matched local SHA-256 digests.
No deployment configuration or credentials changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 835 | 835 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Engine harmonic integration target | 16 | 16 | Existing serialized/workflow regression scope |
| Public complex-force integration target | 7 | 7 | Existing public regression scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets only |
| Total | 1,599 | 1,599 | Not full workspace or all-physics coverage |

The new profile selects eight precision/pivot units, seven existing coefficient
range units, ten existing arithmetic-range/bit units and 23 existing public/Engine
tests. Its 48 tests are a subset of the table, not additional coverage to sum.
Historical reports retain their own algorithms, source provenance and counts.

The native catalog validates 94 profiles without executing them. The new profile
separately executes all four commands and its 48 selected tests successfully.
macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. The 26-file HTML book, documentation inventory and
800/2,000-line organization audit pass with zero tracked line-limit debt.
Tensor structure and command checks pass for 13 modules and 11 paradigms;
qualification remains blocked with zero structural gaps, two maturity gaps,
23 evidence-grade gaps and 18 P0 gaps. No qualification threshold or release
grade is promoted by this local evidence.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_normalization_precision
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_coefficient_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_complex_force_range --test harmonic_constraint_admission
```

Profile: `harmonic-binary-normalization-local-reliability`, in the registered
`harmonic-cancellation.json` shard. Shared policy, integration into both
factorizations and mathematical tests remain separate small source files.

## Remaining Boundary

This is bounded harmonic binary-normalization hardening, not exact factorization or division, a componentwise forward-error certificate, independent physical qualification or durable restart.

It does not recover previously rounded assembly terms, represent every extreme
contrast, eliminate legacy-fallback roundoff or certify all tiny unknowns after
factorization. Physical/material references, all-model forward-error budgets,
scale benchmarks, installed/SDK journeys and persisted continuation remain
separate work. Cancellation controls are in-process cooperative tests, not
process-kill or host-power-loss recovery evidence.
