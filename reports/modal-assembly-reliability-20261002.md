# Modal Frame Assembly Reliability: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, plus the current working-tree overlay.
This repair covers `solve.modal_frame_2d` and `solve.modal_frame_3d`. Shared
assembly helpers stay inside Solver; Engine dispatch, task schemas, GUI and
public result fields are unchanged. This is not a new physical formulation.

All seven initial public regression cases failed before the repair. These are
test cases, not seven independent defects. The initial loop stopped at its
first failing dimension; the repaired suite executes both 2D and 3D branches.
The final target adds two further rotation/cancellation cases, for nine tests.
Original numerical assertion tolerances were not relaxed to obtain a pass.

The preceding path could reject finite representable results or accept an
invalid model after restraint reduction removed its invalid entries:

- `density * area * length` could overflow or underflow at an intermediate
  multiplication even when the final element mass was representable.
- For `m=3e307` and `L=3`, `m*L*L/24` overflowed before division, although
  the nodal rotary mass `1.125e307` is finite.
- A finite length `1e155` was rejected by the squared-distance input check.
- Lost local bending stiffness, invalid fully restrained members and
  overflowed assembled stiffness could be hidden by removing fixed DOFs.
- Invalid rotary mass at fixed rotations, including overflow from summing
  individually finite contributions, could survive as a successful spectrum.

## Numerical Contract

The existing lumped approximation is retained:

```text
element mass:               m = density * area * length
nodal translational mass:   m / 2
nodal rotary mass:          m * length^2 / 24
generalized eigenproblem:  K * phi = lambda * M * phi
axial-only one-member case: lambda = 2 * E / (density * length^2)
```

The analytical axial reference follows from the retained discrete model:
`K=E*A/L`, `M=density*A*L/2`. It is not an external solver measurement or a
continuum beam-frequency claim. Uniformly scaling stiffness and density by the
same positive factor preserves the generalized eigenvalues and frequencies.
These relations independently constrain outputs rather than checking only
successful return or agreement between two routes to the same implementation.

Positive mass factors are ordered and combined according to the running
product magnitude to avoid unnecessary intermediate overflow/underflow. The
rotary factor includes the division by 24 during that balanced product.
Element mass and both nodal mass terms must still be positive and finite;
true range loss, including a half-nodal mass rounded to zero, is rejected.
This does not provide arbitrary precision or certify every subnormal result's
relative accuracy. Input lengths use `hypot`; the length floor remains `1e-12`.

Local and transformed element stiffness are checked for finite entries and
positive diagonal entries. Full assembled mass and stiffness are checked
before eliminating restrained DOFs. A fixed DOF is not an exemption from
model validity: previously accepted inputs with unrepresentable inactive
bending or rotary terms now fail explicitly. Diagnostics identify the element
and local/global stage, or the assembled DOF. The full-system scan checks for
cancellation in bounded chunks, before reduction can discard those rows.

The modal eigensolver, mode selection and result normalization are unchanged.
Expanded shapes retain their unit Euclidean participation norm, not a
mass-normalized return contract. Existing sparse probes and residual checks
are not global lowest-eigenvalue certificates for arbitrary large systems.

## Coverage And Evidence

Three internal tests check balanced products, finite rotary mass despite the
old intermediate overflow, and rejection of an unrepresentable half-nodal
mass. The nine public tests cover both modal dimensions:

- Common stiffness/density factors `1e-200`, `1`, `1e200`, `1e307`, retaining
  all three planar or six spatial fixed-root/free-tip modes.
- Analytically finite masses and axial eigenvalues after intermediate product
  overflow/underflow, and a finite member whose squared length overflows.
- Invalid local bending terms, fully fixed local/assembled stiffness and
  individual/assembled rotary mass; a healthy model replays after rejection.
- Rigid rotation and reversed connectivity at common scale `1e307`. The
  spatial case uses equal section bending inertias and isotropic nodal masses;
  it does not certify arbitrary anisotropic assembled modal objectivity.
- Cancellation at full-system DOF 64, before reducing 240/480 DOFs to 40 free
  axial DOFs. The test requires failure inside the solve, not merely at
  observer scope exit, then verifies a clean replay and the axial reference.

Successful product/scaling cases compare borrowed and owned Solver routes.
Returned eigenvalue, rad/s, Hz and period must agree, expanded restrained DOFs
must be zero, and the Euclidean shape norm is recomputed from returned values.
The comparative relative tolerance remains `2e-10`.

Three additional Rust headless tests extend the existing modal target to five
tests. They construct and validate a batch/plan, resolve the bridge manifest,
and invoke the registered Engine operator. They check common-scale spectra,
propagation of restrained mass/assembled-stiffness failures and valid replay.
This is actual in-process dispatch, not a mock or an installed Agent run.

| Completed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 modal assembly unit tests | 3 | 0 | 0 |
| macOS ARM64 public modal assembly target | 9 | 0 | 0 |
| macOS ARM64 expanded Solver selection, including unit tests | 440 | 0 | 9 |
| macOS ARM64 modal headless target | 5 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1126 | 0 | 9 |
| macOS ARM64 modal profile, all nine commands | 41 | 0 | 0 |

The expanded selection uses `--lib` plus `modal_assembly_reliability`,
`modal_spectrum_reliability`, `modal_frame_input_reliability`,
`modal_frame_2d_review`, `modal_frame_3d_review`, `modal_frame_sanity_regression`,
`operator_modal_reliability`, `mechanical_convergence` and
`frame_3d_orientation_reliability`. Overlapping lanes are not additive coverage
percentages. The ignored opt-in tests provide no fresh benchmark evidence.
The full run completed all 181 result groups without failures; the profile
also reran the final assembly tests after strengthening their mode-count
assertions. Solver all-target and CLI modal-target Clippy passed with warnings
denied. Workspace formatting, whitespace, document book/inventory, validation
and tensor self-tests, and source-800/document-2000 organization gates passed.
The reliability guide stays within its limit at 1999 lines.

The `modal-frame-sanity` validation profile registers and executes the public
assembly target and three internal product checks, retaining the headless route.
The separate numerical/recovery claim is registered in
`config/architecture/module-function-coverage-evidence/runtime-frame-fields.json`.
Tensor structure and command checks pass; the overall status remains blocked,
with 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps.
These executable numerical assertions are not machine-checked formal proofs
and do not promote general physical qualification.

Key tested source fingerprints, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_frame_assembly.rs` | `07d11430531d9754846fe34aef6b359adc808478a47eb17f634fbb334c070f1a` |
| `solver/tests/modal_assembly_reliability.rs` | `944a285daa07f80f00416a52bd55c9873dc4275175dd2ffe0f553c223981b57d` |
| `cli/tests/modal_spectrum_operator.rs` | `b4112796e7ac2cca026c9c855fc3bcac93cbd9d87f99da19a62af858f24d85d2` |

Fresh Linux verification was not completed: both the local server discovery
service and SSH connection timed out. No server files or configuration were
changed. Earlier Linux evidence remains dated evidence for its earlier source
snapshot and is not reused as a pass for this repair.

## Limits And Use

This is bounded modal-assembly validation, not general dynamic or large-mesh qualification.
The extreme ranges are arithmetic stress cases, not realistic material models.
This change does not introduce consistent mass, free-free modes, nonlinear
dynamics, shear deformation, arbitrary precision, an external solver comparison
or new physical benchmark data. No performance, million-node, Windows, GUI,
installed-Agent or distributed recovery claim is made.

Previously saved results affected by hidden invalid terms require corrected
inputs and recalculation before research reuse; stored data is not rewritten.
The existing cantilever qualification remains bounded to its original scope.
No version bump, commit, push or App rebuild is part of this change.

Run from the repository root with the pinned Rust 1.88.0 toolchain:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_frame_assembly
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_assembly_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-assembly-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
