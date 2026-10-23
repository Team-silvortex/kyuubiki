# Tridiagonal Back-Update Underflow Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[forward-underflow repair](tridiagonal-forward-underflow-reliability-20261023.md).
No version bump, commit, package or service deployment is included.

## Reproduced failures

Three initial tests failed before the repair. Let `q` be the smallest positive
subnormal and `m` the minimum positive normal value.

- `A = [[0.5,0.5],[0,1]]`, `b = [0,q]` has the representable solution `[-q,q]`.
  The numerator `-q/2` rounded to zero before division, and the old result
  `[0,q]` could pass the quantized-tail admission allowance.
- `A = [[0,0.5,0],[1,0.25,0.5],[0,0,1]]`, `b = [q,0,q]` needs a row swap and
  two superdiagonal products. Their half-quantum contributions were lost,
  causing backward-error rejection of a system with solution `[-q,2q,q]`.
- Replacing the second row with `[0.5,0.25,-1]` and the final load with `m`
  gives `[2m-q,2q,m]`. The old result's first value was `2m`: an earlier tiny
  update was lost although the final numerator and quotient were normal.

Both signs are exercised. These are floating-point arithmetic constructions,
not evidence of extreme physical-load qualification.

## Repair and organization

`finish_back_substitution` now owns the ordered FMA loop for ordinary, pivoted
and forward-replay back rows. Callers no longer compute and discard a sequence
of intermediate numerators before asking for range checks. Each update is
checked; verified early loss remains recorded even if a later update is normal.

The finite-update probe and mantissa/exponent value moved without algorithmic
changes to `chain_tridiagonal_range_value.rs`. Forward and back substitution
use that same implementation rather than separate range thresholds. Ordinary
normal updates, exact cancellation and exactly represented subnormal updates
keep their FMA order and signed-zero behavior.

Rows with a finite result but verified intermediate loss use the existing
compensated row-range arithmetic and the chain's original-equation rounding
policy. Nonfinite results retain their previous strict range entry point.
No factors are recomputed and no dense fallback or full vector replay is added
for a back-row repair. Extra recovery arithmetic/storage is bounded by the
one or two superdiagonal terms, not matrix size. This is not a latency or RSS
benchmark claim. The solver/operator/engine boundary is unchanged.

Original-equation admission, tolerances, final quantized-tail policy and the
dense/banded strict erased-result policy remain unchanged. Existing stage-83
entry and row-pass checkpoints support cancellation without publishing a
partial solution or mutating factors. No new persistent checkpoint is implied.

## Regression controls

Twelve unit tests and two integration tests cover:

- Odd-subnormal magnitudes, both signs, ordinary and swapped factors, including
  row-equilibrated copies of the pivoted construction.
- Both superdiagonal products, early loss followed by a normal update, fused
  cancellation, and values rounded up to the minimum normal boundary.
- Exact FMA/signed-zero parity without range entry and an unchanged finite
  probe whose insignificant distant tail cannot be aligned reversibly.
- A 2,048-row geometric back tail, compared with binary powers, plus an
  independent erased load that still fails original-equation admission.
- Back-row recovery while a separate block is undergoing forward-overflow
  replay; no global RHS scaling is introduced.
- Entry/pass/completion cancellation and late cancellation in a 514-row block
  system, followed by exact clean reuse of the same factors.
- Deliberately corrupted original equations, true final-response overflow,
  irreversible recovery alignment, and clean replay after each failure.
- Two-, three-, 17- and 65-row normal-load/dense-reference cross-checks over
  five odd-subnormal load magnitudes and both signs.
- A three-node spring chain with stiffnesses `0.5` and `0.25`, loaded at the
  tip: free displacements are `2*load` and `6*load`, and both forces equal the
  load. Borrowed/owned calls, reversed node/element orientation, operator
  dispatch, workflow output and serialized JSON round trips are covered.

## Validation

The final implementation passed the same regression scope on macOS AArch64
and Linux x86_64 with the pinned Rust 1.88.0 toolchain:

| Scope | macOS passed | Linux passed | Exclusions per platform |
| --- | ---: | ---: | --- |
| Solver library | 663 | 663 | 9 ignored; 292 modal-spectrum tests filtered out |
| Engine library | 711 | 711 | 1 ignored |
| 42 integration targets | 251 | 251 | None |
| Total | 1,625 | 1,625 | 10 ignored; 292 filtered out |

The 14 new tests are included in these totals, not added a second time. The
integration targets are listed in the
[row-validation report](tridiagonal-row-validation-reliability-20261023.md#reproduction)
and include the 10,000-row transient chain and 52 live-Agent cancellation tests.
This is not a full-project run: the long modal-spectrum matrix and opt-in
benchmarks were not run.

Local solver/engine all-target Clippy with warnings denied, workspace formatting
and patch-whitespace checks passed. Affected source/configuration files remain
within the 800-line cap. Existing local and remote build caches were reused;
no new build roots or retained numerical dumps were created.

The native operator-validation metadata check passed for 74 profiles with
`executed=false`; it validates registration, not numerical execution. The
coverage tensor structure/command checks passed for 13 modules and 11 paradigms.
Its overall status remains `blocked`, with 0 structural gaps, 2 maturity gaps,
23 evidence-grade gaps and 18 P0 gaps. This bounded repair does not promote the
whole numerical capability to complete or physically qualified.

## Remaining limits

This does not add arbitrary mantissa precision or solve factorization/RHS
equilibration range loss. Unknowns are still ultimately f64 values. A detected
loss that cannot be repaired with reversible common-exponent alignment returns
an error: e.g. the pivoted early-loss construction above with final load `1`
instead of `m`. Such conservative rejection is intentional, not evidence that
the mathematical system lacks a finite solution. Finite probes that cannot
establish an improvement retain their original FMA and admission gate, as in
the earlier forward repair. General conditioning and physical accuracy require
separate qualification. Cancellation observations are not durable restart.

## Reproduction

From the repository root with the pinned Rust toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib back_underflow -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
cargo fmt --manifest-path workers/rust/Cargo.toml --all -- --check
```

The linked row-validation report retains the 42-target integration command.

This is bounded back-update underflow recovery, not arbitrary precision, general factorization range, physical qualification or durable restart.
