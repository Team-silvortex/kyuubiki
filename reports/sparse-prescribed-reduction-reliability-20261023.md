# Sparse Prescribed-Value Reduction Reliability

Date: 2026-10-23

Version line: daji 3.6.x, after `9aed9a2f` (`daji 3.6.8`) and the uncommitted
[chain boundary repair](tridiagonal-prescribed-range-reliability-20261023.md).

## Reproduced failures

Six initial tests failed before the repair:

- With `q` the smallest positive subnormal, four prescribed contributions
  `-0.25*q` should produce reduced load `q`. The old result was zero.
- One `-0.5*q` contribution was erased, and `-0.5*(3q)` rounded to `-2q`.
  Such a rounded reduced load can yield an incorrect solution even when its
  eventual unknown is representable. Neither case was rejected.
- With `H=2^1023`, `1 - 2H + 2H` produced NaN instead of the unit load.
- Matrix/vector dimension mismatch could silently drop rows or index outside
  a vector; the test failed on a silently accepted empty vector first.
- Out-of-range constraints were ignored, nonfinite prescribed values could
  be accepted, and conflicting duplicate assignments overwrote earlier data.
- Nonfinite values in constrained rows or loads were hidden by elimination.

These observations concern the shared sparse reducer. The preceding chain
repair uses its separate segment path and range policy.

## Implementation

Both sparse reduction entry points now validate dimensions, the entire finite
RHS and matrix, and sorted unique in-range columns before constructing a
reduced system. Invalid constraint indices return errors. Prescribed inputs
must be finite. Duplicate assignments must be numerically equal, including
equivalent signed zeros; identical repeats and homogeneous repeats remain
idempotent. This tightens the private reducer contract for malformed inputs.

Ordinary rows retain their multiply/subtract order and free coefficients.
Nonzero non-normal boundary products, subnormal RHS updates or nonfinite
updates enter the existing compensated binary-range row arithmetic. It merges
the original RHS and all prescribed products, then restores only the result.

The added `RequireExactRange` restoration policy rejects both complete erasure
and partial subnormal rounding when undoing the binary scale. The check applies
to exponent restoration of the accumulated value; it does not turn finite
mantissa operations into arbitrary precision. Other substitution callers keep
their existing policies.

The free matrix is unchanged, preserving symmetric coefficients. There is no
row-only rescaling that could break an SPD backend's symmetry assumption, and
no dense conversion or factorization retry in reduction. The reduced RHS must
itself be representable: e.g. the single half-quantum contribution is rejected
even where a smaller later pivot would give a finite unknown.

Validation adds one O(n+nnz) pass and uses O(1) scan state. The reducer keeps its
existing O(n) maps and reduced matrix. Exceptional row recovery allocates a
pair buffer for that row's prescribed entries, bounded by the widest affected
row. Normal rows add no pair buffer. No speedup, peak-RSS reduction or new
large-mesh performance qualification is claimed.

`constraint_reduce_row` is appended as stage 85. Existing stage IDs are
unchanged. Wide validation and ordinary reduction rows poll every 1,024 entries;
recovery uses the shared arithmetic's entry, batched pass and completion checks.
The stage may repeat for a row's collection and arithmetic passes. Partial
tuples remain private and are discarded on cancellation or failure. Original
matrix/vector slices remain available for subsequent solves.

## Added coverage

Fifteen tests were added: twelve reduction unit tests, one stage-ID test, one
solver integration test and one engine integration test.

- Positive and negative quarter-quantum aggregation over four odd subnormal
  magnitudes, exact subnormal restoration and rejected rounded loads.
- Overflow cancellation with a retained unit load, true overflow, irreversible
  alignment, and successful subsequent reduction with supported input values.
- Bad dimensions, invalid columns/order/duplicates, hidden nonfinite constrained
  data, conflicting assignments, empty/all-fixed systems and identical repeats.
- Ordinary RHS bit parity and unchanged symmetric free coefficients.
- A 2,049-node wide-row case with cancellation during ordinary scanning, operand
  collection and both recovery passes, followed by clean input reuse.
- Cancellation during full input validation, homogeneous wide-row reduction and
  final tuple publication.
- A real four-arm magnetic junction: each arm has permeance `0.25`, all four
  fixed leaves have the same tiny potential, and the free center must have that
  potential with zero field and energy. Both signs, reversed element direction,
  borrowed/owned APIs, direct dispatch, workflow and JSON round trips are covered.

## Validation

Completed on the final source with Rust 1.88.0, offline locked dependencies
and the existing build caches:

| Scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions per platform |
| --- | ---: | ---: | --- |
| Solver library | 689 | 689 | 9 ignored; 292 modal-spectrum tests filtered out |
| Engine library | 711 | 711 | 1 ignored |
| 67 integration targets | 331 | 331 | None |
| Total | 1,731 | 1,731 | 10 ignored; 292 filtered out |

The 15 new tests are included in these totals. Focused repeated runs are not
counted again. Integration coverage includes the 10,000-row transient chain,
52 live Agent cancellation tests, scalar-plane field/reference checks, heat
contacts and two-/three-dimensional sparse cohesive-interface cases.
The long modal matrix and opt-in performance benchmarks were not run.

Local solver/engine all-target Clippy with warnings denied, workspace formatting
and patch-whitespace checks passed. Affected source/configuration files remain
within 800 lines and this report within 2,000 lines. No separate build root,
retained numerical dump, application archive or service deployment was added.

Native operator-validation metadata passed for 76 profiles with
`executed=false`. Coverage tensor structure/command checks passed for 13 modules
and 11 paradigms, with 0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps. Overall readiness remains `blocked`. Registration checks are
separate from the numerical executions above.

## Scope and remaining limits

The reduced RHS remains f64. Recovery can conservatively reject mathematically
finite solutions when the reduced load needs a wider exponent, a fractional
subnormal quantum, or irreversible common-exponent alignment. General
constraint elimination with extended RHS storage is still separate work.

This does not add a full original-equation gate to every sparse consumer.
Consumers retain their existing reduced-system and domain-specific admission.
Assembly range, general conditioning, derived physical fields and the chain
path's older duplicate-prescribed tolerance remain separate concerns.
Cancellation observations are not persistent restart records.

Follow-up: the [prescribed-value consistency repair](prescribed-value-consistency-reliability-20261023.md)
subsequently aligns the chain path's duplicate-value rule. This report's
validation counts remain those of the preceding sparse-reduction run.

## Reproduction

From the repository root with the pinned Rust 1.88.0 toolchain:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib linear_algebra::reduction::tests -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib constraint_row_stage -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib -- --skip modal_frame_spectrum:: --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine \
  --test heat_plane_triangle_2d_review --test heat_plane_quad_2d_review \
  --test heat_plane_contact --test heat_plane_contact_reliability \
  --test heat_plane_output_reliability --test electrostatic_plane_triangle_2d_review \
  --test electrostatic_plane_quad_2d_review --test electrostatic_plane_mesh_refinement \
  --test magnetostatic_plane_reliability --test magnetostatic_plane_input_reliability \
  --test magnetostatic_plane_triangle_2d_review --test magnetostatic_plane_quad_2d_review \
  --test electric_conduction_reference_invariance \
  --test electric_conduction_plane_input_reliability \
  --test electric_conduction_plane_quad_2d_review \
  --test electric_conduction_plane_mesh_refinement --test magnetostatic_plane_workflow \
  --test electric_conduction_operator --test cohesive_interface_mesh_2d_sparse \
  --test cohesive_interface_mesh_3d_sparse --no-fail-fast -- --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
```

These 20 additional integration targets extend the
[preceding 47-target scope](tridiagonal-prescribed-range-reliability-20261023.md#reproduction).
The two magnetic boundary targets include this repair's new junction tests.

This is bounded sparse constraint-reduction evidence, not general original-equation admission, arbitrary precision, physical qualification or durable restart.
