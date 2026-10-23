# Banded Substitution Range Reliability

Date: 2026-10-23

Version line: daji 3.6.x, on top of the uncommitted tangent-validation changes
following `ec3260d5` (`daji 3.6.8`). No version bump is included.

Banded forward and transposed substitution now share the range-aware arithmetic
already used by dense LU. Tested finite large responses no longer need dense
fallback, and tested underflowed nonzero responses cannot silently become zero.

## Reproduced failures

Three new tests failed before the change:

- For `h = 2^1023`, `A = [[8,-7],[-7,8]]`, and `b = [h,h]`, the exact
  solution `[h,h]` is finite, but a banded back-substitution numerator overflowed.
  Previously only a size-limited dense fallback could recover this case.
- For `q` the smallest positive subnormal and
  `L = [[1,0,0],[0,1,0],[0.5,0.5,1]]`, solving `L L^T x = [q,q,q]`
  returned `[q,q,q]` instead of `[q,q,0]`. Both half-quantum products vanished
  before accumulation. The new test verifies the original equations as well.
- Solving `[4] x = [q]` returned zero instead of rejecting an unrepresentable
  nonzero response. The prepared factor must remain usable after rejection.

Both load signs are covered. These are arithmetic controls, not material models.

## Shared implementation and scope

`linear_substitution.rs` owns one row algorithm for dense slices and banded
iterators. The dense adapter retains dimension validation. Banded rows and
transposed columns are traversed directly, without a dense row copy or new
row-sized allocation. The existing two banded solution vectors are unchanged.

Ordinary rows keep their accumulation order and division. Overflow, subnormal
products or numerators, and nonzero-to-zero range loss trigger the existing
two-pass binary-exponent recovery. Compensated accumulation retains fused
product tails; the pivot exponent is combined before restoring the result.

The normalizer is intentionally conservative: if a nonzero contribution cannot
be normalized reversibly, it returns an error even when exact arithmetic might
ultimately recover a representable answer. True overflow, unrepresentable
nonzero results and invalid operands remain errors. This does not extend the
representable range of the forward vector or repair every factorization issue.

Factorization thresholds, tangent residual tolerances, dense size limits and
operator/engine separation are unchanged. `BandedSubstitutionRange` is appended
to the observation enum without renumbering older stages. Its observations are
cooperative cancellation points, not persisted restart checkpoints.

## Added controls

Eight new unit tests include the three reproduced failures and:

- A 513-dof tridiagonal chain with a known signed large solution, verified on
  the banded path with dense fallback disabled and original-row residual checks.
- A prepared factor whose transposed products overflow but leave an exact
  `2^970` fused-product tail. This isolates substitution from factor formation.
- A 1,026-row prepared factor with cancelling overflowing products in its last
  forward row. Cancellation at entry, within each range pass, and at completion
  returns an error; the same factor then replays correctly. Normalization loss
  is also rejected before a successful replay.
- Bitwise parity with the original ordinary substitution, including signed
  zero, with no range-pass observations.
- Invalid RHS dimensions, non-finite values and true result overflow followed
  by clean replay.

The previous tangent overflow test is strengthened to require banded success
with dense fallback disabled. Its original 2026-10-20 report records the earlier
repair stage; this report describes the follow-up behavior.

## Validation

Rust 1.88.0, offline locked dependencies and existing build caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 183 passed, 3 ignored | 183 passed, 3 ignored |
| Solver unit filter `solver_control` | 12 passed | 12 passed |
| Solver operator integration targets | 172 passed | 172 passed |
| Engine workflow targets | 17 passed | 17 passed |
| CLI operator targets | 26 passed | 26 passed |
| Total excluding repeated focused runs | 410 passed | 410 passed |

The eight new tests are included in `linear_`, not counted again. Three ignored
tests are existing opt-in benchmarks. This is targeted regression, not a
full-project test run or a measured performance improvement.

Local Clippy passed with `-D warnings` for solver/engine libraries and selected
cohesive, frame-continuation and buckling integration targets. Workspace
formatting and whitespace checks passed. Changed source/config files remain
below 800 lines and reports below 2,000 lines. No application package or new
build root was created.

Native operator validation passed with 69 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. These are metadata checks, separate from
the numerical executions above; this bounded component evidence does not
upgrade general physical maturity.

The 38 integration targets are the 26 targets in the
[tangent-validation run](symmetric-tangent-validation-reliability-20261020.md#reproduction),
plus the 12 buckling targets below. They cover beam/frame cross-checks, clustered
modes, sparse paths, input validation, workflow dispatch and CLI execution.

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib linear_ -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib solver_control -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-cli \
  --test buckling_beam_1d_closed_form --test buckling_beam_1d_input_reliability \
  --test buckling_frame_2d_closed_form --test buckling_frame_2d_input_reliability \
  --test buckling_frame_2d_portal --test buckling_beam_1d_clustered_mode \
  --test buckling_frame_2d_clustered_modes --test buckling_beam_frame_crosscheck \
  --test buckling_assembly_reliability --test buckling_beam_workflow \
  --test buckling_frame_workflow --test buckling_assembly_operator -- --quiet
```

This is bounded banded-substitution validation, not general factorization range, conditioning, physical qualification or durable restart.
