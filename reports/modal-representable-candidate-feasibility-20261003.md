# Representable Modal Candidate Feasibility

## Findings And Scope

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. A bounded test-only search constructed an ordinary f64
first-mode direction for the 128-element bending fixture that satisfies
the original `1e-8` relative residual gate. Independent double-double
evaluation confirms the result. Its physical unit shape can also pass
the original gate after existing protocol JSON serialization and readback.

This is an existence result for sampled candidates, not a production
solver fix. The public 128-element request still returns nonconvergence
without partial modes. No runtime dispatch, production refinement budget,
residual tolerance, result format or dependency was changed. The new
search and numerical reference are compiled only under tests.

## Controlled Candidate Search

The [preceding diagnosis](modal-long-bending-roundoff-diagnosis-20261003.md)
showed that more inverse corrections alone did not resolve this fixture,
and naive rounding of a resolved wide direction could itself exceed the
gate. The new experiment partitions the known fixture into rotational
and translational coordinates. The helper accepts a supplied partition;
it does not identify beam types or add an operator-specific runtime route.

Approximate columns of `A - lambda*I`, or `D*(K - lambda*M)` for physical
shapes, define a small least-squares proposal. A normal Gram matrix for
the rotational columns is factored and reused. Translational updates
are evaluated in the projected residual, first individually and then
jointly for adjacent supplied coarse coordinates. Pair candidates include
the current value and the two adjacent representable f64 values, allowing
a coupled improvement that individual rounding can miss. Fine coordinates
are refitted privately before the real operator recomputes acceptance.

The helper bounds its matrix to 256 dofs, the outer search to four cycles,
and each fine refit to four updates. These are test-side bounds, not new
runtime limits or an extension of the production four-inverse-step budget.
The normal matrix may be singular or poorly conditioned for other inputs;
its suitability for arbitrary operators is not established here. Subset
selection and adjacency use known fixture structure, not a generic topology
or permutation-invariant selection algorithm.

Proposal scores never certify a candidate. Acceptance uses the actual
compensated physical operator and an unchanged gate. Independent wider
evaluation uses separate integer beam assembly and the original mass for
physical shapes, not the rounded proposal matrix. The borrowed seed is
never overwritten. Failure or cancellation returns no candidate, and a
checked candidate polls cancellation before acceptance. Already accepted
directions return unchanged rather than being rounded again.

## Measured Results

The production four-step kernel retains a first-mode pair with residual
`2.4968650305e-8` and root `4.60497244109418561e-8`. The test-only joint
candidate preserves that root and reaches the following residuals.
The candidate uses only f64 storage; the wider reference is an evaluator.

| Lane | Actual compensated residual | Independent wide residual |
| --- | --- | --- |
| Retained seed scaled by 2^-80, then joint correction | 7.0688121950e-9 | 7.0688121982e-9 |
| Retained seed, then joint correction | 7.0688121950e-9 | 7.0688121982e-9 |
| Retained seed scaled by 2^80, then joint correction | 7.0688121950e-9 | 7.0688121982e-9 |
| Physical unit shape after correction and JSON readback | 8.3142469462e-9 | 8.3142469455e-9 |

The same accepted internal direction, when expanded and unit-normalized,
initially has physical residual `4.6971016191e-8`. Consequently, internal
success alone is still insufficient: the physical representation needs
its own correction and validation. The production physical-shape checker
accepts the corrected physical candidate without any tolerance change.

Reference-rounded first modes at 64, 80, 100 and 128 elements were also
checked. Already accepted 64/80-element directions stay bitwise unchanged.

| Elements | Actual candidate residual | Independent wide residual |
| --- | --- | --- |
| 64 | 3.0690260608e-9 | 3.0690260593e-9 |
| 80 | 6.6937509961e-9 | 6.6937510003e-9 |
| 100 | 7.1833467697e-9 | 7.1833467679e-9 |
| 128 | 9.2030550993e-9 | 9.2030551024e-9 |

These vectors are first modes of the known fixtures, not complete spectra.
Root error against the independent reference remains below `1e-14`; the
four-step seed candidate direction stays within `1e-10` of the wide
reference after scalar projection. Those checks supplement rather than
replace the actual residual gates.

## Physical Output And Recovery Checks

The physical test expands the restrained planar shape, checks zero root
and axial dofs and a unit participation norm, and constructs the existing
`ModalFrame2dModeResult`. Every shape component and the root keep their bits
after serde JSON readback; the complete mode record compares equal.
Frequency/period consistency and both actual and independent physical
residuals pass after readback. This exercises a constructed mode record,
not a successful public solve, full result envelope, Engine task or Agent RPC.

Six new tests cover first-mode feasibility with binary vector scales,
reference-rounded samples, physical shape/readback, failed-budget replay,
invalid subsets/dimensions/certificates and cancellation. A tighter
test-side `1e-30` request fails explicitly while preserving the borrowed
seed, then replays at the original gate. Invalid or singular fitting
problems are rejected. Cancellation is checked inside substitution and
after a candidate has passed numerical checking but before acceptance.

## Integration Decision

This fixture does not force a new wide-precision result format: a valid
ordinary f64 mode record exists. It also does not justify silently adding
the experiment to production. A runtime candidate method must select
partitions and coupled coordinates without assuming ordered beam dofs,
preserve multimode orthogonality and repeated eigenspaces, bound factor
work and memory, and validate both normalized and published representations.
Reordered nodes, heterogeneous frames, coordinate scales, spatial modes,
late-mode cancellation and full Engine/SDK/Agent execution remain separate
requirements. The test-only matrix bound cannot become a hidden runtime
capacity change.

The tensor claim is limited to candidate numerical feasibility. It does
not remove general modal qualification gaps or claim a performance gain.
Existing negative and recovery paths remain the production behavior.

## Reproduction

From `workers/rust`:

```sh
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement::resolution_tests::conditioning::representable --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib modal_frame_spectrum::refinement --locked --offline
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
```

The registered `modal-frame-sanity` refinement command includes these tests.
Profile registration is not execution of its live Agent lane.
This is bounded representable first-mode feasibility, not a production modal solver or general spectrum qualification.

## Validation Summary

| Check | Result |
| --- | --- |
| New representable-candidate tests | 6 passed, 0 failed |
| Refinement target, including the new tests | 39 passed, 0 failed |
| Full Solver suite | 1352 passed, 0 failed, 9 existing ignored tests across 186 result groups |
| Headless modal operator suite | 35 passed, 0 failed |
| Solver and CLI all-target Clippy with warnings denied | Passed |
| Workspace formatting and patch whitespace | Passed |
| Operator profile self-test and registration | Passed; 59 profiles, `executed=false` |
| Tensor self-test and structural validation | Passed; 13 modules, 11 paradigms, 0 structural gaps |
| Documentation book and inventory | Passed; 26 HTML files, development/shipping 3.4.4 |
| Project organization audit | Passed; source limit 800, documentation limit 2000, tracked debt 0 |

The targets overlap; these totals are not a test coverage percentage.
The nine ignored tests predate this experiment, and no new ignored test
was added. The unchanged production rejection and fresh-request recovery
paths remain covered by the headless modal suite. No live TCP Agent or
remote run was performed for this feasibility experiment.

Tensor readiness remains blocked with 4 maturity gaps, 16 evidence-grade
gaps and 11 P0 gaps. Registering the narrower numerical claim does not
qualify production admission, distributed execution or industrial use.
No version bump, commit, push, packaging or installation was performed.

## Source Fingerprints

SHA-256 paths below are relative to the repository root and pin the
current source overlay rather than claiming a committed revision.
The production refinement fingerprint is unchanged from the preceding
roundoff diagnosis.

| Source | SHA-256 |
| --- | --- |
| `workers/rust/crates/solver/src/modal_frame_conditioning_tests.rs` | `b57f2ed7098c7a7960b3fe00a9589623c10b9fa5c18715747128db2a6395713e` |
| `workers/rust/crates/solver/src/modal_frame_conditioning_reference.rs` | `f9c286da6364fdf336807aff7437777221c381ed1c3aea40f97cc7cc055419ad` |
| `workers/rust/crates/solver/src/modal_frame_representable_tests.rs` | `05e173755daeefc43c039a2dff7567f1e3a1f125f1c46a56faf19cc5fe3dd107` |
| `workers/rust/crates/solver/src/modal_frame_lattice_reference.rs` | `982d42aed72609cd82bd11232bdd8bec4003775089aceff79415cf55785121e2` |
| `workers/rust/crates/solver/src/modal_frame_refinement.rs` | `7cb3bbb92287010bd0eab5b1c351fb2ca7ac906621465d39cbbbacb72d774724` |
