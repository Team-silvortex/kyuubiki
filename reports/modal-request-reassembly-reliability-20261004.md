# Modal Public Request Reassembly And Output Restoration

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88.

## Scope And Decision

This is bounded public request-reassembly verification, not general modal recovery or release qualification.

The preceding [wide-beam comparison](modal-grid-wide-beam-reliability-20261004.md)
jointly permuted fixed matrices and retained seeds. This follow-up instead
renumbers original request nodes, remaps element endpoints, changes element
order, and calls the actual public Solver entrypoints. Stiffness, mass, seed,
eigenvalue and physical publication are computed afresh. No experimental
candidate helper supplies the production seed or answer.

The pre-fix diagnostic exposed acceptance depending on node numbering:

- Graded, 96 segments, scale `1e-10`: original numbering passed, but shuffle-113
  failed physical publication with relative residual `1.3416343911284187e-8`.
- Layered, 96 segments, scale `1e-10`: original numbering failed physical
  publication at `1.3163046885973113e-8`, while reversed numbering passed.
- Unequal lengths, 128 segments, scale `1`: original numbering failed physical
  publication at `1.1666734210702403e-8`, while reversed and shuffle-113 passed.

These are synthetic regression fixtures, not independently qualified material
research datasets. Passing a different pre-fix layout was not itself evidence
of an invalid physical result; the defect was the inconsistent sampled boundary.

## Production Correction

The 2D Solver now uses a stable geometric internal node order for a deliberately
narrow horizontal paired-bending path. Eligibility requires explicit
`mode_count=1`, at most 257 nodes, between 2 and 256 active coordinates, every
node's axial displacement restrained, matching vertical/rotation restraint
flags, and a common y coordinate. Distinct node coordinates provide the key.
Signed zeros are the same geometric point. Coincident coordinates or identity
order preserve the previous path rather than guessing a tie-break from IDs.

Only internal assembly indices, nodal mass indices and restraint indices are
mapped. The original element geometry and coefficients are unchanged. The
solver executes once: no alternate-order retries, raised work/check budgets,
new dependency, tolerance relaxation, wide-grid production fallback, Engine
coupling or protocol change is introduced. Axial, mixed-restraint, inclined,
3D, multimode and larger paths retain their existing routing.

Before returning success, full shapes and free coordinates are restored to
the caller's original numbering. Free coordinates are sorted in original
index order, and participation norm is recomputed and checked against the
unchanged `1e-10` unit-norm gate. The input payload remains original for both
borrowed and ownership-transferring entrypoints. The extra state is two
bounded node-index maps and one temporary shape, not another cloned model,
snapshot history or persistent cache. This is not a measured whole-pipeline
allocation or RSS bound.

Output restoration has cancellation checkpoints, including its final node.
Cancellation returns an error without publishing a partial result; fresh
replay does not reuse cancelled output or poison the original request.

## Independent Public Cross-Check

Three profiles (graded, layered, unequal lengths), three sizes (64, 96, 128
segments) and three common scales (`1`, `1e14`, `1e-10`) form 27 physical
fixtures. Each has identity, reversed and two shuffled node/element layouts.
Both public entrypoints execute for each layout; the identity borrowed call
is the baseline, not an extra run.

```text
public reassembly fixtures=27 layouts=108 entrypoint_runs=216 accepted=144 normalized_rejections=48 physical_rejections=24
```

| Canonical fixture outcome | Physical fixtures | Layouts | Entrypoint runs |
| --- | ---: | ---: | ---: |
| Accepted | 18 | 72 | 144 |
| Normalized-coordinate rejection | 6 | 24 | 48 |
| Physical-publication rejection | 3 | 12 | 24 |
| Total | 27 | 108 | 216 |

Accepted outputs are serialized and read back, then stiffness/mass are
reassembled from the published original request using the independent test
reference, not production assembly helpers. Physical relative residual must
remain at most `1e-8`; physical unit norm must stay within `1e-10`. Restored
roots and shapes have baseline bits after node/sign mapping, total mass stays
within `1e-14` relative difference, free coordinates match the request, and
original input JSON remains unchanged. Both entrypoints return equal results.

Rejected layouts must match the baseline error exactly, including normalized
versus physical stage and the unchanged residual gate. All six graded/layered
128-segment cases still reject during normalized recovery. Layered 96-tiny and
unequal-length 128-unit/tiny still reject physical publication. Their former
layout-dependent successes are not used as fallback choices. This correction
fixes the graded 96-tiny layout failure; it does not recover every hard fixture.

A separate actual request cancels at the final output-restoration node and
verifies the observer was reached. Normalized failure, physical failure and
malformed connectivity are followed by healthy replay with exact baseline
results and unchanged input. Four focused mapping tests check bijection,
signed-zero/coincident geometry, eligibility boundaries and early cancellation.

The sampled element-order invariance is limited to these beam fixtures. It is
not a proof for arbitrary branched topology, summation order, geometry or
material parameters, nor a formal proof of generalized eigensolver convergence.

## Rust Headless Bridge

Two additional tests build an official Rust Headless execution plan and run
its existing local Engine bridge. They verify original payload/shape mapping,
independent physical JSON residuals for graded 96-unit/tiny, final restoration
cancellation, explicit graded 128-large failure and healthy replay. The full
39-test modal bridge suite passes, including earlier 2D/3D controls.

These are local in-process source tests, not a live Agent RPC/TaskIR journey,
remote test, installed/source-detached SDK study, GUI or cross-platform run.
The separate Agent and Headless study qualification requirements remain open.

## Verification And Replay

| Check | Final source result |
| --- | --- |
| Optimized full Solver unit library | 664 passed, 0 failed; 16 explicit benchmark/reference tests ignored |
| 2D/3D review, input, assembly and sanity integration suites | 21 passed, 0 failed across five suites |
| Debug Rust Headless modal Engine bridge | 39 passed, 0 failed |
| Debug node-order boundary and mapping controls | 4 passed, 0 failed; overlapping optimized library cases |
| Tensor source regression | 21 passed, 0 failed; verified reassembly cannot close its qualified target |
| Solver/CLI and tensor runner all-target Clippy | Passed with `-D warnings` |
| Rust formatting and whitespace | Passed |
| Operator validation registry and self-test | Passed; 59 profiles; registry-only `executed=false` |
| Coverage tensor and self-test | Passed structurally; 4 dimension-presence, 19 evidence-grade and 14 P0 gaps remain; readiness blocked |
| Documentation inventory and book | Passed; 26 HTML files; development/shipping 3.4.5 |
| Project organization audit and self-test | Passed; source <=800, docs <=2000; tracked debt 0 |

The first tensor source invocation ran before the new report was written and
correctly failed on its missing evidence file (20 passed, one failed). After
retaining the report, final registry and source checks above pass. No missing
file check, numerical threshold or qualification rule was relaxed. Repeated
optimized/debug subsets are not extra physical dataset coverage. The 16
explicit ignored tests were not freshly executed in this follow-up.

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release modal_public_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The validation registry records the focused public and Headless commands.
Registry validation alone does not execute them. The tensor adds one scoped
`verified` claim and binds it to independent request reassembly, whose target
remains `qualified`. It preserves the 224 calibration claims, now totaling
225 (223 proven, two partial). Target results remain 58/77 coordinates and
10/32 named scenarios; readiness remains `blocked`.

The next calculation boundaries are heterogeneous production recovery,
multimode/general-frame request reassembly, cumulative pipeline work/memory,
and actual Agent/official Rust Headless study journeys. No new benchmark,
package, installed application, release version or release claim is made.
