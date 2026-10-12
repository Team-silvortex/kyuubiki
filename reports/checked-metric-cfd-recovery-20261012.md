# Checked metric derivation and CFD diagnostic recovery

Date: 2026-10-12. Current line: daji 3.5.0. Source macOS tests, working-tree
overlay at `69da83e9`; no installed runtime, database, package or version change.

## Confirmed defects

The preceding decision admission round checked canonical metrics, but the
Elixir resolver still skipped corrupt earlier aliases and incomplete derived
sample rows. A present invalid value could therefore become a favorable later
alias or a reduction over only the surviving samples. Before this repair,
focused tests reproduced canonical-null display fallback, earlier-alias guard
acceptance, and invalid dynamic-alias guard acceptance.

The legacy CFD reducer also admitted incomplete rows and fabricated zero
diagnostics. Its means resolved metric aliases, while its peaks read canonical
fields only, so the same evidence could produce inconsistent statistics.
Extreme finite inputs could overflow an intermediate sum or squared vector norm.
These are result-integrity defects, not demonstrated security exploits.

## Repair

`WorkflowDomainMetricResolver.checked_metric_value/3` distinguishes absent data
from corrupt selected data. The eight domain guard/pair families propagate the
checked error, including the actual source field and collection index. Display
helpers return no value on corruption, rather than selecting a healthier alias.

- Canonical fields take precedence; absent fields may use declared aliases, but
  a present invalid canonical or earlier alias must reject.
- Selected frequency, mode and transient reductions require every consumed row
  and metric. Present empty frequency collections cannot fall through to nodes.
- Valid zeros remain data. Unused later aliases or unrelated fields do not veto
  a valid earlier metric. Numeric strings are not silently coerced.
- Derived bounds and stable vector norms reject nonrepresentable results.
  Equal frequency-response peaks select the last sample, matching native Rust.
- Collection extrema use a single reduction without filtered/sorted copies.

CFD diagnostics now require nonempty node and element collections, complete
consumed metrics and valid convergence markers. Peaks, means and totals use
the same strict metric resolution, including aliases and velocity components.
One pass per collection accumulates the fixed metric set. Weighted means avoid
overflowing an unused sum; required dissipation totals and spans fail explicitly
if unrepresentable. Velocity and pressure span fields are now emitted alongside
the existing bounds. Equal absolute peaks use the native last-sample rule.

This remains service-side contract/reduction logic, not a second physical solver.
Existing native Engine implementations remain the numerical execution boundary.

## Verification

The follow-up adds 52 tests: 26 resolver/decision controls, 10 CFD diagnostic
controls and 16 additional asynchronous API controls. The focused three-file
selection contains 68 tests because it also retains the preceding 16 API tests.
The CFD matrix checks corrupt or missing metrics at first, middle and last rows.

The complete 32-test API file executes 128 small jobs: 64 rejected inputs and
64 explicit corrected fresh submissions across structural, acoustic, modal,
dynamic, transport, electrostatic, magnetostatic and CFD guards/pairs. Failures
must retain their reason and failed recovery state without decision/export
artifacts or successful downstream progress. Fresh submissions must finish with
complete reports and distinct IDs while the original failures remain unchanged.
The new half specifically covers corrupt aliases and derived frequency samples.

During test development, an override configuration was accidentally assigned to
the operator ID. Admission correctly rejected that malformed test graph. The
fixture was corrected; positive complete-report assertions remain mandatory.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 599985
```

Result: 1,589 total, 1,518 passed, 71 conditional skips, zero failures. New tests
are included in that total, not additional executions to add to it. Optional
environments that skipped are not execution evidence.

From `workers/rust`, the existing native integrity controls also pass:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib workflow_metric_integrity --quiet
```

Result: 22 passed, zero failures. These controls cover native strict selected
metrics and CFD reductions; they are not a new cross-language parity benchmark.

## Remaining boundaries

The API runs use controlled summaries and the actual in-process Router/job
coordinator, not new physical solves or live Agent dispatch. CFD reduction tests
use controlled raw fields. Recovery means explicit fresh submission, not automatic
resume of the same interrupted job, durable restart or exactly-once execution.

Other legacy raw diagnostic reducers still need review for dropped rows and
fabricated zeros; strict decision admission cannot detect a plausible number
fabricated upstream. Thermal decision entrypoints retain their narrower prior
gate. This round does not establish full Rust/Elixir contract equivalence,
numerical accuracy, remote/Linux, installed GUI or performance qualification.

The tensor receives scoped control-plane contract/execution/recovery evidence,
without raising global maturity or claiming release readiness.

Subsequent work: the [electromagnetic follow-up](electromagnetic-diagnostic-integrity-20261012.md)
addresses the legacy static electric/magnetic reducers and native charge-density
source selection. This report retains its original verification scope and counts.
