# Direct sweep expansion admission and fresh recovery

Date: 2026-10-12. Daji 3.6.0 working-tree overlay at `0e1ac6cf`, following
next-round, sweep-request and materialized-plan admission. No version bump,
commit, installation, packaging or remote run.

## Failure and repair

Direct `transform.expand_parameter_sweep` calls could bypass the materializer's
quality checks. Blocked readiness without repair intent, warning evidence without
review, failed convergence, incomplete rankings and contradictory repair metadata
were copied into executable cases. Configuration merging and payload precedence
could also discard conflicting metadata declarations. A focused regression
reproduced these paths before the admission hook was added.

Both runtimes now check evidence before case generation and before disabled or
budget-blocked early returns. They reuse their existing quality-plan admission
rules rather than introducing another numerical readiness evaluator.

- Supplied envelopes and bodies must be objects with valid convergence evidence.
  Case limits and nonblank ID prefixes are checked in both supplied configs,
  including an embedded value that a caller override would otherwise hide.
- Quality wrappers and metadata carrying reserved quality fields opt into the
  shared repair/review checks. Enabled execution rejects incomplete, stopped,
  disabled or budget-blocked metadata. An upstream budget block remains a
  zero-case result even when the downstream limit is increased.
- Payload metadata, embedded-config metadata and caller-config metadata must
  agree when quality evidence is present. A declared null cannot erase another
  alias; repeated equal metadata is accepted and preserved without merging.
  Candidate and plan-contract copies must agree when both are non-null.
- Ordinary raw scans keep arbitrary JSON metadata and their existing precedence
  when no reserved quality fields are present. Null optional configs/budgets
  remain supported; malformed values are not treated as missing defaults.
- Invalid direct service input reports a contract error rather than an unsupported
  operator. Canonical disabled wrappers may still omit executable bodies.

The native admission pass borrows the body and configuration: it does not clone
the base model or axes to fabricate another plan. This is not a measured CPU or
memory performance claim.

## Verification

- Both runtimes pass 70 shared cases: 56 invalid cases and 14 accepted controls.
  Controls include valid repairs/reviews, ordinary custom metadata, null optional
  values, disabled wrappers and a billion-sample deferred budget block that emits
  zero cases. The latter is not billion-case execution evidence.
- Each runtime checks 125 metadata alias placements after JSON exchange. Accepted
  cases retain the selected metadata exactly; mismatched aliases and invalid
  shapes reject before case generation.
- Each runtime rejects 18 single-field mutations of actual materializer outputs
  for ready/warning/block states, and expands three unchanged controls with all
  metadata preserved. These use real next-round, planner and materializer calls.
- Each runtime completes 24 controlled graph executions: six rejected inputs under
  fail/skip policies, each followed by a corrected fresh run. No dependent native
  cases or service task-batch exports escape rejection. Fresh native runs expand
  two cases; fresh service runs export two compiled tasks retaining all metadata.
  Solver callbacks are prohibited in these admission tests.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
cargo clippy --locked --offline -p kyuubiki-engine --lib --tests --no-deps -- -D warnings
```

Engine result: 708 total, 707 passed, one ignored, zero failures. Engine Clippy
passes with warnings denied.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68102
```

Expanded service result: 2,435 total, 2,364 passed, 71 conditional skips, zero
failures. The full service run requires local TCP sockets for the test framework;
it was run with permission after sandbox startup was denied. Skips are not proof.

Formatting, diff whitespace, document-book/inventory, tensor self-test/validation
and organization checks pass. Source and documentation caps remain 800 and 2,000
lines. Tensor release qualification remains blocked: four maturity, 20
evidence-grade and 15 P0 gaps are unchanged.

## Scope and remaining work

This is local controlled admission and explicit fresh-run recovery, not physical
accuracy, live Agent execution, installed/remote acceptance, restart/power-loss
recovery or durable continuation. Optional metadata is caller-owned, not signed:
deleting all evidence does not prove research validity.

This round checks supplied quality metadata and budget-block markers, not every
field of the low-level expansion format. Full declared-count/deferred-axis mirror
parity, caller-authored execution batches, producer-side readiness retention
through ranking and objective recomputation remain separate audits. Existing
case-generation limits remain in place; no expanded scale claim is made.
