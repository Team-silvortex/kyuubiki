# Modal legacy first physical recovery and retained results

October 5, 2026. Local macOS ARM64 Solver research follows the
[coupled two-stage campaign](modal-coupled-shape-selection-reliability-20261005.md).
HEAD remains `dc5647af` (`daji 3.4.6`), with package metadata 3.4.5. No version,
installed application, production Solver admission, Engine ownership or Agent
execution policy changes.

This is test-only legacy-first hybrid research, not production modal admission.
The retained 42-input comparison now preserves all 37 old successful JSON
outputs exactly and recovers one previously rejected input, for 38 acceptances
and zero old-success losses. Four internal ThreeLayers failures remain open.
This is a bounded horizontal single-mode result, not general qualification.

## Isolating the previous losses

Twelve independent cold routes cross three inputs, two internal selectors and
two physical recovery tails. All routes freshly reconstruct current requests,
roots, ordinary seeds and four-step Wide initializers. The selectors are old
ranked greedy versus coupled GridNorm; tails are old greedy versus width-sixteen
physical beam. No already successful reference direction seeds a cold route.

| Input | Old internal and old physical | Old internal and beam physical | Coupled internal and old physical | Coupled internal and beam physical |
| --- | --- | --- | --- | --- |
| UnequalLengths 128 at `1e-10` | Physical reject | Accept | Physical reject | Accept |
| UnequalLengths 128 at `1e14` | Accept | Accept | Physical reject | Physical reject |
| Old Layered 128 at `1e-10` | Accept | Accept | Internal reject | Internal reject |

For the large unequal-length input, the old internal direction restores
success; the selected coupled direction fails under either physical tail.
For the layered tiny input, the old internal portfolio succeeds under Reverse
after GridNorm rejects, while the coupled experiment tested only GridNorm.
That loss cannot be attributed solely to the mapped ranking hint.

The old internal direction also recovers the new tiny unequal-length input
under physical beam, with independent readback `8.212649066327543e-9`. The
coupled path obtains a lower residual, `6.757600695706912e-9`, but changing its
internal selection is not necessary to obtain an additional physical success.
This ablation motivates keeping the old internal policy unchanged.

## One physical factor and a bounded continuation

The new cold policy retains the old ranked-then-Reverse internal recovery,
including its maximum two fits and thirteen actual receipts. It then prepares
the existing physical seed, canonical grid, selected order and frozen
mass-weighted anchor. The original physical chart is retained.

One preselected rounded QR factor first runs the unchanged four-pass greedy
physical recovery. If it succeeds, its shape is retained. The extra final
actual certificate and unit-norm check preserve the old accepted output;
no beam proposals are constructed for that result.

Only a typed residual rejection with no earlier passing greedy receipt can
continue to a width-sixteen beam from the original physical seed. It reuses
the same factor and frozen grid, not a newly prepared fit or a changed anchor.
Faults, malformed receipts, cancellation, lost greedy final eligibility and
unit-norm rejection stop instead of continuing. A failed final beam receipt
also terminates without any later retry.

The selected shape receives another fresh actual physical certificate before
acceptance. Unit norm is checked without post-certificate renormalization,
and anchor bits remain identical. Actual residual `1e-8` and unit norm `1e-10`
remain unchanged. The shared inertia, original numbering, positive-zero fixed
DOFs and independent JSON physical reassembly tail remain mandatory.

This is one test-only search policy, not an Engine/Agent retry mechanism.
There is no recipe, material-name or request-size dispatch in its selection.
The twelve ablation outcomes and all 42 full-comparison stage triples are
asserted independently of the policy implementation.

## Proposal cost boundaries

| Bound | Value |
| --- | ---: |
| Active coordinates | 2 through 256 |
| Beam width | 1 through 64 |
| Integer radius | 1 through 4,194,304 |
| Physical QR starts per stage | 1 |
| Physical greedy passes | at most 4 |
| Aggregate physical receipt cap | width plus 9 |
| Width sixteen physical receipt cap | 25 |
| Width sixty-four physical receipt cap | 73 |
| Width sixty-four payload reservation at 256 coordinates | 4,739,072 bytes |
| Width sixty-four component visit reservation at 256 coordinates | 608,403,456 |

The reservation extends the existing rounded-beam plan with conservative
greedy/continuation/final-receipt overhead. A shared receipt counter enforces
the aggregate cap across both searches and the extra final validation.
No separate physical factor is started at the continuation boundary.
A width-sixty-four synthetic route reaches all seventy-three allowed receipts,
including six greedy, sixty-six beam and one extra final receipt, rather than
only asserting the planned cap.

Internal and physical stages keep separate budgets. Their combined proposal
caps are three grid fits and thirty-eight actual operator receipts, not a
whole-pipeline cost claim. This excludes Wide initialization, borrowed
matrices/operators, assembly, seed polishing, inertia, JSON reassembly,
allocator overhead and process RSS. No benchmark, throughput or memory
improvement is claimed from these test durations or reservations.

The gained input actually uses one internal and one physical fit, with four
internal and twenty-three physical receipts: four failed greedy receipts,
eighteen beam receipts and one extra final physical receipt. Old successes
retain their exact old factor and receipt counts; the supplementary beam is
not invoked for them.

## Fresh material and mesh comparison

The same four recipes, three member counts (80, 100, 128) and three coordinate
scales (`1`, `1e14`, `1e-10`) supply 36 inputs. Six Graded/Layered 128-member
baselines make 42. Each runs three independent cold routes: old full pipeline,
old internal plus unconditional physical beam, and old internal plus the new
legacy-first physical hybrid. These are not reference-seeded recovery tails.

| Policy | Accepted | Internal rejection | Physical rejection |
| --- | ---: | ---: | ---: |
| Old full pipeline | 37 | 4 | 1 |
| Old internal with beam physical | 37 | 4 | 1 |
| Old internal with legacy-first hybrid physical | 38 | 4 | 0 |

Unconditional beam gains the tiny unequal-length input but loses the
unit-scale unequal-length 128-member input. Equal aggregate counts conceal
this exchange, so the exact per-input boundary is retained. Legacy-first
hybrid preserves the latter's old successful result and gains the former.

All 37 old successful serialized outputs, roots, ordinary seeds, independent
readbacks, factor counts and actual receipt counts match exactly under hybrid.
Both previous coupled losses are restored. The new tiny unequal-length gain
has a smaller margin than the coupled candidate, but does not trade any of
these old successes for that gain.

The four ThreeLayers inputs remain internally rejected and never start a
physical fit: 100 members at `1e-10`, and 128 at `1`, `1e14`, `1e-10`.
Exact retention of this sample is not a theorem for arbitrary ties, geometry,
material ranges, clustered eigenvalues or multimode requests.

## Fault cancellation and fresh recovery controls

Synthetic tests preserve initially passing legacy seeds, verify the combined
receipt cap and reject invalid dimensions, width, radius, threshold, nonunit
seed and incompatible Wide factor. Six known-null cases use actual Laplacian
row products, both signs and three binary matrix scales. A greedy candidate
with lost unit norm stops after three receipts without invoking beam.

Ten callback positions receive explicit faults and fifty malformed receipts.
Four final-gate losses cover greedy final certification, the extra final
receipt, beam final certification and its extra final receipt. None admits a
partial shape. Five cancellations cover factor preparation, the greedy-to-beam
handoff, last beam checks and final validation, with five fresh healthy replays.

Real gained-input tests inject four operator faults at the rejected greedy
final receipt, first beam receipt, final beam receipt and extra final receipt.
Malformed and lost-gate extra final receipts also stop. One cancellation occurs
inside the actual final physical operator product. Seven freshly rebuilt
healthy physical replays match shape bits and counts exactly; signed physical
seed recovery retains the corresponding shape relation. One physical factor
is observed throughout every injected search failure.

Two independently numbered full outputs pass JSON physical reassembly with
residuals `8.212649066327543e-9` and `8.212649066327536e-9`. Shape bits match by
geometry, roots and ordinary seeds match, and each route has receipts `[4,23]`.
Independent summation order permits the small residual difference.

Five full-request cancellations target the actual physical handoff, extra
final product, final physical validation, final node restoration and final
publication. The observer is armed only after internal recovery completes;
asserted stage and receipt counts exclude accidentally cancelling the similarly
named Wide-initializer checkpoint. No result is published. Four separately
renumbered hard requests still fail internally without a physical fit. Nine
fresh healthy JSON replays exactly match the healthy control for that numbering.

## Verification

The final seven-test focused release run, including the maximum-width receipt
boundary, passes with zero failures in 47.86 seconds. The full release Solver
library regression passes 743 tests, with zero
failures and the existing 34 ignored specialist tests, in 270.17 seconds.
Both real recovery controls also pass in debug mode with Wide assertions
enabled, in 311.48 seconds. Runs overlap other local verification work;
these durations are execution records, not comparative performance evidence.

All-target Clippy with warnings denied passes for Solver, CLI and script-runner.
The 21 tensor tests pass. Eight native tensor, operator registry, document
inventory/book and project-organization checks pass, including available
self-tests. Tensor calibration retains zero structural gaps, four maturity
gaps, nineteen evidence-grade gaps and fourteen P0 gaps; Daji qualification
remains blocked. Positive private research does not close those production scopes.

The operator registry reports 59 profiles with `executed=false`; registry
validation is not numerical execution of all profiles. The book check covers
26 HTML files and current 3.4.5 metadata. Formatting/diff checks pass, with
800/2000 source/document caps and zero tracked organization debt. HTML links
and structure are checked, but GUI rendering and installed Apps are not tested.

## Reproduction and qualification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release --locked --offline -- --nocapture modal_hybrid_grid modal_material_hybrid modal_material_coupled_loss_stage_ablation
cargo test -p kyuubiki-solver --lib modal_material_hybrid_real --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

Evidence stays in the existing normalized-candidate research scope. No public
Solver, Engine, Agent, installed GUI, Headless study, remote/Linux/Windows or
general topology/multimode qualification is claimed. Production scopes and
defaults remain unchanged. Next priority is to resolve the four retained
internal failures and validate wider geometry without losing these old outputs,
not to promote a 42-input research result as a universal solver guarantee.
