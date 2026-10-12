# Native sweep scoring and quality-selection recovery

Date: 2026-10-12. Source line: daji 3.5.0, macOS working-tree overlay at
`69da83e9`. No version bump, commit, packaging or installed-runtime migration.

## Findings and repair

This follows the [service result-selection repair](sweep-result-selection-recovery-20261012.md)
through the native Rust scoring and score-to-quality mapping path. The initial
33-test native sweep selection had 13 failing tests; this is a regression count,
not 13 independently classified vulnerabilities. Shared fixtures exposed several
additional failing inputs within one test. A later source-metric consistency
control also failed before its fix.

The public operator IDs and Worker SDK registry remain unchanged. The old
results facade now delegates to focused scoring and mapping modules sharing
admission helpers. No Solver algorithm or Agent execution authority changed.

- Scoring rejects malformed objectives, negative/nonfinite weights, invalid
  targets/bounds, duplicate or conflicting IDs, failed/nonconverged rows and
  contradictory incomplete evidence. Arithmetic overflow rejects before JSON
  could turn a nonfinite score into null.
- Feasibility sorts before score, so a fixed numeric penalty cannot promote a
  large infeasible objective above a feasible result. Ties use exact case IDs;
  signed zeros compare equally. All-infeasible evaluations retain diagnostics
  with `best: null`, zero `feasible_count` and `selection_ready: false`.
- Both Rust and Elixir score breakdowns retain per-term feasibility and supplied
  bounds/targets, including zero-weight hard constraints. The mapper preserves
  every known blocker and focuses a blocked term instead of the largest valid
  objective. Weighted formulas and the legacy diagnostic penalty are unchanged.
- Mapping requires explicit unique case identity and boolean feasibility.
  Present counts, selection state, best-row identity/content and breakdowns must
  agree. Present source metrics must match their term values. Weighted term
  formulas and aggregate totals are checked with finite arithmetic; malformed
  options cannot silently select defaults.
- A positive objective cannot become a negative quality cost. Mapping accepts
  an explicit fixed `score_reference`, defaulting to zero; negative or overflowing
  derived costs reject. It never subtracts the current batch winner or clamps a
  damaged score to zero, either of which could falsely satisfy a stop threshold.

Two pre-existing handwritten mapping fixtures had aggregate scores inconsistent
with their term sums. Their aggregates were corrected rather than weakening the
new consistency assertions; the independent aggregate-only fixture is unchanged.

## Fixed-reference mapping contract

`transform.score_parameter_sweep` produces a higher-is-better `objective_score`.
`transform.map_parameter_sweep_scores_to_quality_candidates` produces a
lower-is-better, nonnegative quality cost:

```text
quality_score = score_reference - objective_score
```

Example mapping configuration, with a reference chosen by the study author:

```json
{"quality_domain":"material_sweep","score_reference":20.0}
```

Objective scores 10 and 5 then map to costs 10 and 15. A target cost of 3 is not
met merely because the first candidate leads this batch. Use the same fixed
reference and objective definitions across comparable rounds, and calibrate
downstream thresholds in that cost space. If a score exceeds the reference,
mapping fails rather than silently changing the scale. Changing a reference
requires explicit policy recalibration; historical candidates must not be mixed
as if their scales were identical.

Output includes `quality_score_mapping` with mode `reference_minus_objective`
and the reference, alongside the ordinary candidate contract. The reference is
not automatically propagated as a comparison guard by all downstream consumers.
This is an explicit scale contract, not automatic physical calibration.

Aggregate-only external rows remain supported when they supply an identity,
finite score and explicit feasibility. Without a breakdown the mapper cannot
reconstruct or authenticate that claim. An infeasible aggregate still carries a
synthetic blocker. Missing historical term weights/limits similarly cannot prove
their original formula or the exact failing constraint. Present contradictory
evidence rejects; omitted provenance is not manufactured.

## Verification

18 new Rust tests and 25 new Elixir tests cover the bounded contracts. A shared
JSON fixture contains 25 cases: six valid scoring controls and nineteen invalid
inputs. Both runtimes execute that fixture independently and check the specified
output fields or rejection. This is not an exhaustive equivalence proof or a
live cross-runtime transport test.

Three new native graph tests execute fourteen controlled graphs. Six invalid
graphs and six corrected fresh graphs exercise malformed scoring, malformed
mapping and an uncalibrated maximum under both fail and skip policies. Damaged
nodes cannot reach ranking, next-round expansion or lineage output. Corrected
fresh runs select `material_panel_1`, expand four cases and round-trip their
artifacts through JSON. Two further graphs verify all-infeasible replanning and
fixed-reference maximization without a false stop. One bounded exchange test
also remaps 100 serialized fractional-weight score payloads, including feasible
and infeasible results, without tolerance relaxation.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
```

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 65002
```

Expanded service result: 2,024 total, 1,953 passed, 71 conditional skips, zero
failures. Complete native Engine result: 678 total, 677 passed, one ignored,
zero failures. Skipped/ignored tests are not execution evidence.

Project organization, book, documentation inventory, tensor structure and tensor
self-test checks pass, together with targeted Rust/Elixir formatting and diff
whitespace checks. The tensor retains 13 modules, 11 paradigms, zero structural
gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps; daji qualification
remains blocked. Changed source files stay within 800 lines and documentation
within 2,000 lines. These checks do not turn the scoped controls into global
release acceptance.

## Limits and next work

These are source in-process Engine graphs with controlled result summaries,
not new physical solves, live Agent jobs, installed GUI, remote acceptance,
automatic retries or durable crash/power-loss recovery. Recovery here means an
explicit corrected fresh execution, not continuation of a persisted attempt.
The shared scoring fixture does not qualify the entire Rust/Elixir workflow
surface. Native join/summary nonconvergence and reserved-field alignment, broader
native quality composition/ranking admission and authenticated result provenance
remain separate audits.

Subsequent same-day work addresses the identified native collection/summary
alignment gaps in the [collection follow-up](native-sweep-collection-recovery-20261012.md).
Historical totals above remain unchanged; its shared fixtures do not prove full
cross-runtime equivalence or authenticated provenance.

Tensor evidence is scoped to native workflow composition/validation and the
shared scoring contract. It must not promote distributed/installed maturity or
claim optimization convergence and scientific accuracy. No new persistent
snapshots, binaries or benchmark datasets are retained for this round.
