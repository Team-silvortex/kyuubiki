# tamamono 1.x Minor Lines

Use this document when you want a durable map for how `tamamono 1.x` should
evolve across many minor releases without turning into an unbounded feature
grab.

This is not a locked release train. It is a guidance map for what each stretch
of the `1.x` line is supposed to improve.

## Why this exists

`tamamono 1.x` is expected to stay on major version `1` unless there is a
special structural reset later.

That means the line needs a stable internal rhythm:

- early minors strengthen trust
- middle minors strengthen operator and workflow maturity
- later minors prepare the handoff to `moxi`

The point is not to promise exact features for `1.7` or `1.13`.
The point is to keep priorities coherent.

## The five long-running tracks

Every `tamamono` minor should move at least one of these tracks forward:

1. `accuracy`
   solver correctness, benchmark coverage, tolerances, and regression trust
2. `bugs and consistency`
   removal of rough edges, mismatched behavior, and user-visible surprises
3. `operator readiness`
   Hub, remote runtimes, provenance, diagnostics, and first-line support paths
4. `workflow maturity`
   study setup, results review, export, and cross-domain workflows
5. `product scope discipline`
   honest support boundaries, verified vs lighter families, and release posture

## Suggested minor groupings

### `1.1` to `1.4`

Primary goal: accuracy groundwork.

Expected emphasis:

- grow the automated accuracy baseline set across verified families
- define per-family tolerance expectations
- distinguish `verified`, `partially verified`, and `lighter` more explicitly
- fix obvious solver/result mismatches before widening scope

Success looks like:

- the core mechanical and thermal baselines are no longer anecdotal
- benchmark failures are treated as release-significant
- users can tell which result paths are the most trustworthy

### `1.5` to `1.8`

Primary goal: operator readiness.

Expected emphasis:

- make Hub viable as a daily runtime shell
- improve remote-target behavior, remote logs, and provenance visibility
- harden first-line troubleshooting paths
- use real non-local machines, not only localhost loops

Success looks like:

- one-machine and remote-node pilots feel repeatable
- runtime watch is useful for real issues, not just demos
- local vs remote vs imported workload provenance stays understandable

### `1.9` to `1.12`

Primary goal: workflow maturity.

Expected emphasis:

- make representative studies feel complete from sample to export
- reduce family-to-family inconsistency in controls, reports, and result views
- strengthen `Thermal -> Thermo-mechanical` bridge workflows
- improve sample quality and starter guidance for non-specialists

Success looks like:

- a new user can choose a domain, open a sample, run, read, and export without
  guessing
- the assistant can guide first-study use without sounding generic
- supported workflows are clearer than unsupported ones

### `1.13` to `1.16`

Primary goal: product UX and operational confidence.

Expected emphasis:

- remove the most persistent friction from real use
- tighten desktop/browser consistency
- improve failure wording for non-authors
- keep automation, import/export, and project flows feeling predictable

Success looks like:

- fewer “I know it works, but I forgot how to get there” moments
- better confidence when using the system alone instead of while developing it
- fewer support answers that depend on source-level knowledge

### `1.17` to `1.20`

Primary goal: pre-`moxi` industrialization.

Expected emphasis:

- close the gap between “capable tool” and “credible industrial baseline”
- decide what absolutely must be true before `moxi`
- freeze or retire low-value experiments that do not strengthen the line
- make release decisions depend more on quality evidence than on new capability

Success looks like:

- `tamamono` ends with clear industrial habits
- the `moxi` line can begin from a hard baseline instead of a cleanup backlog
- the project knows what it trusts, what it supports, and what it still avoids

## What should not define a minor release

By default, a `tamamono` minor should not be considered successful merely
because it adds more study kinds.

New capability is welcome only when it strengthens one of these:

- benchmark coverage
- workflow completeness
- operator usefulness
- product coherence

If a feature does not improve one of those, it should usually wait.

## How to use this map

Use this file as a prioritization lens, not as a rigid release calendar.

When deciding whether work belongs in the current line, ask:

1. which long-running track does this strengthen?
2. which minor grouping does it belong to most naturally?
3. does it improve industrial quality, or only feature count?

If the answer is only “it adds one more thing,” it is probably not the right
default for `tamamono`.

## Related docs

- [current-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/current-line.md)
- [version-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/version-line.md)
- [accuracy-plan.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-plan.md)
- [accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)
