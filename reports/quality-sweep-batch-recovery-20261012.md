# Quality sweep admission and executable batch recovery

Date: 2026-10-12. Source line: daji 3.5.0, macOS working-tree overlay at
`69da83e9`. No version bump, commit, packaging or installed-runtime migration.

## Findings and bounded repair

The first 41 targeted controls failed against the old planning/expansion/batch
implementation. They exposed filtered bad axes/cases, permissive defaults,
ambiguous assignments, lost metadata and successful empty/partial batches.
This is a regression-case count, not 41 independent vulnerabilities. A subsequent
JSON exchange control found an additional concrete failure: generated batch
routing hints contained Elixir keyword tuples that Jason could not encode.

Existing public transform entrypoints delegate to focused planning, expansion
and batch modules with shared count/path/identity checks. Engine/Agent execution
authority and physical algorithms have not moved into the service.

- Planning validates every declared search axis before explicit axis selection.
  Malformed present counts, values and configuration cannot become defaults.
  Incomplete ranking or rejected candidates disable scanning pending repair.
  An explicit axis cap remains visible with a follow-up recommendation.
- Checked case products precede range allocation. Over-budget range samples are
  deferred with their counts, not allocated and then discarded. Zero budgets
  block execution; fractional counts and u64 product overflow reject. Stable
  interpolation handles large opposite-sign finite endpoints.
- Materialization recomputes estimates and preserves upstream budget blocks.
  Increasing downstream limits cannot turn a blocked plan into executable work.
  Stopped exploration stays disabled; diagnostic-only empty results cannot issue
  an executable batch.
- Expansion resolves typed object/array paths against the base before generation.
  Empty path segments, signed/out-of-range indexes, overlapping targets, array
  index aliases and duplicate parameter labels reject instead of overwriting.
  Numeric object keys remain distinct. Null array leaves and structured JSON
  values are assignable, metadata survives, and the last axis varies fastest.
- Mixed-radix expansion builds each final model directly rather than retaining
  intermediate Cartesian model lists. This is an allocation-shape improvement,
  not a measured latency/RSS benchmark or a total-memory guarantee.
- Batch construction validates the complete nonempty case collection, identity
  aliases, declared counts, selected payload objects and per-case configuration
  before building TaskIR entries. Invalid routing lists/flags/configuration and
  disabled or blocked inputs reject; no silent partial-batch success remains in
  this constructor. Existing TaskIR construction and verification remain the
  authority for executable tasks.
- Exported `agent_rpc.routing_opts` is now a JSON object with string keys. The
  internal keyword-list routing API is unchanged. The batch schema accepts that
  object and the historical JSON pair-array representation; verified TaskIR,
  not these descriptive hints, remains the executor's routing authority.

No persistent snapshots, report backups or large generated models were added.

## Verification

56 new tests: 33 sweep admission controls, 17 batch admission/exchange controls,
four direct graph recovery controls, and two public asynchronous API controls.

The direct suite performs 14 controlled graph executions across `fail` and
`skip`: four invalid-stage configurations, one corrected fresh execution and
two blocked scans per policy. Invalid stages cannot publish a batch/export;
the successful chain runs input, plan, materialize, expand, batch, export and
output. Exported JSON is decoded, verified and passed to the batch executor.

The API suite performs three actual asynchronous service jobs: rejected search,
corrected fresh search and an incomplete-ranking block. The successful exported
batch reaches both `/api/v1/operator-tasks/prepare-batch` and
`/api/v1/operator-tasks/execute-batch` through the Router. The old failed job
retains its identity, failed recovery record and absence of executable artifacts.

Three executed batches produce six actual local material thermal-shock
assessments. Temperatures of 120 and 160 are checked independently against
`stress = 70e9 * 1.2e-5 * temperature_delta * 0.7 / 0.67` and
`safety_factor = 320e6 / stress`. These are analytic material assessments through
real local execution, not FEM solves, live Rust Agent dispatch or experimental
material validation. Public API coverage here uses the in-process HTTP Router,
not an installed network server.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 63006
```

Expanded result: 1,943 total, 1,872 passed, 71 conditional skips, zero failures.
Compared with the preceding objective/ranking round, this selection also includes
47 pre-existing operator-task tests; the increased total is not all new coverage.
Conditional skips do not count as execution evidence.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib workflow_quality_sweep --quiet
cargo test --locked --offline -p kyuubiki-engine --lib workflow_parameter_sweep --quiet
```

13 existing quality-sweep and 62 parameter-sweep native controls pass unchanged.
The native TaskIR contract check passes five examples and its self-test passes.
These are reference regressions, not new native execution/recovery qualification.

Project organization, book and documentation inventory checks pass. Formatting
and diff-whitespace checks pass. The tensor structure check and self-test pass
with 13 modules, 11 paradigms and no structural gaps; release qualification stays
blocked with four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps. Source
and documentation files remain within the 800/2,000-line organization limits.

## Limits and next work

Recovery requires explicit fresh submission, not automatic durable continuation.
No installed GUI, remote host, distributed execution or crash/restart recovery was
qualified in this round. A caller can still explicitly admit a large case budget;
the operation materializes admitted cases rather than providing global RSS limits
or streaming backpressure. Full native focus-domain/repair-strategy equivalence
is not established by these service tests.

Result joining, sweep summarization and sweep scoring remain separate audit work.
Their identity matching, missing-result handling and status admission must not be
inferred safe from the pre-execution controls above. This round does not certify
optimization convergence, physical accuracy or the entire research loop.

Tensor evidence is limited to `orchestra-control-plane` workflow composition
and validation in contract/execution/recovery dimensions. Global maturity,
evidence-grade and P0 gaps remain independently evaluated.

Subsequent follow-up: [result matching and selection recovery](sweep-result-selection-recovery-20261012.md)
addresses the service-side join, summary and score boundary with explicit missing
evidence and hard-feasibility selection. The preceding limits describe this
earlier round; native direct scoring/mapping alignment remains separate work.
