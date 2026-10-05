# Banded floating grid construction and retained modal failures

October 4, 2026. Local macOS ARM64 research follows the
[wide Givens comparison](modal-wide-givens-range-reliability-20261004.md).
The source package metadata remains Daji 3.4.5; HEAD was `dc5647af`
(`daji 3.4.6`) when this campaign began. No version, package, install,
Engine authority, Agent execution policy or public Solver admission changes.

This is test-only banded-grid construction evidence, not production modal admission.
The candidate reduces the difficult inputs' residuals but does not recover
their normalized directions. It also rejects all six previously recovered
128-member baseline inputs. It must not replace the retained QR/chart policies.

## Construction and acceptance

The candidate uses the natural structural order of a horizontal chain with
at most 256 active bending coordinates. The shifted Wide normalized matrix
must have bandwidth at most three. Out-of-band nonzero entries, including
tiny entries, are rejected rather than discarded. This is not arbitrary
topology admission or a canonical permutation of a dense matrix.

Four existing bounded Wide inverse iterations initialize each fresh request.
The original root and seed bits are retained. Each free component considers
its nearest f64 value and the adjacent values toward and away from zero.
The original dominant anchor remains frozen at its nearest value.
Unsupported arithmetic ranges decline before operator certification.

A row's squared Wide residual is scored once its seven-coordinate support is
assigned. States with the same last six choice keys have identical possible
future row contributions, so only the lowest computed prefix score is kept.
There are at most `3^6 = 729` frontier states. Backpointers reconstruct one
proposal; full vectors are not copied for every branch. The final three rows
are explicitly scored after the last coordinate, including small matrices.

Two separate cold policies are compared: one pass, or four preselected passes.
Four passes center each new finite box on the previous proposal, without using
operator receipts, changing the root or unfreezing the anchor. Every pass keeps
the previous center among its choices and checks nonincreasing computed score.
This is not a search across a global four-ULP box, a runtime retry portfolio,
an interval proof or a proof of global representability under the residual gate.

The objective is proposal-only. Acceptance requires the original operator's
unchanged `1e-8` normalized residual gate, a matching finite residual vector,
the unchanged direction norm range `0.25..=2`, and two fresh passing receipts.
An initial rejection stops after one receipt. A later fault, malformed receipt,
lost residual gate or cancellation discards the candidate without another fit.
No physical-unit-shape correction, result publication or JSON readback is added.

## Retained budgets

The extra payload bounds exclude the borrowed dense matrix, initializer,
operator storage, allocator overhead and complete research pipeline.
They are payload reservations, not process RSS or a production work budget.
Plans reject sizes outside `2..=256` and pass counts outside `1..=4` before
construction. Actual trial, term and created-node counts are checked against
both the per-pass plan and the cumulative preselected plan.

| Bound at 256 coordinates | One pass | Four passes |
| --- | ---: | ---: |
| Extra payload bytes on this host | 3,141,536 | 3,147,680 |
| Candidate branch trials | 559,872 | 2,239,488 |
| Scored matrix terms | 3,934,413 | 15,737,652 |
| Total created backpointer nodes | 186,624 | 746,496 |
| Peak frontier states | 729 | 729 |
| Dense QR starts | 0 | 0 |
| Actual certificate cap | 2 | 2 |

Backpointer storage is freed between passes; the four-pass node count is total
creation, not simultaneous node residency. Prior shape and Wide center storage
are included in the four-pass payload reservation. Both bounds remain below
4 MiB; this comparison makes no end-to-end timing or throughput claim.
Scored-term counts exclude the size-bounded input scans and control checkpoints.

## Difficult input results

All four diagnostic inputs retain their failed ordinary four-step refinement.
The four-pass continuous Wide direction was already below `1e-13`, whereas
its nearest f64 direction failed actual certification. Neither new grid policy
uses a reference direction, shifts the root or relaxes the gate.

| Three-layer input | Nearest f64 actual residual | One-pass actual residual | Four-pass actual residual |
| --- | ---: | ---: | ---: |
| 100 members, scale `1e-10` | 1.0148166789631285e-7 | 8.707354496166359e-8 | 7.64395208072248e-8 |
| 128 members, scale `1` | 2.6950291105613276e-7 | 2.1365210869189027e-7 | 1.9140196796503781e-7 |
| 128 members, scale `1e14` | 2.88553774083644e-7 | 2.1286092652800957e-7 | 1.9200497996778793e-7 |
| 128 members, scale `1e-10` | 2.8771005220908543e-7 | 2.145184708351204e-7 | 1.912629667125953e-7 |

All eight cold diagnostic routes reject after one actual receipt and zero
dense factors. Continuous prediction on the proposed f64 direction closely
matches actual residuals here; the diagnostic output retains both numbers.
The result does not establish that no representable passing direction exists
outside these bounded neighborhoods or under another construction policy.

## Broader retained boundaries

The same 36 recipe/mesh holdouts are reused, not relabeled as unseen validation:
four recipes, 80/100/128 members and coordinate scales `1`, `1e14`, `1e-10`.
Six original 128-member graded/layered baselines are added. Together they form
42 distinct inputs and 84 cold one/four-pass routes; the eight difficult routes
above overlap this campaign and do not increase unique coverage.

Each policy accepts only nine of the 36 holdout normalized directions:
UnequalLengths, NonbinaryGradient and Jittered at 80 members, all three scales.
ThreeLayers at 80 members and every 100/128-member case reject. Paired counts
are 27 common rejections and nine common acceptances, with zero one/four-pass
acceptance gains or losses. Both policies reject all six original baselines.
Every input's expected boundary is asserted, not just the aggregate counts.

The accepted 18 routes use two receipts each; 66 rejected routes use one each,
for 102 actual certificates over the 84 routes. Root/seed bits and input JSON
remain unchanged. These normalized-only successes are not comparable to the
older 31-of-36 completed physical/readback routes as equivalent result outputs.
Independent physical publications and readbacks for this candidate are zero.

## Controls and replay

Small sizes 2..8 are compared with exhaustive enumeration of all nonanchor
three-choice combinations, including the final-row objective. Twenty-one
sign/binary-amplitude comparisons preserve exact proposal bits. Twenty-one
known constant null directions with binary matrix scaling are reconstructed
without rounding retained matrix entries away.

Synthetic controls retain two callback faults, ten malformed receipts,
one lost final receipt, eight cancellation checkpoints and exact healthy replay.
Eleven malformed input cases cover size, shape, range, anchor, nonfinite values,
retained out-of-band fill and scaling loss. A separate second-pass cancellation
confirms that an earlier constructed proposal does not leak on later failure.

The real failing 128-member unit-scale case retains three sign/binary-amplitude
bit comparisons, six cancellation checkpoints and six fresh changed-numbering
replays. The successful 80-member unequal-length case separately covers final
operator fault, malformed final receipt, final gate loss, cancellation before
the second certificate and cancellation after both receipts, with fresh
changed-numbering replay. No cancelled/faulted route returns a partial candidate.

## Verification

The final focused release run passes all nine new tests. The complete Solver
release library run passes 714 tests with zero failures and 34 ignored
specialist benchmark cases, in 129.45 seconds. The real final-fault/cancellation
and fresh-request replay test also passes in the debug build (12.02 seconds).
These timings record test execution, not performance comparisons.

All-target Clippy with warnings denied passes for Solver, CLI and script-runner.
All 21 tensor tests pass. Eight native registry/document/organization checks,
including self-tests, pass, as do formatting and diff checks. The operator
registry still reports `executed=false` for its 59 profiles; checking that
registry is not a numerical execution claim for every profile. Tensor status
retains zero structural gaps, four maturity gaps, nineteen evidence-grade gaps
and fourteen P0 gaps. No claim is added or promoted to production qualification.
Source/doc caps remain 800/2000 lines with zero tracked organization debt.

## Reproduction

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release banded_grid --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-solver --lib modal_material_banded_grid_real_final --locked --offline -- --nocapture
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

The production qualification scopes remain open. Local test controls and
registry checks do not establish remote, Linux, Windows, installed GUI,
Agent/Headless execution, full work budgets, multimode or external correlation.
The next construction must retain old successful boundaries as well as recover
the difficult inputs; merely increasing local passes is not justified by this run.
