# Normalized Modal Recovery And Physical Publication

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88.

## Scope And Decision

This is a test-only two-stage candidate comparison, not production normalized recovery or public solver admission.

The [heterogeneous boundary comparison](modal-grid-heterogeneous-reliability-20261004.md)
retained six inputs rejected before physical-shape preparation. This follow-on
captures their actual best dense-refinement pairs and qualifies a separate
internal-direction contract. Five fixtures now have a candidate chain that
passes both internal and independently reassembled physical residuals. One
large-coordinate graded fixture still rejects. The original public solver
continues rejecting all six; no runtime fallback has been enabled.

No production algorithm, residual/unit-norm gate, retry budget, dependency,
public API, file format, version or installed application is changed. New
candidate code remains under the existing `cfg(test)` portfolio registration.
Earlier reports retain their historical scope and are not relabeled as new
public-runtime successes.

## Internal And Published Contracts

The six fixtures are 128-element horizontal, root-clamped graded or layered
beams at coordinate scales `1`, `1e14`, `1e-10`, using the same validated
requests, rounded element assembly and synthetic parameter scaling as the
previous report. They are not six experimental material datasets.

Production Jacobi supplies the initial pairs. The existing four-step dense
refinement is run with its test-only no-roundoff budget entry, which retains
the real best pair when that budget fails. The retained eigenvalue is checked
against the existing residual-bounded neighborhood of the original root, then
frozen for all comparisons. No wide-reference root is used as an eigenvalue or
as a new seed, and the internal vector is not normalized again before search.

Internal vectors have norms near `0.5` or `1` after exact binary scaling; they
are not final unit shapes. The distinct canonical-direction entry admits only
finite resolved coordinates and norms in `[0.25,2]`, supporting the sampled
binary-scale controls. It uses unit weights because its coordinates are already
mass-normalized. The physical entry retains positive physical masses and the
unchanged strict unit-shape contract. Shared preflight, ordering, budgets and
original-operator callbacks avoid duplicating the search engine or silently
relaxing the published-shape gate.

Canonical directions are wide products of the actual rounded physical stiffness
and the production inverse square-root masses, minus the frozen eigenvalue on
the diagonal. These approximate columns only propose corrections. Every
certificate comes from the actual production compensated operator, restored
to the original coordinates. Unique signatures and exact frozen-anchor bits
remain mandatory; ambiguous keys do not receive index-based tie breaking.

## Root And Rejected Proposal Cross Checks

A separate wide banded LDL inertia method brackets the first root in a fixed
relative `1e-6` neighborhood, then performs 32 bisections. It checks dimensions
up to 256, finite bounded entries, bandwidth three, approximate wide symmetry,
nonzero bounded pivots and coefficients, and endpoint inertia counts zero/one.
Its band factors have linear storage. An analytic two-coordinate eigenproblem
and malformed/range/singularity controls check the reference independently of
the beam candidate outcomes.

The final bracket width is below `1e-14` relative; the retained root differs
from its midpoint by about one f64 ulp in all six fixtures, with a retained
regression bound of `1e-12`. This is a cross-check on the supplied rounded
system, not independent assembly, interval arithmetic, or a proof of every
spectral index. It supports investigating representation/search error rather
than altering the root to make a residual test pass.

All six fixtures still reject under the retained alternative controls:

- Production automatic Gram block recovery, using 26 or 34 actual checks.
- Automatic local QR and fixed-anchor full QR block recovery.
- Fixed-anchor full wide projection with its rounding neighborhood.
- Raw reverse-order grid from the original retained pair or the best Gram
  measurement; only one best vector is retained, not a history of snapshots.
- Thirty-two adjacent scalar-amplitude probes of that best Gram vector.

These are separate diagnostic runs, not an unreported combined runtime retry
loop. Their frozen negative outcomes prevent a factorization swap or more
amplitude probes being mistaken for a solution to the six-case boundary.

## Two Stage Candidate Results

Four joint numberings (identity, reversed, shuffle 7 and 113) and both signs
give 48 internal candidate searches over six fixed assembled inputs. Forty
accept and eight reject. Each accepted internal vector then passes a separate
physical candidate stage, strict unit norm and independent stiffness/mass
reassembly after constructed result JSON readback. Sign-restored internal and
physical candidate bits match across all eight transformations per fixture.

| Profile | Scale | Retained seed residual | Internal candidate residual | Independent physical JSON residual | Internal factors/checks | Physical factors/checks |
| --- | --- | --- | --- | --- | --- | --- |
| graded | 1e0 | 5.477783e-8 | 2.667466e-9 | 7.705861e-9 | 3 / 11 | 1 / 4 |
| graded | 1e14 | 5.144329e-8 | rejected | not executed | bounded rejection | not executed |
| graded | 1e-10 | 5.454794e-8 | 3.771156e-9 | 7.078472e-9 | 3 / 11 | 1 / 4 |
| layered | 1e0 | 4.806591e-8 | 5.286986e-9 | 8.333676e-9 | 3 / 11 | 1 / 4 |
| layered | 1e14 | 5.215958e-8 | 7.471159e-9 | 8.981978e-9 | 1 / 4 | 1 / 4 |
| layered | 1e-10 | 5.649368e-8 | 8.675553e-9 | 8.589527e-9 | 1 / 4 | 1 / 4 |

The physical stage expands the accepted internal vector with the existing
shape routine, uses the ordinary production shape proposal as its starting
seed, then applies the alternative canonical unit-shape entry instead of the
production Gram roundoff fallback. All five accepted fixture families use one
reverse-order physical factor and four actual grid certificates. The original
production publication path is also tried separately on each identity positive
candidate and still rejects all five, with residuals from about `1.58e-8` to
`5.48e-8`. This demonstrates why internal success alone is not a runtime fix.

Twelve separate controls scale the same retained seeds by exact factors `0.5`
and `2`: ten complete both candidate stages, two reject internally. Internal
candidates recover their original bits after dividing out the binary factor;
published shapes match the corresponding baseline bits without that division.
These controls and cancellation replays are not extra physical fixtures or
additive public-API coverage. The eight rejected main searches never execute a
physical stage and are not assigned a successful output.

## Budgets And Recovery

Each candidate entry retains at most three sequential factors and nineteen
actual certificates, starting every ordering from the same immutable seed.
The existing canonical 256-coordinate plan reserves 6,914,048 payload bytes
and 1,009,090,560 modeled component visits per stage. Norm restoration, frozen
anchors and actual used reservations are checked in both coordinate domains.
Operator faults, malformed receipts, cancellation and a failed fresh final
certificate stop the entry; they do not become ordering retries.

The two grid stages therefore have a combined cap of six sequential factors
and 38 grid certificates, with summed modeled grid work 2,018,181,120 visits.
Successful sampled chains actually use 15 or eight grid checks. These counts
exclude the ordinary shape proposal, direction assembly, diagnostic production
comparisons and independent reference/readback work. A per-stage payload model
does not prove whole-pipeline peak memory: caller fixtures and both direction
sets can coexist in the test harness. No new runtime/RSS benchmark is claimed.
This alternative is not appended to the existing production 80-check/350-million
work budget. Production integration still needs an explicit complete cost and
allocation review rather than hidden extra retries.

A real graded-beam internal correction is cancelled at the final restored-vector
checkpoint after eleven certificates; no candidate is returned. A fresh entry
reproduces the baseline bits and then completes independent physical readback.
Synthetic control callbacks separately cover malformed certificates, original
operator faults, fresh-final-certificate failure and late cancellation/replay.
They are safety controls, not numerical research results or live Agent claims.

## Verification And Remaining Work

Replay from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib triangular_grid_normalized_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release triangular_grid_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

| Check | Result |
| --- | --- |
| Full Solver regression | 1,476 passed, 0 failed, 10 ignored; 186 result groups |
| Optimized complete triangular-grid replay | 31 passed, 0 failed, 1 cost test ignored |
| Affected normalized debug replay | 5 passed, 0 failed; subsequent malformed-wide-part guard also replayed separately |
| Final-source normalized optimized replay | 5 passed, 0 failed, no ignored tests |
| Final Solver/CLI strict all-target Clippy | Passed with `-D warnings` |
| Formatting, whitespace and organization | Passed; source <=800, docs <=2,000, tracked debt 0 |
| Profile/tensor validators and self-tests | Passed; 59 profiles, registry-only `executed=false` |
| Documentation book/inventory | Passed; 26 HTML files, development/shipping 3.4.5 |

The full regression completed before the final preflight/reference-input
hardening. Affected tests were replayed after those edits; the final wide-part
guard has separate debug coverage and all five normalized tests pass optimized
on the final source. Overlapping replays are not additive numerical coverage.
The ten full-suite ignores include the pre-existing explicit cost test; no new
benchmark was executed.

Scoped evidence is registered in the modal validation profile and architecture
tensor without promoting production maturity or release qualification. The
tensor retains 13 modules, 11 paradigms, no structural gaps, four maturity gaps,
16 evidence-grade gaps and 11 P0 gaps; qualification remains blocked.
Registry-only validation is not a numerical qualification run. This is local
macOS execution, not a Linux/Windows, Headless transport, installed application
or new performance result.

Next priorities are the remaining large-scale graded internal rejection,
explicit complete-pipeline cost qualification, and reviewed production admission.
Independently reassembled numbering, arbitrary geometry and clustered multimode
membership/mass orthogonality remain open. Gates and eigenvalues must stay
unchanged while qualifying those paths.

The [fresh pipeline cost and rejected alternatives follow-on](modal-grid-normalized-pipeline-cost-20261004.md)
now records independent order/radius controls and complete local candidate
timing/RSS, including preparation and readback. The statements above describe
this preceding numerical packet; the follow-on does not enable public recovery.
