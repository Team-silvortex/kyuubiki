# Mass-Normalized Modal Inverse Reliability: 2026-10-02

## Scope And Reproductions

Base revision: `99e494d1`, daji 3.4.3, with the preceding uncommitted modal
reliability overlay. This round fixes artificial mass-coordinate conditioning
in the general single-mode inverse solve and reuses its prepared linear
system within a request. Fifteen new regressions cover five numerical,
six public-operator and four Rust headless cases. No Engine or protocol
change is required, and no new dependency is introduced.

The previous outer modal iteration already used `A = M^-1/2 K M^-1/2`, but
its inner inverse callback transformed each right-hand side back into
physical coordinates, solved `K`, and transformed the result back. It
repeated linear-system preparation on every inverse step. In exact
arithmetic these solves are equivalent; in floating-point arithmetic `K`
can have a large coordinate-induced condition number that `A` does not.

Two failures were reproduced before changing this callback:

- A three-dimensional operator `A = 1.6 I - 0.2 uu^T`, `u = [1,1,1]`,
  has spectrum `{1, 1.6, 1.6}`. With `K = sqrt(M) A sqrt(M)` and
  diagonal `M = [1e-24, 1, 1e24]`, the old inverse returned
  `system is singular` despite the well-conditioned normalized operator.
- A connected 66-element 2D bending cantilever, with 132 free DOFs,
  worked at segment length `L=1` but returned `system is singular` at
  `L=1e14`, with section area/inertia one, `E=L^3` and density `1/L`.
  Its normalized discrete stiffness is invariant under this scaling.

These are numerical coordinate tests, not material parameter recommendations.

## Implementation

The single-mode general sparse path now constructs its sparse normalized
stiffness once, using balanced products for each existing coefficient.
Representable couplings must survive intermediate underflow; genuinely
nonrepresentable nonzero entries and overflow return an explicit error.
No threshold is used to discard a small coupling.

The reduction to a standard symmetric eigenproblem follows the
[LAPACK generalized symmetric-definite eigenproblem description](https://www.netlib.org/lapack/lug/node54.html).
For the existing positive diagonal lumped mass, its Cholesky factor is
`sqrt(M)`. LAPACK is a method reference only; implementation remains native
Rust and the existing lumped-mass physics is unchanged.

`PreparedSpdSolver::factor_with_options` accepts the modal IC(0) options;
the existing default preparation entry point retains its default options.
The modal operation owns one prepared inverse and reuses it for both
deterministic probes and all their inner solves. There is no global cache,
cross-request ownership, or hidden persistent solver state. The unused
second physical sparse-matrix copy was removed from the reduced system.

Existing path limits remain intact: the single-mode full-spectrum fallback
is bounded at 128 free DOFs, general inner dense LU at 1024, and requested
complete-spectrum assembly at 4096. Recognized axial/tridiagonal modal
paths do not prepare this general inverse. Above the inner dense limit,
the existing diagonally scaled IC(0)/PCG path is used. This does not claim
that every sparse outer modal solve has a sparse inner factorization.

Accepted modes still pass the original physical-stiffness, mass-normalized
operator residual check. Both probes remain mandatory, with 128 iterations
and relative tolerance `1e-6`. There is no relaxed residual threshold or
extended iteration budget. Mode shapes retain unit Euclidean norm and
exact zeros at restrained DOFs.

## Independent References And Recovery

The small inverse has the exact solution
`A^-1 rhs = 0.625 rhs + 0.125 u (u^T rhs)`. Five mass vectors cover unity,
`1e-24` through `1e24`, and `1e-240` through `1e240`, with reversed
coordinate order and three right-hand sides each. Separate tests retain a
`-1e-300` normalized coupling whose naive intermediate product underflows,
and reject actual normalized underflow/overflow without cutting graph edges.

For the 66-element bending chain, every x translation is restrained; only
the root y translation/rotation are fixed. With `D=diag(1,L)` for each free
tip's y/rotation pair, `K(L)=D K(1) D` and `M(L)=D M(1) D`. Therefore all
normalized matrices coincide. Tests at `L=1`, `1e14`, `1e-10` compare the
lowest eigenvalue within `1e-7` relative error and recovered physical shapes
in common coordinates within `1e-6`. They also check full dimensions,
finite values, total mass, restraints and frequency/period consistency.
Both borrowed and owned public entry points are covered.

The connected 3D two-row fixture extends the previous unit-mass reference.
Each row has mass `m_r` per free tip, row coupling `c*m_r`, grounding
stiffness `(1 +/- delta)*m_r` and rung stiffness `w`. Adjusted grounding
densities account for all attached coupler masses. With unit-length members,
area one, `Iy=Iz=J=1/12` and `G=0.4E`, its normalized row-constant block is:

```text
a = 1-delta + w/m_0
d = 1+delta + w/m_1
b = w/sqrt(m_0*m_1)
B = [[a,-b],[-b,d]]
lambda_low/high = (a+d)/2 +/- hypot((a-d)/2,b)
```

The remaining row-path spectrum is shifted by `c*mu_j`, `c=10000`,
`mu_j=2*(1-cos(j*pi/n))`. The tested first two roots are the row-constant
ones. At `n=67`, sparse single-mode and dense two-mode outputs are compared
against these independent roots for masses `[1,4]`, `[4,1]`, `[0.5,10]`
and contrasts `0`, `0.01`. Shapes are checked against the mass-whitened
two-by-two residual, not an incorrect unit-mass reference.

At `n=513`, there are 2052 nodes and 1026 free DOFs. Solver observers confirm
exactly one IC(0) preparation, multiple inner PCG solves, zero dense LU
preparations and both modal probes. This is direct coverage above the real
inner sparse threshold; the earlier 134-DOF outer-sparse case did not prove
that. These counts prove preparation reuse, not a measured general speedup.

Cancellation is injected inside normalization, dense factorization,
substitution, IC(0) factorization and PCG iteration. The actual operation
must fail before observer-scope exit, then the same valid request replays.
A numerical test also cancels a solve and reuses the very same prepared
factor for a successful subsequent solve. Independent calls at different
coordinate scales each prepare their own factor.

Four new headless regressions run a real Rust execution batch, plan, bridge
route and in-process Engine operator. They cover the coordinate-scaled
bending model, actual 1026-DOF inner PCG and both dense/sparse cancellation
paths. These are not GUI mocks, remote-agent tests or Python/Elixir checks.

## Validation

| Lane | Result |
| --- | --- |
| macOS ARM64 mass-inverse numerical target | 5 passed, 0 failed |
| macOS ARM64 bending mass-coordinate public target | 3 passed, 0 failed |
| macOS ARM64 connected public target | 9 passed, 0 failed |
| macOS ARM64 modal Rust headless target | 24 passed, 0 failed |
| macOS ARM64 Solver library | 439 passed, 0 failed, 9 ignored |
| macOS ARM64 complete modal profile, twenty-one commands | 163 passed, 0 failed |
| macOS ARM64 full Solver suite, library/integration/doc harnesses | 1,229 passed, 0 failed, 9 ignored |
| Strict Clippy, formatting and repository gates | Passed |

The full Solver run exited successfully with 186 completed result groups;
the doc-test harness completed with zero tests. Runtime and test sources
were frozen before this run. The complete modal profile also exited
successfully, with 21 commands and `ok=true` in the generated artifact.
Strict Clippy passed for all Solver targets and the modal CLI target with
warnings denied. Formatting, operator-validation and tensor self-tests,
documentation book/inventory, whitespace and the source-800/document-2000
organization checks passed. The reliability guide remains at 1999 newline
lines. The nine ignored tests are existing opt-in benchmarks/reference work,
not new evidence. Overlapping lanes must not be summed into coverage.
Historical reports retain their own counts and snapshots.

Tensor structure and command checks pass, but its overall status remains
blocked: 0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps. This round does not certify complete project readiness.

Frozen code SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/linear_spd_prepared.rs` | `0601eff937e33526980f26eb593b7e09a0e8b6a86c3d3ebf1f82ef4bf3ee47f5` |
| `solver/src/modal_frame_assembly.rs` | `19cddfac2b3799a327ec61c343a93f49df4a0cc1b808ac903b212cefa8b8fc1a` |
| `solver/src/modal_frame_spectrum.rs` | `36f3f0856d845930ff61ddf44810424b8de8524c1c830430f2323e88bf0c8dfb` |
| `solver/src/modal_sparse.rs` | `977a85b90220b7a147fb3ffa4aa3f7d96c85163124464814f2e427e2233f6286` |
| `solver/src/modal_mass_inverse_tests.rs` | `6e0172459ab120331c48a5e6abc4e4950cfe32d92772bd79bd2398650bca64eb` |
| `solver/tests/modal_mass_scaling_reliability.rs` | `db9c3d299b38a81fb12ae20799a7500a7386ddefd9e58e3d9c639f9b1168a386` |
| `solver/tests/modal_connected_reliability.rs` | `dfaf3050d9819fe8d454d70e01422793c6f76ab785405241eaae3d0a92e49d87` |
| `solver/tests/modal_connected_reliability/nonuniform.rs` | `926d47b12ac11486e9c5892729db6049fe43cfec6e5d88638de98a65c7154dcd` |
| `cli/tests/modal_spectrum_operator.rs` | `5eb3ffedd5358a8df9ce8e816f19a4b5ec9dea1bcf865f3ab57e5583789e44f1` |
| `cli/tests/modal_spectrum_operator/connected.rs` | `298c5ed9d603def5ea4fa790301513ad3dbd772d07ec3d9f1646dd3af3a69cc2` |
| `cli/tests/modal_spectrum_operator/mass_scaling.rs` | `b0087d18babbac6eba0e409f2db82f20a7b1ddc065f638eacace0abc60f274f4` |

## Remaining Boundary

This is bounded mass-normalized inverse validation, not general modal qualification.
The same 66-element bending chain at `L=1`, when requesting two modes,
still fails the complete-spectrum gate with
`modal frame mode 0 failed its relative residual check`. This is a tracked
availability/accuracy limitation, not a successful reference run. The
mass-coordinate regression therefore uses the exact discrete congruence
identity and a valid single-mode baseline; it does not claim an independent
dense reference for this slender chain. The two-mode reference checks for
the separate grounded-row fixture do pass. Follow-up work must resolve
the slender-chain complete-spectrum accuracy without weakening validation.

Finite deterministic probes and a small residual do not certify the global
lowest mode for every matrix. This round does not establish arbitrary
ill-conditioned spectra, general nonuniform material convergence,
consistent mass, nonlinear dynamics, experimental validity or 1M modal
performance. Registration is bounded to headless/solver execution,
numerical validation and recovery; unrelated tensor coordinates are not
promoted.

All work is local macOS ARM64 with pinned Rust 1.88.0. Linux is intentionally
not tested. No server access, deployment, version bump, Git commit/push,
installed GUI test or App rebuild is part of this round. Models are
generated in tests; no large fixtures or outputs are tracked. Execution
evidence belongs in ignored `tmp/modal-mass-inverse-validation.json` and
test logs outside the repository.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_sparse::mass_inverse_tests
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_mass_scaling_reliability --test modal_connected_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-mass-inverse-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
