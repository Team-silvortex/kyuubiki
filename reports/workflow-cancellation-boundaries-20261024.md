# Workflow Cancellation Boundaries

Date: 2026-10-24

Source base: `70340d07`, retaining the uncommitted numerical and workflow ownership
follow-ups. Package metadata and the installed runtime remain 3.7.1. No version
bump, commit, deployment or desktop rebuild is included. No operator, Agent,
TaskIR, result-payload or numerical checkpoint identifier is changed.

## Reproduced Inner Execution Gap

The enclosing Solver control scope already rejects cancelled success at its
return boundary. This follow-up does not claim that a normally wrapped cancelled
call previously published a successful result to its caller.

The inner Engine runner nevertheless had no workflow-level cancellation polls.
An observed diagnostic cancellation under `on_error: skip` could be recorded as
an ordinary recoverable node error. The runner could then execute later nodes,
return an inner partial-success graph, or return a subsequent unrelated missing
input error before the outer scope replaced it with cancellation. A pure
forwarding graph could also complete inside a scope whose token was requested
after that scope began. A cancelled final recoverable node could fall through
to graph completion without another numerical safe point.

Four of five new public regressions fail before repair. The ordinary-error
control already passes. They capture the inner result separately from the
outer scope verdict, so outer protection cannot conceal continued inner work.
The queued-node fixture deliberately omits an independent input: before repair
the inner runner reaches that later missing-input error; after repair it stops
at cancellation. A fresh run without cancellation still reaches the original
missing-input error, proving that this fixture was not simply made healthy.

## Reuse The Existing Control Scope

The existing `solver_control::check_cancellation` poll is made public with
documentation. It observes this thread's active control scopes without adding a
numerical checkpoint, changing the last observed stage, calling progress
observers, introducing another token or creating a scheduler-specific stage.
It remains a no-op outside an active scope. Parent interruption stays sticky;
other threads need their own explicitly installed scopes.

The Engine polls at workflow entry, before each planned node, after successful
node work before completion bookkeeping, and before final graph completion.
A skipped cancellation therefore cannot enter the next queued node or return
a completed inner graph, including when the failed node is last. All, output-
only and no-artifact projections obey the same admission boundary.

This is state-based polling, not parsing error text. Ordinary errors still use
their configured fail/skip behavior. A fail-fast ordinary error returns before
the post-success poll, preserving the existing rule that a later unobserved
cancellation request must not relabel that error. The outer scope guard remains
in place as another boundary; it is not removed by this repair.

## Deliberate Existing-Test Strengthening

The older five-domain diagnostic graph regression inspected an inner partial
trace solely to confirm that the outer execution scope would reject it. Its
historical report retains that scope. The live regression now requires the
inner runner itself to return cancellation under skip policy.

To preserve its independent-work checks rather than deleting them, the test
separately supplies an ordinary malformed diagnostic payload in a fresh scope.
That non-cancelled graph still records the failed diagnostic, skips dependent
quality/objective/decision nodes, and retains raw and independent evidence.
Fresh healthy domain graphs still produce their complete decisions. This
distinguishes whole-execution cancellation from recoverable numerical or input
failure instead of weakening either assertion.
The older live evidence bundle now anchors the explicit inner-completion denial
instead of its removed outer-scope-only comment; its historical report is not
rewritten as a new run.

## New Controls And Scope

Seven new test functions cover two Solver control units and five public Engine
integrations:

- Top-level and nested skip policy cannot return inner success after an observed
  64-record diagnostic cancellation, under all three artifact projections.
- A cancelled graph cannot enter the queued missing-input fixture. Without
  cancellation the exact original missing-input error is retained.
- A final cancelled recoverable node with no output ports cannot complete the
  graph. A requested token stops a pure forwarding graph before numerical work,
  without inventing a last numerical checkpoint.
- An ordinary bad-data node whose id includes `cancelled` still follows skip
  policy and retains its independent output. A late unobserved request after a
  fail-fast error does not replace that original pressure diagnostic.
- Polls leave the last real checkpoint and observer count unchanged, remain
  sticky after interruption, observe parent cancellation, and do not install a
  cancellation scope on an unrelated thread. Fresh controls remain independent.
- The existing eight-test cross-domain target still covers five diagnostic/
  quality chains, CFD record boundaries, thermo component fallback and an
  actual one-quad Stokes result. Its synthetic record fixtures are not new
  physical qualification or scale evidence.

Preflight, JSON budget walks, serialization and already-running opaque callbacks
are not made preemptible by these boundary polls. An external request can only
be noticed at the next poll; no wall-clock latency promise is made. No callback
side effects are rolled back, no durable journal/checkpoint is added, and no
distributed cancellation or remote Agent execution is qualified here. The
Engine continues to use the caller's existing same-thread scope.

## Completed Checks

Pinned Rust 1.88.0 and locked offline dependencies reuse the macOS AArch64 cache:

| Completed scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 926 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 712 | 1 ignored |
| Selected workflow, harmonic, domain-integrity and dynamic integrations | 153 | Not every integration target |
| Total | 1,791 | Not a full workspace or all-physics run |

The dedicated profile executes 12 Solver control units, five new boundary
integrations and eight existing diagnostic integrations: 25 tests overlapping
the table. Only seven functions are newly added; the larger total also includes
older integrity controls added to this round's selection. Existing numerical,
payload-ownership, 1,024-node workflow and ordinary branch-recovery controls pass.
All-target Solver/Engine Clippy with warnings denied passes.

Native catalog inspection validates 107 profiles without executing them all.
The new runtime profile uses a separate shard rather than overflowing the
existing workflow profile file's 800-line limit. Source/config and document
limits remain 800 and 2,000 lines. Earlier source reports keep their original
scope and their counts are not additive to this round.
Book, documentation inventory, organization, formatting and diff checks pass.
The tensor check passes after updating the live inner-completion denial anchor:
13 modules and 11 paradigms, zero structural gaps, two maturity gaps, 23
evidence-grade gaps and 18 P0 gaps remain. Daji status is still blocked; local
contract/recovery evidence does not promote distributed or release qualification.

This is bounded same-thread cooperative workflow cancellation admission, not preemption of opaque callbacks, rollback of operator side effects, a wall-clock latency bound, distributed or installed cancellation qualification, or durable restart.
