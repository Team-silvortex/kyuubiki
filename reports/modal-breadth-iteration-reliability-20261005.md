# Bounded modal breadth and iteration comparison

October 5, 2026. Local macOS ARM64 Solver research extends the
[legacy-first physical hybrid](modal-legacy-first-hybrid-reliability-20261005.md).
The retained 42-input comparison now accepts 39 physical outputs, preserving
all 38 previous successful JSON outputs and factor/receipt counts exactly.
The new success is the ThreeLayers 100-member input at coordinate scale `1e-10`.
Three 128-member ThreeLayers inputs remain internally rejected.

This is test-only breadth and iteration research, not production modal admission.
Package metadata remains Daji 3.4.5 and HEAD remains `dc5647af` (`daji 3.4.6`).
No version, installation, public Solver admission, Engine authority or Agent
strategy changes are made. This is a numerical reliability comparison, not a
performance benchmark or an arbitrary-topology qualification.

## Construction and budgets

The initializer is still four bounded Wide banded-inverse iterations on each
fresh assembled request. The ordinary eigenvalue and seed bits remain frozen.
Each canonical grid freezes its dominant anchor, original coordinate steps,
rounded QR factor and total radius `2^22`. No reference direction seeds a search.

The new construction compares one pass of width 64, two passes of width 32,
and four passes of width 16. The product of width and passes may not exceed 64.
An actual operator receipt selects the best direction and supplies the next
pass's residual. Every correction is reconstructed from the original seed and
cumulative integer offsets, not repeatedly rounded around a drifting center.
Branches leaving the original radius are skipped. No improvement ends the
search; a fresh final actual receipt is mandatory even after earlier eligibility.

The cold construction starts at the initializer. A separate warm comparison
first runs the old greedy correction on the same factor, then continues only
after numerical rejection without any passing old receipt. The retained center
must be exactly reconstructible on the frozen integer grid within its radius.
Faults, malformed receipts, cancellation and a lost passing old final gate stop.
The warm comparison is not the supplementary policy selected below.

At 256 coordinates the common conservative proposal plan reserves 4,747,264
extra numeric payload bytes and 608,403,456 component visits. It includes the
greedy prelude reservation for both comparisons; these are not measured RSS,
whole-pipeline allocation, wall-clock work or a production admission budget.
Borrowed matrices, request assembly, spectral initialization and operator
storage are outside this proposal-only accounting.

The cold receipt cap is 66; the warm cap is 72. Synthetic monotone callback
controls reach both caps and verify the frozen radius. Such synthetic callbacks
test control flow and resource accounting, not real numerical recovery.
Known three-coordinate Laplacian null directions separately exercise twelve
actual matrix-residual combinations of both signs and binary matrix scales.

## Breadth and iteration results

Four retained difficult inputs are compared across three orders, three budget
splits and two starting policies: 72 isolated routes, not 72 distinct models.
All routes use one prepared rounded QR factor. Six internally accepted routes
belong to the same 100-member tiny input under Reverse order. All six continue
through fresh physical preparation with the same frozen root and ordinary seed.

| Reverse policy on 100 members at scale `1e-10` | Internal actual residual | Physical outcome |
| --- | ---: | --- |
| Width 64, one pass, cold | 9.381795166012982e-9 | Independent readback passes |
| Width 32, two passes, cold | 9.298210161259298e-9 | Physical gate rejects |
| Width 16, four passes, cold | 8.260798081769468e-9 | Physical gate rejects |

The warm policies have the same three acceptance boundaries and the same
physical outcomes here. Only the two width-64 routes publish. The narrower
routes' better internal residual does not imply a better physical result;
their physical minimum is about `1.5015873467601256e-8`, above the unchanged gate.
Repeated passes add no new hard-input publication in this diagnostic campaign.

Across all compared diagnostic orders and splits, the three 128-member inputs
remain rejected. Best internally measured margins are `1.1557142471390426e-8`
at unit scale, `1.172556673695903e-8` at scale `1e14`, and
`2.130546151660412e-8` at scale `1e-10`. These are finite-search observations,
not proofs of global nonrepresentability or a reason to relax tolerances.

## Retained physical pipeline

The separate full-pipeline candidate first executes the unchanged old internal
ranked policy. Only exhaustion of its numerical policies with no passing old
receipt can start one preselected cold Reverse width-64 pass. It does not switch
on recipe, mesh size or scale. Old faults and stale final gates never trigger
the supplementary construction. One extra internal fit is allowed; existing
physical greedy-first same-factor recovery is unchanged.

The whole internal stage has at most three QR starts and 79 actual receipts,
checked before every callback. The physical stage retains its separate one-fit,
25-receipt cap. These caps do not promote the old portfolio's production work
budget or imply that the full initializer/publication pipeline has been qualified.

All 42 retained inputs are independently rebuilt under both old and candidate
policies: 84 cold routes. They are four recipes, 80/100/128 members and three
coordinate scales, plus six graded/layered 128-member baselines. These are
reused research boundaries, not new unseen validation coverage.

| Outcome | Old physical hybrid | Supplementary breadth candidate |
| --- | ---: | ---: |
| Full independent JSON readback passes | 38 | 39 |
| Internal rejection | 4 | 3 |
| Physical rejection | 0 | 0 |
| Exact old successful outputs retained | 38 | 38 |
| New physical results | 0 | 1 |
| Old successful outputs lost | 0 | 0 |

Every old successful output preserves root, ordinary seed, selected order,
refinement status, grid factor counts, actual receipt counts, physical readback
bits and complete serialized JSON bytes. The new input uses three internal
fits and one physical fit, with actual receipt counts `[74, 23]`. Its independently
reassembled physical readback is `8.943162509310455e-9`, below the unchanged
`1e-8` residual gate. Unit norm remains subject to the unchanged `1e-10` gate.

Fresh numbering key 113 also passes, at `8.943162509296278e-9`. Root, ordinary
seed, selected order, receipt counts, canonical physical metric and
geometry-aligned output shape bits match exactly. The independently accumulated
readback residuals differ by about `1.42e-20`; no bitwise residual invariance is
claimed across reordered assembly. Both independent receipts must pass the
same original gate. Each layout's subsequent fresh JSON replay is exact.

## Failure and recovery controls

Synthetic controls cover 42 callback fault positions, 210 malformed receipts,
two stale final gates and six cancellation checkpoints with fresh replay.
Invalid size, width, pass product, radius, tolerance, norm and preselected factor
decline before operator callbacks. No failure returns a partial candidate.

The real recovered internal direction separately retains both signed shapes,
four actual callback faults, five malformed final receipts, one lost final gate,
and cancellation inside the final sparse operator product. Eleven fresh
independently prepared healthy replays retain exact direction bits and receipt
counts; root and ordinary seed bits remain unchanged.

The complete new physical result is cancelled at the supplementary substitution,
before its final receipt, after final internal validation, after physical unit
validation, during original numbering restoration and immediately before result
publication. Six cancelled requests and three retained failing 128-member
requests are followed by nine new changed-numbering requests with exact healthy
JSON replay. None publishes a partial result or carries a failed candidate into
the next request.

## Verification

The complete Solver release library run passes 750 tests with zero failures
and 34 ignored specialist cases, in 439.36 seconds. After adding explicit
per-route diagnostic assertions, the final focused release run passes all
seven new tests in 32.34 seconds. The two real fault/cancellation/numbering
controls also pass in the debug build with Wide assertions enabled, in
418.10 seconds. Some runs overlap; these elapsed test times are not comparable
throughput measurements or benchmark claims.

All-target Clippy for Solver, CLI and script-runner passes with warnings denied.
All 21 tensor tests pass. Eight native registry/document/organization checks,
including their available self-tests, pass, as do formatting and diff checks.
The book checker covers 26 HTML files. Source/doc limits remain 800/2000 lines
with zero tracked organization debt.

The operator registry still reports 59 profiles with `executed=false`; registry
validation is not numerical execution of every profile. Tensor structure has
13 modules, 11 paradigms and zero structural gaps, but four maturity gaps,
19 evidence-grade gaps and 14 P0 gaps remain. Production qualification stays
blocked; the candidate evidence does not close a qualified production scope.

## Reproduction

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release --locked --offline -- --nocapture modal_iterated_grid modal_material_iterated_beam
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-solver --lib --locked --offline -- --nocapture modal_material_iterated_beam_real
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

The three hard internal failures remain open. Broader geometry, multimode,
remote or installed execution, full work budgets, external solver correlation
and public Solver/Engine/Agent qualification are not established by this run.
The next step is a genuinely different bounded construction or representation
study, not simply more iterations of the same discrete search.
