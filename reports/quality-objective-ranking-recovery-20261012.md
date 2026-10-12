# Quality objective, ranking and next-round recovery

Date: 2026-10-12. Source line: daji 3.5.0, macOS working-tree overlay at
`69da83e9`. No version bump, commit, packaging or installed-runtime migration.

## Findings and bounded repair

Following the domain-quality admission work, 57 new targeted tests initially
failed against the old service implementation. They exposed silent filtering,
permissive configuration defaults, uncaught missing-score/overflow exceptions,
lost blocking evidence and premature next-round stopping. This is a regression
case count, not 57 independent vulnerabilities.

The public `WorkflowQualityObjectiveRuntime` entrypoints remain unchanged.
Composition, ranking and next-round admission now live in focused modules;
parameter-sweep expansion and executable TaskIR batch construction are unchanged.
No physical algorithms or Engine/Agent execution authority moved into the service.

- Composition requires nonempty named object summaries, numeric nonnegative
  scores, valid counts/flags, and explicit convergence where present. Invalid
  summaries cannot disappear beside valid ones. Shorthand candidate metadata
  remains envelope data, not another physical domain.
- Each ordinary summary has exactly one quality-score family. Its companion
  fields come from that same prefix; arbitrary suffix matches cannot mix domains.
  Present bad values reject rather than becoming omitted defaults. All declared
  weights are checked, including unused ones, and zero remains a valid weight.
- Weighted scores, penalties, total scores and u64 counters have checked
  arithmetic. Dominant terms, watch counts and blocking details survive composition.
  Blocking evidence overrides declared readiness even when its numeric cost is zero.
- Tolerance-validation reports participate as quality evidence. Passing requires
  nonempty checked evidence and consistent success counters; incomplete reports
  remain blocking. Failure fields propagate into actionable next-round hints.
- Ranking preserves rejected candidate IDs and reasons, original list positions,
  input/accepted/rejected counts and `ranking_complete`. Invalid collection shape,
  blank/duplicate IDs or global configuration fail the operation. Failed candidate
  evaluations produce an explicit partial ranking if any valid candidates remain;
  an all-rejected collection returns an actionable error.
- Ready/score/ID tie ordering remains deterministic, with candidate metadata
  retained. `best_candidate_ready` describes that individual candidate, not the
  completeness of the study. A completed service job can return a partial ranking.
- Next-round admission checks ranking entries, counts, rejection details, duplicate
  identities and redundant best-candidate fields. Partial rankings always replan,
  even if the surviving best score meets the target. Unready low scores cannot
  stop. Selected objective blockers/nonconvergence cannot be hidden by outer flags;
  coupled warnings require review and coupled blocks require repair.

Default thresholds and normal weighted-score formulas are retained. Service
checks deliberately reject ambiguous multi-score summaries and malformed present
flags/counts rather than reproducing permissive native helper fallbacks. This is
not a claim of exhaustive cross-language equivalence.

## Verification

76 new tests: 64 composition/ranking/next-round admission controls, eight direct
graph recovery controls, and four public asynchronous API controls.

Direct graphs perform 16 controlled executions across both `fail` and `skip`:
invalid composition, all-rejected ranking and contradictory next-round evidence
cannot reach export; corrected fresh runs succeed. Partial rankings export an
explicit `replan`, then corrected fresh runs export a complete decision.

The API suite executes eight actual asynchronous service jobs. Rejected jobs
retain failure records without downstream artifacts. New successful jobs have
different identities and cannot rewrite earlier failures. Correcting a partial
ranking likewise does not rewrite the retained earlier `replan` artifact. Inputs
are controlled summaries; these are not live physical solves or Agent tests.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 62004
```

Final expanded result: 1,840 total, 1,769 passed, 71 conditional skips, zero
failures. Conditional skips do not count as execution evidence.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib workflow_quality --quiet
```

53 existing native tests pass unchanged, including partial-ranking controls.
They are reference regressions, not new native recovery qualification.

## Limits and next work

This is control-plane admission and explicit fresh-submission recovery, not
automatic durable continuation, installed GUI acceptance, distributed recovery,
physical accuracy, optimization convergence or exhaustive ranking provenance.
No persistent snapshot or report-backup mechanism was introduced.

Parameter-sweep planning, expansion and execution-batch admission remain separate
audit work. Their permissive input filtering/defaulting and invalid-or-empty
case handling must not be inferred safe from these ranking tests. A `stop` here
means the configured quality policy is satisfied, not scientific certification.

Tensor evidence is scoped to `orchestra-control-plane` workflow composition and
validation, in contract/execution/recovery dimensions. Global maturity, evidence
grade and P0 gaps remain independently evaluated; no release claim is raised.

Subsequent follow-up: [sweep admission and executable batch recovery](quality-sweep-batch-recovery-20261012.md)
addresses that pre-execution boundary with strict cases/configuration, JSON
exchange and actual local batch execution. The scope statements above describe
this earlier round; result joining and sweep-result evaluation remain separate.
