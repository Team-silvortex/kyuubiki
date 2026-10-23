# Sparse Erased-Load Admission Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[prescribed-value consistency repair](prescribed-value-consistency-reliability-20261023.md).

## Reproduced failures

Six tests failed before this repair: four solver unit tests and two integration
tests. Let `q` denote the smallest positive subnormal.

- For `A=[4]`, `b=[q]`, candidate `x=[0]`, the ordinary relative residual was
  reported as zero. A zero unknown's half-quantum budget was `4*q/2 = 2q`,
  which incorrectly covered the entire nonzero load.
- Explicit binary-range row validation and the compressed iterative candidate
  gate accepted the same erased response.
- A 1,025-DOF branched sparse system accepted an independent `q` load with
  zero response. The matrix forces the iterative backend; the valid equations
  beside the tiny load did not prevent this false success.
- The same failure reached a real branched spring operator and engine dispatch.
  The new workflow regression first failed because direct dispatch accepted the
  erased load. The corrected tests check failure at both dispatch and workflow
  entry, then replay with a representable normal load.

This is an extreme floating-point range construction, not a material-load
qualification. Its exact response `q/4` is outside f64; the required behavior
is rejection rather than successful publication of zero.

## Shared admission policy

`row_solution_roundoff` now supplies the common row policy to ordinary sparse
validation, binary-range validation and compressed candidate equilibrium.
When the expected load is nonzero and the row has no nonzero response product,
its unknown-rounding allowance is zero. Other equations cannot lend their
response or load to this test.

The ordinary pass tracks response products in the existing ordered scan. Tiny
products enter the existing range pass; that pass records response presence
using product mantissas before restoring their exponents. Thus actual nonzero
terms that cancel are distinct from a row with no response at all.

Raw residual values, tolerance constants and the representation allowance for
non-erased rows remain unchanged. Valid zero-load rows and quantized responses
remain admissible under the existing policy. No new vector buffer, recovery
pass, solver stage or factorization fallback is added. The extra state is one
row-local boolean; the existing ordinary residual buffer remains O(n).
No timing or peak-RSS improvement is claimed.

The final original-matrix gate now rejects the tested independent load after
it disappears in earlier scaling. This repair does not restore that load or
extend RHS storage. It also does not reconstruct every pre-elimination system
and its prescribed values for a complete constraint-equation audit.

## Added coverage

Eleven tests were added: nine unit tests and two integration tests.

- Ordinary, explicit range and compressed negative controls for a missing
  response, including a nonzero unrelated equation.
- A whole-matrix range fallback with exactly cancelling overflowing terms,
  preserving the independent weak row and its raw residual.
- Zero loads, signed zeros, exact subnormal responses, legitimate rounded
  responses and a larger wrong-response negative control.
- Nonzero cancelling terms and candidate RHS scales, retaining existing
  normalization-loss rejection.
- 2,049-entry ordinary/range rows, cancellation during wide and second range
  passes and row completion, followed by clean admission replay.
- Direct and prepared 1,025-DOF iterative solves with Jacobi, symmetric
  Gauss-Seidel and incomplete-Cholesky preconditioners, then successful reuse.
- A 1,025-free-DOF spring network with both load signs, reversed numbering and
  element orientation, borrowed/owned APIs, serialized input, direct dispatch,
  workflow rejection and successful replay after the load is corrected.

Existing long-chain quantized-tail and product-underflow controls remain part
of the regression scope. They are not replaced by these new examples.

## Validation

The final source passed on both local macOS and the remote Linux test copy.
The new tests are included in these counts, not counted a second time.

| Scope | macOS passed | Linux passed |
| --- | ---: | ---: |
| Solver library, excluding `modal_frame_spectrum::` | 703 | 703 |
| Engine library | 711 | 711 |
| 67 solver/engine integration targets | 333 | 333 |
| Total | 1,747 | 1,747 |

Each platform had zero failures, ten ignored library tests and 292 filtered
modal tests. The integration set includes the 10,000-node transient chain and
52 live-agent cancellation cases. The excluded modal matrix, opt-in benchmarks
and general physical qualification were not run as part of this repair.

Local all-target solver/engine Clippy passed with warnings denied; workspace
formatting and whitespace checks also passed. Native operator-profile metadata
validation passed for 78 profiles without executing their registered commands.
Coverage-tensor structure and command checks passed, with zero structural gaps,
two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps across 13 modules and
11 paradigms. Its overall status remains `blocked`, not fully qualified.

All changed source/config files remain within the 800-line limit and this report
within the 2,000-line documentation limit. Existing build caches and the remote
test copy were reused; no new package archive or numerical dump was produced.

## Scope and remaining limits

This closes the complete-erasure allowance hole for the supplied sparse
equations. It does not establish general conditioning, arbitrary precision,
or physical correctness of every admitted model. Existing range arithmetic
can reject irreversible alignment or unrepresentable raw residuals.

Earlier RHS scaling, assembly and general original constrained-system
validation remain separate work. Rejection is not a claim that every such
mathematical system lacks a finite solution. Cancellation checks are
cooperative observations rather than durable restart points.

## Reproduction

From the repository root with the pinned toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib linear_algebra::residual::erased_load_tests -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine \
  --test spring_mixed_scale_equilibrium --test spring_mixed_scale_workflow \
  erased_load --no-fail-fast -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
```

The full integration scope is the same 67-target set in the
[sparse reduction report](sparse-prescribed-reduction-reliability-20261023.md#reproduction),
with this repair's new tests included in the existing spring targets.

This is bounded erased-load admission evidence, not general original-equation constraint validation, arbitrary precision, physical qualification or durable restart.
