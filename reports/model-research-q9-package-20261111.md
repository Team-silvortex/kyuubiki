# Native rectangular Q9 operator package

Date: 2026-11-11. Host source line: daji 3.7.8. Package contract: 0.1.0.
Continues the [independent order comparison](model-research-bending-order-20261111.md).
Reproduction commands and schemas are in the
[HTML research guide](../docs/research-model-driven.html#q9-package).

## Implemented boundary

`solve.thermal_plane_q9_rect_2d` now has a native numerical kernel in the Rust
solver crate and an optional operator-SDK cdylib in
`workers/rust/crates/operator-thermal-q9`. The generic engine owns transport and
package dispatch, not the Q9 formulation. This is separate from the earlier
research example; the production Q4 solver and its accuracy gates are unchanged.

The solver accepts prescribed nodal temperature deltas, not absolute temperature,
and performs full 3 x 3 Gauss integration for stiffness, thermal load and elastic
energy. It reuses the existing dense/sparse linear solve and original-equation
validation. No reduced integration, stiffness regularization or benchmark-specific
displacement is injected. Results report displacements, volume-mean stress and
mechanical strain, integrated elastic energy, iterations and residual diagnostics.
Mean stresses are not local peaks; engineering shear strain is used.

Scope: one edge-connected conforming mesh of axis-aligned nine-node rectangles,
linear isotropic plane stress, zero displacement restraints, finite input values,
at most 4,096 nodes and 512 elements. Nine-node order is row-major from bottom-left
to top-right, not perimeter ordering. Unsupported geometry, missing restraints,
unused/coincident nodes with different indices, overlapping/disconnected cells,+hanging edges, invalid material values and unknown JSON fields are rejected.

## Bugs exposed and fixed

1. TaskIR treated every semantic `solver` as built-in solver RPC, blocking a valid
   external solver before package execution. Protocol preview, Agent dispatch and
   Elixir lowering/summary now distinguish execution ABI from semantic kind.
   External solver ABI requires explicit package identity and matching entrypoint;
   native built-in IDs cannot use it to bypass their original RPC route.
   Reservation is tested against every advertised native solver RPC, not only the
   three solvers currently executable through built-in TaskIR, plus the two public
   Stokes operator aliases.
2. Reconstructing a shared rectangle endpoint as `origin + extent` rounded
   `-0.3 + (0.1 - -0.3)` above `0.1`, falsely rejecting a conforming mesh as
   overlapping. Topology now compares the original endpoints; a translated decimal
   coordinate regression exercises the failure and affine displacement recovery.

## Numerical evidence

The bilayer uses E = [70, 20] GPa, nu = [0.2, 0.35], alpha = [10, 30]e-6/K,
geometry 1 x 0.25 m and thickness 0.01 m. Gradient nodal temperatures correspond
to the analytical k = [10, 40] W/(m K) conduction field; this native package does
not itself perform that heat solve. Consistent quadratic end loads are integrated
in closed form in the test independently of the kernel's Gauss implementation.
These are manufactured self-equilibrated end tractions, not traction-free strips.

| Temperature case | Cells | Absolute energy error (J) | Sparse iterations |
| --- | --- | --- | --- |
| Gradient | 2 x 2 | 8.4377e-15 | 0 (dense) |
| Gradient | 8 x 4 | 5.4956e-15 | 0 (dense) |
| Gradient | 16 x 8 | 3.9097e-11 | 59 |
| Uniform | 2 x 2 | 2.1372e-15 | 0 (dense) |
| Uniform | 8 x 4 | 1.1102e-15 | 0 (dense) |
| Uniform | 16 x 8 | 2.9188e-11 | 59 |

The target energies are 0.30305220883534134 J and 0.12650602409638567 J.
All six solves satisfy the predeclared 1e-8 J energy and 1e-10 m absolute nodal
displacement gates. The largest case has 561 nodes and 1,119 free mechanical DOFs;
the gradient free residual norm is 4.1458e-5 N. This checks polynomial reproduction
and dense/sparse paths, not general convergence, peak-stress accuracy or throughput.

The isolated macOS arm64 Agent loads the actual cdylib through TaskIR and checks
its library digest. One nine-node free-expansion model is executed three times,
interleaved with two malformed-mesh and two wrong-digest rejections. The valid
replay remains successful. This live test is not the full six-case bilayer study;
those numerical checks currently call the native solver directly.

## Remaining qualification

Follow-up: the [matching-node heat chain report](model-research-q9-heat-chain-20261111.md)
adds package-owned transfer and eight real-Agent bilayer chains. The scope below
describes this earlier package-only round, not the follow-up evidence.

The manifest and descriptor remain `partial`. Heat-solver integration, a typed
temperature-field bridge, generalized/distorted Q9 geometry, additional material
interfaces and cross-platform dynamic packages are not qualified by this round.
Native solver cooperative cancellation discards output and cleanly replays, but
the JSON C ABI does not propagate host cancellation/progress into the package's
separate solver context. Hard time limits and process isolation remain open; the
mesh-size limit is not a wall-clock guarantee.

This source-built package and Agent are not installed runtime capabilities yet.
The repository crate follows version 3.7.8; the independent package/descriptor
contract stays 0.1.0. A matching installed host version alone is not evidence that
it contains this unshipped TaskIR routing change.

## Verification scope

- Complete solver library unit suite: 1,195 passed, 0 failed, 34 already opt-in
  tests ignored. Run with `CARGO_PROFILE_TEST_OPT_LEVEL=1 cargo test --locked
  --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib -- --quiet`.
  Test execution took 256.98 seconds after compilation. The earlier unoptimized
  full-library attempt was interrupted after more than twenty minutes of ongoing
  modal tests, not counted as a pass. No test was newly ignored and no persistent
  build-profile setting was changed. This is test scheduling evidence, not solver
  throughput benchmarking.
- Protocol suite: 154 tests passed, including all advertised native RPC names
  being reserved against a solver-package ABI substitution.
- Operator SDK and Q9 package: 35 unit tests passed, including the actual C ABI
  buffer ownership round trip and manifest/descriptor identity checks.
- Thermal-plane and Q9 integration suites: 29 tests passed, including six bilayer
  solves across dense/sparse paths, malformed topology and native cancellation replay.
  The seven Q9 tests also passed in the optimized test build; repeated runs are
  not counted as additional test coverage.
- CLI TaskIR regression: 32 tests passed. Separate real-Agent Q9 and native-bar
  tests both passed using temporary local ports and owned package directories.
- Elixir TaskIR lowering, summary, readiness, executor and API: 54 tests passed.
- Strict Clippy, scoped Rust/Elixir formatting, documentation inventory/book,
  version contracts (231 matching), architecture topology and tensor structure
  checks passed. JSON schema documents parsed successfully; this is not a claim
  of independent full JSON-Schema validation for every generated result.

The tensor still reports 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps;
this partial package is not used to promote those gates. No full repository-wide
test run or Linux/Windows qualification is claimed.

No installed runtime or GUI was replaced, no repository commit was made, and no
model API call or provider review was performed. Tests create and delete their
owned temporary package roots; compact evidence and reproducible tests are kept
instead of duplicated full research payloads.
