# Beam Field Output Reliability: 2026-09-27

## Scope And Reproduced Failures

Base revision: `6ab4c2dd`, daji 3.4.2, plus the current working-tree overlay.
This repair covers `solve.beam_1d` and `solve.thermal_beam_1d`, including their
in-process Rust headless execution-plan routes to the Engine. No task schema,
Engine protocol, GUI, application bundle or installed service was changed.

Seven of the initial eight public Solver tests failed on the old implementation;
the zero-load control already passed. These are failing tests, not seven
independent defects:

- End-action work `0.5 * (K*u - f_equiv) dot u` was reported as strain energy.
  A uniformly loaded cantilever returned approximately zero instead of
  `1.818531609 J`; the same fixed-fixed beam returned zero instead of
  `0.050514767 J`. These examples use `L=2.4 m`, `q=1350 N/m`,
  `E=210 GPa`, and `I=9.5e-6 m^4`.
- A single pinned-pinned element returned zero maximum moment/stress because
  only end moments were inspected. Its interior maximum is `972 N*m`.
- Fully restrained thermal curvature returned zero energy despite nonzero
  elastic bending. The retained example requires `7.75656 J`.
- Reversing node connectivity changed global rotations because the code used
  positive length without reordering the element's global DOFs.

## Recovery Contract

Mechanical and thermal beams now share a Solver-local element kernel and
assembly/reduction path. Elements are internally ordered by increasing global
`x`; nodal `uy` and `rz=duy/dx` remain global quantities. Recovered endpoint
actions are mapped back to the original `node_i`/`node_j` identities. No element
kernel cache is retained between assembly and recovery.

For constant `EI`, constant distributed load `q` and constant thermal curvature
within each element, recovery uses a quadratic moment field. With `t=x/L`,
canonical left/right nodal moment actions `m_left/m_right`, and the code's sign
convention:

```text
M(t) = -m_left * (1 - t) + m_right * t + (q * L^2 / 2) * t * (t - 1)
U    = integral_0^L M(x)^2 / (2 * EI) dx
```

The field includes the uniform-load particular solution, rather than only the
cubic nodal displacement interpolant. Three-point Gauss integration is exact
for its squared quadratic polynomial, up to floating-point arithmetic.
Both endpoints and any interior stationary point contribute to `max_moment`
and `max_bending_stress`. Moment normalization and balanced nonnegative products
avoid the tested intermediate overflow from directly squaring a large moment.

The independent uniform-load oracles are:

| Support | Maximum absolute moment | Elastic field energy |
| --- | --- | --- |
| Cantilever | `abs(q)*L^2/2` | `q^2*L^5/(40*EI)` |
| Pinned-pinned | `abs(q)*L^2/8` | `q^2*L^5/(240*EI)` |
| Fixed-fixed | `abs(q)*L^2/12` | `q^2*L^5/(1440*EI)` |

For a fully restrained constant thermal curvature `kappa_T`,
`U=EI*kappa_T^2*L/2`. Free thermal curvature contributes no elastic moment;
superposing it on the cantilever's mechanical loading preserves mechanical
bending energy. Mixed tip force, tip moment and distributed load are checked
against an independently integrated polynomial, including an off-center peak.
Piecewise `EI` and unequal lengths are checked by integrating each interval.

The governing beam equation, cubic Hermite discretization and consistent load
concepts are cross-checked against the primary
[TU Delft Euler-Bernoulli beam notes](https://interactivetextbooks.citg.tudelft.nl/computational-modelling/structural_linear/euler_bernouilli.html).
The continuum recovery distinction is documented in TU Delft's
[postprocessing notes](https://oit.tudelft.nl/CIEM5000/2026/lecture2/postprocessing.html)
and [matrix/FEM comparison](https://oit.tudelft.nl/CIEM5000/2026/lecture2/fem.html).
Our signed expression above follows `rz=duy/dx`; it must not be substituted
unchanged into a reference using the opposite rotation or moment convention.
The tabulated test oracles are integrations of the stated equilibrium fields,
not outputs copied from this kernel or an external solver.

## Execution Evidence

The final public Solver target contains **18 passing tests**, spanning both
operators, borrowed/owned paths, positive/negative loads, `1/2/4/8/16`
refinement, reversed/mixed connectivity, nonuniform geometry, piecewise
stiffness, numerical-range failures and replay. Full fixed-DOF stiffness/load
validation prevents constraints from hiding invalid assembly. Node values,
element forces, stress, element energy and total energy are checked before
success. Error cases include finite input products overflowing stiffness,
thermal curvature or thermal moment, accumulated load/matrix overflow,
unrepresentable stress, element-energy underflow/overflow and total overflow.

Preparation, assembly, matrix validation, solution restoration, node/element
recovery and summaries observe existing cooperative Solver checkpoints. Tests
cancel at stage entry/exit and at 64-item boundaries of a 130-node model, assert
that the raw solve returns an error, and compare a fresh replay to normal output.
Four additional Rust headless tests build a batch, validate its plan, resolve
the bridge manifest route and invoke the registered Engine operator. They cover
interior moment/energy, restrained thermal bending, reversed connectivity, and
error propagation followed by normal replay.

| Executed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 full Solver, debug | 1059 | 0 | 9 |
| macOS ARM64 new headless target | 4 | 0 | 0 |
| Linux x86_64 selected Solver, release | 451 | 0 | 9 |

Both hosts used Rust **1.88.0** with `--locked --offline`. Linux ran Solver
`--lib` plus `beam_output_reliability`, `beam_1d_review`,
`thermal_beam_1d_closed_form`, `thermal_beam_1d_refinement`,
`thermal_beam_1d_review`, `beam_frame_classic_regression`,
`beam_torsion_input_reliability`, `mechanical_convergence`,
`truss_output_reliability` and `thermal_truss_output_reliability`.
The Linux total comprises 372 unit tests and 79 selected integration tests.
It also verifies the preceding mechanical/thermal truss output changes in
the same overlay. Overlapping runs are not additive coverage percentages;
the ignored opt-in tests are not executed benchmark evidence.

Solver all-target Clippy, the new CLI target's Clippy, and workspace formatting
passed with warnings denied where applicable. Validation profiles
`beam-frame-classic` and `thermal-beam-1d-closed-form` executed all ten and six
commands successfully, respectively, including both new targets.
Bounded evidence is registered in
`config/architecture/module-function-coverage-evidence/runtime-beam-fields.json`.
Tensor structure/self-test, document book/inventory and source-800/document-2000
organization gates passed. The tensor still has 0 structural gaps, 4 maturity
gaps, 16 evidence-grade gaps and 11 P0 gaps; overall release status remains
blocked. These bounded checks do not discharge those separate obligations.

Linux used temporary source-only scratch space and the existing managed Cargo
cache. Only that scratch directory was removed after successful completion.
No installed service, credentials, server configuration, or persistent research
data was changed. SHA-256 matched locally and remotely for these paths relative
to `workers/rust/crates/solver/`:

| File | SHA-256 |
| --- | --- |
| `src/beam_1d.rs` | `cf5b4f1c540013d5a0b4553ebb5787c4e3717242b3d2899196d3f3eb4b3e188b` |
| `src/beam_1d_element.rs` | `e80f07bdc9678e36162f1af137aedf633c964808e37c597f221dc4d8ce828533` |
| `src/beam_1d_system.rs` | `2526a7744ce35f5b28bda8b32b3c45ca469272b4860fc6a4546943dd0420cfc6` |
| `src/beam_1d_validation.rs` | `42cf20e54e2db2bdbc4eb297d06f41705f208a44bdb69129664fc0c292d65f20` |
| `tests/beam_output_reliability.rs` | `5d8446a559cdb889ca9992eb07c04de4f95ecd11b5c8c20fa861f64cf4cd6081` |

## Limits And Reproduction

This is bounded beam-field output validation, not general structural qualification.
Existing qualified scopes are unchanged. This is a linear prismatic
Euler-Bernoulli model per element, not Timoshenko shear deformation, geometric
nonlinearity, plasticity, buckling or thermal-transient validation. Frame 2D/3D
recovery is not repaired or newly qualified by this work.

Displacement and rotation maxima remain nodal samples, not continuous-field
extrema. In particular, a single pinned-pinned loaded element has zero nodal
deflection despite nonzero continuum bending and strain energy. Large-rotation
validity is not newly guaranteed. Arbitrary finite inputs are not all
representable: `EI`, thermal curvature and other intermediates still impose
numeric limits. Extreme synthetic cases are not realistic material ranges.

No new end-to-end performance, installed-Agent, HTTP/network failure, GUI,
Windows or 1M-node claim is made. The full assembled matrix check adds an
`O(nnz)` pass; correctness fixes are not measured speedups. Previously saved
uniform-load, restrained thermal-energy or reversed-connectivity beam results
need recalculation before research reuse. Stored results were not rewritten.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test beam_output_operator
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test beam_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile beam-frame-classic --execute --out tmp/beam-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-beam-1d-closed-form --execute --out tmp/thermal-beam-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
