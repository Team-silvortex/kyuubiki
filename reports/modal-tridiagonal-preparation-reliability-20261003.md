# Sparse Modal Tridiagonal Preparation Reliability

## Scope And Reproductions

On 2026-10-03, the daji 3.4.4 source tree based on `155d6bc6` was checked
on macOS ARM64. This round closes the tridiagonal preprocessing follow-up
identified in the [sparse product report](modal-sparse-product-range-reliability-20261002.md).
Algorithms remain in Solver. Agent, Engine, SDK, TaskIR, model admissibility,
search budgets and numerical acceptance tolerances are unchanged.

Eight new kernel tests reproduced six failures before the implementation
changed. Two topology/discrete-spectrum checks already passed. A two-DOF
fixture has physical diagonal stiffness `[2e200, 5e-200]`, mass
`[1e200, 1e-200]` and signed symmetric couplings of magnitude `1e-300` or
`1e-220`. Fixed-order multiplication previously lost or distorted a coupling
before the compensating mass factor could recover it. The normalized diagonal
is `[2, 5]`; the independent mode-component ratio is approximately
`-coupling / 3`, including reversed DOF order and positive couplings.

Other failures silently declined intrinsically unrepresentable normalized
entries, ignored mismatched or missing lower-triangle entries, or entered
spectral search/residual products before observing preprocessing cancellation.
Cancellation in a valid tridiagonal prefix could also be swallowed by a later
nonbanded entry being interpreted as unsupported topology.

## Implementation Boundary

`modal_sparse_tridiagonal.rs` owns physical-band extraction, uniform-chain
admission and general tridiagonal normalization. CSR entries are extracted
once for both paths. The lower band is checked against the upper band before
being released; missing partners and unequal coefficients fail explicitly,
rather than solving a different symmetric matrix. For at least two DOFs, only
genuine zero couplings or nonbanded topology decline the fast path. Errors and cancellation use a
`Result<Option<_>, _>` internally and cannot become a normal `None` fallback.

`modal_normalization.rs` exposes the same checked scalar normalization to
dense construction, the prepared sparse inverse and the tridiagonal path.
Balanced multiplication preserves representable weak couplings; intrinsically
non-finite entries or nonzero entries rounded to zero fail explicitly. There
is no coupling cutoff, independent numerical implementation or coefficient
cache. The general path normalizes the extracted bands in place.

Physical structure admission precedes uniform-chain mass reconstruction,
avoiding that allocation for immediately nonuniform models. Extraction,
symmetry checks, uniform admission, mass reconstruction, vector construction,
norm accumulation and vector normalization have cooperative 64-entry
checkpoints. Residual norm accumulation is checked in the same block size.
The existing Sturm search and residual acceptance remain unchanged, including
the existing bounded `2e-4` uniform-chain floor. Storage and preparation remain
linear; this round makes no measured throughput or whole-solve speedup claim.

## Independent Regression And Public Replay

- Weak signed couplings and their tiny mode components survive either DOF
  order; the compensated original-operator residual remains below `1e-9`.
- Intrinsic normalized range loss fails explicitly and is followed by a valid
  replay. Asymmetric, opposite-sign and missing triangle partners also fail.
- Zero-coupling and nonbanded fixtures still decline the fast path. Cancellation
  in the valid prefix fails before a later topology decline can swallow it.
- Generic constant-diagonal chains with 2, 6 and 129 free DOFs match independent
  discrete sine eigenvalues/directions at `2^-600`, `1` and `2^600`.
- Uniform preparation cancels before search/products in extraction, the
  separate symmetry pass, admission, mass reconstruction, vector construction,
  norm accumulation and normalization. The same immutable operator replays.
- Public planar/spatial solvers use 129-segment uniform and periodic
  heterogeneous-modulus/density chains at all three binary scales. They cancel
  in preprocessing, return an error inside the solver call, then replay.
- Published shapes retain restrained zeros and unit Euclidean participation.
  Frequency/period consistency, the uniform discrete root and heterogeneous
  Rayleigh bounds are checked independently. A segment-wise physical
  `K*phi - lambda*M*phi` reference removes only the common scale and retains
  the heterogeneous field; every sampled relative residual is below `1e-9`.
- Owned and borrowed public entrypoints produce identical replay results.

The dedicated preparation target passes 8 tests. The public axial-chain target
passes 18 tests, including the two new preparation/replay cases. The unchanged
uniform-admission and general Sturm targets pass 9 and 11 tests respectively.
These lanes overlap and must not be summed as distinct coverage.

## Validation Results

| Lane | Result |
| --- | --- |
| Full Solver suite including integration and doc harnesses | 1310 passed, 0 failed, 9 existing ignored; 186 result groups |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Executed modal profile | 29 commands, 276 passed, 0 failed, 0 ignored |
| Strict Solver Clippy on all targets | Passed with warnings denied |
| Formatting and whitespace | Passed |
| Documentation book and inventory | Passed |
| Source 800 and document 2000 organization audit | Passed; zero tracked debt |

The current executed profile artifact has `executed=true` and `ok=true`; its
retained-report gate also passes. It includes the existing live local TCP
Agent, TaskIR capability, headless numerical/recovery and unresolved-spectrum
fail-closed checks. Validation configurations now include the new kernel
target and public preprocessing cases. No ignored test, tolerance exception
or numerical dependency was added. This is not an installed desktop or remote
Linux verification, nor a rerun of the entire CLI test suite.

The coverage tensor includes the scoped numerical/recovery claim and passes
its structural and command checks: 13 modules, 11 paradigms, zero structural
gaps. Release readiness remains blocked with four maturity gaps, sixteen
evidence-grade gaps and eleven P0 gaps. The new verification does not promote
arbitrary modal qualification or satisfy unrelated operational obligations.

Final SHA-256 fingerprints, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_sparse.rs` | `5a883f7b260b824b39ed767e3f0a2d74c1a362e61d2e838ada3c96a7d0cea5e4` |
| `solver/src/modal_normalization.rs` | `196ecc401e9cea7ef1a98c2e7beef1a353e20f804658d94572d080a58f47950b` |
| `solver/src/modal_sparse_tridiagonal.rs` | `d03949d38e139456db7301e4dcfe32036c9c1e20afed0af68bb44dd7cd4210c4` |
| `solver/src/modal_sparse_preparation_tests.rs` | `3c2ce0c53f8f6666e117cf27d446636d737fbeedb911613b8e83278c1aff9537` |
| `solver/tests/modal_chain_reliability.rs` | `6515ca54bb7328cc6a093ba6861cf91c15e32e22da1a1c795fc09b8d025e908f` |
| `solver/tests/modal_chain_reliability/preparation.rs` | `1f8ebfc1d1bc5fc49bc4ec75213ef8608a8fe5647d39bdeeeeae281a789bf5ec` |

## Remaining Boundaries

This is bounded modal preprocessing validation, not general modal qualification.
The two-DOF coupling probes and 129-segment public fixtures are not million-DOF
performance, industrial material correlation or installed/remote Agent evidence.
No arbitrary precision is introduced; genuinely unrepresentable coefficients
remain explicit failures. Unsupported topology still needs the existing dense
or sparse inverse route, with its own numerical and recovery obligations.

The unresolved 128-element bending spectrum still fails within its original
budget. This round does not relax that gate or qualify arbitrary complete
spatial spectra. Historical reports retain their original source fingerprints
and versions; this report applies only to the current source overlay.
