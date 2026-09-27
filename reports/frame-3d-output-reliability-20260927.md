# Spatial Frame Output Reliability: 2026-09-27

## Scope And Baseline

Base revision: `6ab4c2dd`, daji 3.4.2, plus the current working-tree overlay.
This repair covers `solve.frame_3d` and `solve.thermal_frame_3d`, including their
in-process Rust headless execution-plan routes to the Engine. Shared section
orientation math also serves modal frames, so regression checks include that
family. No task schema, Engine protocol, GUI, application bundle or installed
service was changed.

Six of seven initial public Solver tests failed before the implementation
changes. These are failing tests, not a count of independent defects:

- Thermal energy was computed as elastic quadratic work minus thermal work
  plus initial thermal energy. Under free thermal expansion/curvature, an axial
  mechanical load requiring `4e-13 J` returned `4.547473508864641e-13 J`, about
  13.7% high. This specimen uses `L=2 m`, `E=200 GPa`, `A=0.02 m^2`,
  axial load `0.04 N`, expansion coefficient `1e-5`, temperature delta `100`,
  and local thermal curvatures `0.002` in both bending planes.
- A direction `[1e200, 0, 0]` overflowed its squared norm. Its normalized vector
  became zero, effectively removing the directional spring. The retained axial
  spring example returned `5e-9 m` instead of `2.5e-9 m`.
- Squared displacement/rotation norms erased `2e-200` nonzero responses.
- The mean of two finite `1e308` temperatures overflowed, causing a later
  constraint-reaction error even though the scaled thermal strain was finite.
- Fully fixed mechanical supports hid an invalid rigidity product and returned
  success. Result collection also ignored the requested cancellation stage.

The mixed axial/torsional/dual-bending zero-temperature control already passed.
The energy formula was not simply missing a bending term as in the preceding
2D repair: the 3D defect was cancellation between large thermal-work terms.

## Recovery Contract

Both static spatial operators now use a Solver-local checked element kernel.
No kernel cache is retained between assembly and result recovery. Existing
sparse-solver options and the 90,000-node thermal SGS threshold are preserved.
The thermal exact-constraint projection remains a Solver implementation,
independent of Engine task routing.

For local translations `[u,v,w]` and rotations `[rx,ry,rz]` at each endpoint,
the code uses `ry=-dw/dx`, `rz=dv/dx`. Public thermal curvature subscripts refer
to the gradient direction: `kappa_T_y` produces bending about local `z`, and
`kappa_T_z` produces bending about local `y`. Recovery uses:

```text
epsilon = (u_j-u_i)/L - epsilon_T
twist   = (rx_j-rx_i)/L
mean_y  = (ry_j-ry_i)/L - kappa_T_z
mean_z  = (rz_j-rz_i)/L - kappa_T_y
var_y   = sqrt(3) * [ry_i + ry_j + 2*(w_j-w_i)/L] / L
var_z   = sqrt(3) * [rz_i + rz_j - 2*(v_j-v_i)/L] / L
U = L/2 * [EA*epsilon^2 + GJ*twist^2
           + EIy*(mean_y^2 + var_y^2) + EIz*(mean_z^2 + var_z^2)]
```

Each Hermite bending curvature contributes its squared mean and variance.
Constant thermal curvature shifts the mean, not the variance. Evaluating
positive elastic-mode terms avoids subtracting large quadratic thermal-work
terms. The numeric implementation groups endpoint/chord differences and uses
weighted strain amplitudes with balanced products. Unrepresentable nonzero
energy is an error rather than a silent zero or JSON null.

The independent cantilever oracle integrates axial force, torque and linear
bending moment fields. With tip loads `[N,Py,Pz,T,My,Mz]`, define
`B(P,M,EI)=(M^2*L+M*P*L^2+P^2*L^3/3)/(2*EI)`. Then:

```text
U = N^2*L/(2*EA) + T^2*L/(2*GJ) + B(Py,Mz,EIz) + B(-Pz,My,EIy)
```

Zero-temperature energy also matches half the external nodal work. Uniform
free thermal extension and dual curvature superpose without changing elastic
mechanical energy. Fixed thermal closed forms and existing oblique support
closed forms remain separate controls.

The bending interpolation and energy identity are derived from the Hermite
Euler-Bernoulli formulation documented in the primary
[TU Delft beam notes](https://interactivetextbooks.citg.tudelft.nl/computational-modelling/structural_linear/euler_bernouilli.html).
The axial/torsion/dual-inertia section parameters are also distinguished in
the primary [OpenSees elastic beam-column documentation](https://opensees.github.io/OpenSeesDocumentation/user/manual/model/elements/elasticBeamColumn.html).
OpenSees was not executed here; these references are formulation checks, not
an external-solver benchmark. Signed formulas above follow this code's axes.

Scale-first normalization is shared by section orientation, translational and
rotational springs, and exact directional constraints. Finite norms use
`hypot`; temperature means use `midpoint`. The `1e-12` length/direction floors
remain. Section-axis parallelism is now checked after normalization, so a huge
nearly parallel axis cannot bypass the relative orientation tolerance.
In the reversed-connectivity test, explicit local `y` remains fixed and local
`z` reverses, so the local-z temperature gradient is negated to preserve the
physical field. This is distinct from the 2D frame's reversal convention.

Full element and assembled stiffness/load checks run before constraints discard
rows. Recovery validates node norms, local displacement, end actions, strain,
stress, member energy, directional spring response, exact constraint response
and total energy. Constraint projection/restoration and response collection
observe cooperative checkpoints. This is interruption plus fresh replay, not
resumption of a partially completed numerical solve.

## Execution Evidence

The counts and hashes below record this repair's original snapshot. A later
same-day [support-basis repair](frame-3d-support-reliability-20260927.md) extends
the thermal support and headless tests; its execution totals are separate.

The new Solver target has **17 passing tests**. It covers mixed-mode analytic
energy/work at `1/2/4/8` refinement, small mechanical energy under a larger
thermal background, finite extremes, fixed-support assembly failures,
stress/energy range errors, member-total overflow, scaled axes/supports,
rotated mixed connectivity, and borrowed/owned result equivalence.

Cancellation tests assert that raw Solver calls fail at retained stage entry
and terminal points. A 130-node fixed chain checks 64-item boundaries in
validation, preparation, assembly, matrix validation, output and totals, plus
thermal constraint indexing, basis construction, projection, restoration and
reaction residuals. Every canceled run is followed by an unchanged fresh solve.
Four headless tests build a batch, validate its plan, resolve the bridge route
and call the registered Engine operator. They cover mixed energy, thermal
cancellation sensitivity, scaled springs, and numerical-error propagation
followed by normal replay.

| Completed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 new Solver target | 17 | 0 | 0 |
| macOS ARM64 new headless target | 4 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1092 | 0 | 9 |
| Linux x86_64 selected Solver, release | 458 | 0 | 9 |
| Linux x86_64 new headless target, release | 4 | 0 | 0 |

Both hosts used pinned Rust **1.88.0**, `--locked --offline`. The full macOS
Solver run completed 178 result groups, including existing modal/frame and
thermal directional-support regressions as well as the earlier 2D fixes.
Linux Solver ran `--lib` plus `frame_3d_output_reliability`,
`frame_3d_closed_form`, `frame_3d_objectivity`, `frame_3d_review`,
`thermal_frame_3d_review`, `thermal_frame_3d_closed_form`,
`thermal_frame_objectivity`, `thermal_frame_mesh_convergence`,
`thermal_frame_directional_spring`, `thermal_frame_directional_constraint`,
`frame_input_reliability`, `mechanical_convergence`, `modal_frame_3d_review`,
`modal_frame_sanity_regression` and `frame_2d_output_reliability`.
Its total is 372 unit tests and 86 selected integration tests. Overlapping
runs are not additive coverage percentages; ignored opt-in tests are not
executed benchmark evidence.

Solver all-target Clippy and the new CLI target's Clippy passed with warnings
denied; workspace formatting passed. Profiles `frame-3d-closed-form` and
`thermal-frame-3d-closed-form` executed all seven and ten commands successfully,
respectively. The static 3D profile now has its own
`config/operator-validation-profiles/structural-spatial-frame.json` shard,
keeping the original structural-frame shard below the 800-line limit without
compressing or dropping validation. Tensor structure/self-test, document
book/inventory and source-800/document-2000 organization gates passed.
Bounded evidence is registered separately from the 2D claim in
`config/architecture/module-function-coverage-evidence/runtime-frame-fields.json`.
The tensor still has 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps
and 11 P0 gaps; overall release status remains blocked. These executed
assertions are not machine-checked formal proofs or new release qualification.

Remote execution used temporary source-only scratch space and the existing
managed Cargo cache. After completion, only that scratch directory was removed;
the cache was retained. Installed services, server configuration and credentials
were untouched. SHA-256 matched locally and remotely for these paths relative
to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/frame_3d.rs` | `26ebff1d2a408b4642c0f62eefca19306727e8d9ac0dff61b8138daa9ea9949f` |
| `solver/src/frame_3d_element.rs` | `386f34e6ca7e6a4c00d50af86bada5dfab0c98ca258b6202c67b9a8985c53d60` |
| `solver/src/frame_3d_math.rs` | `61ada6213872621b234f6d07789064f33392b63988da13739c78db554c48b850` |
| `solver/src/thermal_frame_3d.rs` | `0a2125d41a218b43c5f7b39b67d7f5e59a910913862f5e056186ccc81afae778` |
| `solver/src/thermal_frame_3d_constraints.rs` | `6340f366d313196272f2e14469545bfd4a89758558b012efac9416e7a061b51e` |
| `solver/src/thermal_frame_3d_validation.rs` | `508a233c1169dc95225ad4a56e22b7f1fa51402552714650134958565a338728` |
| `solver/tests/frame_3d_output_reliability.rs` | `e126ccbd863c6ce35bf8c29eab1fce440f8d2de17d3ca0cbe392e71956d9665a` |
| `cli/tests/frame_3d_output_operator.rs` | `4548b7b09313072c8a201b7cb7b3dbc351a63bb58ffc189514862b078fe819e3` |

## Limits And Reproduction

This is bounded spatial-frame output validation, not general frame qualification.
Existing qualification scopes remain unchanged. These are static linear
prismatic Euler-Bernoulli members with nodal mechanical loads and constant
thermal strain/curvature per element, not distributed-load, shear-deformation,
warping-torsion, plasticity, contact, nonlinear or transient validation.
Displacement and rotation maxima remain nodal samples. The public combined
stress remains the existing axial-plus-bending quantity, not a new torsional
shear or yield criterion.

Finite inputs are not all representable. `EA`, `GJ`, inertias, curvature products,
local transformations and matrix operations retain numerical limits. Endpoint
strain subtraction still has finite precision; positive modal energy does not
recover information absent from the solved displacements. Extreme scalar
examples are arithmetic tests, not realistic material ranges. The `1e155`
length case fixes unexcited transverse/rotational DOFs to isolate axial
arithmetic, not arbitrary-condition-number solvability.

No new installed-Agent, HTTP/network failure, GUI, Windows, performance or
1M-node claim is made. Full-matrix validation adds an `O(nnz)` pass; correctness
is not a measured speedup. Previously saved thermal-cancellation-sensitive
energies or results using overflow-prone direction magnitudes need recalculation
before research reuse. Stored results were not rewritten. No version bump,
commit or push is part of this repair.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_3d_output_operator
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_3d_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile frame-3d-closed-form --execute --out tmp/frame3d-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-frame-3d-closed-form --execute --out tmp/thermal-frame3d-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
