# Native quality decision admission and recovery

Date: 2026-10-12. Source macOS working-tree overlay at `69da83e9`.
No version change, commit, packaging, installed-runtime update or remote run.

This follows [native collection recovery](native-sweep-collection-recovery-20261012.md)
through quality composition, candidate ranking and next-round decisions. Invalid
input now rejects before it can be treated as successful research evidence;
incomplete evaluation remains distinct from a valid request to search for repairs.
Public transform IDs, physical formulas and Engine/Solver separation are unchanged.

## Admission repairs

- Composition requires exactly one quality score field. Its readiness, counts,
  grade, contract and diagnostic companions must belong to the same prefix;
  independent suffix lookup can no longer mix different physical domains.
- Present flags, counts and text fields must have their declared types. Blank
  source IDs and invalid known numeric configuration fail explicitly. Missing
  optional fields retain documented defaults; omitted validation counts remain
  non-ready evidence, rather than being confused with malformed present counts.
- Validation cannot declare readiness while required missing-field details are
  nonempty, even if its missing count claims zero. Failure lists and entries are
  checked before their values become decision evidence.
- An explicitly invalid candidate ID no longer receives an invented identity.
  Missing list IDs still receive deterministic names, including for rejected
  non-object candidates, so partial evaluation remains fully accounted for.
- Ranking wrappers and next-round wrappers check final convergence. Next-round
  admission validates every ranking row, rejected IDs and errors, uniqueness
  across both lists, all declared counts and best-candidate claims. It does not
  silently accept malformed tail entries after selecting the first row.
- Selected objective convergence/readiness is respected. Outer blockers are
  merged with inner objective blockers instead of hiding them behind an empty
  outer list. Selected coupled-readiness state/recommendation must be supported
  values, not silently ignored malformed data.

Shared primitive checks and ranking admission live in two focused Rust modules.
Ranking admission retains only the selected row reference plus identity tracking,
not a second vector of all ranked rows. This is a bounded allocation avoidance,
not a measured performance improvement.

## Replan is not automatic rejection

Three behaviors remain separate. Malformed evidence fails the transform and its
dependent graph nodes cannot publish new decisions or cases. A partial ranking
retains rejection diagnostics, requests repair of failed evaluations and disables
case expansion. A valid but blocked quality objective requests `replan`; it can
still generate a focused repair sweep from the explicitly supplied search space.
It must not masquerade as an accepted `stop` decision.

The fixed-reference maximization control now checks both readiness policies:
a cost of 12.2 exceeds the default ready limit of 12 and requires replan; an
explicit study limit of 20 permits continuation toward the separate stop target
of 5. No default threshold was weakened to make the test pass.

## Verification

The three new native shared-contract groups failed before the repair. Both
runtimes now independently run the 49-case JSON fixture: 39 rejection cases and
10 valid or diagnostic controls. Assertions compare selected result fields and
rejection behavior, not every serialized byte or the entire runtime contract.
The Elixir implementation is unchanged; its new fixture suite has 49 passing tests.

Thirty controlled native graph executions in the new recovery module cover six
malformed-input scenarios under fail/skip policies, corrected fresh executions,
partial rankings, inner objective blockers and false objective readiness. Invalid
inputs cannot publish downstream decisions or cases. Partial evaluation pauses;
valid repair plans retain their repair direction; corrected fresh graphs expand
four cases. The twelve corrected fail/skip runs round-trip artifacts through JSON.
The existing fixed-reference graph additionally covers both readiness limits.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
```

Complete native Engine result: 688 total, 687 passed, one ignored, zero failures.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 67002
```

Expanded service result: 2,116 total, 2,045 passed, 71 conditional skips, zero
failures. Ignored and skipped tests are not execution evidence.

Engine Clippy with warnings denied, targeted Rust/Elixir formatting and
diff-whitespace checks pass. Project organization, documentation book/inventory
and tensor structure/self-test checks pass. Source files meet the 800-line cap
and documents the 2,000-line cap. The tensor retains 13 modules, 11 paradigms,
zero structural gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 gaps;
daji qualification remains blocked. Added claims cover native controlled-graph
recovery and shared contract fixtures, not broader maturity.

## Remaining limits

Recovery is an explicit corrected fresh graph, not durable resume, power-loss
recovery, live Agent dispatch or installed/remote acceptance. The graph fixtures
use controlled quality evidence, not new physical solves or accuracy benchmarks.
Final-state markers and candidate identities are consistency checks, not
authenticated task/attempt provenance.

Shared fixtures are not an exhaustive Rust/Elixir equivalence proof. Ranking
order versus actual optimality, nested objective-score consistency, conflicting
coupled-readiness representations and downstream request admission remain
separate audits. This round does not assign new maturity grades or qualify the
whole research workflow as reliable.

The [ranking consistency follow-up](quality-ranking-consistency-recovery-20261012.md)
subsequently checks canonical order, explicit rank positions and row/objective
score copies in both runtimes. Recomputed physical optimality remains outside
that bounded repair.
