# Quality ranking consistency and recovery

Date: 2026-10-12. Source macOS working-tree overlay at `69da83e9`.
No version change, commit, packaging, installed-runtime update or remote run.

The [quality-decision admission audit](native-quality-decision-recovery-20261012.md)
checked ranking rows and best-candidate claims but did not check array ordering
or agreement between row scores and embedded objective scores. Both runtimes
could accept a row score of zero with an inner objective score of 20 and return
`stop`. The new shared fixture initially exposed 21 accepted invalid cases in
each runtime; the ten valid/diagnostic controls already passed.

## Contract repairs

Rust Engine and the Elixir service now validate the same canonical order:
declared-ready rows first, then ascending finite score, then exact candidate ID.
Production ranking and next-round admission share the ordering rule within each
runtime. Signed zeros compare equally; identity is not trimmed for tie breaking.
An out-of-order array fails rather than being silently sorted or selecting a
different winner. Matching best-candidate claims cannot legitimize a bad order.

If a row declares `rank`, it must be an unsigned integer equal to its one-based
array position. The field remains optional for shorthand input. A present
`objective.composite_quality_score` must be nonnegative, finite and numerically
equal to the row score after normal numeric parsing. This checks two copies of
the same score, not two independent calculations; no approximate tolerance hides
a mismatch. Integer/float spellings such as 1 and 1.0 remain equivalent.

Every row's optional objective is checked, not just the selected objective.
Malformed objects, nonconverged final markers, invalid readiness flags and
malformed blocker arrays in lower-ranked entries cannot be hidden by a valid
first row. An absent/null objective or an objective without a score still
supports the established shorthand contract. A valid false objective readiness
continues to request repair instead of being interpreted as successful research.

Order checking is one adjacent pass and does not re-sort or clone the entire
ranking. Existing identity tracking and per-runtime normalization remain. This
is an algorithmic bound, not a measured scale, memory or speedup claim. Physical
formulas, public transform IDs and Engine/Solver responsibilities are unchanged.

## Verification

The shared JSON fixture contains 31 cases: 21 rejection cases and ten valid or
diagnostic controls. Both runtimes now pass them. These cover ordering and tails,
rank positions/types, selected and tail objective scores, failed tail objectives,
shorthand input, readiness priority, ties, exact identity, large finite scores,
signed zero, partial evaluation and valid blocked-objective replanning.

Each runtime independently checks three candidate sets under all six input
permutations. Eighteen generated rankings select the expected canonical order
and survive JSON exchange into next-round admission. Eighteen further reordered
rankings rewrite their ordinal and best-candidate claims consistently; only the
three canonical permutations are accepted. This tests ordering rather than
merely relying on stale best-candidate metadata to detect changes.

Sixteen controlled native graph executions and sixteen controlled service graph
executions cover reversed score order, selected score mismatch, tail score
mismatch and duplicate rank positions under fail/skip policies, each followed
by an explicit corrected fresh run. Invalid native inputs cannot publish a
next-round request, plan or expanded cases. Corrected native inputs select `a`,
continue toward the 0.5 target and expand four cases. Service export observers
confirm that invalid inputs cannot publish a decision report; corrected inputs
select `a` and stop at the service fixture's default target of 3. The service
harness fails if it attempts a solve or extraction.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib --quiet
cargo clippy --locked --offline -p kyuubiki-engine --lib --no-deps -- -D warnings
```

Complete native Engine result: 691 total, 690 passed, one ignored, zero failures.
Engine Clippy passes with warnings denied.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68003
```

Expanded service result: 2,149 total, 2,078 passed, 71 conditional skips, zero
failures. Ignored and skipped tests are not execution evidence.

Targeted Rust/Elixir formatting, diff-whitespace, project organization,
documentation book/inventory and tensor structure/self-test checks pass. Source
and documentation remain within the 800/2,000-line limits. The tensor still has
13 modules, 11 paradigms, zero structural gaps, four maturity gaps, 20
evidence-grade gaps and 15 P0 gaps; daji qualification remains blocked. The new
scoped claim records consistency and controlled fresh-run recovery only.

## Limits and next work

Canonical ordering is relative to the supplied declared readiness and scores;
it is not proof that a candidate is physically optimal. This round does not
recompute an objective from its component terms, authenticate input provenance
or reconcile every duplicated readiness representation. Conflicting coupled
readiness at different nesting levels and downstream request admission remain
separate audits.

The fixtures use controlled quality evidence, not new physical solves, actual
Agent jobs, installed/remote acceptance or power-loss recovery. Fresh execution
is explicit, not automatic durable continuation. Shared cases demonstrate bounded
contract agreement, not exhaustive runtime equivalence. No release maturity
grades are raised by this repair.
