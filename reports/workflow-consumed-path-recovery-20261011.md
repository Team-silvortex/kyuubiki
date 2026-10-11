# Workflow recovery binds the actual consumed source path

Orchestra now prevents successful solve checkpoints, condition decisions and
native TaskIR receipts from being attached to a different consumed source path,
even when the current final input value is identical. Existing checkpoint
invocation, node, parent/envelope, ownership, dispatch and native TaskIR identity
checks remain in force.

This is a source macOS recovery follow-up to graph input-arity validation.
The base commit is `6d9e1094` (`daji 3.5.6`); package metadata remains `3.5.0`.
There is no release bump, commit, packaging or installed-app acceptance.

## Reproduced failure

A controlled workflow exposes a source-switch ambiguity without modifying its
retained graph, original inputs or parent identity:

1. An upstream typed solve produces a valid but oversized output, so its result
   is deliberately not cached under the existing 512 KiB output bound.
2. A condition has a nonnumeric predicate input and fails under explicit
   skip-on-error. Failed conditions retain no successful branch decision.
3. A first-available join selects a backup input. A downstream solve completes
   with a small retained result; a later solve is held pending.
4. After stopping and restarting the application, the upstream source becomes
   valid. The main route normalizes its output to the exact same downstream
   input value that the backup previously supplied.

Before this change, both node declaration orders completed by reusing the old
downstream solve result under the new route. The existing invocation identity
covered its input value but not the join's actual selected ancestor. Two
unchanged-backup controls passed despite changes to the failed unused source.
The red run had four cases, two expected failures.

After the fix, source switches stop with
`workflow_solve_checkpoint_input_changed` before the downstream solve or held
dependent executes again. The original checkpoint bucket remains unchanged,
the recovery envelope is retained, and no successful artifacts are published.
The two unchanged-backup controls still complete without repeating the retained
downstream solve, preserving selected-source lineage and ordinary terminal
checkpoint pruning.

Source switches are tested with a controlled typed-solve client and an actual
application stop/start in the same BEAM. They are not physical solver results,
an actual BEAM-kill test, remote deployment or installed-app acceptance.

## Recursive binding without snapshots

`WorkflowGraphProvenance` seeds each input artifact's identity with its producer
node and port. Every published artifact receives a short SHA-256 digest over
its node, port and the keys/digests of the artifacts it actually consumes.
Downstream input bindings therefore reflect source switches hidden behind the
same join output key, including through additional local transforms.

- Named-input nodes bind all their consumed sources in execution order.
- First-available joins bind only the selected source. Unused alternative
  changes do not invalidate the same path, and node declaration order does
  not change the digest for the same consumption order.
- Output ports propagate their own source binding, matching existing lineage.
- The graph runner supplies the computed digest to internal condition, solve,
  transform and extract callbacks. Existing callback arities remain supported.
- Engine wiring adds `workflow_input_binding_sha256` to existing checkpoint
  contexts and native TaskIR orchestration context. The computed value replaces
  caller-provided values without changing control-mode or ownership authority.
- Successful reuse still requires all existing exact invocation, claim, lease,
  envelope and dispatch evidence checks. The binding grants no replay authority.

Only one 64-character digest is retained in memory per published artifact. No
ancestor tree, input copies, persistent scheduler snapshot or additional
checkpoint bucket is introduced. The digest contains path identity, not
artifact values; checkpoint invocation digests bind the current exact payload,
while native receipts remain tied to the existing TaskIR identity contract.
The 128-node chain test checks fixed-size per-artifact retention, not benchmark
throughput or total process memory.

The context field participates in existing native TaskIR identity, so equal
payloads with a changed source path are rejected before receipt retrieval or
redispatch. An unchanged bound invocation restores the original selected
receipt without execution or journal mutation. Previously unbound successful
checkpoints/receipts cannot silently become bound evidence; a differing
invocation blocks and requires explicit fresh job submission. No migration
heuristic rewrites old evidence or deletes it to authorize a rerun.

## Verification

Nineteen new Elixir tests cover recursive propagation, all-source vs selected
binding, source ports, unused alternatives, node order, spoofed caller values,
callback binding, fixed-size chain retention, solve/branch checkpoint
binding, native receipts and controlled application restart. The native receipt
tests also reject old unbound evidence before a probe or dispatch, preserving
the original journal bytes.

| Selection | Result |
| --- | --- |
| Expanded SQLite workflow/recovery/dispatch selection | 663 listed, 654 passed, 9 JSON-only skipped, 0 failed |
| Isolated JSON checkpoint/recovery/binding/API selection | 329 passed, 0 failed |
| Complete Rust Engine library suite | 651 listed, 650 passed, 1 prebuilt-plugin test ignored, 0 failed |
| Rust Engine/CLI all-target Clippy | Passed with warnings denied |
| Elixir test-environment compilation | Passed with warnings treated as errors |
| Rust and changed Elixir file formatting | Passed |

These selections overlap and are not additive coverage percentages. The ignored
Rust plugin case requires a prebuilt template dynamic library.

The focused actual-service test
`real_branch_workflow_restart_retains_route_before_native_receipt_recovery`
also passed in 36.99 seconds. It starts real Rust Agents and source Orchestra,
persists SQLite/journal state, loses the actual BEAM after successful diagnosis
and resumes the parent through the Rust Headless SDK. The thermal true route
and electrostatic false route rebuild their first-available join with the same
selected path. Two rounds require exactly six Agent executions: one solve,
one diagnosis and one score per round, without duplicate work on the peer.
Original successful dispatch evidence stays byte-stable; normal completion
prunes private checkpoint payloads.

This is a freshly rerun focused service test, not a rerun of the whole service
suite. The changed-source negative case remains controlled application-restart
evidence; the actual service rounds establish unchanged-route compatibility.

## Remaining limits

This is consumed-path consistency, not authenticated or complete data
provenance. It does not hash every ancestral value or retain the whole execution
history. Checkpoint payload checks and native TaskIR identity remain separate. It is
not general durable per-node continuation, exactly-once execution or a numeric accuracy
claim. Local transforms and joins still rebuild under the original replay
policy. Unknown native outcomes, lost Agent receipts, side-effect restrictions
and changed invocations still block recovery.

The tensor registers verified contract/recovery evidence for Orchestra only.
It does not promote Rust local-graph recovery, independent numerical accuracy,
benchmark, operational or release qualification. Remote Linux, mobile, Windows,
installed-app acceptance and scale behavior are outside this source test scope.

Topology, module/function matrix, extension standard, tensor and tensor
self-test, runtime API surface, document inventory/book and version-line checks
pass. Project organization passes the 800-line source and 2,000-line document
limits with no tracked debt. The tensor still reports zero structural gaps,
four maturity gaps, nineteen evidence-grade gaps and fourteen P0 gaps; Daji
qualification remains blocked.
