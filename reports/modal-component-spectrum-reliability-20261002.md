# Modal Component Spectrum Reliability: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, with the current working-tree overlay,
including the preceding [assembly repair](modal-assembly-reliability-20261002.md).
This follow-up fixes loss of independent soft modes during dense modal
diagonalization. The change is confined to shared Solver numerical code;
Engine routes, task schemas, physical formulations and result fields are
unchanged. The shared Jacobi routine is also used by buckling and symmetric
critical-mode extraction, so those paths require regression coverage.

Four initial numerical tests and three public modal tests failed before the
repair. These are test cases, not seven independent defects. The public tests
first failed on their planar branch; the repaired cases execute both planar
and spatial branches. Assertions were not weakened to obtain a pass.

The former dense routine scaled the entire matrix by its largest entry. With
independent stiffness scales `1e-200` and `1e200`, valid soft entries rounded
to zero. A soft negative eigenvalue could similarly become negative zero.
An unrelated stiff diagonal also set the symmetry-check tolerance, allowing
a visibly asymmetric soft block through the shared numerical API. Finally,
nonzero connecting terms could silently become zero during global scaling.

## Numerical Contract

After finite/square validation, symmetry is checked against the scale of each
participating diagonal pair and its two coupling entries, not the largest
unrelated diagonal. The normalized comparison retains the `1e-10` tolerance
without subtracting large raw entries or underflowing a tiny absolute bound.

An iterative graph traversal finds independent matrix blocks. An edge exists
if either directed off-diagonal entry is exactly nonzero. There is no
threshold-based topology cut: the smallest positive subnormal is still an
edge, while positive and negative zero are not. Indices are deterministic,
and traversal observes cancellation in bounded chunks.

Each independent block uses the retained Jacobi rotations and stopping rule
with its own scale. Returned vectors are expanded into the original DOF
ordering and eigenpairs are sorted globally. For a connected matrix the
routine continues directly with the original matrix, without allocating a
second dense block copy. The exact algebraic reference is:

```text
P^T A P = diag(A_1, ..., A_k)
spectrum(A) = sorted union of spectrum(A_i)
each component eigenvector expands with zeros outside its component
```

This is a permutation of an exactly block-diagonal problem, not a physical
decoupling approximation. If scaling a genuinely connected block rounds a
nonzero entry to zero, the routine now returns an explicit range error rather
than quietly changing the coupling graph. This conservative failure policy
does not certify the relative accuracy of every surviving subnormal entry.

The existing dense DOF cap of 4096, sparse single-mode routing, eigenpair
residual checks, mode-selection policy and unit Euclidean returned-shape
contract are retained. General sparse probes remain bounded evidence, not a
global lowest-eigenvalue certificate. This does not repair earlier range loss
in callers that normalize a matrix before passing it to Jacobi.

## Coverage And Evidence

Six numerical tests cover interleaved independent blocks across scales
`1e-200`, `1`, `1e200`, orthonormality and relative residuals, local asymmetry,
retention of zero/negative modes, refusal to erase a nonzero connection,
exact graph discovery and cancellation at completed DOF 64 followed by replay.
The block `[[2s,s],[s,2s]]` has the independent reference eigenvalues `s,3s`.

Four added public tests extend `modal_spectrum_reliability` to 13 tests:

- Both 2D and 3D two-member assemblies preserve all six/twelve modes and agree
  between single-mode and complete-spectrum requests.
- Node permutation and connectivity reversal preserve the full spectrum.
- Each assembled independent component agrees with its isolated solve.
- A connected extreme-range problem fails explicitly and a healthy
  independent-component model can subsequently replay.

The public reference retains the existing one-member lumped-mass model with
`L=A=density=1`, `I_y=I_z=J=0.01`, `G=0.4E`. For each stiffness scale `s`,
planar eigenvalues are `s * [0.01*(60-12*sqrt(21)),
0.01*(60+12*sqrt(21)), 2]`. Spatial modes contain both bending pairs, the
torsional eigenvalue `0.096s`, and axial eigenvalue `2s`. These are discrete
model references, not measured material data or an external solver run.
Tests re-derive rad/s, Hz, period, frequency extrema and shape norms, require
correct counts/order, and check zero shape components outside the active
free tip. The public relative tolerance remains `1e-8`; direct numerical
block tests use `1e-12`.

Two additional Rust headless cases extend the modal target to seven tests.
They validate a batch/plan, resolve the bridge manifest, execute the registered
Engine operator, and check both extreme-scale spectra and propagation of a
connected range error followed by clean replay. No mock, GUI or installed
Agent is substituted for this in-process route.

| Completed lane | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| macOS ARM64 component numerical tests | 6 | 0 | 0 |
| macOS ARM64 modal spectrum target | 13 | 0 | 0 |
| macOS ARM64 unit tests plus spectrum/assembly targets | 407 | 0 | 9 |
| macOS ARM64 modal headless target | 7 | 0 | 0 |
| macOS ARM64 full Solver, debug | 1136 | 0 | 9 |
| macOS ARM64 modal profile, all ten commands | 53 | 0 | 0 |
| Linux x86_64 selected Solver, release | 480 | 0 | 9 |
| Linux x86_64 modal headless target, release | 7 | 0 | 0 |

Overlapping lanes are not additive coverage percentages. Ignored opt-in
tests provide no fresh benchmark evidence. The new component tests join the
`modal-frame-sanity` profile; the separate claim is recorded in
`config/architecture/module-function-coverage-evidence/runtime-frame-fields.json`.
These are executable assertions, not machine-checked formal proofs.

The full macOS run completed all 181 result groups. Solver all-target and CLI
modal-target Clippy passed with warnings denied. Formatting, whitespace,
document book/inventory, validation and tensor self-tests, and the source-800/
document-2000 organization gates passed. The reliability guide remains 1999
lines. Tensor structure and command checks pass, but overall status remains
blocked: 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0
gaps. This bounded claim does not remove broader qualification requirements.

Linux uses pinned Rust/Cargo 1.88.0 with `--release --locked --offline`, two
build jobs and the existing managed Cargo cache. Its 18 Solver result groups
contain 385 unit tests and 95 selected integration tests. The selected targets
are `modal_spectrum_reliability`, `modal_assembly_reliability`,
`modal_frame_input_reliability`, `modal_frame_2d_review`, `modal_frame_3d_review`,
`modal_frame_sanity_regression`, `mechanical_convergence`,
`frame_3d_orientation_reliability`, `buckling_assembly_reliability`,
`buckling_beam_frame_crosscheck`, `buckling_beam_1d_clustered_mode`,
`buckling_frame_2d_clustered_modes`, `buckling_frame_2d_portal`,
`buckling_beam_1d_closed_form`, `buckling_beam_1d_input_reliability`,
`buckling_frame_2d_closed_form` and `buckling_frame_2d_input_reliability`.
This is fresh Linux evidence for the current combined assembly/component
snapshot, not a retroactive pass for the preceding report's unreachable lane.

Local and remote SHA-256 match for these paths relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_math.rs` | `cdfcd22d519d8cdf11449581b6d453a5bbfc1c0fe24004e8e21085f63bfebb7f` |
| `solver/src/modal_math_component_tests.rs` | `c66433258099e0e2244dd3b1b5ea518f43d9eac1a530fcaec73427f402895eb6` |
| `solver/tests/modal_spectrum_reliability.rs` | `0e301cf444a4a8eaad5c00b9002a68d7f2b85387f7903190872cd573d0f5f408` |
| `cli/tests/modal_spectrum_operator.rs` | `7cf795b63237fffdd292106f9150cc70aa34fe0624f6b0926eb64e2260c06652` |

## Limits And Reproduction

This is bounded independent-component spectral validation, not general modal qualification.
The extreme numbers are arithmetic stress cases, not realistic material
parameters. It does not qualify arbitrary ill-conditioned connected models,
nonlinear dynamics, free-free modes, repeated-mode direction uniqueness,
general sparse spectral minimality or large-mesh performance. Splitting does
not promise improved asymptotic memory use for the already-dense fallback.
There is no new million-node, Windows, installed-Agent, distributed recovery
or physical material qualification claim.

Affected old results require recalculation before research reuse; stored
data is not rewritten. No version bump, commit, push or App rebuild is part
of this change. Remote tests use temporary source-only files and the existing
managed Cargo cache, not a deployment or change to running services.
After both remote jobs completed, the temporary source directory was removed;
the shared managed cache was retained. No server configuration or credentials
were copied into the repository.

Run from the repository root with pinned Rust 1.88.0:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_math::component_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_spectrum_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-component-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
