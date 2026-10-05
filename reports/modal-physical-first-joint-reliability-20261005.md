# Modal physical first joint certification and recovery boundaries

October 5, 2026. Local macOS ARM64 Solver research follows the
[amplitude and rounded beam campaign](modal-amplitude-rounded-beam-reliability-20261004.md).
Package metadata remains Daji 3.4.5; the campaign began at HEAD `dc5647af`
(`daji 3.4.6`). No version, package, installed application, public Solver
admission, Engine authority or Agent execution strategy changes.

This is test-only physical-first joint research, not production modal admission.
Across 42 fresh inputs and 126 cold routes, the retained full pipeline accepts
37 inputs, the joint GridNorm route accepts one and joint Reverse accepts two.
The joint policy adds a stronger requirement absent from the old contract:
rounded internal re-encoding of the final physical shape must also certify at
the original initializer anchor. This is not a like-for-like unchanged-contract
replacement. Joint rejection does not invalidate old accepted outputs or prove
that a representable solution does not exist.

## Construction and additional requirement

Each route independently rebuilds the original-numbered horizontal single-mode
request and assembles its current material data. The eigenvalue and ordinary
refinement seed remain frozen within each route. Four Wide banded-inverse steps
form a continuous initializer; no old reference direction seeds the search.

The physical seed divides Wide direction components by f64 mass square roots,
uses a Wide sum of squares and two fixed Newton norm updates, then rounds once.
This is compensated component arithmetic, not a fully Wide mass-square-root
calculation or certified interval construction. The diagnostic also retains the
old rounded-direction-to-physical mapping as a separate cold seed chart.

A current-request canonical permutation and one preselected rounded QR factor
define the physical ULP grid. The dominant mass-weighted physical anchor is
frozen before the fit. Bounded beam proposals are driven by actual physical
residual vectors. A candidate is retained only when its physical unit norm
passes and its maximum physical/internal residual improves.

Each paired receipt first computes the actual physical operator product. It
then maps the same physical shape through mass square roots, fixes its gauge
to the original rounded internal initializer anchor, rounds the reconstructed
internal direction and independently computes its actual normalized operator
product. This extra re-encoding condition is different from certifying the
original internal proposal and the subsequently recovered physical shape,
which is what the old two-stage pipeline does.

Both actual residuals must pass `1e-8`, and physical unit norm must pass `1e-10`.
The internal gauge anchor remains bit-identical and reconstructed direction
norm remains in `0.25..=2`. A fresh final paired receipt is mandatory even when
the initial seed already passes. Fault, malformed receipt, last-gate loss or
cancellation produces no accepted partial candidate and no extra fallback fit.

Accepted proposals use the shared unchanged test-only publication tail:
inertia bracket, original node/element numbering, fixed positive-zero DOFs,
free-DOF mapping and independent JSON physical reassembly. There is no selector
in the production Solver, Engine or Agent for this research policy.

## Separate proposal bounds

| Bound | Value |
| --- | ---: |
| Active coordinate range | 2 through 256 |
| Beam width range | 1 through 64 |
| Integer radius range | 1 through 4,194,304 |
| Preselected physical QR starts per route | 1 |
| Paired receipt cap | width plus 2, at most 66 |
| Width 64 payload reservation at 256 coordinates | 4,751,360 bytes |
| Width 64 component visit reservation at 256 coordinates | 606,142,464 |

The reservation covers the selected physical-grid factor and proposal scratch
and scans. It excludes the borrowed operator/matrix, cold initialization,
callback operator products, physical/internal mass mapping, independent
reassembly, allocator overhead and process RSS. It is not a whole-pipeline
memory/work qualification. Width sixteen uses at most eighteen paired receipts,
each requiring two actual operator products; initialization and readback costs
are additional. The paired validator borrows the residual vector without
cloning it simply to validate finiteness and size.

## Difficult input diagnostics

The four retained ThreeLayers inputs have 100 members at scale `1e-10`, or
128 members at scales `1`, `1e14`, `1e-10`. Two physical seed charts, two fixed
orders and widths sixteen/sixty-four give 32 independent physical-first routes.
All reject, with exactly eighteen/sixty-six paired receipts and one physical
QR factor per route. Root and ordinary seed bits remain unchanged. The two
seed charts share the observed rejection boundary, not a proven general
equivalence.

For the 100-member tiny input under the Wide physical seed, the GridNorm width
sixteen final shape has physical residual `8.222295761432855e-9`, internal
re-encoding residual `1.261833676899761e-7` and norm
`0.9999999999998248`. GridNorm width sixty-four has physical residual
`7.027208206837947e-9` but internal residual `1.4906007811806967e-7`.
Reverse width sixty-four has physical residual `7.44913257635355e-9` but
internal residual `1.2748398139318403e-7`. A physically passing candidate is
therefore not enough for this added experimental condition.

Sixteen separate old normalized beam routes inspect all actual internal
candidates after direct unit-shape mapping, without physical recovery. Across
672 paired observations, only the 100-member tiny Reverse route has internal
passes: seventeen at width sixteen and sixty-five at width sixty-four, including
each fresh final duplicate. Every corresponding mapped physical shape fails;
there are zero joint passes and zero physical publications. The three
128-member inputs have no normalized passes in these diagnostics.

These observations isolate sensitivity to cross-space rounding and fixed-gauge
reconstruction in this bounded experiment. They do not prove an impossible
lattice, a broken production result or an Engine failure.

## Fresh material and mesh comparison

The retained holdout grid covers four recipes (UnequalLengths,
NonbinaryGradient, ThreeLayers, Jittered), three sizes (80, 100, 128 members)
and three coordinate scales (`1`, `1e14`, `1e-10`). Six old Graded/Layered
128-member baselines make 42 inputs. Each input runs three independent cold
policies, not a retry portfolio: old full pipeline, joint GridNorm and joint
Reverse. Per-input boundaries as well as aggregate totals are asserted.

| Policy | Accepted | Internal rejection | Physical or joint rejection |
| --- | ---: | ---: | ---: |
| Retained full pipeline | 37 | 4 | 1 |
| Joint GridNorm | 1 | 0 | 41 |
| Joint Reverse | 2 | 0 | 40 |

All four hard ThreeLayers inputs reject both new policies. All six old
128-member baselines also reject the joint policies despite passing the old
pipeline. Joint GridNorm accepts only Jittered 80-member tiny scale; Reverse
accepts Jittered 80-member unit and tiny scales. Tiny seeds already pass and
require two paired receipts; unit-scale Reverse requires eighteen.

The old rejection boundary remains four internal ThreeLayers failures plus
UnequalLengths 128-member tiny physical failure. Spectral and independent JSON
checks pass for the accepted routes. The added internal re-encoding requirement
and different proposal budgets make these outcome counts unsuitable as a
claim of unchanged-contract production regression or improvement.

## Fault cancellation and fresh recovery controls

The synthetic joint controls cover all six width-four callbacks with six
explicit faults and 48 malformed paired receipts. Negative/nonfinite physical
or internal values, missing/oversized/nonfinite residuals and invalid plans
fail closed. Four final physical/internal gate losses cover both initially
passing and searched seeds. Five distinct cancellations have five fresh exact
replays. An incompatible preselected Wide factor declines before callbacks.

Six known-null controls use a three-node Laplacian, both signs and three binary
matrix scales. Actual row products recover the known unit null shape while
retaining the strict final norm gate; this analytic case is not an exhaustive
proof for general beam search or heterogeneous materials.

Real Jittered 80-member unit-scale Reverse controls retain a sign relation and
fresh numbering replay with exact roots, seeds, shapes and receipt counts.
Two last-receipt faults, two lost final gates and five cancellations are
followed by ten freshly rebuilt exact healthy replays, including the separate
material-change boundary. One cancellation occurs after the physical half of
the final paired receipt, inside the second actual operator product. Neither
a half-checked pair nor a partial accepted candidate escapes.

The separate material request scales Young's modulus by sixteen and density
by four. Its freshly computed eigenvalue scales exactly by four, but ordinary
refinement seed bits differ. The joint path rejects: physical residual is
`1.3863092036565867e-9`, while reconstructed internal residual is
`1.017267805013258e-8`, using eighteen paired receipts. The retained old pipeline
accepts the same newly scaled request with identical root and ordinary seed
bits to the joint route. Root scaling alone does not imply exact cold-path
seed or acceptance covariance. This boundary is recorded rather than weakening
the unchanged residual threshold.

Two primary healthy original-numbered outputs have independent JSON residuals
`1.3863121237650508e-9` and `1.386312123765051e-9`. Corresponding nodal shape
bits, roots, seeds and counts match after geometry mapping; independent assembly
can change residual summation order, so bitwise residual equality is not required.
One final node-restoration cancellation and one final publication cancellation
produce no output. Four newly numbered difficult requests also produce no
output or readback. Six fresh healthy JSON replays exactly match the separate
healthy control of the same numbering layout.

## Verification

The final release Solver library regression passes 729 tests with zero failures
and the existing 34 ignored specialist tests, in 161.89 seconds. The focused
joint filter passes all seven added tests in 47.73 seconds. Both real recovery
controls also pass in debug mode with Wide debug assertions enabled, in
30.09 seconds. These are test execution records, not comparative performance
or throughput measurements.

All-target Clippy with warnings denied passes for Solver, CLI and script-runner.
The 21 tensor tests pass. Tensor status retains zero structural gaps, four
maturity gaps, nineteen evidence-grade gaps and fourteen P0 gaps; Daji
qualification remains blocked. Negative research does not close those scopes.
Formatting and diff checks pass. Native tensor, operator registry, document
inventory/book and project-organization checks include their available self-tests.
The operator registry reports 59 profiles with `executed=false`; registry
validation is not numerical execution of every profile. Source/document limits
remain 800/2000 with zero tracked organization debt.

## Reproduction and qualification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release --locked --offline -- --nocapture modal_material_joint modal_joint_grid
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-solver --lib modal_material_joint_real --locked --offline -- --nocapture
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

The established scoped research claim retains these positive controls and
negative boundaries. No production claim is created or promoted. The eight
named modal production obligations remain open. This local campaign is not
remote, Linux, Windows, installed GUI, live Agent/Headless, multimode,
external correlation or end-to-end performance qualification.

The next construction must first establish which internal-to-physical contract
it intends to preserve and compare against old successful cases on that same
contract. Blindly increasing beam width or adding runtime retries is not
justified by the retained results.
