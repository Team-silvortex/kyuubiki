# Tridiagonal Prescribed-Boundary Range Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[back-underflow repair](tridiagonal-back-underflow-reliability-20261023.md).

## Reproduced failures

Four tests failed before this repair. Let `q` denote the smallest positive
subnormal and `H = 2^1023`.

- A two-node chain with diagonal `0.5`, coupling `-0.5`, zero loads and
  prescribed first value `q` returned `[q,0]` instead of `[q,q]`. Its reduced
  load `q/2` was rounded away before the solve began.
- Prescribing both endpoints of a three-node `0.5/-0.5` chain to `q` returned
  a zero middle value. Two half-quantum contributions should add to `q`.
- A two-node chain with diagonal `2`, coupling `-2`, zero loads and prescribed
  first value `H` rejected the overflowing product although `[H,H]` is finite.
- A single free equation `x + 2H - 2H = 1` rejected intermediate infinities
  instead of returning `x=1`.

The reduced system's previous residual check could not detect a boundary
contribution already lost while forming its RHS. These arithmetic examples
exercise range handling and are not material-load qualification.

## Repair

Prescribed-value indexing and segment solving now live in
`chain_tridiagonal_prescribed.rs`; callers still use the shared path solver.
The engine/operator boundary and request formats are unchanged.

Ordinary boundary reductions keep their ordered multiply/subtract operations.
Non-normal nonzero products, subnormal reduced loads and nonfinite updates
enter recovery. Only the affected free equation is normalized by a binary
power based on its remaining coefficients. Every coefficient must scale
reversibly. The existing compensated row arithmetic combines the original
load and both prescribed contributions before restoring the scaled RHS.

The existing tridiagonal solver handles the resulting segment, including
pivoting, equilibration and its reduced-equation admission. Before publishing
a segment, the shared row residual also checks its original endpoint equations
using the prescribed values. Interior equations are unchanged by boundary
reduction and retain the existing solver admission.

Only computed unknowns receive the existing subnormal quantization allowance.
Prescribed inputs are exact supplied data and receive no such allowance.
Fixed-node equations may contain reactions and are not required to balance as
free equations. Tolerances remain unchanged.

Normal coefficient slices are borrowed. Exceptional reduction may copy the
segment's diagonal and affected off-diagonal slices through copy-on-write;
this is O(n) exceptional storage, not an O(1) recovery-memory claim. The RHS,
factorization and solution remain O(n). There is no dense fallback. No latency
or RSS benchmark is claimed.

Existing constraint-index, constraint-map, constraint-reduce, residual-validate
and result-prescribed observations cover long scans, recovery, validation and
publication. Cancellation returns no partial vector. Input slices are immutable
and remain usable after rejection or cancellation. These are cooperative
observations rather than persisted restart checkpoints.

## Regression controls

Thirteen unit tests and three integration tests were added:

- Half-quantum and two-boundary constructions, both signs, both boundary sides,
  four odd-subnormal magnitudes and coefficient scales `2^-600`, `0.5`, `2^600`.
- Overflow cancellation with a retained unit load, multiple free unknowns and
  adjacent-row pivoting after normalization.
- A 514-row segment retaining an independent quantum load without dense fallback.
- Exact ordinary arithmetic bits, fixed-input allowance exclusion and permitted
  fixed-node reactions.
- Valid quantized responses, true result overflow, irreversible coefficient
  scaling, erased scaled RHS and successful subsequent supported solves.
- Cancellation during constraint indexing, recovery phases, original-equation
  admission, final publication and 513-node fixed-value scans, followed by reuse.
- Real magnetostatic bars with constant subnormal potential or an overflowing
  boundary product: borrowed/owned APIs, reversed element orientation, zero
  field/energy, direct dispatch, workflow output and JSON round trips.

## Validation

Completed on the final source with Rust 1.88.0 and offline locked dependencies:

| Scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions per platform |
| --- | ---: | ---: | --- |
| Solver library | 676 | 676 | 9 ignored; 292 modal-spectrum tests filtered out |
| Engine library | 711 | 711 | 1 ignored |
| 47 integration targets | 258 | 258 | None |
| Total | 1,645 | 1,645 | 10 ignored; 292 filtered out |

The 16 new tests are included in these totals. Focused repeated runs are not
counted again. The 47 integration targets comprise the base 42-target set,
the two new magnetic boundary targets and three additional field/reference
targets listed below. They include the 10,000-row transient chain and 52 live
Agent cancellation tests. The long modal matrix and opt-in benchmarks were
not run.

Local solver/engine all-target Clippy with warnings denied, workspace
formatting and patch-whitespace checks passed. Affected source/configuration
files remain within 800 lines. Existing build roots/caches were reused on both
machines; no numerical input/output dumps or package archives were retained.

Native operator-validation metadata passed for 75 profiles with
`executed=false`. Coverage tensor structure/command checks passed for 13
modules and 11 paradigms, with 0 structural gaps, 2 maturity gaps, 23
evidence-grade gaps and 18 P0 gaps. Overall readiness remains `blocked`.
Metadata registration checks are separate from the numerical executions above.

## Remaining limits

The scaled reduced RHS must still fit f64 without complete erasure, and all
free coefficients must normalize reversibly. Some mathematically finite
systems therefore remain conservatively rejected. General constraint
elimination, sparse/dense constraint reducers, assembly range and derived
physical-result arithmetic are outside this repair. Final unknowns remain
f64, and conditioning requires separate qualification.

The previous duplicate-prescribed-value tolerance is retained; no new general
constraint-consistency policy is implied. Full modal-spectrum requalification
and opt-in performance benchmarks are excluded from this bounded regression.

Follow-up: the [prescribed-value consistency repair](prescribed-value-consistency-reliability-20261023.md)
subsequently removes the duplicate-value tolerance and aligns chain/sparse
input identity. Counts above remain those of the preceding range-recovery run.

## Reproduction

From the repository root with the pinned toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib prescribed_range -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine \
  --test magnetostatic_prescribed_range --test magnetostatic_prescribed_workflow \
  --test electrostatic_bar_closed_form --test magnetostatic_bar_refinement \
  --test advection_diffusion_bar_refinement -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
```

The [row-validation report](tridiagonal-row-validation-reliability-20261023.md#reproduction)
retains the base 42-target integration command. Add the two new integration
targets and three additional field/reference targets above for this repair.

This is bounded prescribed-boundary range recovery, not general constraint elimination, arbitrary precision, physical qualification or durable restart.
