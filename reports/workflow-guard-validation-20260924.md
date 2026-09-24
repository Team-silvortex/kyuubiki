# Workflow Guard and Pair Validation (2026-09-24)

## Scope and Reproduction

Follow-up to [result admission](workflow-result-admission-20260924.md), covering
the nine built-in domain guards and nine paired benchmark transforms. No solver
equation, engine scheduling policy or installed runtime was changed.

This is bounded local guard and pair-comparison validation, not complete optimization or industrial physics qualification.

The pre-fix focused regression produced **17 failures and 3 passing controls**.
It reproduced these incorrect successful outcomes:

- Missing/malformed guard rules were filtered away, while `guard_checked_rule_count`
  still counted them and could report `guard_passed=true`.
- A missing comparison metric removed its criterion. Other criteria could then
  select a winner using an incomplete objective.
- Invalid comparisons, severity, goals or nonpositive weights silently became
  defaults, potentially changing the user's intended decision rule.
- A corrupt explicit metric could fall back to a healthy alias.
- Finite inputs could produce infinite derived metrics, deltas or accumulated
  scores, later serialized as successful JSON `null` values.
- Equal candidate labels overwrote keyed output values; a candidate named `tie`
  could be mistaken for the reserved tie verdict.

## Contract

`workflow_guard_contract.rs` validates rule/criterion inputs independently of
the physics kernels. The shared transform implementation uses these validated
values and retains the previous result-admission check.

- Every configured rule must have a nonblank field, a finite threshold (or legacy
  `value` alias), a supported comparison and a supported severity. If both
  threshold aliases are supplied, they must be finite and equal.
- Every configured pair criterion requires finite resolved metrics on both sides.
  Common `field` and explicit `left_field`/`right_field` remain supported. Present
  invalid fields cannot fall back to another configured field.
- Omitted options keep their previous defaults: `gte`, `warn`, `min`, weight 1,
  and candidate labels `left`/`right`. Present null, wrong-typed, blank or unknown
  options are rejected instead of reinterpreted.
- Weights must be finite and positive. To remove a criterion, remove it from the
  configuration rather than using zero as an implicit disabling mechanism.
- Normal metric aliases remain supported when the requested canonical field is
  absent. A present invalid canonical field is an error even if an alias is valid.
- Resolved metrics, pair deltas and score accumulations must be finite. Positive
  finite score totals also keep the final score margin representable.
- Candidate labels are trimmed, must be nonblank and distinct, and cannot use
  the reserved `tie` sentinel, case-insensitively.

Invalid inputs produce an error identifying the operator, rule/criterion index
and offending field. A physical threshold violation remains a normal `warn` or
`block` result; malformed evidence is not confused with a physical failure.
Output schemas are unchanged for valid requests.

Pair totals and win counts now accumulate from typed scores in one pass rather
than repeatedly reading and filtering serialized JSON entries. This is not a
measured performance speedup claim. No additional result snapshots are retained.

## Workflow Recovery

Public graph tests cover missing guard/pair metrics, malformed comparison/goal,
delta overflow, total-score overflow and colliding labels:

1. Default execution fails at the assessment node with the original cause.
2. Explicit `on_error: "skip"` preserves raw inputs and independent work, but
   marks the assessment failed and skips its dependent decision output.
3. Corrected-input replay completes and reports the full rule/criterion count.

The raw-output branch here preserves assessment inputs for repair, not a newly
introduced checkpoint or durable solver recovery format.

## Verification

- **20 new consumer tests passed**, including table-driven checks through all
  18 domain guard/pair registrations, valid defaults, aliases, all five comparison
  boundaries, malformed configuration, missing metrics and overflow.
- **3 new public workflow tests passed**, exercising the failure/recovery/replay
  paths above. The previous three result-admission workflow tests also passed.
- Protocol, solver, engine and Rust headless SDK library regression: **1,222
  passed, 7 pre-existing ignored**, no failures.
- The retained native component profile: **44 passed** across three commands
  and four suites, including the previous admission tests.
- Optimized release workflow regression: **6 passed**, covering the three new
  recovery tests and three previous result-admission tests.
- Engine all-target Clippy with warnings denied, targeted formatting, diff,
  operator-profile, coverage-tensor, organization and document-inventory checks
  passed. No source exceeds 800 lines or document exceeds 2,000 lines.

These totals overlap; the round adds 23 unique tests, not the sum of all runs.
Reproduce the scoped native profile:

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-guard-validation --out tmp/workflow-guard-validation.json
```

The profile includes the previous admission regression. Generated reports and
machine-local logs are not committed release artifacts.

## Limits

- Alias/derived metric internals are not comprehensively audited here. A finite
  reduced metric is not proof that every underlying sample was complete or valid.
- Other reducers, quality-score target/weight configuration, external operators
  and arbitrary JSON transforms have separate validation boundaries.
- This is not arbitrary-precision arithmetic, physical accuracy qualification,
  an installed-app acceptance run or a distributed/server test.
- The tensor receives scoped local evidence only; overall qualification remains
  blocked by its broader evidence requirements. No version bump, commit, push,
  app rebuild or installation was requested or performed in this round.
