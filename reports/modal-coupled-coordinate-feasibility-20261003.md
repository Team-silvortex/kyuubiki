# Coupled Coordinate Modal Candidate Feasibility

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. A test-only projected-column coupling graph replaces the
previous assumption that adjacent array entries identify useful joint
rounding partners. Sampled freshly reassembled coordinate permutations of
the 128-element bending fixture satisfy the unchanged `1e-8` internal gate.
Their physical unit shapes also satisfy the original gate after existing
mode-record JSON serialization and readback, with independent wider checks.

This extends the [representable-candidate experiment](modal-representable-candidate-feasibility-20261003.md),
not production admission. The fine/coarse partition is still supplied from
the fixture's known rotational/translational identities. The search does not
select that partition automatically. Public 128-element requests remain
nonconvergent without partial modes. Production dispatch, refinement
budget, accuracy gates, result formats and dependencies are unchanged.

## Coupling And Work Bounds

The retained least-squares fit projects each coarse column against the
supplied fine space. Column norms are balanced before dot products, so
pair proposals use absolute correlation rather than raw column magnitude.
Each nonzero column nominates at most four partners with nonzero absolute correlation.
The sorted, deduplicated union therefore has at most `4*c` pairs for `c`
coarse columns. Zero columns do not invent partners. Ties use current index
order; this is deterministic for a given input, not an invariant graph
labeling algorithm for arbitrary permutations or equal-correlation inputs.

The original test helper's adjacent-pair preparation remains available for
the preceding experiment. The new coupled preparation chooses partners
from projected columns without beam labels or adjacency assumptions.
Both preparations are compiled only under tests and share the existing
candidate validation contract. Neither approximate projected scores nor
correlations can authorize publication: each retained candidate must pass
the actual compensated operator callback before acceptance.

The test-only matrix limit remains 256 dofs. There are still at most four
outer cycles and four fine refits per pass. One Gram factor is reused for
graph construction and correction; no factor is created for each pair.
Graph construction requires quadratic coarse-column comparisons and the
fit is still dense. These bounds are not a production performance claim or
permission to add this work silently to the runtime four-inverse-step budget.

## Reassembled Model Checks

The coordinate tests independently assemble physical stiffness and mass
under four bijective reduced-coordinate orders: original, full reversal,
grouped translation/rotation, and mixed affine permutation. Assembly,
mass normalization, compressed sparse products and candidate acceptance
run on the reordered system, not an output array passed through the
original operator. The wide oracle evaluates the candidate in original
physical coordinate order using independent integer stiffness assembly.

The failing production four-step first-mode seed has relative residual
about `2.4968650305e-8`. Its retained eigenvalue stays within `1e-14` of the
independent first root. Coupled candidates at each tested order are within
`1e-10` of that reference direction after scalar projection.

| Reduced Coordinate Order | Internal Residual | Physical Residual After JSON Readback |
| --- | --- | --- |
| Original | 7.0688121174e-9 | 9.8081104233e-9 |
| Full Reversal | 7.4998473395e-9 | 9.4425514071e-9 |
| Grouped | 7.0688121174e-9 | 9.8081104233e-9 |
| Mixed | 8.9821570537e-9 | 9.2544707999e-9 |

Independent double-double residuals agree with each actual value within
`1e-6` relative. Internal candidates at powers `2^-80`, `1`, `2^80` and
negated unit scale pass at every sampled order. Those are vector scales,
not heterogeneous material or coordinate-unit scale qualification.
Outputs need not be bitwise identical between coordinate orders: the
measured residuals differ, and some physical candidates approach the gate.
Every order therefore retains its own independent validation boundary.

Reference-rounded first modes at 64, 80, 100 and 128 elements were checked
under all four orders. Already passing directions remain unchanged.
All sixteen cases pass actual and independent residual gates; corrected
directions stay within `1e-12` of the wide reference after scalar projection.
These are first modes of uniform fixtures, not complete spectra.

The physical tests keep original physical node identities while rebuilding
the reduced algebra. They expand the candidate, validate zero restrained
and axial dofs, unit participation norm and frequency/period consistency,
then construct the existing `ModalFrame2dModeResult`. Complete records and
all shape bits survive JSON readback. This is not a successful public solve,
full result envelope, Engine task, SDK run or Agent RPC.

## Failure And Cancellation Checks

Eight new tests cover the reassembled candidates, reference samples,
physical readback, private failure/replay and the coupling graph. A synthetic
graph contains useful nonadjacent pairs and verifies the nomination bound,
deduplication and exclusion of unrelated zero-correlation columns. Binary
scales through `2^-900` and `2^900`, signs and sampled row/column permutations
retain that synthetic graph. Invalid dimensions, nonfinite columns and
unrepresentable norms are rejected instead of generating proposals.

Cancellation during coupled graph preparation returns no fit; a fresh
control scope can prepare and use the same borrowed matrix data. A failed
test-side `1e-30` request leaves the borrowed mode seed unchanged, then
replays successfully at the original gate using the same prepared fit.
That sampled failure used 26 actual residual callbacks; the test caps the
four-cycle callback count at 80. Instrumentation confirms one Gram factor
across preparation, failed correction and successful replay. This is not
evidence of universal convergence at a fixed work budget.

## Integration Requirements

Coupling selection no longer assumes ordered neighboring dofs in these
samples. Automatic partition selection, general conditioning checks,
heterogeneous frames, spatial modes, repeated eigenspaces, multimode
orthogonality and installed distributed execution remain open. Arbitrary
permutation qualification is not inferred from four samples, especially
with index-ordered correlation ties and order-dependent fitting arithmetic.

The tensor claim is restricted to numerical validation of test-only
candidates. Runtime solver execution, recovery, benchmark and industrial
readiness are not promoted by this experiment. Before production admission,
a general method must retain both internal and published-shape gates with
explicit preparation, iteration, memory and cancellation budgets.

This is bounded coupled-coordinate candidate feasibility, not a production modal solver or general permutation qualification.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement::resolution_tests::conditioning::representable --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement --locked --offline
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The existing registered refinement target includes the new tests. Profile
registration is not execution of its live Agent lane.

## Validation Summary

| Check | Result |
| --- | --- |
| Candidate target, including eight new tests | 14 passed, 0 failed |
| Complete refinement target | 47 passed, 0 failed |
| Full Solver suite | 1360 passed, 0 failed, 9 existing ignored tests across 186 result groups |
| Headless modal operator suite | 35 passed, 0 failed |
| Solver and CLI all-target Clippy with warnings denied | Passed |
| Workspace formatting and patch whitespace | Passed |
| Operator profile self-test and registration | Passed; 59 profiles, `executed=false` |
| Tensor self-test and structural validation | Passed; 13 modules, 11 paradigms, 0 structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development/shipping 3.4.4 |
| Project organization audit | Passed; source limit 800, documentation limit 2000, tracked debt 0 |

Targets overlap and these counts are not a coverage percentage. No new
ignored test was introduced. Headless regression confirms unchanged public
negative and replay behavior; it does not execute the new test-only fit.
No live TCP Agent, remote or installed-app run was performed this round.
Tensor readiness remains blocked with 4 maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps. No version bump, commit, push or package was produced.

## Source Fingerprints

SHA-256 paths are relative to the repository root and identify the current
source overlay, not a committed revision. Production refinement remains
unchanged from the preceding feasibility experiment.

| Source | SHA-256 |
| --- | --- |
| `workers/rust/crates/solver/src/modal_frame_lattice_reference.rs` | `be4c80d31b83200e31a50f4742a7d8493d3289dbc5a496d1bdcd8276537210b2` |
| `workers/rust/crates/solver/src/modal_frame_representable_tests.rs` | `7f294abe17c63befd3dc6f459d33dc381825d7056331f08f7c7fa0413fcbc16c` |
| `workers/rust/crates/solver/src/modal_frame_coupled_tests.rs` | `cd7fef748b6d06f549939092476cc788fdf5babd3b654288b687d6a603036fa2` |
| `workers/rust/crates/solver/src/modal_frame_coupling_graph_tests.rs` | `0a1fc9e3bcafdcddee21f0f074db53491f8165146631a80f5dc86ff36393e988` |
| `workers/rust/crates/solver/src/modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
