# Modal Matrix Normalization Reliability

## Scope And Findings

On 2026-10-02, the daji 3.4.3 working tree based on `99e494d1` was checked
with its preceding uncommitted modal reliability changes preserved. This
round unifies construction of the bounded dense Jacobi matrix and prepared
inverse matrix. It fixes silent normalized-coupling loss and removes repeated
sparse products during dense construction. Agent, Engine, SDK and protocol
responsibilities do not change.

Seven new matrix regressions reproduced six failures before the production
change. One common-scale reference already passed. The most important
reproduction has physical stiffness diagonals `[1e200, 1e-200]`, matching
masses and symmetric off-diagonal stiffness `-1e-300`. Both normalized
couplings are representable, but the old dense constructor returned zero
in one triangle. Its intermediate product underflowed before row mass
weighting could recover the coupling. The prepared inverse constructor
already retained that same coupling.

A second case has unit physical diagonals, masses `[1e200, 1e200]` and
coupling `1e-300`. The final normalized coupling is not representable.
Instead of rejecting it, the old dense constructor silently produced zero,
allowing component discovery to treat a connected matrix as disconnected.
The new constructor rejects such loss, including non-finite normalized entries.

The other failed regressions exposed per-column sparse-product replay, no
allocation limit inside the builder itself, and no normalization-stage
cancellation at row and wide-row boundaries. Previously the public caller
owned the existing size guard; this was a private-entrypoint defense gap,
not proof that the public solver admitted models beyond its stated limit.

## Architecture And Numerical Change

`solver/src/modal_normalization.rs` owns one range-checked sparse-entry walk.
Both dense construction and prepared inverse construction use this walk and
the same balanced factor multiplication. Identical symmetric physical entries
and masses therefore produce bit-identical normalized entries in both
triangles and both matrix representations. No threshold cuts weak couplings.

The dense builder directly assigns normalized stored entries into its output.
The previous implementation applied the entire sparse operator to every unit
basis vector. For `n` free DOFs and `nnz` stored entries, construction changes
from `O(n*nnz + n^2)` to `O(nnz + n^2)`. The dense output still requires
quadratic storage and initialization; this does not make complete-spectrum
Jacobi suitable for million-node models. The existing 4096-DOF guard is now
enforced by the allocating builder, before allocation, rather than repeated
by its frame-spectrum caller.

Cancellation is checked before allocation and during stored-entry traversal,
using `SparseMatrixScale` and `SparseMatrixScaleRow`. A canceled or invalid
construction returns no matrix and does not mutate the source operator.
Sparse and dense replay remain valid. The prepared inverse remains local to
one request and its factor reuse policy is unchanged.

This does not replace the physical compensated operator with the rounded
normalized matrix as a residual oracle. Original physical residual checks,
mass formulation, output normalization, four-step correction budgets and
eigen-search budgets are unchanged. Generic sparse products and the recognized
axial-chain fast path are not rewritten in this round.

## Regression And Recovery Coverage

The final addition is ten test cases: eight matrix tests, including one
non-gating debug microbenchmark, and two public solver recovery tests.

- The extreme heterogeneous-mass reproduction retains both weak couplings
  and compares dense and prepared-inverse entries bit for bit.
- Unrepresentable coupling loss and non-finite normalization are explicit
  errors, not successful independent matrices.
- An independent tridiagonal discrete reference is retained at common scales
  `1`, `1e-200`, `1e200` and subnormal `1e-318`.
- Instrumented dense construction starts zero sparse matvecs. A direct
  4097-DOF call is rejected by the existing 4096-DOF limit.
- Row and wide-row cancellation interrupt construction before returning a
  matrix, followed by valid sparse and dense replay.
- Public 66-element planar and spatial cantilevers cancel at normalization
  row 64, before eigen-search or inverse preparation. The solve itself must
  fail inside the observer scope. Replay yields six modes, the spatial
  repeated pairs are retained, and independently assembled physical residuals
  remain at or below `1e-8`.

## Bounded Construction Measurements

The retained isolated macOS ARM64 Rust 1.88.0 debug run alternates the legacy
column algorithm and direct construction over three samples per size. Its
unit-mass symmetric path matrices have diagonal four and off-diagonal minus
one. All output entries agree bit for bit. Timings include output allocation
and construction, but exclude stiffness assembly and the eigensolve.

| Free DOFs | Stored entries | Legacy median us | Direct median us |
| ---: | ---: | ---: | ---: |
| 128 | 382 | 3025 | 112 |
| 512 | 1534 | 28122 | 320 |
| 1024 | 3070 | 112409 | 830 |

No assertion depends on timing or a speed ratio. Subsequent runs may differ
with host load. These are unoptimized local microbenchmarks, not release-build
throughput, full-model solve-time improvement, memory-pressure qualification
or installed/remote Agent performance evidence.

## Validation Results

| Lane | Result |
| --- | --- |
| Final matrix normalization target | 8 passed, 0 failed, 0 ignored |
| Public modal mass and bending target | 24 passed, 0 failed, 0 ignored |
| Full Solver suite including integration and doc harnesses | 1285 passed, 0 failed, 9 existing ignored; 186 result groups |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Full CLI suite including live modal Agent and Rust headless journeys | 488 passed, 0 failed, 4 existing ignored; 43 result groups |
| Executed modal profile | 28 commands, 251 passed, 0 failed, 0 ignored |
| Strict Solver Clippy on all targets | Passed with warnings denied |

The overlapping lanes must not be summed as distinct coverage. The native
modal profile artifact has `executed=true` and `ok=true`. No ignored test was
added, no package dependency was added, and this is not a new packaged
application or remote Linux qualification run.

Formatting, whitespace, documentation book/inventory and source-800 /
document-2000 organization checks passed. Operator profile self-tests and
the retained execution report validation passed. Tensor structure and
command checks pass with 13 modules, 11 paradigms and zero structural gaps;
release readiness remains blocked with four maturity gaps, sixteen
evidence-grade gaps and eleven P0 gaps. The new bounded claims do not
promote a qualification scenario or remove those obligations.

Final SHA-256 fingerprints, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_normalization.rs` | `fb1ee05c305ece66e5d6c1e4101d246dfbacae98c9e1fbd8176fe7bdce9dd9b5` |
| `solver/src/modal_normalization_tests.rs` | `ac0e3ea39aaddfd2404615c89d38997a5d5222bfd59f05f1d2dda6557b8bec3c` |
| `solver/src/modal_sparse.rs` | `0718416c72e7c21f9552776271ad5a443650d0ad591979a34653ea06b7ce7d4c` |
| `solver/src/modal_frame_spectrum.rs` | `23e0b73538039e3907ff90dc14fba41a586be85e4568c41ac582eae9c556ab3d` |
| `solver/src/modal_mass_inverse_tests.rs` | `9f9c5294783ae5bbaf4f64f2754fc3ff36c43366291d31af111e8f54ccb2dcf3` |
| `solver/tests/modal_mass_scaling_reliability.rs` | `7ec4971e6c7d553a5200a0fcb3331b4383aa6919929780d0cd2aeeb296eda4ea` |
| `solver/tests/modal_mass_scaling_reliability/normalization.rs` | `a4b6d64196ea2dc18256c93424ea371179bcd9031b1ba109133e8d1ca1e59015` |

## Remaining Boundaries

This is bounded modal matrix normalization validation, not general modal qualification.
The existing 128-element unresolved six-mode planar case still fails closed
within its original refinement budget. Public bending modes are checked
through the existing complete planar 96/100-element and spatial six-mode
100-element fixtures, not an arbitrary complete spatial spectrum.

The change is registered as separately scoped verified tensor evidence.
Debug construction measurements cannot close full-engine benchmark security,
contract, installed-platform or sustained-load obligations. Industrial
reference correlation, broader model conditioning and remote/installed
qualification remain separate work.
