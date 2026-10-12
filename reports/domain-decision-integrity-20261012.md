# Cross-domain decision admission and explicit recovery

Date: 2026-10-12. Current line: daji 3.5.0. Source macOS service tests.

## Confirmed defects

The thermal catalog follow-up prompted an audit of structural, acoustic, modal,
dynamic, transport, electrostatic, magnetostatic and CFD decisions. Their Elixir
paths could silently drop malformed rules or unresolved metrics, report incomplete
guard checks, or rank candidates using fewer criteria than requested. Explicit
nonconvergence markers were not admitted consistently with native Rust guards.

Invalid comparison/severity/goal/weight options could become defaults. Labels
could collide after normalization and overwrite scores, or use the reserved tie
label. Invalid canonical metrics could fall through to a favorable alias.
Extreme numeric differences or score sums could raise arithmetic exceptions.

Before the fix, a seed-0 run stopped at three reproduced failures: missing guard
evidence was downgraded to a warning, an invalid rule disappeared, and an explicit
nonconvergence marker was ignored. These are reliability/contract defects, not
claims of remotely exploitable security vulnerabilities.

## Change

`WorkflowDecisionChecks` admits the eight guard and eight pair entrypoints before
their existing domain-specific output rendering. It reuses reporting checks for
objects, finite numbers and convergence, rather than implementing Engine physics.

- Every declared rule and criterion must be an object and resolve required metrics.
- Explicit invalid values, conflicting threshold/value aliases and unknown choices
  reject with an operator, rule/criterion index and field path.
- Convergence markers are checked on the input envelope, both candidates and their
  `stability_result` objects. Absent markers remain allowed by the existing contract.
- Zero remains valid evidence. Defaults apply only when an option is absent.
- Text choices are trimmed before evaluation. Labels must remain distinct under
  each existing domain normalizer; reserved tie labels are rejected.
- A present invalid canonical metric cannot be replaced by a numeric alias.
- Unrepresentable numbers, non-finite differences and score arithmetic fail as
  contract errors instead of escaping as arithmetic exceptions.

Successful output shapes and existing domain summaries remain unchanged. Invalid
decision inputs now return contextual error strings rather than a partial success
or some former coarse error atoms. Resolved metrics are reused by output rendering
instead of rescanning derived collections. No installed runtime, database or app
was changed.

## Verification

The new matrix contains 83 operator-level tests and 16 asynchronous API tests.
API tests execute 64 small jobs: 32 rejected evidence cases and 32 explicit fresh
submissions. They assert the exact rejection path, no decision/report publication,
retained failed recovery state, successful complete reports after input correction,
different job identities and an unchanged old failure record.

The API fixtures use the explicit single `payload` envelope port and the registered
`export.summary_json` operator. During test development, incorrect wrapper binding
and an unregistered export ID were corrected in the fixtures, not production.
Positive recovery assertions prevent a malformed test graph from falsely proving
safe rejection.

Expanded verification from `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 599985
```

Result: 1,537 total, 1,466 passed, 71 conditional skips, zero failures. The selection
includes the 99 new tests; do not add those counts together. Existing thermal,
reporting, native-adapter, checkpoint, library and disposable-storage controls pass
within this selection. Skipped optional environments are not execution evidence.

## Limits and next audit

These are controlled Elixir decision inputs and in-process Router/job execution,
not newly calculated physical solutions, live Rust Agent execution, remote Linux,
installed GUI testing, performance qualification or numerical accuracy evidence.
Recovery here means explicit fresh submission, not automatic resume of the same job.

Legacy metric alias/sample derivation remains best-effort beyond the explicit
canonical-value check; malformed earlier aliases and partial derived collections
need a separate checked-resolution audit. Legacy raw diagnostic reducers also need
review for fabricated zero summaries. A guard cannot infer that a numeric summary
was fabricated upstream. Thermal decisions retain the prior round's narrower gate;
this round does not claim complete thermal/native-contract equivalence.

The tensor receives scoped contract/execution/recovery evidence for the control
plane, without raising global maturity or claiming release qualification.

Subsequent work: the [checked metric and CFD follow-up](checked-metric-cfd-recovery-20261012.md)
addresses the alias/sample gap and the legacy CFD reducer described above. This
report retains its original verification counts and scope; other raw reducers
and thermal admission remain separately scoped.
