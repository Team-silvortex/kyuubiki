# Modal coupled candidate selection and physical recovery

October 5, 2026. Local macOS ARM64 Solver research follows the
[physical-first joint experiment](modal-physical-first-joint-reliability-20261005.md).
The campaign began at HEAD `dc5647af` (`daji 3.4.6`); package metadata remains
3.4.5. No version, installed application, production Solver admission,
Engine authority or Agent strategy changes.

This is test-only coupled two-stage research, not production modal admission.
The new GridNorm route recovers the previously rejected UnequalLengths
128-member tiny input with independent JSON physical residual
`6.757600695706912e-9`. It also loses two old successes. Across 42 inputs and
126 separate cold routes, the old full pipeline accepts 37, coupled GridNorm
accepts 36 and coupled Reverse accepts 28. A new successful input does not
justify default replacement or close production qualification.

## Retained two stage contract

The previous joint experiment required post-physical rounded internal
re-encoding at the original initializer anchor, a condition absent from the
old publication contract. This experiment instead certifies the chosen
internal direction and its subsequently recovered physical shape separately.
It does not impose that extra re-encoding gate.

Each route independently rebuilds the original-numbered horizontal
single-mode request using its current material data. Eigenvalue and ordinary
refinement seed bits stay frozen. Four Wide banded-inverse steps supply a
rounded continuous initializer; no old reference direction seeds the search.

One preselected rounded QR factor supplies bounded internal beam proposals.
Each paired receipt computes the actual normalized operator residual, maps
the same proposal to a unit physical shape and independently computes its
actual physical residual. Only internally eligible proposals can be selected.
Mapped physical residual ranks these proposals, with internal residual as
the tie-breaker; it is a hint, not a physical publication certificate. An
initially internally passing seed remains unchanged and skips the beam.

A fresh final paired receipt must still pass the internal `1e-8` gate.
Malformed values, callback errors, lost final eligibility and cancellation
do not release a partial accepted direction. Direction norm remains in
`0.25..=2`, and the internal anchor stays bit-identical.

After internal selection, the old unit-shape mapping and physical seed
polisher prepare a separate physical grid. The internal canonical dense case
is dropped before constructing this physical case. The physical anchor is
chosen by mass-weighted amplitude and frozen before its sole rounded QR fit.
The separate cold Inward diagnostic chart moves that anchor one ULP toward
zero before the fit, only while it remains uniquely dominant.

Physical beam selection uses actual physical residual receipts and a fresh
final `1e-8` gate. The chosen physical shape must also satisfy unit norm
`1e-10`, without renormalizing after its final certificate. This wrapper is
conservative: if the best residual candidate loses unit norm, it rejects
rather than trying another candidate or another fit.

Accepted shapes pass the shared unchanged test-only publication tail: inertia
bracket, original node/member numbering, fixed positive-zero DOFs and free-DOF
mapping. Serialized output is read back and independently reassembled to
check the physical residual. There is no production selector for this policy.

## Separate bounded costs

| Bound | Internal selection | Physical recovery |
| --- | ---: | ---: |
| Active coordinates | 2 through 256 | 2 through 256 |
| Beam width | 1 through 64 | 1 through 64 |
| Integer radius | 1 through 4,194,304 | 1 through 4,194,304 |
| Preselected rounded QR starts per stage | 1 | 1 |
| Actual receipts per stage | width plus 2 | width plus 2 |
| Width 64 payload reservation at 256 coordinates | 4,751,360 bytes | 4,734,976 bytes |
| Width 64 component visit reservation at 256 coordinates | 606,142,464 | 603,979,776 |

The internal path shares the existing paired-proposal reservation with the
joint path, not its extra admission condition. These are separate per-fit
proposal reservations, not whole-pipeline memory/work or process RSS limits.
They exclude borrowed matrices/operators, cold initialization, callback
operator products, physical mapping, seed polishing, independent reassembly
and allocator overhead. Dropping the internal case shortens its lifetime;
no measured memory or speed improvement is claimed.

This campaign uses width sixteen: at most eighteen paired internal receipts
(thirty-six actual operator products), followed by at most eighteen physical
receipts. The combined proposal certificate cap is therefore fifty-four
actual products, in addition to initialization, polishing, inertia and
readback. Internal rejection does not start a physical fit. Initially passing
stages use two fresh receipts and do not search.

## Difficult candidate diagnosis

Four retained ThreeLayers inputs have 100 members at scale `1e-10`, or
128 members at scales `1`, `1e14`, `1e-10`. A width-sixteen Reverse internal
beam contains sixteen distinct eligible directions for the 100-member input
and none for the three 128-member inputs. Candidate identity is checked by
bits, excluding the duplicate final observation.

All sixteen eligible directions are checked by two freshly rebuilt old
physical charts, giving 32 independent diagnostic tails. Every tail rejects
in the physical stage with one fit and four receipts. They converge to the
same chart-specific failed physical result: Original residual
`1.5015873467601256e-8`, Inward residual `1.6462908586622078e-8`. Simply selecting
a different internally eligible proposal does not repair this input.

Sixteen separate new cold routes cover the four inputs, two orders and two
physical charts. The 100-member Reverse route passes internally but still
rejects physically: Original residual `1.5970091951409624e-8`, Inward residual
`1.526731820719233e-8`, with eighteen receipts per stage. Its GridNorm route
and every 128-member route reject internally with eighteen internal receipts
and no physical fit. There are zero hard-input publications.

These bounded observations do not prove a globally impossible floating-point
solution, an invalid old admitted output or an Engine failure.

## Fresh material and mesh comparison

Four recipes (UnequalLengths, NonbinaryGradient, ThreeLayers, Jittered),
three sizes (80, 100, 128 members) and three coordinate scales (`1`, `1e14`,
`1e-10`) supply 36 inputs. Six Graded/Layered 128-member baselines make 42.
Each input executes three independent cold policies, not a retry portfolio.
All 42 stage triples are asserted, not only their aggregate counts.

| Policy | Accepted | Internal rejection | Physical rejection |
| --- | ---: | ---: | ---: |
| Old full pipeline | 37 | 4 | 1 |
| Coupled GridNorm | 36 | 5 | 1 |
| Coupled Reverse | 28 | 11 | 3 |

GridNorm shares 35 old acceptances, gains one, loses two and shares four
rejections. Its gain is UnequalLengths 128 members at `1e-10`. Its losses are
UnequalLengths 128 at `1e14` (physical residual `1.0082746780306733e-8`) and
the old Layered 128 baseline at `1e-10` (internal rejection). Reverse shares
28 old acceptances, gains none, loses nine and shares five rejections.

The coupled routes use fixed orders and different proposal budgets from the
old ranked portfolio. This is a complete two-stage policy comparison, not
an isolated proof that the ranking hint alone caused the gain or losses.
The four difficult ThreeLayers inputs remain rejected by both new policies.
Preserving old successes while retaining the new gain is still open.

## Fault cancellation and fresh recovery controls

Synthetic controls verify that an eligible internal direction with lower
mapped error can outrank a lower internal residual. An initially internally
passing seed remains unchanged even when its mapped hint fails the physical
threshold; it is only an internal candidate, not a published physical result.

The six width-four callbacks receive six explicit faults and 48 malformed
receipts. Two lost final internal gates cover initially passing and searched
seeds. Six cancellations cover factor, substitution, update, final search
and validation boundaries, followed by six fresh healthy replays. Invalid
plans, incompatible Wide factors and nonunit physical seeds decline. Six
known-null controls use actual Laplacian row products, both signs and three
binary matrix scales. Lost physical unit norm rejects without renormalization.

The real gained input retains two final-pair faults, one lost final internal
gate and five cancellations, followed by eight fresh exact internal replays.
One cancellation occurs inside the second actual operator product of the
last pair, after the normalized half has completed. Neither half a checked
pair nor a partial accepted candidate escapes. Fresh numbering preserves
root, ordinary seed, internal shape bits, paired metrics and receipt counts.

Two independently numbered gained outputs pass original-numbered JSON
reassembly with residuals `6.757600695706912e-9` and
`6.7576006957069195e-9`. Corresponding shape bits match by geometry; summation
order permits the small independent residual difference. Both use one fit
and eighteen receipts per stage; the old policy rejects both physically.

Final physical unit validation, final node restoration and final publication
are separately cancelled. They produce no output. Four newly numbered hard
requests also produce no output or readback. Seven fresh healthy JSON replays
match the independent healthy control of the same numbering exactly. No
extra fallback fit is started after these failures or cancellations.

## Verification

The focused release filter passes all seven added tests with zero failures
in 46.66 seconds. The full release Solver library regression passes 736 tests,
with zero failures and the existing 34 ignored specialist tests, in 288.36
seconds. Both real recovery controls pass in debug mode with Wide assertions
enabled, in 230.05 seconds. These runs overlap other local verification work;
durations are execution records, not comparative performance or throughput.

All-target Clippy with warnings denied passes for Solver, CLI and script-runner.
The 21 tensor tests pass. Native tensor validation retains zero structural
gaps, four maturity gaps, nineteen evidence-grade gaps and fourteen P0 gaps;
Daji qualification remains blocked. A gained test-only physical output does
not close those production scopes.

The eight native tensor, operator registry, document inventory/book and
project-organization checks pass, including the available self-tests. Registry
validation reports 59 profiles with `executed=false`; it is not numerical
execution of every profile. The book checker covers 26 HTML files and current
3.4.5 metadata. Formatting and diff checks pass. Source/document caps remain
800/2000 with zero tracked organization debt. HTML content and links are
validated, but no GUI rendering or installed application is exercised here.

## Reproduction and qualification

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release --locked --offline -- --nocapture modal_material_coupled modal_coupled_grid
cargo test -p kyuubiki-solver --lib modal_material_coupled_real --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

This evidence belongs to the existing Solver normalized-candidate research
scope. Production ordering/admission, heterogeneous physical publication,
whole-pipeline budgets, arbitrary geometry and multimode recovery remain
unqualified. No remote/Linux/Windows, installed GUI, Agent execution or
Headless material-study qualification is claimed. Existing residual gates,
Engine/Agent ownership and production strategy defaults remain unchanged.

Next priority is bounded coordinated construction that preserves old
successes while retaining the gained physical publication. Recipe dispatch,
blindly wider search, tolerance relaxation and global impossibility claims
are not substitutes for that evidence.
