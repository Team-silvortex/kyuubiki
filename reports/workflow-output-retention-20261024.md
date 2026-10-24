# Rust Workflow Output Retention

Date: 2026-10-24

Source base: `308eb93c` after the isolated CI repair. Prior selected-input
changes remain in the working tree and are preserved. Package metadata still
reads 3.7.1; the last version-named Git checkpoint is `daji 3.7.3`. This round
does not bump a version, commit, deploy or rebuild the installed application.

## Reproduced Result-Loss Boundary

An Output node publishes each resolved incoming value under its target input
port. Its optional output-port declarations do not change that publication.
Such declarations are allowed and also permit downstream graph connections.
Output-only projection must retain published artifacts of the nodes listed in
`output_nodes`, regardless of cache hints or later consumers.

Previously, the generic retention collector classified all declared output
ports as transient under output-only projection. The separate Output collector
avoided adding requested final artifacts, but could not undo an earlier
transient entry for the same key. A terminal output with an explicit matching
port therefore completed and emitted correct lineage while losing its result.
A later consumer could likewise move a requested output out of the retained
map. This affected successful and explicitly recoverable downstream failures.

Before repair, four of the five new public tests fail with missing declared
artifacts. The unlisted-output positive control passes. These are result-loss
reproductions, not solver accuracy or durable-storage failures.

## Narrow Retention Repair

Output nodes are excluded from the generic declared-port collector in every
projection. Their actual publication keys are classified only by the existing
incoming-edge collector and the graph's requested `output_nodes`.

- All-artifact mode retains Output publications as before, even with an
  ephemeral hint. Other nodes keep their existing cache behavior.
- Output-only mode protects the requested Output publications, including
  values that also feed later nodes. Independent retained copies remain
  necessary when both an earlier output and a later result are requested.
- Unlisted Output publications remain transient in output-only mode. A final
  transient consumer can still move their owned value instead of cloning it.
- No-artifact mode still releases every payload after its last graph use.

No graph format, operator binding, execution order, cache default, numerical
kernel, cancellation rule, JSON budget or persisted receipt format changes.
The fix does not synthesize data for optional ports without publication, retain
all intermediates, or introduce another copy/snapshot framework.

## Regression Coverage

The five new public tests cover:

- Explicit versus implicit terminal output ports under all three projections,
  cached/ephemeral defaults, local output overrides and reversed declarations.
- A requested output feeding a later transform and output, with equal payloads,
  unchanged execution evidence and independent retained mutable values.
- An unlisted intermediate Output, with complete evidence, no extra projected
  payloads and original string-buffer transfer to its final transient consumer.
- Two ports bound to the same source, preserving both values, per-port direct
  lineage and isolation when one returned value is mutated.
- A raw requested output exceeding the generated-output string budget: the
  downstream transform fails under skip policy, its dependent output skips,
  and the already published raw output survives. A fresh healthy run succeeds.

The existing ownership, selected-input, shared named-input, branch recovery and
cancellation targets protect neighboring contracts. No scope here upgrades
an in-process retained value to a durable recovery checkpoint.

## Completed Local Checks

Pinned Rust 1.88.0, locked offline dependencies and the existing macOS AArch64
build cache are used. No new runtime dependency is introduced.

| Scope | Passed | Exclusions |
| --- | ---: | --- |
| Engine library | 712 | 1 pre-existing ignored test |
| Six selected public integration targets | 32 | Not every integration target |
| Total | 744 | Not the full workspace, Solver suite or a scale run |

The selected integrations are `workflow_output_retention`,
`workflow_selected_input`, `workflow_payload_ownership`,
`shared_named_input_contract`, `shared_branch_recovery` and
`workflow_cancellation_boundaries`. These counts do not include the deliberately
failing pre-fix reproductions or add historical report counts.

The new validation profile contains 22 tests overlapping the completed checks:
five new tests, 13 ownership/selected-input controls, one projection unit and
three execution-plan units. Local regression assertions are not a mathematical
proof or a maturity promotion. Engine all-target Clippy with warnings denied
and Rust formatting pass.

All 109 validation-profile definitions validate; only the new 22-test profile
is executed as a profile here. Documentation book/inventory, diff checks and
the 800-line source / 2,000-line document organization gate pass. The coverage
tensor passes its structure and command checks with 13 modules, 11 paradigms,
zero structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Its Daji status remains blocked; no distributed, installed or numerical
qualification is inferred from these local retention controls.

This is bounded in-process Rust workflow output retention and recovery regression evidence, not durable checkpointing, authenticated provenance, distributed recovery, installed acceptance or a scale qualification.
