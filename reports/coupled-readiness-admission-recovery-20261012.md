# Coupled readiness admission and explicit recovery

Date: 2026-10-12. Daji 3.6.0 source working-tree overlay at `0e1ac6cf`.
No version bump, commit, packaging, installation or remote run.

## Failure and repair

Next-round admission previously selected the first present coupled-readiness
field from the ranking row, nested objective or payload. An outer ready report
or null could hide an inner block. False readiness and nonempty blocking,
missing or warning-domain lists did not independently prevent a successful stop.
Even a recognized block could leave `selected_candidate_ready` true.

The new 56-case shared fixture initially failed 44 service cases; native tests
also reproduced masked evidence and false completion. Both runtimes now check
every supplied row/objective report, including lower-ranked rows, and the
selected candidate's payload-level report. Object shape, boolean readiness,
state/recommendation vocabulary, nonblank domain lists and explicit final
convergence markers are validated before publishing a decision.

These locations are aliases for one report, not instructions to merge different
assessments. Absent/null locations cannot hide another supplied report. Multiple
non-null reports must be structurally equal, including metadata; differing
copies fail rather than selecting a preferred source. Callers must reconcile
stale copies or supply one authoritative report. This deliberately conservative
rule does not authenticate that authority.

Within a report, explicit restrictions dominate permissive labels. False
readiness, block state, hold recommendation, blocking domains or required-missing
domains select `replan` and make the selected candidate unready. Warnings select
`continue` with a review hint even when the score meets the target. Missing
reports preserve metric-only shorthand behavior, not proof of physical readiness.
Original reports remain unmodified in the decision and request payload.

Iteration hints use the same effective gate as the decision. The service now
retains affected domains/counts and emits repair strategy/focus in sweep plans;
these survive materialization and expansion into case metadata. Chain tests
exposed the formerly missing service `repair_strategy`. Malformed direct plan
readiness objects return contract errors rather than exceptions from the new
metadata projection.

Numerical formulas, operator IDs, TaskIR and Solver/Engine/Agent ownership are
unchanged. Native admission borrows reports while checking and clones only the
selected output report. This is not a measured memory or speedup claim.

## Verification

- Both runtimes pass 56 shared cases: 30 invalid cases and 26 accepted
  shorthand, repair or review controls.
- Each runtime checks 125 alias placements after JSON exchange, using null,
  ready, warning, block and false-only reports. Consistent copies preserve
  decisions and forwarded evidence; conflicts fail independently of placement.
- Native controls generate three actual coupled-readiness operator reports
  (pass, warning and block), duplicate them and verify next-round admission.
- Sixteen controlled graph executions per runtime cover four invalid inputs
  under fail/skip policies, each followed by a corrected fresh run. Native
  rejection cannot publish requests, plans or cases; service observers see no
  export. Fresh native runs expand four cases; service controls stop at their
  score target without invoking a physical solver.
- Four additional native graphs and four service transform chains retain repair
  or review intent through expansion. Three malformed direct service plan
  inputs return contract errors rather than exceptions.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
cargo clippy --locked --offline -p kyuubiki-engine --lib --no-deps -- -D warnings
```

Complete Engine result: 696 total, 695 passed, one ignored, zero failures.
Engine Clippy passes with warnings denied.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68100
```

Expanded service result: 2,209 total, 2,138 passed, 71 conditional skips, zero
failures. Ignored/skipped tests are not execution evidence.

## Scope and remaining work

This is controlled admission and explicit fresh-run recovery, not new FEM
accuracy, live Agent, installed, remote, power-loss or automatic durable
continuation. The checks do not recompute domain summaries, objectives or
physical optimality. The readiness generator remains native; it is not a local
Elixir transform and no duplicate service implementation is introduced.

Directly authored sweep requests still need end-to-end admission checks for
duplicate action/hint/readiness representations. Repair-focus retention does
not prove identical search heuristics across runtimes. Producer-side retention
of readiness through ranking also needs separate coverage. Tensor maturity and
qualification thresholds remain unchanged; scoped admission does not close
general release gaps.
