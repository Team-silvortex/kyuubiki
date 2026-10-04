# Automatic Partition Modal Candidate Feasibility

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. A bounded test-only selector chooses modal correction
coordinates from the approximate residual matrix and current f64 vector,
without supplied rotational/translational labels. Its candidates pass the
unchanged `1e-8` internal and physical-shape gates for four freshly
reassembled orders of the 128-element first bending mode. Independent
double-double checks agree, including after existing JSON mode-record readback.

This extends the [coupled-coordinate experiment](modal-coupled-coordinate-feasibility-20261003.md)
by removing the supplied-partition requirement for these samples. It is not
production admission or an optimal partition guarantee. The public
128-element request still fails explicitly without partial modes.
Runtime dispatch, four-inverse-step refinement budget, result formats,
dependencies and accuracy gates remain unchanged. New selection and
candidate code are compiled only under tests.

## Selection Rule And Limits

For a square matrix of 2 to 256 dofs, the selector normalizes entries by
their common maximum magnitude. Any nonzero entry lost to underflow is
rejected. It selects exactly `floor(n/2)` columns, leaving a proper coarse
complement. This half-size choice is a test-side heuristic, not a physical
coordinate classification or a new runtime capacity policy.

At each pivot, remaining columns have undergone two reorthogonalization
passes against each selected direction. A column is screened out if its
remaining norm is zero or at most `64*EPSILON` times its original normalized
column norm. The proposal score is:

```text
log2(remaining_column_norm) - 0.5 * log2(component_grid_spacing)
```

Spacing uses the larger finite distance to the adjacent representable
magnitudes, falling back to the lower distance at `f64::MAX`. Zero,
subnormal and maximum components therefore have finite grid logarithms.
The weight balances column independence and finer representable movement;
it is a heuristic score, not a residual or conditioning certificate.
Equal scores retain index tie ordering. Arbitrary coordinate transforms,
equal-score permutations or optimal splits are not established here.

If too few independent columns survive screening, selection returns an
error rather than a partial subset. A completed selection is sorted and
checked for cancellation before return. Input validation rejects nonfinite
data, a zero vector, mismatched dimensions and out-of-budget matrices.
The chosen subset feeds the existing test-only coupled Gram fit; singular
or unusable factors still fail. Projected scores cannot authorize success:
the actual compensated operator callback validates every accepted candidate.

The fit still has four bounded outer cycles and four fine refits per pass.
One Gram factor is retained for graph preparation and correction. The
selector's dense column updates require cubic worst-case work; the fit and
correlation graph also remain dense. No speedup or production cost budget
is claimed. Selecting half the columns does not prove full-matrix rank,
modal completeness or independence of multiple accepted modes.

## Reassembled Candidate Results

Physical stiffness and mass are freshly assembled under original, reversed,
grouped and mixed bijective reduced-coordinate orders. Candidate acceptance
runs on each reordered compensated operator. The independent oracle uses
integer stiffness assembly in original coordinate order. Coordinate labels
are used to build and compare fixtures, not supplied to the selector.

The retained four-inverse-step first-mode seed fails at roughly `2.50e-8`.
Its eigenvalue is unchanged and remains within `1e-14` of the independent
first root. Automatically partitioned candidate directions stay within
`1e-10` of the wide direction after scalar projection.

| Reduced Coordinate Order | Internal Residual | Physical Residual After JSON Readback |
| --- | --- | --- |
| Original | 7.5374773863e-9 | 9.2043366489e-9 |
| Reversed | 7.5740952854e-9 | 8.2086867810e-9 |
| Grouped | 7.5374773863e-9 | 9.2043846331e-9 |
| Mixed | 8.1983171212e-9 | 9.3048094495e-9 |

Actual and independent residuals agree within `1e-6` relative for every
sample. All four internal orders also pass with vector powers `2^-80`, `1`
and `2^80`, and negated unit scale. The selector is rerun for each scaled
vector rather than supplied a previously chosen physical partition.
The borrowed vector stays unchanged.

Reference-rounded first modes at 64, 80, 100 and 128 elements pass under
all four orders. Already passing directions remain unchanged, and corrected
directions stay within `1e-12` of the reference after scalar projection.

| Elements | Actual Candidate Residual Range Across The Four Orders |
| --- | --- |
| 64 | 3.0690260608e-9 |
| 80 | 6.6937509961e-9 |
| 100 | 7.2971065771e-9 |
| 128 | 7.6428911585e-9 to 8.9152615761e-9 |

The first three rows are rounded display values, not bitwise equality claims
across orders. Results differ from the preceding supplied-partition method;
the new rule is not always lower-residual and stops on the original gate.
Some physical outputs approach that gate, so each representation retains
its independent check instead of inheriting internal convergence.

The physical test automatically selects again using `D*(K-lambda*M)` and
the physical unit shape. The production shape checker accepts its private
candidate at the original tolerance. Zero restrained/axial dofs, unit
participation norm and frequency/period consistency pass after constructing
and reading back `ModalFrame2dModeResult`; the entire record and all shape
bits remain equal. This is not a successful public solve, full result
envelope, SDK execution, Engine task or live Agent RPC.

## Failure And Work Checks

Nine new tests cover candidate and reference samples, physical readback,
failed-budget replay and selector boundaries. A known diagonal case picks
the independently supported columns, while a dependent case explicitly
fails insufficient-rank screening. Instrumentation counts ten dot products
for the four-column/two-pivot example, matching the two reorthogonalization
passes at each pivot; no adaptive retry loop is added.

Primitive selection also preserves the known diagonal subset under sampled
row/column permutations, matrix powers `2^-500` and `2^500`, vector powers
`2^-80` and `2^80` and signs. This is a selector check, not a physical
coordinate-unit or scaled complete-fit qualification. NaN, infinity,
invalid shapes, zero vectors/matrices and nonzero scaling loss are rejected.
Cancelled selection, including cancellation at completed subset return,
produces no subset, and fresh control scopes replay successfully.

A mixed-order rounded reference fails a test-side `1e-30` request without
changing its borrowed seed, then passes at `1e-8` using the same automatic
fit. That sampled failure used 26 actual residual callbacks, below the
test-side cap of 80. One Gram factor is observed across preparation,
failed correction and successful replay. These bounded tests are not
evidence of universal convergence or a precision-floor proof.

## Integration Requirements

The selector removes physical labels from this candidate pipeline, but
does not establish that the heuristic works for arbitrary operators or
coordinate systems. Heterogeneous frames, spatial modes, rotated bases,
repeated eigenspaces, multimode orthogonality, full result serialization
and installed/distributed execution remain open. Fits are candidate-local;
reusing a partition across unrelated mode seeds is not qualified.

Production integration must jointly validate residuals and the accepted
modal subspace with explicit preparation, iteration, memory and cancellation
budgets. The tensor evidence is limited to test-only numerical candidate
feasibility. It does not promote runtime execution, benchmark, recovery or
industrial readiness, and the original public failure remains in place.

This is bounded automatic-partition candidate feasibility, not a production modal solver or general subspace qualification.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib automatic_ --locked --offline -- --nocapture
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
| New automatic-partition tests | 9 passed, 0 failed |
| Complete refinement target | 56 passed, 0 failed |
| Full Solver suite | 1369 passed, 0 failed, 9 existing ignored tests across 186 result groups |
| Headless modal operator suite | 35 passed, 0 failed |
| Solver and CLI all-target Clippy with warnings denied | Passed |
| Workspace formatting and patch whitespace | Passed |
| Operator profile self-test and registration | Passed; 59 profiles, `executed=false` |
| Tensor self-test and structural validation | Passed; 13 modules, 11 paradigms, 0 structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development/shipping 3.4.4 |
| Project organization audit | Passed; source limit 800, documentation limit 2000, tracked debt 0 |

Targets overlap, so the counts are not a coverage percentage. No new ignored
test was added. Headless regression preserves the production negative and
fresh-request replay behavior, without executing this test-only selector.
No live TCP Agent, remote or installed-app run was performed this round.
Tensor readiness remains blocked with 4 maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps. No version bump, commit, push or packaging was performed.

## Source Fingerprints

SHA-256 paths are relative to the repository root and pin the current source
overlay, not a committed revision. Production refinement is unchanged from
the preceding coupled-coordinate experiment.

| Source | SHA-256 |
| --- | --- |
| `workers/rust/crates/solver/src/modal_frame_lattice_reference.rs` | `2064316a3fa577bcab5f89ed908c79286715bdf4b15af664183cdd0fdf1abbe3` |
| `workers/rust/crates/solver/src/modal_frame_coupled_tests.rs` | `e824fb5ee7aac40fc953848370abae14f596ef704032d934a003c4ad498b5497` |
| `workers/rust/crates/solver/src/modal_frame_partition_reference.rs` | `dd24685bf8437b42d92ebec921707903bcf99860955f1f5fbff8da1c1bca47d7` |
| `workers/rust/crates/solver/src/modal_frame_partition_tests.rs` | `300c60b6cb69c07bad2c8abdbe7e6aaff4f30ce5fc5b993ea9adb816380ccba4` |
| `workers/rust/crates/solver/src/modal_frame_automatic_tests.rs` | `13f12295f5c0615c6abd4f2a0edbd0969ce76e902da928684f205df2b48788f0` |
| `workers/rust/crates/solver/src/modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
