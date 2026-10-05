# Modal amplitude charts and bounded rounded beam recovery

October 4, 2026. Local macOS ARM64 Solver research follows the
[banded floating-grid comparison](modal-banded-grid-construction-reliability-20261004.md).
Package metadata remains Daji 3.4.5; HEAD was `dc5647af` (`daji 3.4.6`)
when this campaign began. No version, package, install, public Solver admission,
Engine authority or Agent execution strategy changes.

This is test-only amplitude and rounded-beam evidence, not production modal admission.
A bounded multi-candidate backsolve gains ten fixed-order normalized successes
but loses one old success. Its newly recovered difficult 100-member direction
still fails both physical restoration charts. All three difficult 128-member
directions remain rejected. Neither extra search width nor internal acceptance
justifies a production replacement.

## Construction and retained contracts

Fresh original-numbered horizontal single-mode requests are rebuilt and
independently assembled. The original eigenvalue and ordinary refinement seed
bits remain frozen. Four existing Wide banded-inverse steps form the continuous
initializer. No historical reference direction seeds a search.

The amplitude experiment scales this Wide direction before rounding, using
32 independent charts from `0.75` through `1.71875` in increments of `1/32`.
Each runs the old two-fit/thirteen-certificate ranked normalized search.
The best actually checked QR vector separately seeds a four-pass finite-box
DP diagnostic, retaining its frozen anchor. Faults are not caught as numerical
rejections, and these cold experiments are not an admitted retry portfolio.

Rounded beam backsolve reuses one preselected f64 QR factor of the normalized
fixed-grid columns. From the last row upward, each partial decision branches
to the nearest clamped integer and its two adjacent integers within the radius.
The computed transformed residual ranks at most 64 retained partial decisions;
a sign-oriented tie-break keeps deterministic sign controls. Nonfinite scores
and nonzero squared-error underflow reject rather than disappear. These local
branches and pruning are not an exhaustive global lattice search.

Each completed proposal is converted through the existing Wide grid update
and independently checked with the actual operator. The lowest actual residual
is retained; a new final operator receipt must still pass. This final check
also runs when the initial direction already passes. Matching finite residual
vectors, normalized residual `1e-8`, frozen anchor, and direction norm
`0.25..=2` remain mandatory. Fault, malformed receipt, lost final gate or
cancellation returns no accepted partial candidate or fallback factor.

The existing physical unit-shape recovery, inertia bracket, original numbering
restoration and independent JSON readback now share one unchanged test-only
publication tail. That refactor does not change numerical arithmetic or give
the beam a different unit-norm or residual gate. Physical residual `1e-8` and
unit norm `1e-10` still apply; Engine and Agent do not select these experiments.

## Separate proposal budgets

| Bound | Value |
| --- | ---: |
| Active coordinate range | 2 through 256 |
| Beam width range | 1 through 64 |
| Integer radius range | 1 through 4,194,304 |
| Selected QR starts per route | 1 |
| Actual receipt cap | width plus 2, at most 66 |
| Width-64 payload reservation at 256 coordinates | 4,734,976 bytes |
| Width-64 component-visit reservation at 256 coordinates | 603,979,776 |

The payload/work plan includes the selected rounded-grid factor and bounded
proposal scratch, not the borrowed operator/matrix, initializer, allocator
overhead, physical stage, whole-pipeline cost or process RSS. It is separate
from the old four-pass/six-certificate greedy plan. Width-16 comparisons use
at most eighteen actual receipts. A wider beam is not a monotonically better
actual certificate: transformed scores and rounded physical products differ.

## Difficult input diagnostics

The four retained ThreeLayers inputs have 100 members at scale `1e-10`, or
128 members at scales `1`, `1e14`, `1e-10`. The continuous initializer was
already below `1e-13`; its nearest f64 shape failed actual certification.

Across 128 independent amplitude charts, only the 100-member input at amplitude
`1.03125` passes the normalized gate, with actual residual
`8.832323691767534e-9`. The separate QR-to-four-pass-DP diagnostics add no
normalized successes. All individual chart acceptance boundaries are pinned.

The beam diagnostic runs three orders, four widths (`1`, `4`, `16`, `64`) and
two radii (`4`, `4,194,304`) on each of the four inputs: 96 cold routes.
Radius four has zero acceptances. At the larger radius, only Reverse order
on the 100-member tiny input passes, with width sixteen or sixty-four.
Their actual residuals are respectively `8.260798081769468e-9` and
`9.381795166012982e-9`. The larger beam does not give a better actual residual.

Sixteen fresh full-tail attempts retain both original and inward physical
charts under two numbering layouts. The width-16 100-member normalized
direction fails physical restoration with best residuals
`1.5015873467601256e-8` and `1.6462908586622078e-8`: one physical factor and
four receipts in either chart. The three 128-member cases fail internally
before any physical factor, using one internal factor and eighteen receipts.
All stages, budgets, absence of output and changed-numbering outcomes are pinned.
There are zero difficult-input publications, not four recovered research results.

## Material and mesh comparison

The 36 retained material holdouts cover four recipes, three mesh sizes
(`80`, `100`, `128`) and three coordinate scales. Six old Graded/Layered
128-member baselines make 42 inputs. Each compares GridNorm and Reverse
orders under two separate cold policies: old four-pass greedy backsolve and
new one-pass width-16 beam. This is 168 routes, not a width-only comparison
with otherwise identical iteration or certificate budgets.

| Outcome over 84 fixed-order pairs | Pairs |
| --- | ---: |
| Both accept normalized direction | 58 |
| Beam gains a normalized acceptance | 10 |
| Beam loses an old normalized acceptance | 1 |
| Both reject normalized direction | 15 |

Old greedy accepts 59 of 84 fixed-order routes and beam accepts 68. On the
36 holdouts, these are 52 versus 58; on six old baselines, seven versus ten.
The single lost route is Jittered, 128 members, scale `1e14`, Reverse order.
All 42 four-policy boundaries are asserted rather than only the totals.
This campaign publishes no physical result and does not imply 68 successfully
recovered original-numbered outputs.

## Independent and failure controls

The analytic QR test recovers a known integer solution. Twelve small RHS cases
are compared with all 81 integer pairs in a radius-four box, with twelve sign
controls and twenty-four binary matrix/RHS scaling controls. This finite test
does not prove general beam optimality. Invalid width/radius/RHS, score
underflow, direction norm, preselected factor and size are rejected.

Synthetic actual-receipt controls inject faults at all six width-four callbacks,
thirty malformed receipts, two final-gate losses and five cancellations.
Every cancellation has a fresh healthy replay. Real 100-member accepted
normalized recovery additionally retains two sign/binary relations, two last
receipt faults, one lost final gate, five cancellations, eight exact healthy
replays and a freshly rebuilt numbering control. No partial candidate escapes.

The healthy ThreeLayers 80-member unit-scale control retains four primary
original-numbered physical outputs across two charts and two numbering layouts.
Independent JSON residuals are about `3.529438742668612e-9` for Original and
`5.925808787426667e-9` for Inward, both within the unchanged gate. Root, seed,
counts and corresponding nodal shape bits are identical after coordinate
mapping. Original-numbered independent assembly can differ by two residual
ULPs due to summation order; it is not asserted as bitwise residual equality.
Two final node-restoration and two final publication cancellations return no
output. Four fresh healthy replays each match a separate cold control exactly.

## Verification

Final release Solver library regression passes 722 tests with zero failures
and the existing 34 ignored specialist tests, in 126.59 seconds. The combined
amplitude/rounded-beam filter passes eleven tests, including three retained
older amplitude controls, in 30.86 seconds. The seven new beam-filter tests
separately pass in 20.28 seconds. These durations are test execution records,
not throughput or comparative performance measurements.

The real final-fault/cancellation/sign/numbering control also passes in debug
mode, with Wide debug assertions enabled, in 6.98 seconds. All-target Clippy
with warnings denied passes for Solver, CLI and script-runner. The 21 tensor
tests pass, as do eight native tensor/operator-registry/document/organization
checks, including self-tests, and formatting/diff checks.

The operator registry still reports 59 profiles with `executed=false`; its
check is not numerical execution of all profiles. Tensor status stays at zero
structural gaps, four maturity gaps, nineteen evidence-grade gaps and fourteen
P0 gaps, with Daji qualification blocked. The 800-source/2000-document line
limits retain zero tracked organization debt. No scope is promoted to qualified.

## Reproduction and qualification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release --locked --offline -- --nocapture amplitude rounded_beam
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-solver --lib modal_material_rounded_beam_real_final --locked --offline -- --nocapture
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

The scoped tensor keeps the established research claim and appends these
positive and negative controls. No production claim is created or promoted;
the eight named modal production obligations remain open. This local run is
not remote, Linux, Windows, installed GUI, live Agent/Headless, multimode,
external correlation or an end-to-end performance qualification.

The next construction must coordinate internal quantization with physical
unit restoration, retain old successful cases and keep finite budgets. More
local passes or blindly larger beams are not justified by this evidence.
