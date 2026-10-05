# Modal candidate holdout and failure stage validation

October 4, 2026. Local macOS ARM64 source overlay on `dc5647af`
(`daji 3.4.6`); Cargo and shipping/development manifests remain 3.4.5.
Rust 1.88.0. This follows the
[single-fit inward chart campaign](modal-material-inward-chart-reliability-20261004.md).

This is test-only holdout validation, not production modal admission.
Thirty-six parameter inputs are compared through 72 independently rebuilt cold
original/inward-chart routes. Each policy publishes 31 independently checked
results; both leave the same five inputs rejected. The inward chart improves
16 accepted residuals and worsens 15. It is not selected as a universal default.

## Input design and scope

The candidate policies are frozen before this campaign. Four recipes span
80, 100 and 128 members and coordinate scales 1, 1e14 and 1e-10:

| Recipe | Numeric properties |
| --- | --- |
| Unequal lengths | Existing periodic length/material/section fixture, previously outside the inward-chart design set |
| Nonbinary gradient | New smooth Young/area/inertia/density gradients, including Young factor 1.7 and density factor 2.3 |
| Three layers | New one-third boundaries with Young factors 1, 3.7, 11; distinct areas, inertias and densities |
| Jittered | New bounded per-member length, Young, area, inertia and density variation from a fixed LCG sequence |

The jitter sequence begins at `0x59d2_c391_728b_04a7` and uses the existing
deterministic multiplier 6364136223846793005 with increment 1. All values and
recipes are generated from source, not retained large input files. The new
recipes are not changed to remove observed failures. They are holdouts relative
to the earlier candidate design inputs, not randomized statistical evidence,
unseen experimental materials or a general reliability probability.

All inputs remain horizontal forward chains with restrained axial motion, a
restrained left bending end and one requested mode. The private reconstruction
harness now has a separate bounded 1..=128-member entry. The historical entry
still requires its original 128-member scope, and its old rejection tests remain
unchanged. General public topology, arbitrary orientation and multimode are not
admitted. This does not modify production `NodeOrder` or Engine/Agent policy.

## Independent cold routes

Every chart route receives a new key-7 request reconstruction, fresh stiffness
and mass assembly, Jacobi seed and ordinary four-step refinement. A fallible
preparation entry records whether those four steps converged or exhausted their
unchanged residual gate. It does not assert that every holdout needed recovery.
Unexpected preparation errors stop before subsequent stages.

Eleven inputs already converge in ordinary four-step normalized refinement;
25 do not. Both cold candidate routes publish 20 of those 25 and all eleven
ordinary-refinement controls. This is not proof that the full public Solver
would reject the 25 inputs: its later production recovery path is not the
ordinary four-step preparation used in this comparison.

Both candidate routes use four bounded wide banded initializer steps followed
by the same ranked normalized grid stage. The actual checked internal order
selects one independently prepared physical fit. Original and inward charts
are separate cold routes, not successive live retries. Frozen roots, normalized
request seeds and accepted internal tags match exactly between the paired
routes. Original requests stay unchanged.

After a physical candidate passes <= 1e-8 residual and < 1e-10 unit-norm error,
a separate wide LDL inertia bracket cross-checks the first root. Its width/root
must remain below 1e-14 and midpoint/root error below 1e-12. This finite arithmetic
is not an interval enclosure or formal proof. Complete output restores original
numbering, checks the exact free-degree set and restrained positive-zero values,
then undergoes independent stiffness/mass reconstruction from serialized JSON.

## Observed outcomes

| Stopping stage | Original chart inputs | Inward chart inputs |
| --- | ---: | ---: |
| Request/spectrum preparation | 0 | 0 |
| Banded initialization | 0 | 0 |
| Normalized internal proposal | 4 | 4 |
| Physical proposal | 1 | 1 |
| Inertia cross-check | 0 | 0 |
| Original-numbered readback | 0 | 0 |
| Fully accepted and independently read back | 31 | 31 |

All 31 successful input pairs pass both policies, producing 62 checked outputs.
There are no inward-only or original-only successes in this set. Of the
accepted pairs, 16 have a lower inward-chart independent residual and 15 a
higher one, using a 1e-12 relative comparison tolerance to distinguish equality.
The maximum inward/original residual ratio is 1.5778339948589295. That largest
regression is jittered 80-member geometry at scale 1e-10: about 3.17577e-9
becomes 5.01084e-9. Both pass, but the loss of margin remains recorded.

The five rejected inputs are:

| Recipe and member count | Scale | Boundary |
| --- | ---: | --- |
| Three layers, 100 | 1e-10 | Internal ranked grid exhausted two fits/eight certificates |
| Three layers, 128 | 1 | Same internal boundary |
| Three layers, 128 | 1e14 | Same internal boundary |
| Three layers, 128 | 1e-10 | Same internal boundary |
| Unequal lengths, 128 | 1e-10 | One physical fit/four certificates remains above the strict gate |

The unequal-length physical best residual is 1.1307887396884874e-8 under the
original chart and 1.1307932933514552e-8 under inward movement. Neither produces
a result. The four internal failures never start physical fitting or readback.
Trying to improve the physical anchor cannot repair a failure before that stage.
No reference-only existence direction is supplied for these new failures.

## Work receipts and recovery boundaries

Actual QR factor start events are counted independently for each stage, including
failed attempts. Certificate counts come directly from actual operator callbacks.
A failed internal fit therefore reports two factors, not an incorrect zero
inferred from the absence of an accepted usage receipt. Physical callbacks and
fits cannot be attributed to an internal early exit.

The unchanged candidate caps remain two fits/thirteen certificates internally
and one fit/seven certificates physically. These are not whole-pipeline work,
allocation, elapsed-time or installed-runtime budgets. No ignored benchmark
campaign is rerun, and no performance qualification is advanced.

Four representative stage rejections cover both policies on three-layer and
unequal-length 128-member tiny inputs. Key-113 fresh rebuilds reproduce their
roots, seeds, stopping stage, factors, checks and errors exactly. Each rejected
case is followed by a healthy nonbinary 80-member task whose mapped result
replays exactly; rejected tasks return no output or readback receipt.

Four late cancellations cover both policies at the last original-node restoration
and the final publication boundary after independent checking. Complete output
is stored only after that final cancellation checkpoint. No partial result is
published, and four fresh healthy rebuilds reproduce exact mapped outputs.

Eight additional preparation declines cover an over-cap 129-member chain,
a second requested mode, zero Young modulus and opposite orientation of one
member under both policies. The size, multimode and orientation examples are
prototype-unsupported, not universally invalid public requests. All eight stop
with zero roots/seeds, factors and certificates; eight fresh healthy replays pass.

## Next numerical obligations

The next candidate-construction priority is the normalized internal stage on
the four three-layer failures, followed by the unequal-length physical failure.
Preserve these requests and stopping stages while investigating alternative
bounded directions or chart selection. Do not increase production retries,
silently widen tolerance or pick a policy from recipe identity.

The inward chart is complementary research evidence, not a complete replacement
for the original chart. Its earlier two material recoveries remain valid; the
new common failures and mixed margins rule out a universal improvement claim.
Broader topology/multimode, cumulative budgets and actual Agent/official Headless
qualification stay open. This evidence updates the existing validation claim,
not a qualified production, benchmark or installed release scope.

## Verification

Final checks use the current source overlay, including the shared validated
request reconstruction and per-input stopping-stage regression assertions:

| Check | Observed result |
| --- | --- |
| Optimized Solver library | 694 passed, 0 failed, 34 ignored; 133.09 s test time |
| Optimized independent holdout suite | 3 passed, 0 failed; 28.08 s test time |
| Debug holdout preparation/rejection/cancellation/replay controls | 2 passed, 0 failed; 90.19 s test time |
| Coverage tensor regression suite | 21 passed, 0 failed |
| Solver, CLI and native tooling all-target Clippy | Passed with warnings denied |
| Workspace formatting and diff whitespace | Passed |
| Native tensor check and self-test | Passed; production qualification remains unchanged |
| Operator validation registry and self-test | Passed; 59 profiles, executed=false |
| Documentation inventory and HTML book checks | Passed; 26 HTML files, development/shipping metadata 3.4.5 |
| Project organization audit and self-test | Passed; source limit 800, document limit 2000, tracked debt 0 |

The 34 ignored Solver tests are not executed. Registry validation does not run
the 59 complete operator campaigns. Test times are observed regression-suite
durations, not isolated benchmark measurements or throughput claims. Native
tensor structure has zero gaps; its four maturity and nineteen evidence-grade
gaps, including fourteen P0 gaps, remain explicit with Daji readiness blocked.

Reproduce the numerical and tensor checks from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release holdout --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib inward_chart_holdout --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
```

No remote, live Agent, official Headless study, GUI, ignored benchmark, packaging
or installed-application campaign runs in this follow-up. No commit, push or
version change is made.

## Subsequent construction campaign

The [internal-chart follow-up](modal-internal-chart-construction-reliability-20261004.md)
uses these retained inputs for further construction, without changing this
campaign's original policy outcomes. It adds tests to the same ancestry;
the verification counts above record this earlier run, not the later expanded
test inventory. See the follow-up for its separate recovery/regression results.
