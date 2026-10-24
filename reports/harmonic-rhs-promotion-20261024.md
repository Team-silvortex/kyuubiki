# Harmonic RHS Promotion Failure Atomicity

Date: 2026-10-24

Source base: `c4cc2f96`, retaining the preceding uncommitted harmonic Schur,
factor-compensation and replayed-row quotient follow-ups. Rust package metadata
remains 3.7.0. Harmonic RHS storage now preserves its finite source if promotion
to binary-scaled storage returns an error at a conversion checkpoint. The fix is
private Solver storage management, not a new arithmetic or operator contract.
No version bump, service deployment, commit or desktop rebuild is included.

## Reproduced State Loss

The previous promotion moves the entire finite vector out of its adapter,
leaving an empty finite vector, before converting its entries. A cancellation
checkpoint can return an error before the newly converted buffer is installed.
The temporary conversion buffer then drops, leaving the original adapter empty.

Two new test functions fail on that implementation. A 2,050-row conversion
cancelled midway loses every row; even a one-row conversion cancelled at its
last checkpoint leaves zero rows. Earlier cancellation tests create a fresh
adapter for replay, so they do not exercise the damaged same-object state.

Current public solvers already propagate this error and discard their private
RHS adapter. These failures therefore establish an internal state invariant
violation, not a newly reproduced public crash or incorrect successful solve.
Keeping the adapter valid matters for explicit reuse and future recovery code
that must not index an empty buffer after a failed storage transition.

## Complete Conversion Before Replacement

Promotion reads from the still-owned finite vector and builds the same wide
temporary vector as before. Its start, batched conversion and final conversion
checkpoints remain in the same order. Only after all pass does it install the
incoming wide row and replace the adapter's representation. If a checkpoint
returns an error, normal ownership cleanup drops the temporary while every
finite source row, its bit pattern and allocation remain unchanged.

The ordinary finite-write path still updates its existing allocation. Writes to
an already-wide buffer are unchanged. No backup vector, matrix snapshot,
dependency, persistent state or extra steady-state allocation is introduced.
Both versions hold the finite allocation and growing wide allocation during
conversion; the repair changes ownership order, not that allocation pattern.
There is no measured speedup claim.

This guarantee is local to an error returned by the promotion checkpoints. A
concurrent cancellation arriving after its final checkpoint may still leave a
complete committed wide buffer before the outer solve reports cancellation.
Earlier elimination mutations are not rolled back. Callers must still abort
the interrupted solve or explicitly control their own supported retry scope.

## Regression Coverage

Four new test functions cover three private state controls and one public API
control:

- Start, middle, late-batch and final checkpoint cancellation for 2,050 rows,
  three destination rows, direct writes and normalization-triggered promotion,
  and preparation/dense/path factor checkpoint stages. Every source row and the
  finite allocation address remain intact before retry; successful same-object
  retry and subsequent swaps match a clean conversion.
- Repeated cancellation of the same adapter at sizes 1, 63, 64, 65 and 2,050,
  including the last checkpoint before replacement, followed by successful
  explicit reuse with fresh cancellation control.
- Invalid normalization in both finite and already-wide storage preserves all
  row values and the respective allocation. Source sentinels include signed
  zero and a minimum-subnormal component.
- Public borrowed/owned calls cancel precisely at the final promotion boundary
  for both path and dense fixtures and both load signs. Three repeated attempts
  leave the borrowed input unchanged; fresh complete solves match the baseline.

Existing ordinary-allocation, wide-RHS arithmetic, quantum-range, pivot-swap,
serialized Engine/workflow, late-frequency and output-publication regressions
remain in scope. Same-object adapter reuse is established privately; public API
tests perform complete new solves and do not claim in-place continuation.

## Platform Verification

Rust 1.88.0 with locked offline dependencies reuses the existing macOS AArch64
and Linux x86_64 build directories. Each platform passes 890 Solver library,
711 Engine library and 89 selected integration tests: 1,690 passed. Nine
Solver and one Engine test remain ignored, and 292 `modal_frame_spectrum::`
tests remain excluded. This is selected regression coverage, not a complete
workspace, installed-runtime or all-physics qualification.

The Linux run uses the preceding source overlay, not a clean checkout. Its
three changed files matched the committed baseline before synchronization,
and all three transferred files matched the final local SHA-256 digests.
No credentials, server configuration, service lifecycle or deployment changed.

macOS all-target Solver/Engine Clippy with warnings denied, formatting and diff
whitespace checks pass. Native catalog inspection passes with 101 registered
profiles without executing them all; the existing RHS profile passes its 18
selected tests. Documentation inventory and all 26 book HTML files pass.
The project organization audit passes the 800-line source/config and 2,000-line
document limits, with zero tracked debt.

The coverage tensor passes structural and command checks for 13 modules and
11 paradigms, with zero structural gaps. Its two maturity, 23 evidence-grade
and 18 P0 gaps remain; overall daji qualification is still blocked.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_rhs_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test harmonic_complex_force_range harmonic_complex_rhs_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-engine --test harmonic_constraint_admission harmonic_complex_rhs_range
```

The existing `harmonic-complex-rhs-range-local-reliability` profile now includes
the new controls and selects 11 private, four public and three Engine tests.
Its 18 tests are a subset of the broader regression, not an additional count.
The existing RHS tensor claim is extended without a new qualification grade.

## Scope Boundary

This is local failure atomicity of harmonic RHS promotion at its fallible checkpoints, not whole-solver rollback, in-place solve resume, general forward accuracy or durable restart.

Allocation exhaustion, forced process termination and host power loss are not
cooperative checkpoint cancellation and are not qualified by these controls.
Numerical gates, physical acceptance and all preceding rounding boundaries are
unchanged by this storage repair.
