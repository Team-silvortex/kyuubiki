# Harmonic Complex Arithmetic Range Recovery

Date: 2026-10-23

Version line: daji 3.6.x, after `0a65f2e8`, extending the uncommitted
[original-equation gate](harmonic-original-equation-admission-20261023.md) and
[frequency-product repair](harmonic-frequency-product-range-20261023.md).
The engine, TaskIR, operator payloads and result formats are unchanged.

## Reproduced Failures

Four solver unit controls and one public solve failed before production changes:

- `(q + i q) / (1 + i)` returned zero rather than `q`, where `q = 2^-1074`.
  Applying the two one-half weights separately erased both contributions.
- `(2^-600 + i 2^-600) * (2^-475 - i 2^-475)` likewise returned zero rather
  than real `q`. Both real products were half-quanta before their addition.
- Dense back substitution on a triangular three-row system first overflowed
  while evaluating `1e308 - (-1e308)`, then could not recover when subtracting
  the final `1e308`. The complete numerator and final solution were finite.
- An adjacent-pivot tridiagonal fixture reached the same two-upper-term pattern
  after row interchange. Its back substitution also returned an error.
- A public damped spring solve with load `q`, stiffness/damping `2^-600`, mass
  `2^-700` and angular frequency one passed original-equation admission, but
  published zero element force. The correct rounded force amplitude is `q`;
  the tiny inertial correction is far below one output quantum.

The first four controls target generic complex arithmetic/factorization internals,
not independent physical spring assembly. Their matrices need not be symmetric
physical spring matrices. The fifth case demonstrates an actual public result
publication defect rather than only fault-injected internal state.

## Recovery Contract

`harmonic_spring_complex_range.rs` now provides checked complex product, division,
elimination update and complete-row substitution. Unchecked multiplication and
division operators were removed from the internal complex type so production
consumers must choose a checked operation. Ordinary finite normal paths retain
their previous arithmetic order and bits; extreme paths replay their operands.

Exceptional scalar products are split into binary mantissas/exponents. Two-pass
alignment retains fused-multiply-add product tails and compensated sums. Real and
imaginary numerators are accumulated separately, without first materializing
overflowing or underflowing unscaled products. Alignment must be reversible;
unresolved range is an error, not permission to drop a contribution.

An elimination update recovers the complete `base - factor * value` expression,
then requires its final coefficient/RHS components to restore reversibly. Dense
substitution replays all upper terms, and the pivoted path replays both ordinary
and second-superdiagonal terms. Their complex division occurs before restoring a
wide temporary numerator. The conjugate numerator and squared pivot norm remain
binary-scaled until the final component quotient is restored.

Nonzero rounded subnormal *unknowns* remain admissible candidates, as before,
because the public operator still checks their complete original equations.
True quotient overflow or erasure returns an error. Product/update components,
including reported element-force products, require reversible final restoration.
Thus quarter-quantum per-element forces are rejected instead of reported as zero.
No tolerance relaxation, regularization, hidden static approximation or alternate
physical model was introduced.

Node displacement amplitude is checked before velocity/acceleration products.
Element extension amplitude is checked before force multiplication, and final
force amplitude has its own finite check. A finite weak-spring force does not
justify publishing an unrepresentable extension field. Errors identify the
frequency and applicable node/element field at public entry points.

## Storage And Recovery

Exceptional rows use cloneable borrowed iterators and constant scalar scratch,
not a copied matrix, cached products, extra factorization or a doubled-real system.
Dense and tridiagonal layout/size limits are unchanged. Ordinary scans additionally
check cancellation in bounded chunks. Exceptional scans use existing factor,
substitution-range and element-result stages; no checkpoint IDs were renumbered.

Recovery scans add CPU work when needed. Normal operations also have additional
finite/range predicates; this report claims no speedup or unchanged throughput.
Cancellation or numerical failure returns an error and no successful partial
sweep/force array. Same-input replay remains independent of persistent mutable
solver state, but is not durable process recovery.

## Added Tests

Sixteen tests comprise ten solver units, three solver integrations and three
engine integrations:

- The four internal arithmetic/substitution regressions above.
- An overflowing complex product cancelled by a finite base; this also reaches
  both dense and tridiagonal forward updates through finite-component fixtures.
- A temporary numerator with components `2.25e308` divided by `1.5 + 1.5 i`
  before restoration, producing finite real `1.5e308`.
- Ordinary product/update/division bit identity, invalid operands and pivots,
  true overflow/erasure and a retained nonzero quantized unknown.
- A synthetic 2,050-term exceptional row cancels at entry, interior, first-pass
  completion and second-pass completion, and separately in all five numerator/
  denominator/division phases, followed by identical replay. This is helper-level
  cancellation evidence, not support for a dense public matrix above its limit.
- Public quantum-force recovery under signed loads, reversed element orientation
  and borrowed/owned calls; four parallel quarter-quantum elements fail explicitly.
- Finite opposite nodal displacements near `1e308` on weak springs demonstrate
  that unrepresentable link extension is reported separately from force overflow.
- Serialized engine/workflow quantum-force equality on path and cyclic dense
  topology. Weak links make the latter a bounded publication control, not a
  conditioning/performance certificate for strongly coupled quantum-scale systems.
- Force restoration failure propagates through both engine entries; cancellation
  inside force recovery, after result-stage entry, prevents publication and clean
  replay matches the baseline through both matrix paths.

## Validation

Pinned Rust 1.88.0, offline locked dependencies and existing build directories:

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed |
| --- | ---: | ---: |
| Solver library excluding `modal_frame_spectrum::` | 786 | 786 |
| Engine library | 711 | 711 |
| Engine harmonic integration target | 10 | 10 |
| New solver complex-force integration target | 3 | 3 |
| Existing frequency-range integration target | 7 | 7 |
| Existing harmonic balance/reliability and dynamic closed forms | 23 | 23 |
| Non-overlapping total | 1,540 | 1,540 |

Both platforms have zero final failures, ten ignored library tests and 292 filtered
modal cases. The sixteen new cases are included once, not added again. Existing
resonance, power balance, extreme frequency products, quantized finite response,
cancellation/replay and the 10,000-node shuffled path forest remain green. Local
Solver/Engine all-target Clippy passes with warnings denied.

Native metadata checks pass for 88 validation profiles (`executed=false`), and
all three commands in the new profile pass with `--execute`. Their focused
reruns are not counted again. Tensor structure/command checks pass for 13 modules
and 11 paradigms, with zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps; overall readiness remains `blocked`. Project organization,
formatting/whitespace, the 26-file HTML book and documentation inventory pass.
Source/config files remain below 800 lines and documents below 2,000.

Six numerical source/test files were synchronized to the existing Linux test tree,
without deleting remote content, deploying services, modifying credentials or
server configuration, or creating another cache/build root. No application
package, version bump, staging operation or commit was created.
All six remote source/test SHA-256 hashes match the local final files. Small
generated check reports remain in ignored `tmp/` storage, not Git.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib \
  --test harmonic_complex_force_range --test harmonic_frequency_range \
  --test harmonic_constraint_admission --test harmonic_spring_balance \
  --test harmonic_spring_reliability --test dynamic_spring_closed_form \
  --no-fail-fast -- --skip modal_frame_spectrum:: --quiet
```

## Limits

This is not a new arbitrary-precision complex matrix engine. In particular,
materialized updated coefficients/RHS components must still fit in f64; a truly
out-of-range updated RHS is rejected even if later row scaling or another
representation might recover a final solution. Initial row scaling, coefficient
assembly accumulation, conditioning/pivot policy and all normal-path cancellation
accuracy are not fully qualified by this repair. Wide alignment may conservatively
reject extreme contrasts. Final force/extension fields still must fit their
existing formats. Shared coefficient inputs and arithmetic are not independent
physical assembly. No Windows run, performance certification or durable restart
evidence was added.

This is bounded harmonic complex arithmetic range recovery, not general complex-solver qualification, arbitrary precision, independent physical assembly or durable restart.
