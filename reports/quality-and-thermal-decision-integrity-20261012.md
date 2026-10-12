# Quality admission and thermal decision recovery

Date: 2026-10-12. Current line: daji 3.5.0. Source macOS working-tree overlay
at `69da83e9`. No version bump, commit, installed migration or app rebuild.

## Defects and changes

The service domain scorers still filtered unknown/invalid selected terms,
accepted repeated terms, defaulted malformed numeric overrides, clamped negative
weights to zero and imposed an undocumented target floor. They did not check
convergence. The display-oriented metric resolver also turned damaged selected
evidence into missing data. These paths could change scoring intent, hide the
original error or raise arithmetic exceptions instead of returning a diagnosis.

Thermal decisions had only narrow missing-metric validation. The initial
decision-matrix run reproduced acceptance of explicit nonconvergence; three
other early failures identified coarse error atoms without the expected field
context. The matrix also covers conflicting threshold/value settings, malformed
labels, invalid weights/goals, label collisions and arithmetic overflow.

`WorkflowDomainQualityRuntime` now uses the existing checked metric and reporting
helpers across structural, thermal, acoustic, modal, dynamic, electrostatic,
magnetostatic, CFD and transport scoring:

- Present selections must be nonempty, known and unique after trimming. They
  retain caller order; invalid entries cannot silently change the score terms.
- Every declared target/weight override is validated, including supported but
  unselected metrics. Targets are strictly positive; weights and the ready limit
  are nonnegative. Unknown override keys and malformed objects reject.
- Explicit false/malformed convergence markers reject before scoring. Selected
  corrupt metrics, invalid aliases and incomplete derived samples preserve errors.
- Truly absent metrics retain the native blocking-assessment contract. They are
  not fabricated successful zeros or errors that prevent explaining missing data.
- Zero weights still validate evidence but skip an unused ratio. Positive tiny
  targets are used as declared. Ratio, penalty and score-total overflow return
  contextual errors instead of escaping as arithmetic exceptions.
- Valid scoring formulas and grade boundaries remain unchanged. Ordinary equal
  dominant penalties select the last term, matching the native rule. Unselected
  display mirrors remain best-effort, as in the native scorer.

Thermal guards and coupled-heat comparisons now use `WorkflowDecisionChecks`,
already shared by the other eight domain families. They validate the parent and
both candidate convergence markers, every rule/criterion, explicit choices,
threshold aliases, positive pair weights and distinct nonreserved labels before
evaluation. Normal thermal tie scoring still awards half the weight to each side.
The implementation no longer filters/drops invalid criteria or substitutes values.

Thermal error results now contain operator, rule/criterion index and selected
field paths rather than `missing_thermal_guard_metric` or
`missing_coupled_heat_benchmark_metric` atoms. Existing negative controls were
updated to require those paths; they were not removed or changed to expect success.
The catalog control checks its actual selected `thermal_peak_flux_magnitude`
field. The native-catalog evidence anchors now follow the contextual errors.

## Verification

New controls comprise 74 domain-quality tests and 18 quality graph tests.
Extending the shared decision matrix adds ten thermal tests; extending its
asynchronous API recovery suite adds four tests. Total increment: 106 tests.

The quality graphs execute 162 controlled runs: 72 rejected inputs, 72 corrected
fresh executions and 18 genuinely missing-metric assessments. Both `fail` and
`skip` policies prevent invalid scores reaching objective/export callbacks or
publishing artifacts. Corrected runs complete the five-node chain. Missing-metric
reports may continue to explain the failure, but exported composite readiness
remains false with grade `block` and the missing count preserved.

Four thermal API tests add 16 actual asynchronous service jobs. The complete
nine-domain decision suite now has 36 tests executing 144 rejected/corrected
jobs, with invalid aliases, incomplete samples, missing metrics and nonconvergence.
Fresh successful jobs do not rewrite the retained earlier failures. These jobs
use controlled summaries, not actual physical solver execution.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 599985
```

Result: 1,764 total, 1,693 passed, 71 conditional skips, zero failures. A focused
quality/thermal/API selection passes 233 tests. Skipped environments are not
execution evidence. The new graph fixture initially omitted the runner's required
extract callback; it now explicitly forbids extraction rather than weakening
runner validation. Fixture setup failures are not production defect counts.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib workflow_domain_quality_validation --quiet
```

The existing 22 native quality-validation tests pass unchanged. They supply
reference contract controls, not new native runtime/recovery qualification.

## Limits and next work

This is service admission and explicit fresh-run recovery evidence. It is
not automatic durable continuation, a live Agent run, installed/remote
acceptance, numerical qualification or exhaustive cross-language equivalence.
No physical algorithms, TaskIR format or Engine execution authority changed.

Composite-objective and candidate-ranking admission remain separate audit work.
The controlled graphs test valid downstream use, not malformed externally supplied
quality reports, mixed-branch missing inputs or ranking configuration. Candidate
filtering and defaulting in those entrypoints must not be inferred safe from this
upstream repair. The tensor receives only control-plane contract/execution/
recovery evidence; overall maturity and release qualification remain unchanged.

Follow-up: the [objective/ranking admission round](quality-objective-ranking-recovery-20261012.md)
now covers those independently supplied summaries, explicit partial rankings and
next-round stopping gates. Sweep and execution-batch admission remain separate;
this earlier report's test results and native scope are not retroactively expanded.
