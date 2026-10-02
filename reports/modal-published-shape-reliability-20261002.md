# Published Modal Shape Residual and Recovery

Date: 2026-10-02
Version: daji 3.4.3 working tree
Scope: local macOS, Rust Solver and in-process Rust headless plan-to-Engine execution

This is bounded published-modal-shape validation, not general modal qualification.

## Reproduced Defect

The [previous residual-polish report](modal-polished-bending-reliability-20261002.md)
validated the internal mass-normalized eigenvectors. Its historical measurements
are preserved, but they did not establish that the final normalized physical
shapes still passed the same residual gate. Shape recovery and unit normalization
can introduce fresh rounding, especially for low bending modes.

An independent physical `K - lambda M` reconstruction found that the returned
first modes at unit coordinates had relative residuals `1.4869619139406593e-8`
for 96 elements and `1.908873760270132e-8` for 100 elements. Both requests had
returned success under a `1e-8` internal gate. This was an output-validation bug,
not evidence that the gate should be relaxed.

## Numerical Change

After shape recovery, evaluate the actual returned physical vector `phi`:

```text
D = M^(-1/2)
r = D (K phi - lambda M phi)
relative = ||r|| / max(||D K phi||, ||lambda D M phi||)
```

The original assembled reduced mass is retained, rather than reconstructed from
rounded inverse square roots. The spectrum carries its existing path-specific
tolerance to output validation: `1e-8` for dense spectra, the existing `1e-6`
sparse inverse gate, and the recognized axial path's existing `2e-4` floor.
None of those limits is changed by this work.

Physical products balance `K_ij`, `phi_j` and `D_i` by multiplying the smallest
and largest magnitudes first, retaining both multiplication roundoff terms with
FMA and compensating the row sum. Computing `K phi` before weighting can lose
significant digits when stiffness is subnormal, even if `D K phi` is normal.
The first implementation exposed two such failures in existing headless extreme
mass/axial tests; balanced products restored them without changing the tests.
This does not remove final binary64 range or conditioning limits.

Accurate shapes return without a repair. Otherwise, at most four forward/reverse
coordinate sweep pairs minimize the shifted physical residual. For each column
`b_j` of `B = D (K - lambda M)`, the proposed correction is
`delta = (b_j^T r) / (b_j^T b_j)`. Coefficients are scaled before dot products.
The cached residual is updated using the actual rounded change in `phi_j`.
Symmetric stiffness rows provide column directions without an extra matrix;
after each pair, a fresh compensated row product decides acceptance, protecting
against cached-residual drift and assembly asymmetry.

Only a strictly improved, independently recomputed residual can progress, and
only a shape satisfying the original tolerance is published. Eigenvalues are
unchanged. The shape is not normalized again after validation; its Euclidean
norm must remain within `1e-10` of one, and the reported participation norm is
the actual checked norm. Restrained entries remain zero.

All candidate modifications stay private. Failure, exhausted budget, invalid
data or cancellation returns an error without changing the caller's seed or
publishing partial Solver/Engine results. Checkpoints cover physical input and
row products, residual assembly, coordinate sweeps, wide columns and shape
mapping. Fresh requests can replay after cancellation.

This is bounded coordinate relaxation, not a general convergence proof or a
new eigensolver. It adds no inverse factorization or dense matrix. Work is
linear in sparse entries per sweep pair, with linear-size vectors; retaining
the original reduced mass costs eight bytes per free DOF. There is no claim of
unchanged runtime or benchmark improvement. Solver owns the numerical change;
Engine dispatch, task representation and SDK protocols remain unchanged.

## Independent Coverage

The test-only reference reconstructs unit-length Euler-Bernoulli stiffness
rows and the physical lumped masses, without using the production assembler
or converting the output back into an internal eigenvector. Two orthogonal
bending planes are checked together for the spatial fixture.

- Every returned mode is checked for 96/100-element 2D requests with 6, 20 and
  all 192/200 modes, including reversed numbering at unit coordinates.
- Unit-coordinate 66/80-element six-mode cases remain independently checked.
- The unit-coordinate spatial checks cover three repeated bending pairs at
  66 and 100 elements, not a full 400-mode spatial spectrum.
- Existing coordinate scales `1`, `1e14` and `1e-10` retain root, unit-shape,
  fixed-DOF and mass-orthogonality checks. The independent physical assembly
  reference in this report is specifically for unit coordinates.
- Actual Rust headless plan-to-Engine calls check physical shapes in result values
  for 96/100-element 2D twenty-mode and 100-element 3D six-mode requests.
- Output-phase cancellation is distinguished from internal eigenvector
  refinement, and both Solver and Engine reject before returning partial modes.
- Private tests cover matching dimensions, non-finite/zero inputs, original
  mass weighting, common scales `1e200` and `1e-200`, subnormal stiffness,
  no new factorization, budget exhaustion, wide-column cancellation and replay.

At unit coordinates, the measured worst independent output residual over the
forward-numbered 96-element requests is about `7.312e-9`; the corresponding
100-element value is about `6.889e-9`. All modes, not only the first, are asserted
against `1e-8`. Reversed-numbering cases are separately asserted.

## Remaining Boundary

The 128-element six-mode bending request still fails its internal residual gate
within the existing four inverse corrections. Solver and headless tests retain
that explicit rejection and valid replay. It is not successful 128-element
coverage. Fixture sizes are not universal model limits.

This report does not qualify arbitrary frames, free-free modes, damping,
nonlinear dynamics, installed-Agent recovery, text/network JSON round trips,
Linux execution or industrial accuracy. Internal spectra, published shapes and
headless result values are distinct validation boundaries; none should be
inferred solely from another.

## Verification

Twelve new retained tests: nine private numerical tests, two public Solver
tests and one in-process Rust headless test. Existing full-spectrum, spatial
and headless cases also gain independent output-residual assertions.

| Check | Result |
| --- | --- |
| Private published-shape target | 9 passed |
| Mass-coordinate / complete-bending public target | 20 passed |
| Rust headless modal target, executed through the profile | 33 passed |
| Strict Solver all-target and CLI modal Clippy | Passed, warnings denied |
| Full Solver suite | 1273 passed, 0 failed, 9 existing ignored; 186 result groups including doc tests |
| Executed modal validation profile | 24 commands, 216 passed, 0 failed; artifact `ok=true` |
| Documentation / tensor / organization gates | Passed, including operator-validation and tensor self-tests |
| Formatting, diff check, source fingerprints | Passed; thirteen recorded fingerprints verified |

Test totals from overlapping suites must not be added together. The nine
existing ignored tests are eight opt-in microbenchmarks and one retained
postprocess physical comparison; no ignored test was added.
The tensor remains globally `blocked`: zero structural gaps,
four maturity gaps, sixteen evidence-grade gaps and eleven P0 gaps. This bounded
numerical and in-process recovery evidence does not close those global gaps.

The local machine-readable artifact is intentionally ignored:
`tmp/modal-published-shape-validation.json`. Run it with the native runner:

```text
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-published-shape-validation.json
```

## Source Fingerprints

SHA-256 at the start of the final full-suite run; paths are repository-relative.

```text
ea710121be8cd069b1952ab7d464f399827d341aec31b4ae6042dd797b81b178  workers/rust/crates/solver/src/modal_published_shape.rs
e5ebe6106e5a1399af1d546dd261ddcac4a33ab35e16132b2a93db02f7807147  workers/rust/crates/solver/src/modal_published_shape_tests.rs
9a113c274113c5fc2f26aa6c28d380a0f4f129b1f446a285c7db77ead474cd53  workers/rust/crates/solver/src/modal_sparse_product.rs
fd49171614cfa475605a38bc85e496f702b6d7b25ef30bb136b84ae036c787b5  workers/rust/crates/solver/src/modal_sparse.rs
b581d638376031201086924b252df2f122b6592b3ce1693d55a8572890272457  workers/rust/crates/solver/src/modal_frame_spectrum.rs
80687ea3aee6578cfd06d397d6a65e59e75b222865a5ed068da3cf5fcd987e60  workers/rust/crates/solver/src/modal_frame_2d.rs
c54e28a58339b9d85376f8400c81b5e1f49a2baca40e524e058a87a4fb6fd1d2  workers/rust/crates/solver/src/modal_frame_3d.rs
f54431abd09c8f95c8c421f8e6840aec5e10fedac702cdf53ba4d985ef4ea891  workers/rust/crates/solver/tests/support/modal_published_reference.rs
fe2f8ba66c3707a1864e198498a796c05e10ae1bfb60028d4c068943d7f9e428  workers/rust/crates/solver/tests/modal_mass_scaling_reliability.rs
d4cf0d93836ec6c077537ddd316e86dccb54fef78c9eab49327e2e9d44e6d18a  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/published.rs
8901ab9e77684f377e483a3a4d32ed3d83e987389d4305d239b0bd10b0e19cfc  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/polished.rs
8f326303780cb755f50d25f1ef56cf7e82d56b58237bdb0c8589e0c12b115485  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/spatial.rs
e55e65dffc0925549f188243ee2c629e4c82e4ebc05e1903d78f943e642631aa  workers/rust/crates/cli/tests/modal_spectrum_operator/polished_bending.rs
```
