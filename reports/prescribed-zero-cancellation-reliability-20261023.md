# Prescribed Zero-Cancellation Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[erased-load admission repair](sparse-erased-load-admission-reliability-20261023.md).

## Reproduced failures

Six new tests failed before the runtime change: four solver unit tests, one
solver integration test and one engine integration test. All returned a zero
load or response instead of the closed-form nonzero value.

- Let `d=2^-27`, coefficient `c=s*(1+d)`, prescribed value `g=1-d` and load `s`.
  Rounded multiplication gives `c*g=s`, so ordinary subtraction yields zero.
  With the finite input values treated as exact data, `s-c*g=s*d*d`, not zero.
- The shared sparse reducer erased this tail before solving; the chain path
  erased it before segment solution. Existing original boundary validation
  also admitted zero because its relative backward error was small.
- For the addition-loss example `s-s*2^60+s*2^60`, the ordinary running RHS
  became zero instead of `s`. Both prescribed-value backends reproduced it.
- A real magnetic bar with a fixed potential `-(1-d)`, unit source and
  permeance `1+d` returned a zero free potential. Its finite expected response
  is `d*d/(1+d)`. This reached direct engine dispatch and workflow output.

These are input-defined cancellation controls, not a physical material
qualification. Unlike the preceding erased-load admission issue, the key
normal-scale example needs neither product overflow nor subnormal arithmetic.

## Implementation

Both prescribed RHS paths now mark a row exceptional whenever subtracting a
nonzero boundary product produces zero. The flag persists if this is only an
intermediate update and a later boundary makes the ordinary RHS nonzero again.

The existing compensated binary-range arithmetic replays the original load
and every prescribed product, retaining both product and accumulation tails.
Only changing the final subtraction to FMA would not recover a contribution
already lost earlier in the row. No new arithmetic kernel is introduced.

The sparse reducer preserves the free coefficients and their symmetry and
requires its accumulated load to fit f64 reversibly. The chain keeps its
existing affected-row normalization and can divide a wider numerator before
restoring the finite unknown. For example, the sparse path rejects a
quarter-quantum reduced load while the chain can recover its normal quotient.
Irreversible common-exponent alignment remains a fail-closed error.

Ordinary nonexceptional rows keep their prior multiplication/subtraction order.
Truly exact cancellation is also replayed and still produces zero. Fixed
reaction rows are not required to have zero residual. No tolerance change,
dense conversion, new factorization fallback, or independent-row rescaling is
added. Result publication remains behind existing validation and cancellation.

The runtime change adds a predicate per prescribed update. Exceptional sparse
rows reuse the existing row-sized operand buffer; exact cancellations can now
incur that extra replay even when their answer really is zero. Chain boundary
storage remains fixed-sized. No timing or peak-RSS improvement is claimed.

## Added coverage

Fourteen tests were added: six sparse unit tests, six chain unit tests, one
solver integration test and one engine integration test.

- Positive and negative product-tail controls at binary coefficient scales
  from `2^-1022` to `2^600`, including exact minimum-quantum sparse restoration
  and a smaller unrepresentable load that must be rejected.
- Addition loss over both boundaries, and intermediate zero followed by a
  later small boundary contribution; the latter must not suppress full replay.
- True zeros, independent minimum-quantum loads and unchanged sparse inputs.
- Rejected irreversible alignment, then a successful supported subsequent call.
- A 2,049-entry sparse row with cancellation during the original scan, operand
  collection and both compensated passes, followed by clean input reuse.
- Chain cancellation during recovery and final publication, then clean replay.
- Real magnetic paths and four-arm junctions at three coefficient scales,
  both signs, reversed element orientation, borrowed/owned APIs, serialized
  dispatch, workflow artifact equality and result JSON round trips.

The existing ordinary arithmetic bit controls and valid subnormal response
tests remain in the broad regression set.

## Validation

Final source validation used pinned Rust 1.88.0, offline locked dependencies
and existing build caches on local macOS AArch64 and remote Linux x86_64:

| Scope | macOS passed | Linux passed |
| --- | ---: | ---: |
| Solver library, excluding `modal_frame_spectrum::` | 715 | 715 |
| Engine library | 711 | 711 |
| 67 solver/engine/CLI integration targets | 335 | 335 |
| Total | 1,761 | 1,761 |

Each platform had zero failures, ten ignored library tests and 292 filtered
modal-spectrum tests. The fourteen new tests are included, not counted again
from focused reruns. Integration coverage includes the 10,000-node transient
chain, 52 live Agent cancellation cases, line/plane fields, heat contacts and
two-/three-dimensional sparse cohesive interfaces. The excluded modal matrix
and opt-in performance benchmarks were not run.

Local solver/engine all-target Clippy passed with warnings denied; workspace
formatting and whitespace checks passed. All changed source/configuration files
remain within 800 lines and this report within 2,000 lines. No separate build
root, retained numerical dump, application archive or service deployment was
created. Only the eight affected source/test files were synchronized to the
existing Linux test copy; server credentials and configuration were untouched.

Native operator-profile metadata validation passed for 79 profiles with
`executed=false`; this does not substitute for executing the registered commands.
Coverage-tensor structure and command checks passed across 13 modules and
11 paradigms: zero structural gaps, two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps. Its overall status remains `blocked`, not fully qualified.

## Reproduction

From the repository root with the pinned Rust toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib prescribed_cancellation -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine \
  --test magnetostatic_prescribed_range --test magnetostatic_prescribed_workflow \
  prescribed_cancellation --no-fail-fast -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
```

The integration scope is the same 67-target set in the
[sparse reduction report](sparse-prescribed-reduction-reliability-20261023.md#reproduction).
The two new consumer tests are included in its existing magnetic targets.

## Scope and remaining limits

The repair addresses complete zero cancellation at a prescribed subtraction,
including an intermediate zero. It does not guarantee high relative accuracy
for every nearly cancelling nonzero reduced RHS. Finite compensated mantissa
operations are not exact rational or arbitrary-precision accumulation.

A complete reusable original constrained-system admission gate remains separate
work. This inspection found and repaired an earlier reduction hole; it does not
claim that every sparse prescribed consumer now independently verifies its full
pre-elimination system. Existing row-local backward-error acceptance can still
be insensitive to a small response beside large prescribed contributions.
Cancellation coverage is cooperative failure/reuse evidence, not durable restart.

This is bounded prescribed-zero cancellation recovery, not complete original constrained-system admission, arbitrary precision, physical qualification or durable restart.
