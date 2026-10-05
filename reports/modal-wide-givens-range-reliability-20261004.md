# Modal Wide QR Range and Local Rotation Reliability

October 4, 2026. Local macOS test-only numerical construction research.
The retained wide Householder failures are tiny nonzero intermediate values,
not overflow. A separate Givens candidate avoids those particular failures but
creates other guarded failures and does not solve the three 128-member layered
counterexamples. It is not a replacement or a production recovery policy.

This is test-only wide-factor range research, not production modal admission.

## Retained Householder Boundaries

The four retained ThreeLayers inputs are 100 members at coordinate scale
`1e-10`, and 128 members at `1`, `1e14` and `1e-10`. Each uses its actual
assembled operator, ordinary four-step refinement outcome, frozen eigenvalue
and a separate four-step banded initializer. Directions are canonicalized;
each order/backend receives an isolated fit, not a runtime fallback.

The previous 48-route comparison is retained unchanged: zero normalized
acceptances and six wide preparation faults. Diagnostics now include pivot,
column, component and both double-double parts. Zero-based indices:

| Input and backend | Pivot and column | Rejected high part | Interpretation |
| --- | --- | --- | --- |
| 100 tiny, rounded columns | 24 and 25 | -6.9086809435115515e-103 | Retained trailing-column fill-in |
| 100 tiny, wide columns | 24 and 24 | 5.714936956411375e-101 | Current-column tail, subsequently overwritten |
| 100 tiny, wide beam | 24 and 24 | 5.714936956411375e-101 | Same preparation as wide columns |
| 128 tiny, rounded columns | 93 and 96 | 5.387867848491882e-102 | Retained trailing-column fill-in |
| 128 tiny, wide columns | 93 and 96 | -2.1650734486040876e-102 | Retained trailing-column fill-in |
| 128 tiny, wide beam | 93 and 96 | -2.1650734486040876e-102 | Same preparation as wide columns |

All have finite parts and high magnitude below the existing `1e-100` nonzero
floor. Two are current-column cancellation tails; four affect columns that
would otherwise be retained. The latter cannot be justified away by resetting
the analytically eliminated target column. The guard remains unchanged.

## Independent Givens Construction

`modal_roundoff_grid_wide_givens.rs` stores deterministic row rotations and
shares scaled-column preflight and quantized/beam backsolve with the retained
Householder implementation. No precision backend is tried after a factor fault.

The target column alone receives an analytic diagonal and exact zero at its
eliminated row. Other columns and right-hand sides undergo the full rotation
and the original bounded-component check. No small nonzero fill-in is dropped,
no eigenvalue is changed and neither residual nor unit-norm tolerance is relaxed.
Binary scaling resolves each two-entry norm without squaring its tiny absolute
magnitude. Rank checks retain the `64 * epsilon * initial_column_norm` gate.

At 256 rows the isolated fit plan reserves 7,864,320 payload bytes and
335,544,320 component visits. Rotation storage reserves the dense maximum of
`n * (n - 1) / 2` entries, even when the observed matrix is sparse. There is one
factor start and no more than six proposal certificates per isolated route.
These are conservative static fit reservations, not measured wall time, process
RSS, allocator peak or a composed canonical/physical/published pipeline budget.
No canonical portfolio budget or production budget is extended by this work.

The four difficult inputs compare seven backends over three fixed orders,
84 routes in total. The first four backends retain the old 48-route boundary.
The additional three are rounded-column Givens, wide-column Givens and its
four-candidate beam. Each backend has zero normalized acceptances. Every wide
Householder or Givens backend has two preparation faults, but not on the same
inputs. All faults stop with one factor start and zero certificates.

All three Givens variants fail GridNorm preparation for the 128-member unit and
large inputs at pivot 108, row 232, with retained fill-in about `5.48e-113`.
They complete preparation for the former 100-tiny and 128-tiny faults but still
miss the unchanged `1e-8` normalized residual gate. This is failure relocation,
not six resolved requests, and there are zero physical publications here.

## Broader Material Comparison

The same previously inspected 36-input construction/regression set covers
UnequalLengths, NonbinaryGradient, ThreeLayers and deterministic Jittered
recipes; 80, 100 and 128 members; scales `1`, `1e14` and `1e-10`.
Each input is freshly rebuilt from a renumbered request, then each fixed
GridNorm/Reverse route independently fits the same initialized seed.

There are 144 isolated routes: 72 rounded greedy routes and 72 wide Givens beam
routes. Comparing them measures both factorization and search policy; it does
not isolate a pure Givens precision effect. The four-input seven-backend
comparison above provides that narrower construction diagnostic.

| Backend | Preparation faults | Numerical rejections | Normalized acceptances |
| --- | --- | --- | --- |
| Rounded greedy | 0 | 20 | 52 |
| Wide Givens beam | 5 | 17 | 50 |

Forty-seven fixed-order pairs accept under both; three gain normalized acceptance,
five lose it and seventeen fail under both. There is no automatic union policy.
Each input/order outcome is pinned explicitly, not only aggregate counts.

The three gains are Reverse order for NonbinaryGradient 128 unit and ThreeLayers
100 unit/large. The five losses are GridNorm for UnequalLengths 128 large/tiny
and NonbinaryGradient 128 large, plus Reverse for NonbinaryGradient 128 tiny
and Jittered 80 large.
Some losses are numerical rather than factor faults; the stage is retained.

All five guarded Givens failures are:

| Input and order | Pivot and row | Rejected nonzero magnitude |
| --- | --- | --- |
| UnequalLengths 128 large, GridNorm | 114 and 244 | 8.283426703921657e-102 |
| NonbinaryGradient 128 large, GridNorm | 112 and 225 | 9.113468952828274e-107 |
| ThreeLayers 128 unit, GridNorm | 108 and 232 | 5.482408641284931e-113 |
| ThreeLayers 128 large, GridNorm | 108 and 232 | 5.482467950078972e-113 |
| Jittered 80 large, Reverse | 58 and 133 | 5.1149516739889786e-101 |

The ThreeLayers 128 unit/large/tiny inputs remain unresolved in both fixed
orders. New factorization alone is insufficient within these bounded searches.
This previously inspected dataset is not a new independent unseen holdout.
No physical-fit, unit-shape publication, restored result JSON or SDK research
output is produced by the new Givens campaign. Its internal vectors must not be
counted as the earlier cold chart pipeline's 31 accepted results per policy.

## Failure Isolation and Replay

Analytic controls cover 18 known integer solves and 18 beam solves checked
against all 81 integer pairs in a two-dimensional bounded grid. Row permutations,
sign reversal and binary scales are included. Actual rotated nonzero fill-in
below the old range floor remains rejected. These are finite arithmetic
cross-checks, not interval proofs or physical solver qualification. Unresolved
double-double rotation scales return errors before division, and binary scaling
may not discard a nonzero entry.

Synthetic GridFit controls inject six operator faults and thirty malformed
receipts across all six certificate positions. Two final-certificate loss cases
remain rejections. Six cancellation sites and their healthy replays preserve
the frozen anchor and return no partial candidate. An initial test used a
progress value that chunked preparation never emits; that test hook was corrected
to the actual terminal three-component checkpoint, not bypassed.

Real Jittered 128-unit GridNorm controls retain exact sign/binary-amplitude
candidate bits and one fresh request renumbering replay. Five cancellations
cover preparation, factorization, substitution, final certificate search and
final validation. All return errors before acceptance and have five fresh exact
healthy replays. Every one of the five real factor faults has an exact failed
renumbered replay followed by a fresh healthy computation. Inputs, original
refinement seeds and eigenvalues remain unchanged; no partial candidate is adopted.

## Reproduction and Qualification

Run from `workers/rust`, using the cached locked toolchain:

```text
cargo test -p kyuubiki-solver --lib --release givens --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release material_construction --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-solver --lib modal_material_construction_givens_real --locked --offline -- --nocapture
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

Final local verification: Solver library release tests pass 705, fail zero and
leave 34 explicitly ignored benchmarks unexecuted. The debug real-material
fault/cancellation/sign/renumbering control passes independently. Solver, CLI
and native script-runner all-target Clippy passes with warnings denied;
workspace formatting and whitespace checks pass. Tensor tests pass all 21.

Eight native structure/self-test checks pass: the tensor and its self-test,
operator-validation registry and its self-test, documentation inventory, HTML
book, project organization and its self-test. The registry checks 59 profiles
with `executed=false`, not every profile's numerical campaign. The book checks
26 HTML files; existing manifests remain 3.4.5. Organization reports zero
tracked debt at the 800-source/2000-document line limits. The tensor retains
zero structural gaps, four maturity gaps, nineteen evidence-grade gaps and
fourteen P0 gaps; its Daji readiness assessment remains blocked.

The implementation is reachable only through Solver's test-only ancestry.
Engine, Agent, public Solver ordering/admission and retry semantics are unchanged.
The eight scoped production qualification obligations remain open. This work
does not claim arbitrary topology, multimode recovery, a full-pipeline budget,
remote/installed/GUI/Headless qualification, a benchmark or production recovery.

The next useful numerical work is bounded construction that handles the retained
quantization and representability boundaries without losing existing successes.
Retain both failed factor contexts and the old successful physical publications;
require fresh physical/readback evidence before admitting a candidate.
