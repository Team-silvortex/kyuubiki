# Thermoelastic Output Reliability: 2026-09-27

## Scope And Reproduction

This repair covers native CST and Q4 thermoelastic plane-stress result recovery
and the Rust headless-to-engine routes for `solve.thermal_plane_triangle_2d`
and `solve.thermal_plane_quad_2d`. Task/engine contracts, constitutive models,
quadrature, input/output schemas and frontend code are unchanged.

Base revision: `d7de8968d0eba5e3ed5c08c2ed82ab67ba8aed36`, daji 3.4.1.
The working-tree overlay includes the preceding tetrahedral, Q4 and CST
repairs; existing Orchestra changes remain separate and untouched.

Six of the seven initial public-solver regressions failed before the repair:

- A nonfinite energy density and total were returned as successful solver
  output. JSON serialization replaced them with `null`; finite stress did not
  imply that the complete result was usable.
- Directly summing nodal temperature deltas overflowed for a finite constant
  field. For the CST field `[1e16, 1, -1e16]`, addition lost the middle term and
  returned zero instead of the analytic mean `1/3`, depending on connectivity.
- Q4 temperature and state recovery formed area-weighted sums before dividing
  by area, allowing intermediate overflow or underflow even when the averages
  themselves were representable.
- The energy-density / area / thickness multiplication could overflow or
  underflow before compensating factors were applied.
- Tiny nonzero displacements had zero reported magnitude because their squares
  underflowed. The same square/sum pattern also affected the RHS norm diagnostic.

The initial entry/terminal cancellation regression already passed. It remains
retained, together with the existing larger-grid cancellation suite.

## Changes And Evidence

Temperature interpolation and area-weighted recovery use scaled, compensated
convex means. The bound prevents rounding drift beyond the input magnitude;
invalid values or weights still propagate failure rather than a finite mean.
Q4 Gauss-point temperatures are cached during precompute and reused in result
recovery instead of interpolating a second time.

CST and Q4 share a thermal recovered-state validator and the existing mechanical
energy-total implementation. Element states and node magnitudes are validated
before fallible result collection returns. Nonfinite equivalent or assembled
thermal loads are rejected even when fixed supports would remove the affected
equations. Node and RHS norms use `hypot`; a nonfinite final RHS norm is an
error. Cancellation checkpoints and element/node context are retained.

The new tests check:

- Constant extreme/subnormal temperature means, six CST connectivity orders,
  cancellation-sensitive temperature sums, and unequal positive weights.
- Restrained thermal stress, area-weighted recovery, and representable
  energy-volume products at large and subnormal arithmetic scales.
- Nonfinite state/force rejection and nonzero-density total-energy overflow
  or underflow rejection, followed by a normal valid solve.
- Nonzero tiny displacement norms, finite large norms, and closed-form RHS
  norms at both small and large elastic moduli.
- Entry/terminal cancellation at node, element and total recovery, without
  publishing success, followed by an identical structured JSON replay of the fixture.
- Registered Rust headless execution-plan dispatch for both operators, checking
  numeric temperature/energy results, failure propagation and normal replay.

Local focused checks passed: **3** new helper tests, **9** new public-solver
tests, and **3** new headless-route tests. Remote Linux release regression
passed **372** unit tests and **35** tests across nine selected integration
targets, with 0 failures and 9 opt-in unit tests ignored. The complete macOS
ARM64 solver suite passed **1017** tests with 0 failures and 9 opt-in tests
ignored. No ignored benchmark was silently counted as executed coverage.

Strict solver `--all-targets` and thermal headless-target Clippy passed with
`-D warnings`, as did workspace formatting. The thermal validation profile
executed all **20** commands successfully. Tensor self-test/structure, document
book/inventory and source-800/document-2000 organization gates passed. The
tensor adds only bounded `verified` numerical/recovery evidence: 0 structural
gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps remain; overall
release status remains blocked rather than promoted by these checks.

This is bounded thermoelastic output validation, not general multiphysics qualification.
These synthetic arithmetic envelopes are not plausible material/temperature
ranges or evidence of nonlinear physics accuracy. The unchanged small-strain
plane-stress model, minimum-area threshold, CST mean-temperature approximation
and Q4 full 2x2 integration still apply. Compensated sums do not guarantee
arbitrary cancellation accuracy, and some mathematically finite combinations
can still exceed representable intermediate arithmetic elsewhere in the solver.
No installed-Agent, network-recovery or 1M-node result is claimed here.

No paired performance benchmark was added or run in this round. Reusing
Gauss-point temperatures avoids duplicate interpolation, but added range checks
and stable averaging are not evidence of an end-to-end speedup.

No stored artifacts were rewritten or deleted. Results containing `null`
physical fields, or affected by these arithmetic edge cases, must be recomputed
before research reuse. The new evidence does not promote overall qualification.

## Reproduction And Provenance

Run from the repository root with the pinned toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test thermal_plane_output_operator
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test thermal_plane_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-plane-patch --execute --out tmp/thermal-output-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```

The remote release run used Rust 1.88.0 on Linux x86_64 and selected `--lib`
plus `thermal_plane_output_reliability`, `thermal_plane_mesh_refinement_regression`,
`thermal_plane_quad_isoparametric`, `thermal_plane_input_reliability`,
`thermal_plane_triangle_2d_review`, `thermal_plane_quad_2d_review`,
`plane_triangle_orientation_reliability`, `plane_quad_geometry_reliability` and
`multiphysics_output_range`. Overlapping test runs are not a summed coverage
percentage. The headless checks are in-process SDK/engine calls, not an installed
application or service test.

Remote execution used source-only scratch space and the existing managed Cargo
cache. No installed service, server configuration or credentials were changed.
The temporary source directory was removed after completion; the shared Cargo cache was retained.
Local and remote SHA-256 values matched, relative to `workers/rust/crates/solver/src/`:

| File | SHA-256 |
| --- | --- |
| `thermal_plane_2d.rs` | `4f0025bc9d997f8ad5a7a9f6e9da1c9a2c02eb95ff9f78444036cf1ef59b6bd2` |
| `thermal_plane_2d_quad.rs` | `e9573c9dcf72190ca420bbfb0e467ac5ef219857c3fe23ea4d4a09b49dca4433` |
| `thermal_plane_2d_results.rs` | `338cc4476d1e0b0dd541d8c004a08c680224c2c3664eb014165d49dcccfeb9a6` |
| `thermal_plane_2d_solve.rs` | `73324a8ec5621f89b9e67733513567f746b72011087ebeb6ca58cff14d5be20e` |
| `plane_2d_summary.rs` | `629308c7e2b89b7999d9284a5787664d188b2c8fadf05d76496c75f51656b154` |
