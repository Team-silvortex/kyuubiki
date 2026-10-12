# Sweep result matching, evaluation and explicit recovery

Date: 2026-10-12. Source line: daji 3.5.0, macOS working-tree overlay at
`69da83e9`. No version bump, commit, packaging or installed-runtime migration.

## Findings and repair

The initial 49 targeted admission tests produced 46 failures against the old
service implementation. This is a regression-case count, not 46 independent
vulnerabilities. Key failures were cross-case positional fallback after an ID
miss, retained stale summaries, successful-looking failed/pending results,
silently omitted metrics, uncaught malformed-config/overflow exceptions and a
fixed penalty that could still select an infeasible candidate at large scales.

Public transform IDs remain unchanged. The service facade now delegates join,
summary and score operations to focused modules sharing identity, status and
completeness checks. Engine/Solver algorithms and Agent authority are unchanged.

- Keyed results are indexed once and matched only by exact case identity. IDs
  must be nonblank, unique and consistent across aliases. Duplicate/conflicting
  IDs, malformed envelopes and mixed keyed/positional collections reject.
  Explicit `summaries: null` cannot fall back to another result collection.
- Legacy positional matching remains available only for an entirely unlabelled,
  equal-length collection. `matching_mode` makes the choice visible. Positional
  data cannot prove its own order; distributed callers should provide case IDs.
- Fresh joins remove prior conventional summaries, prior/custom joined fields
  and errors before attaching new evidence. Failed, pending, error-bearing or
  explicitly nonconverged results become rejected diagnostics, not usable data.
  Declared custom summary fields do not fall back to unrelated fields.
- Non-strict joins report missing, rejected and unmatched IDs with counts and
  `join_complete: false`; strict joins reject. Downstream summary/scoring reject
  incomplete evidence even if a caller supplies a contradictory complete flag.
  Result diagnostics themselves must remain JSON-serializable.
- Summary selection preserves identity, parameters and metadata rather than
  allowing metrics to overwrite them. Malformed/duplicate/reserved field names,
  status errors, null selected aliases and inconsistent counts reject. Automatic
  selection uses the union of summary fields, making missing fields observable.
- Missing/null/mixed numeric values reject by default. Explicit
  `fail_on_missing: false` produces a diagnostic table with `missing_fields` and
  `summary_complete: false`, which cannot proceed to scoring. Signed finite
  statistics and ordinary text remain valid; overflowing sums reject with case
  and field context rather than resetting or publishing invalid values.
- Scoring validates every objective and row identity, nonnegative weights,
  finite values/targets/bounds and consistent lower/upper limits. Zero weights
  remain valid but do not disable hard limits. Arithmetic overflow rejects.
- Feasible rows sort ahead of infeasible rows, then by descending score and
  exact ID for stable ties. Existing weighted formulas and the diagnostic
  infeasible penalty remain, but that penalty no longer decides feasibility
  ordering. All-infeasible studies return `best: null`, `feasible_count: 0` and
  `selection_ready: false`, retaining `scored_rows` for diagnosis. A completed
  evaluation job is not necessarily a selected research winner.

Service error tuples now carry contextual strings rather than the two legacy
bare error tags asserted by the old unit tests. Their rejection assertions now
check the affected case/field or objective option. No physical formula changed.

## Verification

56 new tests: 50 admission/evaluation controls, three direct graph controls,
and three asynchronous API controls. The original 49 admission tests all pass,
with an additional non-JSON diagnostic-envelope control.

Direct graph tests execute 28 controlled runs: seven rejection scenarios and
seven corrected fresh runs under each of `fail` and `skip`. They cover missing,
failed, foreign and duplicate results, missing metrics, invalid objectives and
score overflow. Damaged evidence cannot reach a winner/export. Corrected fresh
runs use the actual received metrics instead of stale zero summaries.

One further direct graph and one API graph consume actual local batch results.
Each constructs a sweep, builds TaskIR, round-trips the batch through JSON and
executes two thermal-shock assessments through `OperatorTaskExecutor`. The test
caller explicitly maps the operator assessments to generic `stress`/`safety`
summary fields and reverses result order before collection. Join, summary, score
and JSON export select `quality_round_0`, preserving seed metadata. Formula
checks use `stress = 70e9 * 1.2e-5 * temperature_delta * 0.7 / 0.67`; the 120
temperature-delta case passes the stress cap while the 160 case does not.

Totals for the new suites: 29 direct graph executions, 16 actual asynchronous
service jobs, and four actual local analytic assessments in two batches.
The API jobs comprise seven rejected/corrected pairs, one actual-result selection
and one all-infeasible diagnostic completion. Successful fresh jobs have new IDs;
earlier failures retain their recovery state and cannot acquire new exports.
The API tests use the in-process HTTP Router and asynchronous coordinator, not
an installed network server. These are not FEM solves or live Rust Agent runs.

A 4,096-case reverse-order control checks one-to-one identity association. Join
uses an ID index rather than scanning the entire result list for every case;
positional lookup uses a tuple only when needed. This is an algorithmic shape
improvement and correctness control, not a measured throughput/RSS benchmark.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 64005
```

Expanded result: 1,999 total, 1,928 passed, 71 conditional skips, zero failures.
Conditional skips are not execution evidence. Formatting and diff-whitespace
checks pass.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib workflow_parameter_sweep --quiet
```

62 existing native reference controls pass unchanged. They are not evidence
that native scoring has acquired the stricter service selection policy.

Project organization, book and documentation inventory checks pass, as do the
tensor structure check and self-test. The tensor still has 13 modules, 11
paradigms, zero structural gaps, four maturity gaps, 20 evidence-grade gaps and
15 P0 gaps; release qualification remains blocked. New files stay within the
800-line source and 2,000-line documentation limits.

## Limits and next work

This is explicit fresh-submission recovery, not automatic durable continuation,
crash/power-loss recovery, installed GUI or remote qualification. Case identity
matching is not authenticated task/attempt provenance; positional input still
depends on caller order. The caller's metric projection in the analytic example
is explicit test code, not a newly implemented generic result-extraction adapter.

Native direct scoring and score-to-quality-candidate mapping still require
equivalent admission and feasibility review. Native join/summary controls informed
this service repair, but full cross-language equivalence is not claimed. The
service deliberately has stricter reserved-field and nonconvergence checks.
This round does not establish physical accuracy, optimization convergence or a
complete unattended research system.

Subsequent same-day work addresses the scoring/mapping gap above in the
[native sweep selection follow-up](native-sweep-selection-recovery-20261012.md).
Its shared scoring fixture is bounded evidence, not full join/summary parity;
the historical test totals and remaining provenance limits in this report remain
unchanged.

Tensor evidence is scoped to `orchestra-control-plane` workflow composition and
validation, in contract/execution/recovery dimensions. Global release gaps must
remain independently evaluated; no native or distributed grade is promoted.
