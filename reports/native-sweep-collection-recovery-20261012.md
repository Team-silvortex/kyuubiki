# Native sweep collection and summary recovery

Date: 2026-10-12. Source macOS working-tree overlay at `69da83e9`.
No version change, commit, packaging, installed-runtime update or remote run.

This follows [native scoring and candidate mapping](native-sweep-selection-recovery-20261012.md)
upstream to result collection and summary reduction. The changes preserve the
public transform IDs, Worker SDK dispatch registry and Engine/Solver separation.

## Findings and repair

The initial four native collection tests all failed. The first shared-fixture
service run had 42 tests with five failures. These are failing controls, not a
count of independently classified vulnerabilities. They expose related boundary
errors rather than new failures in the physical solver algorithms.

- Native collection checked status/error but not the explicit final convergence
  state in either the result envelope or selected summary. Summary reduction
  could then discard the failed state via `fields`, leaving ordinary numeric
  rows that the stricter scorer could no longer distinguish from valid evidence.
- Native summaries checked join completeness but could overwrite an existing
  incomplete-summary marker with a fresh complete flag. Missing-field evidence
  now blocks before reducing selected metrics, even with `fail_on_missing: false`.
- Native metric selection protected only three provenance fields. It now uses
  the same reserved-field policy as scoring: identity aliases, model, parameters,
  metadata, result status/error and prior objective outputs cannot become metrics.
- Both runtimes permitted provenance fields such as `metadata` to be explicitly
  selected as a result-summary container. Join source fields and declared or
  configured summary fields now reject those names before extracting data. A
  caller that needs a metadata-derived metric must explicitly project it into a
  separate result contract; metadata is not silently treated as solver evidence.
- Native fresh joins removed conventional summaries but retained a previously
  declared custom summary. They now validate the prior `joined_summary_field`
  and discard it along with conventional aliases, the new output slot and old
  error. Case identity, parameters and metadata remain intact. Reserved prior
  fields cannot cause provenance to be deleted.

Final convergence uses the existing shared admission helper. It checks explicit
`converged` markers and the contract-defined `stability_result` wrapper before
field filtering. An absent marker remains permitted for metric-only contracts;
absence is not proof of convergence. Trial histories are not scanned recursively,
so a successful final result can retain earlier failed trial records.

Non-strict joins retain rejected-case diagnostics and `join_complete: false`;
strict joins reject. Neither route permits an incomplete summary to reach scoring.
This does not delete historical input artifacts or retry a solver automatically.

The native join also filters replaced fields before cloning the retained case
fields, instead of deep-cloning stale result trees and immediately discarding
them. This removes that unnecessary copy, not the copies required for the new
output or retained metadata. No throughput, RSS or large-scale speedup is claimed
without a separate benchmark.

## Verification

Five new Rust tests and 43 new Elixir tests cover the bounded repair. Both
runtimes independently execute 41 shared JSON fixture cases: 28 rejected inputs,
nine non-strict diagnostic rejections and four valid controls. They check selected
output fields and rejection behavior, not full cross-runtime equivalence or live
transport. Separate controls verify custom-summary cleanup, input preservation,
strict versus diagnostic rejection, and explicitly selected reserved metrics.

Sixteen controlled native graph executions cover result-envelope failure,
selected-summary failure, case-level failure and a reserved summary metric under
both fail and skip policies, each followed by a corrected fresh graph. Invalid
collection cannot publish scoring, ranking, next-round cases or lineage output.
Fresh graphs select `material_panel_1`, continue and expand four cases; artifacts
round-trip through JSON.

Four controlled service graph executions cover provenance-as-summary rejection
under fail/skip and corrected fresh runs. An export observer verifies that bad
collection cannot export a report. Corrected configuration consumes the actual
test summary values rather than the zero values placed in metadata. The harness
fails if collection attempts a solve or a new extraction.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
```

Complete native Engine result: 683 total, 682 passed, one ignored, zero failures.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 66004
```

Targeted shared-fixture and graph selection: 43 tests, zero failures. Expanded
service result: 2,067 total, 1,996 passed, 71 conditional skips, zero failures.
Ignored and skipped tests are not execution evidence.

Project organization, book, documentation inventory, tensor structure/self-test,
targeted Rust/Elixir formatting and diff-whitespace checks pass. The tensor still
has 13 modules, 11 paradigms, zero structural gaps, four maturity gaps, 20
evidence-grade gaps and 15 P0 gaps; daji qualification remains blocked. Changed
source files meet the 800-line cap and documentation the 2,000-line cap.

## Limits and next work

The new graphs use controlled input summaries, not new physical calculations,
actual Agent jobs, asynchronous API jobs, installed GUI or remote acceptance.
Recovery means an explicit corrected fresh execution; this is not a durable
resume, network-loss or crash/power-loss test. Result IDs and convergence flags
are checked for consistency but are not authenticated task/attempt provenance.
Positional collections still depend on caller order. The collector can discard
only conventional aliases and the explicitly declared old/custom output fields,
not guess which arbitrary metadata might be stale.

The 41-case fixture closes the identified collection/summary alignment gaps;
it does not prove complete Rust/Elixir equivalence. Broader native quality
composition/ranking and next-round input admission remain separate audits.
No solver formulas, automatic replay policy, persistent snapshots or release
maturity grades change in this round.

The subsequent [quality-decision admission audit](native-quality-decision-recovery-20261012.md)
addresses identified native composition, ranking and next-round gaps with shared
fixtures and controlled fresh-run graphs; its limits remain explicitly bounded.
