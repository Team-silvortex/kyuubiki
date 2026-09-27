# Thermal Truss Output Reliability: 2026-09-27

## Scope And Reproduction

This repair covers `solve.thermal_truss_2d` and `solve.thermal_truss_3d` in the
native Solver, plus their in-process Rust headless execution-plan routes to
the Engine. Base revision is `6ab4c2dd`, daji 3.4.2, with this working-tree
overlay. Task/engine contracts, input/output schemas, constitutive models,
GUI code, installed applications and services are unchanged.

Ten of the initial eleven public-solver regression tests failed before the
repair; the restrained closed-form connectivity-reversal check already passed.
These are failing tests, not a count of independent defects:

- A nonfinite energy density or energy total could be returned as success and
  serialized to JSON `null`; nonzero-density volume products could also silently
  collapse to zero before a compensating factor was applied.
- Direct addition overflowed the average of two finite extreme temperatures.
  Squaring coordinates or displacements overflowed otherwise finite lengths
  or erased tiny nonzero displacement magnitudes.
- Bounds initialized with the origin and unit extent made small-deformation
  acceptance depend on model translation and gave short models an artificial
  minimum size.
- Fixed supports could remove equations with invalid stiffness or equivalent
  thermal loads before the reduced solve noticed the invalid coefficients.
- Result recovery did not observe cancellation at node, element and total
  stages, so a raw solver could return success after cancellation was requested.

Further retained tests cover thermal-force evaluation order, accumulated
stiffness overflow, and cancellation after 64 items of a 130-node chain.

## Changes And Evidence

Both truss solvers use `f64::midpoint` for endpoint temperature means and
`hypot` for member lengths and displacement norms. Thermal assembly evaluates
thermal strain before multiplying by axial rigidity, matching recovery and
avoiding the tested intermediate overflow/underflow of `E * A * alpha` before
temperature is applied. Nonfinite or nonpositive scalar stiffness is rejected;
equivalent and accumulated thermal loads, the full assembled stiffness matrix,
node magnitudes, and recovered element states are checked before success.
Errors identify the element, node or degree of freedom where available.

The existing plane energy-volume accumulator was moved unchanged into
Solver-local `solver_postprocess`, allowing trusses to share balanced
nonnegative factor multiplication and energy overflow/underflow checks without
depending on a plane operator. Plane error wording and checkpoint order are
retained. Nodes/elements use fallible result collection; assembly, full-matrix
validation, displacement restoration, summaries and totals use the existing
Solver cancellation mechanism. No operator logic moved into the Engine.

Bounds are now streamed from actual model coordinates, removing the temporary
point vector and phantom origin/unit extent. The 25%-of-maximum-axis-extent
heuristic is otherwise unchanged. No element cache or protocol was added;
checking the assembled matrix adds an O(nnz) read pass.

The retained tests cover both dimensions:

- Restrained uniform-temperature closed form, reversed connectivity, thermal
  strain split, Hooke-law stress and summed member energy.
- Finite extreme/subnormal temperature means, long finite members, tiny
  displacement norms, and representable large/small energy-volume products.
- Invalid stiffness, assembled stiffness/load overflow, nonfinite recovered
  state, member-energy underflow/overflow, and total-energy overflow, followed
  by valid-input replay.
- Axially free small-strain response at scaled material/temperature values,
  and the same displacement-limit decision at multiple translations/scales.
- Entry/terminal cancellation during node/element/total recovery, and a
  64-item interruption in assembly, matrix validation, node recovery, element
  recovery and totals, followed by identical structured JSON replay.
- Rust headless plan/bridge dispatch to both registered operators, verifying
  numeric temperature means, error propagation, normal replay and translated
  excessive-deformation rejection.

The complete macOS ARM64 Solver run passed **1030** tests with 0 failures and
9 opt-in tests ignored. It included the first 13 new public-solver tests;
after adding the chunk-boundary case, the focused target passed all **14**.
The new headless target passed **3** tests. Remote Linux x86_64 release
regression passed **372** unit tests and **37** integration tests across ten
selected targets, with 0 failures and 9 opt-in unit tests ignored, including
the final 14-test truss target and the thermal-plane output suite.
Overlapping runs are not a summed test-coverage percentage, and ignored
benchmarks are not executed evidence.

Strict Solver/CLI Clippy passed with `-D warnings`, as did workspace formatting.
Both thermal-truss validation profiles executed all six commands successfully.
Tensor self-test/structure, document book/inventory and source-800/document-2000
organization gates passed. The tensor still reports 0 structural gaps,
4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps, with overall release
status blocked. The added evidence does not erase those outstanding gates.

This is bounded thermal-truss output validation, not general thermoelastic qualification.
Synthetic arithmetic envelopes are not plausible material ranges. Existing
linear/small-strain assumptions and the member-length threshold above `1e-12`
remain. The displacement heuristic is not a local strain bound or a general
rotation-invariant/nonlinear stability criterion. General partial restraint,
thermal gradients, buckling, plasticity, contact and dynamics are not newly
qualified by these checks. Reordered products do not guarantee arbitrary
floating-point cancellation or all mathematically finite inputs; `E * A / L`
and other intermediates can still exceed the supported numeric range.

No paired performance benchmark was run. Removing a temporary bounds vector
is not evidence of end-to-end speedup, especially with the added validation.
No installed-Agent, HTTP/network recovery, GUI, or 1M-node proof is claimed.
No stored artifacts were rewritten; results with `null` physical values,
silently lost energy or origin-dependent acceptance must be recomputed before
research reuse. The tensor claim is bounded `verified` numerical/recovery
evidence, not overall release qualification.

## Reproduction And Provenance

Run from the repository root with the pinned Rust toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --test thermal_truss_output_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test thermal_truss_output_operator
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test thermal_truss_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-truss-2d-closed-form --execute --out tmp/thermal-truss-2d-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-truss-3d-closed-form --execute --out tmp/thermal-truss-3d-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```

The remote run used Rust 1.88.0 with `--locked --offline --release`, selecting
Solver `--lib` plus `thermal_truss_output_reliability`,
`thermal_truss_2d_review`, `thermal_truss_3d_review`,
`thermal_truss_2d_closed_form`, `thermal_truss_3d_closed_form`,
`thermal_truss_input_reliability`, `thermal_truss_2d_objectivity`,
`thermal_truss_3d_objectivity`, `thermal_plane_output_reliability` and
`operator_thermal_structural_reliability`. Headless evidence is local
in-process SDK/Engine dispatch, not a remote service test.

The Linux run used source-only scratch space and the existing managed Cargo
cache; no installed service, credentials or server configuration changed.
The temporary source directory was removed after the run; the shared Cargo
cache was retained.
Local and remote SHA-256 values matched for these paths relative to
`workers/rust/crates/solver/`:

| File | SHA-256 |
| --- | --- |
| `src/thermal_truss.rs` | `29fbae887c6b4eda61c2b71d4777b9549b716a9715291881c1d39abe5e6ee561` |
| `src/thermal_truss_validation.rs` | `7e651145191d8b379bb8958bfbbc3e3ad7410c450ebcb6a2d605627cb2e83037` |
| `src/solver_postprocess.rs` | `feca3016dfca4880ac074ed20d24e4281213a5a33a68f44a3467ff3383f70e29` |
| `src/plane_2d_summary.rs` | `d4668b834d6af520827317ce519b1b00ee708900a395b4400acd70d2b22ab758` |
| `tests/thermal_truss_output_reliability.rs` | `9c53d24a540380634294689a115ef34361311a211186e5de8da0a1ac8029869d` |
