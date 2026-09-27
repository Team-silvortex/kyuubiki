# Planar Frame Output Reliability: 2026-09-27

## Scope And Reproduced Failures

Base revision: `6ab4c2dd`, daji 3.4.2, plus the current working-tree overlay.
This repair covers `solve.frame_2d` and `solve.thermal_frame_2d`, including
their in-process Rust headless execution-plan routes to the Engine. The
Solver-local changes do not alter task schemas, Engine protocols, GUI assets,
application bundles or installed services.

Six of the initial seven public Solver tests failed on the old implementation;
the restrained thermal control passed. These are failing tests, not six
independent defects:

- Thermal frame energy used only mean curvature, omitting its variation along
  each Hermite element. A transverse tip-loaded single-member cantilever gave
  `1.5785864661654132 J` instead of `2.104781954887218 J`, a 25% deficit.
  The specimen uses `L=2.4 m`, `E=210 GPa`, `I=9.5e-6 m^4`, `P=1350 N`.
- Adding axial force `1200 N`, tip moment `400 N*m` and area `0.018 m^2`
  gave `2.4548330827067635 J` rather than `2.9810285714285714 J`.
  A two-member right-angle portal likewise gave `11.91490037594070 J`
  against `12.85685526315837 J` from external nodal work.
- Directly adding two finite `1e308` temperatures overflowed the mean and
  allowed nonnumeric JSON output. Squaring a `2.4e-200` displacement erased
  its nonzero magnitude.

Further negative tests cover nonfinite full-matrix/load assembly hidden by
fixed supports, nonrepresentable recovered stress and energy, and cooperative
cancellation before an apparently successful result.

## Recovery Contract

Mechanical and thermal planar frames share a checked element kernel and
assembly/reduction path. Assembly validates the full matrix and load before
constraints remove rows. Recovery checks local displacement, end actions,
strains, stresses, member energy and total energy. No element-kernel cache is
retained between assembly and recovery. The existing sparse-solver options and
90,000-node thermal preconditioner threshold are preserved.

For element length `L`, local nodal displacement vector
`[u_i, v_i, theta_i, u_j, v_j, theta_j]`, axial rigidity `EA`, bending rigidity
`EI`, uniform thermal strain `epsilon_T` and constant thermal curvature
`kappa_T`, the complete field energy is:

```text
epsilon_m = (u_j - u_i) / L - epsilon_T
kappa_m   = (theta_j - theta_i) / L - kappa_T
chord     = (v_j - v_i) / L
variation = sqrt(3) * [(theta_i - chord) + (theta_j - chord)] / L
U         = L/2 * [EA * epsilon_m^2 + EI * kappa_m^2 + EI * variation^2]
```

The last term is the variance of the linear Hermite curvature. A constant
thermal curvature changes its mean, not that variance. Positive modal energy
terms avoid subtracting large quadratic thermal-work terms. Weighted strain
amplitudes and balanced products avoid the tested direct-square underflow;
nonfinite energy or a nonzero strain producing zero energy returns an error.
Finite member energies may still overflow their sum, which is rejected too.

Temperature averages use `midpoint`, geometry/displacement norms use `hypot`,
and stiffness evaluates `EI/L`, then successive divisions, rather than forming
`L^2` and `L^3`. Thermal strain is formed before multiplying axial rigidity.
This expands the tested representable range without promising every finite
input is representable or well-conditioned.

The independent cantilever oracle for axial force `N`, transverse force `P`
and tip moment `M` is:

```text
U = N^2*L/(2*EA) + [M^2*L + M*P*L^2 + P^2*L^3/3]/(2*EI)
```

This integrates the equilibrium fields, independently of the new recovery
function. Zero-temperature portal energy is checked against half the external
nodal work. Free uniform thermal extension/curvature superposed on cantilever
loading preserves its elastic mechanical energy. The fixed thermal control
retains `L/2 * [EA*epsilon_T^2 + EI*kappa_T^2]` with zero displacement.

The governing beam equation and cubic Hermite discretization are cross-checked
against the primary
[TU Delft Euler-Bernoulli beam notes](https://interactivetextbooks.citg.tudelft.nl/computational-modelling/structural_linear/euler_bernouilli.html).
The formula above is derived in the code's `theta=dv/dx` convention; signed
moment conventions must not be copied unchanged from a different reference.
Frame local/global coordinate principles are also described in TU Delft's
[frame element notes](https://interactivetextbooks.citg.tudelft.nl/computational-modelling/structural_linear/space_frame.html).
That latter reference includes Timoshenko shear deformation, which these
planar Euler-Bernoulli operators do not implement.

Thermal gradients are element-local. Reversing connectivity reverses local
`y`, so the reversed test negates `temperature_gradient_y` to preserve the same
physical field. Global nodal translations/rotations and public endpoint
identities remain unchanged; this is not the 1D beam's canonical-x ordering.

## Execution Evidence

The new Solver target has **16 passing tests**, covering mechanical/thermal
operators, borrowed/owned paths, `1/2/4/8/16` refinement, rotation, mixed
connectivity reversal, a portal, thermal superposition, numeric range failures,
and raw cancellation followed by fresh replay. A planar restriction of the
existing thermal 3D solver cross-checks zero-temperature energy at two angles
and three mesh sizes. It is not an independent external solver or general 3D
thermal validation.

Cancellation tests observe existing preparation, assembly, matrix validation,
solution-restoration and result checkpoints. They cancel at retained stage
entry/terminal points and 64-item boundaries in a 130-node fixed chain, assert
that the raw Solver call errors, and compare a fresh replay with normal output.
Four additional headless tests build a batch, validate the execution plan,
resolve the bridge manifest and invoke the registered Engine operator. They
check energy/work, free thermal superposition, invalid-input propagation and
fresh replay, plus preservation of tiny nonzero numerical output.

| Executed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 full Solver, debug | 1075 | 0 | 9 |
| macOS ARM64 new headless target | 4 | 0 | 0 |
| Linux x86_64 selected Solver, release | 431 | 0 | 9 |
| Linux x86_64 new headless target, release | 4 | 0 | 0 |

Both hosts used Rust **1.88.0** with `--locked --offline` for the final lanes.
The first Linux headless attempt stopped before compilation because the offline
cache lacked `rustls 0.23.45`. A locked online run populated the cache and passed;
the final offline rerun also passed. The failed cache-preflight attempt is not
counted as test execution, and no lockfile or dependency version was changed.
Linux ran Solver `--lib` plus `frame_2d_output_reliability`, `frame_2d_review`,
`thermal_frame_2d_review`, `thermal_frame_2d_closed_form`,
`thermal_frame_objectivity`, `thermal_frame_mesh_convergence`,
`frame_input_reliability`, `beam_frame_classic_regression` and
`mechanical_convergence`. Its total is 372 unit and 59 integration tests.
The full macOS run covers callers of the shared frame stiffness helper,
including nonlinear, buckling, modal and cohesive paths. Passing those
regressions does not newly qualify their complete physical domains.
Overlapping test runs are not additive coverage percentages. The nine ignored
opt-in tests are not executed benchmark evidence.

Solver all-target Clippy and the new CLI target's Clippy passed with warnings
denied; workspace formatting also passed. Validation profiles
`beam-frame-classic` and `thermal-frame-2d-closed-form` executed all twelve and
seven commands successfully, respectively. Bounded evidence is registered in
`config/architecture/module-function-coverage-evidence/runtime-frame-fields.json`.
Tensor structure/self-test, document book/inventory and source-800/document-2000
organization gates passed. The tensor still has 0 structural gaps, 4 maturity
gaps, 16 evidence-grade gaps and 11 P0 gaps; overall release status remains
blocked. These regressions are executed assertions, not machine-checked formal
proofs, and do not discharge those separate release obligations.

Linux used temporary source-only scratch space and the existing managed Cargo
cache. Only that scratch directory was removed after successful completion;
the managed cache was retained. No installed service, credentials, server
configuration or persistent research data was changed. Local/remote SHA-256
matched for these paths relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/frame_2d.rs` | `4a7b6779bf7b8e8add58cd88ae612317e09a087ea111249cf50106ddac00e171` |
| `solver/src/frame_2d_element.rs` | `360092e429bfc716f7f5926f02c4a07223c2e874235e96a5a36b4f60f7d5f689` |
| `solver/src/frame_2d_system.rs` | `1921284d2eb0b7c8110aa977bc642e2995ff794b2f7351f5f3e594dafce095a7` |
| `solver/src/frame_2d_math.rs` | `8480889f8c9e7f13169daefb8de89f098994578c221c309caa9d5a89480f3ef6` |
| `solver/src/frame_2d_validation.rs` | `433ef4aa4d29ae8086977ea275c0ae0937c927d483bdd170b1d3f7b697ad0ff7` |
| `solver/src/frame_energy.rs` | `c6a4a7d8612312bb70ce42c3dc0f3676a2fb0aeedd47f5ff54687632b81df8ce` |
| `solver/tests/frame_2d_output_reliability.rs` | `79900b9818aa6310dd6bc9ae0198a07e6f4731fc99ff9e1e7d4f454d5cebe49d` |
| `cli/tests/frame_2d_output_operator.rs` | `b9bb9efef957a4e14d55222ccd9c00f3fbfcf4a93a92a6085f166e5a9ff0c565` |

## Limits And Reproduction

This is bounded planar-frame output validation, not general frame qualification.
Existing qualified scopes are unchanged. These operators remain linear,
prismatic Euler-Bernoulli members with nodal mechanical loads and per-element
thermal strain/curvature. No distributed frame load, shear deformation,
geometric nonlinearity, plasticity, contact or dynamic formulation was added.
The 3D thermal energy implementation and directional-support implementations
were not changed or newly qualified by this work.

The extreme `1e155`-length geometry test restrains unexcited transverse/rotation
DOFs to isolate axial arithmetic; the unrestrained bending block is severely
ill-conditioned at that scale. It does not establish arbitrary-condition-number
solvability or a realistic material range. Rigidity, local transforms,
thermal-curvature products and recovered fields still impose numerical limits.
Displacement/rotation maxima remain nodal samples, not continuous extrema.

No new installed-Agent, network-failure, GUI, Windows, performance or 1M-node
claim is made. Full assembled-matrix validation adds an `O(nnz)` pass; this is
a correctness change, not a measured speedup. Previously saved thermal-frame
energy results involving varying curvature need recalculation before research
reuse. Stored results were not rewritten. No version bump, commit or push was
performed for this repair.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver
cargo test --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_2d_output_operator
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked -p kyuubiki-cli --test frame_2d_output_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile beam-frame-classic --execute --out tmp/frame2d-beam-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile thermal-frame-2d-closed-form --execute --out tmp/frame2d-thermal-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
