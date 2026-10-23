# Tridiagonal Forward-Underflow Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[forward-overflow repair](tridiagonal-forward-range-reliability-20261023.md).
No version bump, commit, package or deployment is included.

Follow-up: [back-update underflow recovery](tridiagonal-back-underflow-reliability-20261023.md)
extracts the unchanged finite-update probe and work-value arithmetic to
`chain_tridiagonal_range_value.rs`, shared by forward and back substitution.
It also addresses the finite back-update limitation recorded below.

## Reproduced defect

Three initial tests failed before this repair. With `q` the smallest positive
subnormal, `A = [[1,0],[0.5,0.5]]` and `b = [q,0]` have the exactly representable
solution `[q,-q]`. The forward work entry `-q/2` rounded to zero before division
by `0.5`, producing `[q,0]`. The existing row-local allowance for quantized
unknowns admitted this result; this was silent loss, not just a rejected solve.

An adjacent-swap construction `A = [[0.5,0.5],[1,0]]`, `b = [0,q]` showed the
same loss. A three-row triangular system propagated two erased entries.
Odd multiples of `q` also exercise nonzero but incorrectly rounded work values.
These are arithmetic boundary constructions, not physical-load qualification.

## Repair

Both ordinary and adjacent-pivot forward updates now probe results with
magnitude at or below the minimum normal value when the eliminated product is
nonzero. The minimum-normal boundary is included because a subnormal result can
round up to that value. Normal results above the boundary, zero factors and
zero previous work retain their ordered-FMA path.

The probe uses the existing per-entry mantissa/exponent arithmetic without
allocating a work vector. Only a verified difference or an intermediate not
exactly representable as f64 triggers replay. Exact cancellation and exactly
represented subnormals do not trigger replay. If a finite probe cannot align
its terms reversibly, it cannot establish an improved intermediate: the
ordinary FMA and original-equation gate remain in use. This is not an assertion
of extra precision for arbitrarily distant tails. Actual extended replay still
rejects irreversible alignment instead of silently discarding a contribution.

Replay reuses the immutable factors and swap sequence, drops the ordinary
workspace first and carries extended work through successive forward steps.
Underflow and overflow can coexist in either block order without globally
rescaling the RHS or switching to a dense factorization.

Final output rounding is distinct from premature intermediate rounding.
`SubnormalResult::ValidateOriginalEquations` allows the chain's extended back
row to produce a rounded zero only subject to the unchanged original-equation
gate. This preserves legitimate long-chain quantized tails while still
rejecting an independent nonzero load whose response disappears. The default
dense/banded substitution entry point retains `RejectErasure`; it has not been
relaxed. Neither residual tolerances nor the validation algorithm changed.

The exceptional replay remains O(n) work/storage, with an f64/i32 work entry
per row and a separate f64 solution. Suspect finite probes add constant-size
arithmetic without allocation; no throughput or RSS improvement is claimed.
Stage 84 observations, phase boundaries and cancellation remain unchanged.
They are cooperative diagnostics, not durable/power-loss checkpoints.

## Regression controls

Twelve unit tests and two integration tests cover:

- Both signs, erased and rounded odd-subnormal work, ordinary and swapped rows.
- Successive intermediates below the smallest float and nonzero-RHS FMA
  cancellation, including the `2^-27` product-tail construction.
- A value rounded up to the minimum normal number before division.
- Bitwise exact-zero/exact-subnormal controls that forbid recovery entry, and
  an unalignable distant tail that must not become a new finite-path error.
- A 2,048-row geometric tail, compared row by row with binary powers, plus
  rejection and clean replay of an independent erased load.
- Simultaneous overflow/underflow blocks in both orders.
- Ordinary and pivoted 514-row systems cancelled at entry, conversion, forward,
  backward and completion checkpoints, followed by exact factor reuse.
- Original-equation corruption/recovery with ordinary and equilibrated factors.
- Two-, three-, 17- and 65-row cross-checks against normal-load scaling and a
  separate dense-LU solve, with six odd-subnormal magnitudes and both signs.
- A three-node spring chain with a loaded middle node and an unloaded tip:
  both free displacements must equal twice the load, the anchor force equals
  the load and the final spring force is zero. Borrowed/owned solver calls,
  reversed node/element orientation, operator dispatch, workflow output and
  serialized JSON round trips retain these values.

The wider regression also retains the previous overflow, strict row-range,
ordinary FMA/signed-zero, workflow admission and live-agent cancellation tests.

## Validation

Completed with Rust 1.88.0, locked offline dependencies and existing caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver library excluding `modal_frame_spectrum::` | 651 passed, 9 ignored | 651 passed, 9 ignored |
| Engine library | 711 passed, 1 ignored | 711 passed, 1 ignored |
| 42 integration targets, including the two new consumer cases | 249 passed | 249 passed |
| Total excluding repeated focused runs | 1,611 passed, 10 ignored | 1,611 passed, 10 ignored |

The fourteen new tests are included above, not counted again. The original
42-target run passed 247 cases; the two updated consumer targets were rerun
with their additional tests, replacing their earlier 6/2 counts with 7/3.
Coverage includes the 10,000-node transient chain and 52 live-agent cancellation
tests. The completed solver run filters out 292 modal-spectrum cases; the long
modal matrix and opt-in performance benchmarks were not run. This is bounded
regression, not a full-project coverage or maturity claim.

Local Clippy passed for all solver and engine targets with `-D warnings`.
Formatting and whitespace checks passed. Source/config files changed in this
iteration remain below 800 lines, and reports below 2,000 lines. No new build
root, retained numerical dump, application package or server configuration was
added. Linux validation reused the existing CI test copy without deployment.

Native operator validation passed with 73 profiles and `executed=false`.
Tensor structure/command checks passed: 13 modules, 11 paradigms, 0 structural
gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall readiness
remains blocked. Metadata checks are separate from the executed numerical
tests; this component evidence does not promote global readiness.

## Remaining limits

Factor formation and RHS equilibration may still reject range loss before
substitution. Finite back-substitution numerators that remain on the ordinary
path are not comprehensively given extended precision by this change. Mantissas
remain f64; extreme term separation, conditioning and physical correctness
still require independent validation. A failed finite probe does not establish
that its discarded precision is recoverable by this algorithm.

## Reproduction

Run from the repository root with the pinned Rust toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib forward_underflow -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
cargo fmt --manifest-path workers/rust/Cargo.toml --all -- --check
```

The integration command is retained in the linked row-validation report.

This is bounded forward-underflow recovery, not arbitrary precision, general factorization range, physical qualification or durable restart.
