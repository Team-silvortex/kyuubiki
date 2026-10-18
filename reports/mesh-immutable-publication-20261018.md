# Mesh Immutable Record Publication

Date: 2026-10-18. Source metadata: daji 3.6.0. Git base: `3a5ef43f`
(`daji 3.6.5`), with existing working-tree changes. Tests ran on local macOS
native binaries, not installed applications or remote hosts.

## Confirmed Gaps

The store unconditionally removed its pending pathname on a commit error,
including failure of exclusive creation. A preexisting regular pending file or
symlink was therefore deleted even though this writer had never created it.

Final publication used an existence check followed by overwriting rename. A
destination appearing after that check could be replaced, contradicting the
immutable-record contract. This included a dangling destination symlink.

Six focused regressions were added after extracting the existing rename into a
behavior-preserving publication helper. Four failed before the repair: two for
deleting preexisting staging entries, two for replacing late destinations. The
publication tests exercise the exact final-publication primitive rather than
depending on scheduler timing. Normal publication and constructed restart-state
controls already passed.

## Repair

- Only enter error cleanup after successfully creating the pending file with
  exclusive creation. A failed create does not authorize deleting its occupant.
- Sync the prepared file, atomically hard-link it to the final name without
  replacement, unlink its pending name, then sync the directory where supported.
  No copy or overwriting-rename fallback is allowed. Same-filesystem hard-link
  support is now an explicit requirement for this durable execution store.
- Reuse startup cleanup: remove recognized pending names before record reads.
  A final record and leftover pending alias collapse to the single final name;
  pending-only content is never promoted, even if its checksum is valid.
- Expose these constraints in `record_publication`. Existing unknown-outcome,
  admission-blocking and no-replay rules remain. No extra journal, snapshots,
  record format change or engine/operator coupling was introduced.

## Native Recovery Scenarios

`agent_mesh_publication_live` adds two native-process scenarios:

1. Hold a Mesh task, introduce a preexisting receipt staging file, then release
   computation. The failed commit preserves that file, leaves the task unknown,
   and blocks new admissions without affecting a neighbor's real computation.
   After force-kill/restart, startup drops the pending file; duplicate sync/async
   submissions return the original unknown identity without executing it. New
   explicitly identified work can complete again.
2. Construct three exact on-disk publication states while a real Mesh-mode Agent
   exists: admission plus pending hard-link alias, receipt plus pending alias,
   and a checksummed receipt present only under its pending name. Force-kill and
   restart twice per state. Published receipts retain exact results and bytes;
   admission-only and pending-only cases remain unknown, with original identity
   and index entries. Neither duplicate entry point replays them.

The second scenario constructs filesystem states; it does not claim to kill the
process at an instrumented instruction inside the publisher. It complements the
existing kill-after-receipt-sync/before-transport tests, not physical power loss.

## Validation

- CLI unit suite: 295 passed, including 44 execution-store tests.
- Fourteen native suites: 78 passing test executions, comprising 50 scenario
  tests and 28 repeated shared readiness checks. These cover async/synchronous
  execution, restart, panic, response contracts/limits, Task IR cache coherence,
  Mesh cancellation/owner/binding/missing-admission/publication and store contention.
- Desktop runtime direct/Mesh tests: 50 passed. Strict CLI Clippy across all
  targets, formatting and `git diff --check` passed.
- Documentation book/inventory and project-organization audits passed. The tensor
  audit retains zero structural gaps, two maturity gaps, 22 evidence-grade gaps
  and 17 P0 gaps; release readiness remains blocked rather than being inferred
  from local regression counts.

## Limits

Atomic destination no-replace does not make the entire store safe against
arbitrary external concurrent mutation, including replacement of the source
pending pathname or parent directory. Keep the existing operation-boundary and
owner-file checks; do not modify live stores externally. Unsupported filesystems
fail closed rather than quietly weakening publication semantics. Hard-link and
power-loss behavior on Linux, Windows, network filesystems and removable media
was not newly qualified by this local run. Tensor evidence is extended at the
existing local-isolation coordinate without promoting its grade.
