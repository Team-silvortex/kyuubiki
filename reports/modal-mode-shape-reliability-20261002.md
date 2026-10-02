# Modal Mode-Shape Reliability: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, plus the working-tree
[assembly](modal-assembly-reliability-20261002.md) and
[component-spectrum](modal-component-spectrum-reliability-20261002.md) repairs.
This follow-up changes private Solver mode-shape recovery. It does not change
Engine routes, task schemas, lumped-mass physics, eigenvalue algorithms or the
unit Euclidean returned-shape convention. Both 2D and 3D frame modes use it.

Two initial numerical tests and two public tests failed before this repair:

- An inactive DOF with the smallest positive mass set the global inverse-mass
  scale. An active `[0.6, 0.8]` direction with masses `1e308` entered the
  subnormal range before normalization. Its recovered norm was approximately
  `0.9999999955544826`, rather than one to the retained tolerance.
- Finite rescaling of an otherwise equivalent eigenvector could overflow
  the intermediate inverse-mass product and report a zero/non-finite shape.
- In planar and spatial public solves, a heavy member with `E=density=1e307`
  acquired a slightly different shape when an independent member with
  `E=density=1e-318` was added. One component changed from
  `0.5341757003802741` to `0.5341757003826992`; the eigenvalues still agreed.

These four failing cases are not four independent physics defects. They
expose premature rounding/overflow in shape recovery. Extreme inputs are
arithmetic stress fixtures, not credible material parameter recommendations.

## Recovery Contract

For normalized eigenvector `v`, the retained relation is
`phi = M^(-1/2) v / ||M^(-1/2) v||_2`. The implementation keeps each nonzero
weighted component in binary mantissa/exponent form until the mode-local
scale is known. Exactly zero components do not influence that scale.
Subnormal input vectors are normalized in exponent space; output scaling
performs its final multiplication into the subnormal range rather than
constructing an already-underflowed power of two. This avoids the previously
observed intermediate range loss without logarithms or a new dependency.

The largest scaled active component is at least one, so normalization cannot
need to amplify a coefficient that scaling already rounded away. Rounding
still applies to genuinely unrepresentable output components. This is not
arbitrary-precision arithmetic or a relative-error guarantee for all tiny
components. Eigenvector information lost before recovery is not reconstructed.

Dimensions, finite vector values, positive finite free-DOF masses, index bounds
and duplicate DOFs are checked before using the mapping. Arbitrarily ordered
unique free DOFs are accepted; constrained output entries stay zero. The
output buffer doubles as the temporary visited map and is fully overwritten
at every free DOF, avoiding another per-DOF allocation. Invalid input returns
an error rather than a panic or a partial shape. Full assembled mass validation
remains the caller's responsibility, as in the preceding assembly repair.

Validation, expansion, chunked norm evaluation and normalization use existing
Solver checkpoint stages with at most 64 numerical entries per chunk. Tests
require cancellation inside the recovery operation, not merely at observer
scope exit. A cancelled public/Engine call does not return partial success,
and a fresh solve can replay. Allocation itself is not interruptible; this is
cooperative in-process cancellation, not durable recovery or task resumption.

## Evidence

Nine new numerical tests cover inactive-mass isolation, finite vector scale
and sign, compensating heterogeneous masses, subnormal vectors, a 25-pair
power-of-two range grid, exact subnormal rounding boundaries, unsorted mapping,
16 malformed input cases, and cancellation/replay in four recovery stages.

Six public tests cover planar/spatial independent-component comparisons,
the planar discrete closed form, owned/borrowed equality, node permutation
and cancellation/replay. For the heavy cantilever, the two bending modes obey
`rotation / displacement = -3 +/- sqrt(21)` and
`abs(displacement) = 1 / hypot(1, rotation / displacement)`. This is an
independent reference for the retained one-element lumped-mass discretization,
not an external solver or an experimental benchmark.

The permutation fixture uses distinct cross-component frequencies. Repeated
eigenvalues do not define a unique mode order/direction; an initial permutation
assertion on two identical axial frequencies was corrected to remove that
degeneracy, not by weakening its shape tolerance. Returned shape comparisons
use `2e-13`, norm checks use `2e-14`, and frequency invariance uses `1e-12`.

Two new headless tests extend `modal_spectrum_operator` to nine cases. They
build a Rust headless batch/plan, resolve the bridge manifest and execute the
registered Engine route. The shape comparison and recovery cancellation are
checked across both planar and spatial operators. This is a real in-process
route, not a mock, GUI test or installed/distributed Agent run.

| Completed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 numerical shape tests | 9 | 0 | 0 |
| macOS ARM64 public shape target | 6 | 0 | 0 |
| macOS ARM64 modal headless target | 9 | 0 | 0 |
| macOS ARM64 modal profile, all twelve commands | 70 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1151 | 0 | 9 |

The full local suite completed all 182 result groups. Overlapping lanes must
not be added into a coverage percentage. The nine ignored opt-in tests provide
no fresh benchmark evidence.

Solver all-target and CLI modal-target Clippy passed with warnings denied.
Formatting, whitespace, document book/inventory, validation/tensor self-tests
and the source-800/document-2000 organization gates passed. The reliability
guide remains 1999 lines. Tensor structure and command checks pass, but overall
status remains blocked: 0 structural gaps, 4 maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps. The bounded recovery claim does not erase these gaps.

The Linux host initially authenticated and accepted the temporary source-only
transfer. Subsequent SSH attempts and the address-discovery endpoint timed
out before the release test or remote hash commands could execute. No fresh
Linux pass or local/remote hash equality is claimed; the previous report's
Linux results do not qualify this changed recovery code.

Local SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_math.rs` | `55e437018acff0ca31fd2559641cb96c06a6117b840104e5843d8a8b1dc1664e` |
| `solver/src/modal_frame_spectrum.rs` | `509b79bbd4070ed6003efd41be971a1206e99380202626995021e0c071e02dd7` |
| `solver/src/modal_mode_shape_tests.rs` | `4390226c34509b5345525bcd303f81a0ebba9e6c39e8aee0492dab8dcd47ee8c` |
| `solver/tests/modal_shape_reliability.rs` | `90b8be999243b4c416797dda0f13c3ce5b6922cb7b065935860f18e1dc32d08d` |
| `cli/tests/modal_spectrum_operator.rs` | `d7f13da89a8ccb390bff12c881fcd2d88aa67ffaeae3079e3830f7e520149a54` |

The two new test targets are part of `modal-frame-sanity`. A bounded claim is
registered in the runtime-frame-fields coverage evidence; these executable
checks do not constitute machine-checked formal proofs or general qualification.

## Limits And Reproduction

This is bounded modal shape-recovery validation, not general dynamic qualification.
It does not qualify free-free modes, repeated-mode uniqueness, arbitrary
ill-conditioned coupled systems, nonlinear dynamics, sparse spectral
minimality, material calibration or million-node performance. There is no
new Windows, installed-App, distributed-Agent or Python/Elixir lane in this
change. Previously affected research results require recalculation; stored
results are not rewritten or represented as newly verified.

The version remains daji 3.4.3. No commit, push or App rebuild is included.
Remote validation uses temporary source-only files and the existing managed
Cargo cache, without changing deployed services or storing server credentials
or configuration in the repository. Remote source staging
`kyuubiki-modal-shape.MLIVIm` is pending cleanup after connectivity recovers;
no remote test process was started by the failed connections.

Run from the repository root with pinned Rust 1.88.0:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_math::shape_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_shape_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-shape-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
