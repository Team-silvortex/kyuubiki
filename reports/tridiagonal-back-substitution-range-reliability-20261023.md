# Tridiagonal Back-Substitution Range Reliability

Follow-up: [back-update underflow recovery](tridiagonal-back-underflow-reliability-20261023.md)
extends this iteration's nonfinite-only trigger to verified loss in finite
ordered back updates. The original scope and counts below remain historical.

Date: 2026-10-23

Version line: daji 3.6.x, following `9aed9a2f` (`daji 3.6.8`) and the
uncommitted [row-validation repair](tridiagonal-row-validation-reliability-20261023.md).
No version bump, commit, packaging or deployment is included.

## Reproduced failures

Three new tests failed with `tridiagonal back substitution diverged` before
the repair. Let `h = 2^1023` and `d = 2^-27`:

- `A = [[8,-7],[-7,8]]`, `b = [h,h]` has the finite solution `[h,h]`.
  The first back-substitution numerator is `8h`, which overflows before its
  division by 8. The same solution is also checked after scaling A and b by
  `2^-600`, exercising row equilibration.
- `A = [[0,1,0],[8,-7,-7],[0,-7,8]]`, `b = [h,0,h]` has the finite
  solution `[1.75h,h,h]`. Adjacent pivoting creates a second superdiagonal;
  its two large terms overflow before the final division by 8.
- `A = [[0,1,0],[1,2(1+d),-2],[0,0,1]]`, `b = [h(1-d),0,h]` has the
  solution `[2^970,h(1-d),h]`. Two overflowing terms cancel, leaving a
  nonzero fused-product tail which must not be rounded away.

Both load signs are tested. These are exact arithmetic constructions, not
material models or high-load physical qualification cases.

## Repair boundary

Ordinary and pivoted chains share `finish_back_substitution`. The original
ordered FMA numerator and division remain the fast path. Only a non-finite
result invokes the existing `linear_substitution::solve_row` algorithm, which
combines binary exponents before restoration and retains compensated product
tails. No independent arithmetic implementation or dense fallback is added.

The row iterator contains at most two coefficient/response pairs. Recovery
requires constant auxiliary storage; existing factor and solution storage are
unchanged. No throughput or peak-memory speedup is claimed without a benchmark.

Finite candidates, including rounded zero and subnormal tails, continue through
the row-local original-equation admission introduced in the preceding repair.
This change does not introduce a different underflow policy for those rows.
Recovered candidates also pass that gate; a deliberately corrupted saved
equation is still rejected. True response overflow and invalid RHS values are
errors, not saturated results.

Forward elimination, factor formation, RHS equilibration, pivot thresholds and
backward-error tolerances are unchanged. In particular, an unrepresentable
forward work-vector entry can still fail even when a mathematical final answer
would be finite. This is not a claim to recover every finite-solution system.
The engine/operator boundary is unchanged.

`TridiagonalSubstitutionRange` is appended as stage 83; older stage identifiers
are unchanged. Entry and both range-pass completion observations support
cooperative cancellation. They do not persist partially computed solutions.

## Regression controls

Eight tests are new in this iteration: seven chain tests and one stage-identity
test. In addition to the three reproduced failures, they check:

- Cancellation at entry, normalization completion and accumulation completion
  on both ordinary and pivoted recovery, followed by clean reuse of the factor.
- Bitwise parity with the previous ordinary FMA operations, including signed
  zero and zero-factor FMAs, without entering range recovery.
- Rejection against corrupted original equations, true overflow and invalid
  RHS, followed by successful replay with the same prepared factor.
- A 514-row block-tridiagonal system with 257 independent overflowing rows,
  late cancellation and clean replay, with dense fallback forbidden. This is a
  block-system traversal control, not a 514-node coupled physical benchmark.
- Stable observation identifiers and stage-name decoding.

## Validation

Completed on Rust 1.88.0 with offline locked dependencies and existing caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver library excluding `modal_frame_spectrum::` | 627 passed, 9 ignored | 627 passed, 9 ignored |
| Complete engine library unit suite | 711 passed, 1 ignored | 711 passed, 1 ignored |
| 42 solver/engine/CLI/agent integration targets | 247 passed | 247 passed |
| Total excluding repeated focused runs | 1,585 passed, 10 ignored | 1,585 passed, 10 ignored |

The eight new tests are included in the solver total, not counted twice.
The integration suite includes the 10,000-node transient chain and 52 live-agent
cancellation tests. The solver run deliberately excludes 292 modal-spectrum
tests; this iteration did not start the long-running full modal matrix. Existing
opt-in benchmarks remain ignored. No full-project or general physical
qualification is implied by these counts.

Local Clippy passed for all solver and engine targets with `-D warnings`.
Workspace formatting and whitespace checks passed. Changed source/config files
stay below 800 lines and reports below 2,000 lines. Existing build roots were
reused, without creating package archives or retained numerical input dumps.

Native operator validation passed with 71 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. These metadata checks are separate from
the executed tests and do not promote a general maturity or release claim.

## Reproduction

Use the pinned Rust toolchain from the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib -- --skip modal_frame_spectrum:: --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-engine --lib -- --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
cargo fmt --manifest-path workers/rust/Cargo.toml --all -- --check
```

The 42 integration targets and full command are unchanged from the
[preceding report](tridiagonal-row-validation-reliability-20261023.md#reproduction).
They cover physical consumers, workflow dispatch, CLI and live-agent cancellation.

This is bounded back-substitution recovery, not general factorization or forward-substitution range, conditioning, physical qualification or durable restart.
